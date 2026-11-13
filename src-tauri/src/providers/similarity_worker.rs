//! Background artist-similarity maintenance: rebuilds per-user co-occurrence
//! and slowly fills Last.fm `artist.getSimilar` edges for known artists.
//! Yields to playback the same way the metadata gap worker does.

use super::lastfm::{LastFmClient, LastFmError};
use crate::core::{AppResult, Event, EventBus};
use crate::recommendations::cooccurrence;
use crate::recommendations::signals::artist_credits;
use sqlx::SqlitePool;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Notify;

pub const SETTING_KEY: &str = "enable_lastfm_similarity";
const OK_TTL_SECS: i64 = 30 * 86_400;
const ERROR_RETRY_SECS: i64 = 86_400;
const COOC_MIN_INTERVAL_SECS: i64 = 6 * 3_600;
const REQUEST_SPACING: Duration = Duration::from_millis(1_200);

pub struct SimilarityService {
    pool: SqlitePool,
    bus: Arc<EventBus>,
    client: Option<LastFmClient>,
    cooc_dirty: AtomicBool,
    last_cooc: std::sync::Mutex<Option<i64>>,
    wake: Notify,
}

impl SimilarityService {
    pub fn new(pool: SqlitePool, bus: Arc<EventBus>) -> Self {
        Self::with_client(pool, bus, LastFmClient::from_builtin_key())
    }

    pub fn with_client(pool: SqlitePool, bus: Arc<EventBus>, client: Option<LastFmClient>) -> Self {
        Self {
            pool,
            bus,
            client,
            cooc_dirty: AtomicBool::new(true),
            last_cooc: std::sync::Mutex::new(None),
            wake: Notify::new(),
        }
    }

    async fn enabled(&self) -> bool {
        sqlx::query_scalar::<_, String>("SELECT value FROM application_settings WHERE key = ?")
            .bind(SETTING_KEY)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(true)
    }

