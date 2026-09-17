use crate::config::HistoryConfig;
use crate::core::error::AppResult;
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::models::PlaybackHistoryRecord;
use crate::database::repositories::{HistoryDetail, HistoryRepository, StatsRepository, UserProfile};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error, warn};

/// In-memory state for a single active playback session.
/// Elapsed-time accounting replaces the old max_position_secs approach:
/// - `listened_secs` accumulates wall-clock seconds while actually playing.
/// - `playing_since` is set when playback resumes and cleared on pause/stop.
#[derive(Debug)]
struct ActiveSession {
    id: String,
    stat_date: String,
    user_id: String,
    track_id: String,
    started_at: i64,
    duration_secs: f64,
    /// Accumulated playing time (wall-clock elapsed, pauses excluded).
    listened_secs: f64,
    /// Set to Some(Instant) when the track is actively playing; None when paused.
    playing_since: Option<Instant>,
    source: String,
    /// listened_secs at last journal checkpoint write (for crash-recovery throttling).
    last_journal_secs: f64,
}

impl ActiveSession {
    /// Compute the total elapsed playing time including the current unaccounted interval.
    fn total_listened(&self) -> f64 {
        let extra = self.playing_since
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.listened_secs + extra
    }
}

pub struct HistoryService {
    history_repo: Arc<dyn HistoryRepository>,
    stats_repo: Arc<dyn StatsRepository>,
    config: Arc<RwLock<HistoryConfig>>,
    current_session: Arc<Mutex<Option<ActiveSession>>>,
    current_user: Option<Arc<RwLock<Option<UserProfile>>>>,
    pool: Option<sqlx::SqlitePool>,
}

impl HistoryService {
    pub fn new(
        history_repo: Arc<dyn HistoryRepository>,
        stats_repo: Arc<dyn StatsRepository>,
        config: HistoryConfig,
        event_bus: Arc<EventBus>,
        current_user: Option<Arc<RwLock<Option<UserProfile>>>>,
    ) -> Arc<Self> {
        Self::new_with_pool(history_repo, stats_repo, config, event_bus, current_user, None)
    }

    pub fn new_with_pool(
        history_repo: Arc<dyn HistoryRepository>,
        stats_repo: Arc<dyn StatsRepository>,
        config: HistoryConfig,
        event_bus: Arc<EventBus>,
        current_user: Option<Arc<RwLock<Option<UserProfile>>>>,
        pool: Option<sqlx::SqlitePool>,
    ) -> Arc<Self> {
        let service = Arc::new(Self {
            history_repo,
            stats_repo,
            config: Arc::new(RwLock::new(config)),
            current_session: Arc::new(Mutex::new(None)),
            current_user,
            pool,
        });

        let rx = event_bus.subscribe();
        let s_clone = service.clone();
        tokio::spawn(async move {
            s_clone.listen_to_events(rx).await;
        });

        service
    }

    async fn listen_to_events(&self, mut rx: tokio::sync::broadcast::Receiver<Event>) {
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    warn!(count, "History event receiver lagged; session accuracy may be affected");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            match event {
                Event::PlaybackStarted {
                    session_id,
                    track_id,
                    duration_secs,
                    source,
                    ..
                } => {
                    self.on_playback_started(session_id, track_id, duration_secs, source).await;
                }
                // PlaybackPositionChanged: no longer updates max_position_secs.
                // Journal checkpoints happen here for crash recovery (every 30s of listened time).
                Event::PlaybackPositionChanged { session_id, .. } => {
                    self.maybe_checkpoint_journal(&session_id).await;
                }
                Event::PlaybackPaused { session_id, .. } => {
                    let mut guard = self.current_session.lock().await;
                    if let Some(ref mut session) = *guard {
                        if session.id == session_id {
                            // Accumulate elapsed time from the playing_since marker.
                            if let Some(since) = session.playing_since.take() {
                                session.listened_secs += since.elapsed().as_secs_f64();
                            }
                            // Persist journal entry on pause so crash recovery has an up-to-date value.
                            self.persist_journal_entry_inner(session).await;
                        }
                    }
                }
                Event::PlaybackBuffering { session_id, track_id } => {
                    let mut guard = self.current_session.lock().await;
                    if let Some(ref mut session) = *guard {
                        if (session_id.is_empty() && session.track_id == track_id) || session.id == session_id {
                            // Pause active listening elapsed time while stream is buffering chunks.
                            if let Some(since) = session.playing_since.take() {
                                session.listened_secs += since.elapsed().as_secs_f64();
                            }
                        }
                    }
                }
                Event::PlaybackResumed { session_id, track_id, .. } => {
                    let mut guard = self.current_session.lock().await;
                    if let Some(ref mut session) = *guard {
                        if (session_id.is_empty() && session.track_id == track_id) || session.id == session_id {
                            if session.playing_since.is_none() {
                                session.playing_since = Some(Instant::now());
                            }
                        }
                    }
                }
                Event::TrackFinished {
                    session_id,
                    seconds_listened,
                    completed,
                    ..
                } => {
                    // Match by session_id, not track_id — prevents stale-completion bugs.
                    let session_opt = {
                        let mut guard = self.current_session.lock().await;
                        if guard.as_ref().map(|s| s.id == session_id).unwrap_or(false) {
                            guard.take()
                        } else {
                            None
                        }
                    };

                    if let Some(mut session) = session_opt {
                        // Finalize any remaining playing interval.
                        if let Some(since) = session.playing_since.take() {
                            session.listened_secs += since.elapsed().as_secs_f64();
                        }
                        // Use the greater of elapsed monotonic timer or explicit seconds_listened from the finished event.
                        let listened = if completed {
                            session.duration_secs.max(session.listened_secs).max(seconds_listened)
                        } else {
                            session.listened_secs.max(seconds_listened)
                        };
                        self.finalize_active_session(session, listened, completed, false).await;
                    } else {
                        // session_id mismatch — silently ignore stale TrackFinished events.
                        warn!(session_id = %session_id, "TrackFinished session_id mismatch; ignoring");
                    }
                }
                Event::PlaybackStopped { session_id } => {
                    let session_opt = {
                        let mut guard = self.current_session.lock().await;
                        if guard.as_ref().map(|s| s.id == session_id || session_id.is_empty()).unwrap_or(false) {
                            guard.take()
                        } else {
                            // If session_id is non-empty and doesn't match, ignore.
                            if !session_id.is_empty() {
                                warn!(session_id = %session_id, "PlaybackStopped session_id mismatch; ignoring");
                            }
                            None
                        }
                    };
                    if let Some(mut session) = session_opt {
                        if let Some(since) = session.playing_since.take() {
                            session.listened_secs += since.elapsed().as_secs_f64();
                        }
                        let listened = session.listened_secs;
                        self.finalize_active_session(session, listened, false, true).await;
                    }
                }
                _ => {}
            }
        }
    }

    async fn on_playback_started(&self, session_id: String, track_id: String, duration_secs: f64, source: String) {
        let user_id = match &self.current_user {
            Some(cu) => {
                let guard = cu.read().await;
                guard.as_ref().map(|u| u.id.clone()).unwrap_or_else(|| "default".to_string())
            }
            None => "default".to_string(),
        };

        let new_session = ActiveSession {
            id: session_id,
            stat_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            user_id,
            track_id,
            started_at: Utc::now().timestamp(),
            duration_secs,
            listened_secs: 0.0,
            playing_since: Some(Instant::now()),
            source,
            last_journal_secs: 0.0,
        };

        // Insert new session, capturing any previous session for finalization.
        let prev_session = {
            let mut guard = self.current_session.lock().await;
            guard.replace(new_session)
        };

        // Finalize the previous session (interrupted by new track starting).
        if let Some(mut session) = prev_session {
            if let Some(since) = session.playing_since.take() {
                session.listened_secs += since.elapsed().as_secs_f64();
            }
            let listened = session.listened_secs;
            self.finalize_active_session(session, listened, false, true).await;
        }

        // Write journal entry for the new session.
        {
            let guard = self.current_session.lock().await;
            if let Some(ref session) = *guard {
                self.persist_journal_entry_inner(session).await;
            }
        }
    }

    /// Write or update the active_playback_journal row for crash recovery.
    async fn persist_journal_entry_inner(&self, session: &ActiveSession) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };
        let now = Utc::now().timestamp();
        let device_id = crate::cloud::device::get_or_create_device_id(pool).await
            .unwrap_or_else(|_| "_unknown".to_string());
        let _ = sqlx::query(
            "INSERT INTO active_playback_journal
                (session_id, user_id, track_id, device_id, stat_date, started_at, listened_secs, duration_secs, source, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_id) DO UPDATE SET
                 listened_secs = excluded.listened_secs,
                 updated_at    = excluded.updated_at"
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.track_id)
        .bind(&device_id)
        .bind(&session.stat_date)
        .bind(session.started_at)
        .bind(session.listened_secs)
        .bind(session.duration_secs)
        .bind(&session.source)
        .bind(now)
        .execute(pool)
        .await;
    }

    /// Delete the journal entry after a clean finalization.
    async fn delete_journal_entry(&self, session_id: &str) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };
        let _ = sqlx::query("DELETE FROM active_playback_journal WHERE session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await;
    }

    /// Check if enough new listened time has accumulated to warrant a journal checkpoint (~30s).
    async fn maybe_checkpoint_journal(&self, session_id: &str) {
        let mut guard = self.current_session.lock().await;
        if let Some(ref mut session) = *guard {
            if session.id != session_id {
                return;
            }
            let current_total = session.total_listened();
            if current_total - session.last_journal_secs >= 30.0 {
                session.last_journal_secs = current_total;
                // We need a snapshot for the async call — avoid holding the lock.
                let snapshot_listened = session.listened_secs;
                let since_extra = session.playing_since
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                let effective_listened = snapshot_listened + since_extra;
                // Temporarily update listened_secs for the journal write.
                let orig = session.listened_secs;
                session.listened_secs = effective_listened;
                self.persist_journal_entry_inner(session).await;
                session.listened_secs = orig;
            }
        }
    }

    /// Recover any interrupted sessions recorded in active_playback_journal.
    /// Called at startup before normal operation begins.
    pub async fn recover_interrupted_sessions(&self) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };

        let rows: Vec<(String, String, String, String, String, i64, f64, f64, String)> =
            sqlx::query_as(
                "SELECT session_id, user_id, track_id, device_id, stat_date,
                        started_at, listened_secs, duration_secs, source
                 FROM active_playback_journal"
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        for row in rows {
            let (session_id, user_id, track_id, _device_id, stat_date,
                 started_at, listened_secs, duration_secs, source) = row;

            info!(
                session_id = %session_id,
                track_id   = %track_id,
                listened   = listened_secs,
                "Recovering interrupted playback session from journal"
            );

            let now = Utc::now().timestamp();
            let percentage = if duration_secs > 0.0 {
                (listened_secs / duration_secs).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let is_meaningful = listened_secs >= 30.0 || percentage >= 0.8;

            let entry = PlaybackHistoryRecord {
                id: session_id.clone(),
                user_id: user_id.clone(),
                track_id: track_id.clone(),
                started_at,
                ended_at: now,
                seconds_listened: listened_secs,
                percentage_listened: percentage,
                completed: 0,
                skipped: 0,
                source,
                playlist_id: None,
                recommendation_session_id: None,
            };

            if let Err(e) = self.history_repo.record_session(&entry, &stat_date, is_meaningful).await {
                error!(session_id = %session_id, %e, "Failed to recover interrupted session");
            } else {
                // Clean up the journal row.
                let _ = sqlx::query("DELETE FROM active_playback_journal WHERE session_id = ?")
                    .bind(&session_id)
                    .execute(pool)
                    .await;
            }
        }
    }

    async fn finalize_active_session(
        &self,
        session: ActiveSession,
        seconds_listened: f64,
        completed: bool,
        skipped_hint: bool,
    ) {
        let now = Utc::now().timestamp();
        let dur = session.duration_secs;
        let percentage = if dur > 0.0 {
            (seconds_listened / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let cfg = self.config.read().await;
        let is_meaningful = seconds_listened >= cfg.min_meaningful_seconds
            || percentage >= cfg.min_meaningful_percentage
            || completed;

        let is_skipped = !is_meaningful && skipped_hint;

        let entry = PlaybackHistoryRecord {
            id: session.id.clone(),
            user_id: session.user_id.clone(),
            track_id: session.track_id.clone(),
            started_at: session.started_at,
            ended_at: now,
            seconds_listened,
            percentage_listened: percentage,
            completed: if completed { 1 } else { 0 },
            skipped: if is_skipped { 1 } else { 0 },
            source: session.source,
            playlist_id: None,
            recommendation_session_id: None,
        };

        info!(
            user_id = %session.user_id,
            track_id = %session.track_id,
            seconds = seconds_listened,
            is_meaningful = is_meaningful,
            completed = completed,
            "Recording playback session"
        );

        match self.history_repo.record_session(&entry, &session.stat_date, is_meaningful).await {
            Ok(_) => {
                // Clean up the crash-recovery journal entry ONLY on successful finalization.
                self.delete_journal_entry(&entry.id).await;
            }
            Err(err) => {
                error!(session_id = %entry.id, %err, "Failed to commit playback session; leaving crash journal entry for recovery on next launch");
            }
        }
    }

    pub async fn get_recent_history(&self, limit: u32, user_id: &str) -> AppResult<Vec<HistoryDetail>> {
        self.history_repo.get_recent_history(limit, user_id).await
    }

    pub async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()> {
        self.stats_repo.set_track_like(track_id, like_status).await
    }

    pub async fn set_track_like_scoped(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()> {
        self.stats_repo.set_track_like_scoped(user_id, track_id, like_status).await
    }

    pub fn stats_repo(&self) -> Arc<dyn StatsRepository> {
        self.stats_repo.clone()
    }
}