    pub async fn run(self: Arc<Self>) {
        let mut events = self.bus.subscribe();
        // Let startup (scan, restore, first render) settle before any work.
        let mut busy_until = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            let work = async {
                tokio::time::sleep_until(busy_until).await;
                self.tick().await
            };
            tokio::select! {
                result = work => match result {
                    Ok(true) => { tokio::time::sleep(REQUEST_SPACING).await; continue; }
                    Ok(false) => (),
                    Err(e) => tracing::warn!(error = %e, "Artist similarity worker will retry"),
                },
                event = relevant_event(&mut events) => {
                    match event {
                        Ok(Event::PlaybackBuffering { .. } | Event::OnlinePlaybackRequested { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(20),
                        Ok(Event::PlaybackStarted { .. } | Event::PlaybackError { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(2),
                        Ok(Event::TasteProfileChanged { .. }) => self.cooc_dirty.store(true, Ordering::Release),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => (),
                    }
                    continue;
                }
                _ = self.wake.notified() => continue,
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(600)) => (),
                _ = self.wake.notified() => (),
                event = relevant_event(&mut events) => match event {
                    Ok(Event::PlaybackBuffering { .. } | Event::OnlinePlaybackRequested { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(20),
                    Ok(Event::PlaybackStarted { .. } | Event::PlaybackError { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(2),
                    Ok(Event::TasteProfileChanged { .. }) => self.cooc_dirty.store(true, Ordering::Release),
                    Err(_) if self.pool.is_closed() => break,
                    _ => (),
                },
            }
        }
    }

    pub fn wake(&self) {
        self.wake.notify_one();
    }

    /// One unit of work. `Ok(true)` means more work is likely pending.
    pub async fn tick(&self) -> AppResult<bool> {
        let now = chrono::Utc::now().timestamp();
        let cooc_due = self.cooc_dirty.load(Ordering::Acquire)
            && self
                .last_cooc
                .lock()
                .map(|l| l.map_or(true, |t| now - t >= COOC_MIN_INTERVAL_SECS))
                .unwrap_or(true);
        if cooc_due {
            self.rebuild_cooccurrence().await?;
            self.cooc_dirty.store(false, Ordering::Release);
            if let Ok(mut last) = self.last_cooc.lock() {
                *last = Some(now);
            }
            return Ok(true);
        }
        if self.client.is_none() || !self.enabled().await {
            return Ok(false);
        }
        let Some(artist) = self.next_artist(now).await? else {
            return Ok(false);
        };
        self.fetch_lastfm(&artist).await?;
        Ok(true)
    }

    pub async fn rebuild_cooccurrence(&self) -> AppResult<usize> {
        let users: Vec<String> =
            sqlx::query_scalar("SELECT DISTINCT user_id FROM playback_history")
                .fetch_all(&self.pool)
                .await?;
        let mut total = 0;
        for user in users {
            total += cooccurrence::rebuild(&self.pool, &user).await?;
        }
        Ok(total)
    }

    /// Most-listened artists first, then the rest of the library; skips fresh fetches.
    async fn next_artist(&self, now: i64) -> AppResult<Option<String>> {
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM (
                 SELECT COALESCE(a.name, e.artist) AS name, COUNT(*) AS n
                 FROM playback_history h
                 LEFT JOIN tracks t ON t.id = h.track_id
                 LEFT JOIN artists a ON a.id = t.artist_id
                 LEFT JOIN external_tracks e ON e.id = h.track_id
                 GROUP BY name
                 UNION ALL
                 SELECT a.name AS name, 0 AS n FROM artists a
             ) WHERE name IS NOT NULL AND name != ''
             GROUP BY name ORDER BY MAX(n) DESC, name ASC LIMIT 2000",
        )
        .fetch_all(&self.pool)
        .await?;
        let fresh: HashSet<String> = sqlx::query_scalar(
            "SELECT artist_norm FROM lastfm_artist_fetch
             WHERE (status IN ('ok','missing') AND fetched_at > ?) OR (status = 'error' AND fetched_at > ?)",
        )
        .bind(now - OK_TTL_SECS)
        .bind(now - ERROR_RETRY_SECS)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect();
        for name in names {
            for (norm, display) in cooccurrence::display_credits(&name) {
                if !fresh.contains(&norm) && !is_placeholder(&norm) {
                    return Ok(Some(display));
                }
            }
        }
        Ok(None)
    }

    async fn fetch_lastfm(&self, artist: &str) -> AppResult<()> {
        let Some(client) = &self.client else { return Ok(()) };
        let Some(norm) = artist_credits(artist).into_iter().next() else { return Ok(()) };
        let now = chrono::Utc::now().timestamp();
        let status = match client.similar_artists(artist).await {
            Ok(list) => {
                let mut tx = self.pool.begin().await?;
                sqlx::query("DELETE FROM artist_similarity WHERE user_id = '' AND source = 'lastfm' AND source_norm = ?")
                    .bind(&norm)
                    .execute(&mut *tx)
                    .await?;
                for similar in list {
                    for (target, display) in cooccurrence::display_credits(&similar.name) {
                        if target == norm {
                            continue;
                        }
                        sqlx::query(
                            "INSERT INTO artist_similarity
                             (user_id, source_norm, target_norm, target_display, score, source, updated_at)
                             VALUES ('', ?, ?, ?, ?, 'lastfm', ?)
                             ON CONFLICT(user_id, source_norm, target_norm, source)
                             DO UPDATE SET score = MAX(score, excluded.score), updated_at = excluded.updated_at",
                        )
                        .bind(&norm)
                        .bind(&target)
                        .bind(&display)
                        .bind(similar.score)
                        .bind(now)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
                tx.commit().await?;
                "ok"
            }
            Err(LastFmError::NotFound) => "missing",
            Err(LastFmError::Transient(e)) => {
                tracing::debug!(artist = %artist, error = %e, "Last.fm similarity deferred");
                "error"
            }
        };
        sqlx::query(
            "INSERT INTO lastfm_artist_fetch(artist_norm, fetched_at, status) VALUES (?, ?, ?)
             ON CONFLICT(artist_norm) DO UPDATE SET fetched_at = excluded.fetched_at, status = excluded.status",
        )
        .bind(&norm)
        .bind(now)
        .bind(status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn is_placeholder(norm: &str) -> bool {
    matches!(norm, "unknown artist" | "unknown" | "various artists" | "various")
}

async fn relevant_event(
    events: &mut tokio::sync::broadcast::Receiver<Event>,
) -> std::result::Result<Event, tokio::sync::broadcast::error::RecvError> {
    loop {
        match events.recv().await {
            Ok(
                e @ (Event::PlaybackBuffering { .. }
                | Event::OnlinePlaybackRequested { .. }
                | Event::PlaybackStarted { .. }
                | Event::PlaybackError { .. }
                | Event::LibraryScanCompleted { .. }
                | Event::TasteProfileChanged { .. }),
            ) => return Ok(e),
            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(e) => return Err(e),
        }
    }
}
