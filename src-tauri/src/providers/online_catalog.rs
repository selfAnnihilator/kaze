//! Bind online catalog rows before presentation. Playback only follows the saved ID.
use super::{jiosaavn::JioSaavnProvider, strict_match::normalized, ExternalTrackMetadata};
use crate::{
    core::{AppError, AppResult},
    database::{
        models::ExternalTrackRecord,
        repositories::{SqliteWishlistRepository, WishlistRepository},
    },
    recommendations::mixes::ensure_online_track,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, Semaphore};

pub struct OnlineCatalog {
    pool: SqlitePool,
    provider: Arc<JioSaavnProvider>,
    gate: Semaphore,
    misses: Mutex<HashMap<String, Instant>>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    bus: std::sync::RwLock<Option<Arc<crate::core::event_bus::EventBus>>>,
    changed: std::sync::atomic::AtomicBool,
}

impl OnlineCatalog {
    pub fn new(pool: SqlitePool, provider: Arc<JioSaavnProvider>) -> Arc<Self> {
        Arc::new(Self {
            pool,
            provider,
            gate: Semaphore::new(4),
            misses: Mutex::new(HashMap::new()),
            locks: Mutex::new(HashMap::new()),
            bus: std::sync::RwLock::new(None),
            changed: std::sync::atomic::AtomicBool::new(false),
        })
    }

    pub fn set_event_bus(&self, bus: Arc<crate::core::event_bus::EventBus>) {
        *self.bus.write().unwrap() = Some(bus);
    }

    /// Playback follows an identity already bound during import/presentation.
    /// This method never searches or chooses another release.
    pub async fn source_id(&self, id: &str) -> AppResult<Option<String>> {
        let stored: Option<String> =
            sqlx::query_scalar("SELECT provider_id FROM online_track_sources WHERE track_id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        if stored.is_some() {
            return Ok(stored);
        }
        Ok(sqlx::query_scalar(
            "SELECT provider_id FROM external_tracks WHERE id=? AND provider='jiosaavn'",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn bind(&self, id: &str) -> AppResult<Option<ExternalTrackMetadata>> {
        // A physical/local-library track is never replaced by an online version.
        let path: Option<String> = sqlx::query_scalar("SELECT file_path FROM tracks WHERE id=?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        if path
            .as_ref()
            .is_some_and(|path| !path.starts_with("online://"))
        {
            return Ok(None);
        }
        let lock = {
            let mut locks = self.locks.lock().await;
            if locks.len() >= 2048 {
                locks.retain(|_, lock| Arc::strong_count(lock) > 1);
            }
            locks
                .entry(id.into())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _lock = lock.lock().await;
        let stored: Option<String> =
            sqlx::query_scalar("SELECT metadata_json FROM online_track_sources WHERE track_id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(stored) = stored {
            let metadata: ExternalTrackMetadata =
                serde_json::from_str(&stored).map_err(|e| AppError::Internal(e.to_string()))?;
            self.persist(id, &metadata).await?;
            return Ok(Some(metadata));
        }
        let row: Option<ExternalTrackRecord> =
            sqlx::query_as("SELECT * FROM external_tracks WHERE id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        if self
            .misses
            .lock()
            .await
            .get(id)
            .is_some_and(|at| at.elapsed() < Duration::from_secs(300))
        {
            return Ok(None);
        }
        let metadata = if row.provider == "jiosaavn" || id.starts_with("jiosaavn:") {
            // These rows were fetched from the provider already; never search their titles again.
            ExternalTrackMetadata {
                provider: "jiosaavn".into(),
                provider_track_id: if row.provider == "jiosaavn" {
                    row.provider_id
                } else {
                    id.trim_start_matches("jiosaavn:").into()
                },
                title: row.title,
                artist_name: row.artist,
                album_title: row.album,
                duration_secs: row.duration_secs,
                cover_art_url: row.cover_art_url,
                artist_id: None,
                album_id: None,
                year: None,
                track_number: None,
            }
        } else {
            let enabled: Option<String> = sqlx::query_scalar(
                "SELECT value FROM application_settings WHERE key='enable_jiosaavn'",
            )
            .fetch_optional(&self.pool)
            .await?;
            if enabled.as_deref() == Some("false") {
                return Ok(None);
            }
            let _permit = self
                .gate
                .acquire()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let candidates = self
                .provider
                .search(&format!("{} {}", row.title, row.artist), 25)
                .await?;
            // Catalog import selects a provider version once, before it is shown.
            // Prefer its album, then retain the provider's order for alternate releases.
            // This is separate from strict matching used to enrich physical local files.
            let mut eligible: Vec<_> = candidates
                .into_iter()
                .filter(|track| {
                    normalized(&track.title) == normalized(&row.title)
                        && normalized(&track.artist_name) == normalized(&row.artist)
                })
                .collect();
            if let Some(album) = &row.album {
                eligible.sort_by_key(|track| {
                    track
                        .album_title
                        .as_ref()
                        .is_none_or(|candidate| normalized(candidate) != normalized(album))
                });
            }
            let Some(metadata) = eligible.into_iter().next() else {
                let mut misses = self.misses.lock().await;
                if misses.len() >= 2048 {
                    misses.retain(|_, at| at.elapsed() < Duration::from_secs(300));
                    if misses.len() >= 2048 {
                        misses.clear();
                    }
                }
                misses.insert(id.into(), Instant::now());
                return Ok(None);
            };
            metadata
        };
        self.persist(id, &metadata).await?;
        Ok(Some(metadata))
    }

    async fn persist(&self, id: &str, metadata: &ExternalTrackMetadata) -> AppResult<()> {
        let serialized =
            serde_json::to_string(metadata).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,?,?,?) ON CONFLICT(track_id) DO NOTHING")
            .bind(id).bind(&metadata.provider_track_id).bind(serialized).bind(chrono::Utc::now().timestamp()).execute(&mut *transaction).await?;
        // Provider identities are unique in external_tracks. Keep an old alias's
        // original identity; multiple saved aliases may link to one JioSaavn song.
        let updated = sqlx::query("UPDATE external_tracks SET title=?,artist=?,album=?,duration_secs=?,cover_art_url=?,preview_url=NULL,matched_local_track_id=NULL WHERE id=? AND (title!=? OR artist!=? OR album IS NOT ? OR duration_secs IS NOT ? OR cover_art_url IS NOT ? OR preview_url IS NOT NULL)")
            .bind(&metadata.title).bind(&metadata.artist_name).bind(&metadata.album_title).bind(metadata.duration_secs).bind(&metadata.cover_art_url).bind(id)
            .bind(&metadata.title).bind(&metadata.artist_name).bind(&metadata.album_title).bind(metadata.duration_secs).bind(&metadata.cover_art_url).execute(&mut *transaction).await?.rows_affected();
        transaction.commit().await?;
        // Keep the stable Kaze ID: playlists, likes, history and stats reference it.
        let current: Option<(String, f64, Option<String>, Option<String>)> =
            sqlx::query_as("SELECT t.title,t.duration_secs,a.name,al.title FROM tracks t LEFT JOIN artists a ON a.id=t.artist_id LEFT JOIN albums al ON al.id=t.album_id WHERE t.id=?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        if current
            .as_ref()
            .is_some_and(|(title, duration, artist, album)| {
                title != &metadata.title
                    || Some(*duration) != metadata.duration_secs
                    || artist.as_deref() != Some(metadata.artist_name.as_str())
                    || album != &metadata.album_title
            })
        {
            ensure_online_track(
                &self.pool,
                id,
                &metadata.title,
                Some(&metadata.artist_name),
                metadata.album_title.as_deref(),
                metadata.duration_secs,
                metadata.cover_art_url.as_deref(),
                None,
            )
            .await?;
        }
        if updated > 0 {
            self.changed
                .store(true, std::sync::atomic::Ordering::Release);
        }
        Ok(())
    }

    fn notify_changes(&self) {
        if self
            .changed
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            if let Some(bus) = self.bus.read().unwrap().as_ref() {
                let _ = bus.publish(crate::core::event::Event::LibraryMetadataUpdated);
            }
        }
    }

    pub async fn prepare_existing(self: &Arc<Self>) -> AppResult<()> {
        let ids: Vec<String> = sqlx::query_scalar("SELECT e.id FROM external_tracks e LEFT JOIN tracks t ON t.id=e.id WHERE t.id IS NULL OR t.file_path LIKE 'online://%' ORDER BY e.created_at DESC LIMIT 2000").fetch_all(&self.pool).await?;
        self.prepare_ids(ids).await;
        self.notify_changes();
        Ok(())
    }

    pub async fn prepare_ids(
        self: &Arc<Self>,
        ids: Vec<String>,
    ) -> HashMap<String, ExternalTrackMetadata> {
        let mut tasks = tokio::task::JoinSet::new();
        for id in ids.into_iter().collect::<HashSet<_>>() {
            let catalog = self.clone();
            tasks.spawn(async move {
                let result = catalog.bind(&id).await;
                (id, result)
            });
        }
        let mut linked = HashMap::new();
        let deadline = tokio::time::sleep(Duration::from_secs(20));
        tokio::pin!(deadline);
        loop {
            let next = tokio::select! { result=tasks.join_next()=>result, _=&mut deadline=>None };
            let Some(next) = next else {
                break;
            };
            match next {
                Ok((id, Ok(Some(metadata)))) => {
                    linked.insert(id, metadata);
                }
                Ok((_, Ok(None))) => (),
                Ok((id, Err(error))) => {
                    tracing::warn!(track_id=%id,%error,"Could not prepare online source")
                }
                Err(error) => tracing::warn!(%error,"Source preparation task failed"),
            }
        }
        linked
    }

    /// Apply the same source snapshot to every track-bearing IPC payload.
    pub async fn present(self: &Arc<Self>, mut value: Value) -> AppResult<Value> {
        let mut descriptors = HashMap::new();
        collect(&value, &mut descriptors);
        let mut online = HashSet::new();
        for (id, descriptor) in &descriptors {
            let local: Option<String> =
                sqlx::query_scalar("SELECT file_path FROM tracks WHERE id=?")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?;
            if local
                .as_ref()
                .is_some_and(|path| !path.starts_with("online://"))
            {
                continue;
            }
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM external_tracks WHERE id=?)")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await?;
            if !exists {
                // Discovery results can arrive before a track has been saved locally.
                if descriptor.get("external_track_id").is_none() && local.is_none() {
                    continue;
                }
                let text = |key: &str| descriptor[key].as_str().map(str::to_string);
                SqliteWishlistRepository::new(self.pool.clone())
                    .upsert_external_track(&ExternalTrackRecord {
                        id: id.clone(),
                        provider: text("provider").unwrap_or_else(|| "online".into()),
                        provider_id: text("provider_id").unwrap_or_else(|| id.clone()),
                        title: text("title").unwrap_or_default(),
                        artist: text("artist")
                            .or_else(|| text("artist_name"))
                            .unwrap_or_default(),
                        album: text("album").or_else(|| text("album_title")),
                        duration_secs: descriptor["duration_secs"].as_f64(),
                        cover_art_url: text("cover_art_url"),
                        preview_url: None,
                        genre: text("genre"),
                        match_status: "NOT_FOUND".into(),
                        matched_local_track_id: None,
                        created_at: chrono::Utc::now().timestamp(),
                    })
                    .await?;
            }
            online.insert(id.clone());
        }
        let linked = self.prepare_ids(online.iter().cloned().collect()).await;
        apply(&mut value, &online, &linked);
        self.notify_changes();
        Ok(value)
    }
}

fn identity(value: &Value) -> Option<&str> {
    // A source pointer also has track_id. It is a reference, not another track.
    if value.get("title").is_none() {
        return None;
    }
    value
        .get("external_track_id")
        .or_else(|| value.get("track_id"))
        .or_else(|| {
            if value.get("file_path").is_some() {
                value.get("id")
            } else {
                None
            }
        })
        .and_then(Value::as_str)
}
fn collect(value: &Value, descriptors: &mut HashMap<String, Value>) {
    if let Some(id) = identity(value) {
        descriptors.insert(id.into(), value.clone());
    }
    match value {
        Value::Array(items) => {
            for item in items {
                collect(item, descriptors);
            }
        }
        Value::Object(fields) => {
            for value in fields.values() {
                collect(value, descriptors);
            }
        }
        _ => (),
    }
}
fn apply(
    value: &mut Value,
    online: &HashSet<String>,
    linked: &HashMap<String, ExternalTrackMetadata>,
) -> bool {
    if let Some(id) = identity(value).map(str::to_string) {
        if online.contains(&id) {
            let Some(track) = linked.get(&id) else {
                return false;
            };
            if let Some(object) = value.as_object_mut() {
                object.insert("title".into(), json!(track.title));
                for (key, data) in [
                    ("artist", json!(track.artist_name)),
                    ("artist_name", json!(track.artist_name)),
                    ("album", json!(track.album_title)),
                    ("album_title", json!(track.album_title)),
                    ("duration_secs", json!(track.duration_secs)),
                    ("cover_art_url", json!(track.cover_art_url)),
                    ("provider", json!("jiosaavn")),
                    ("provider_id", json!(track.provider_track_id)),
                    ("preview_url", Value::Null),
                    ("matched_local_track_id", Value::Null),
                    (
                        "playback_source",
                        json!({"provider":"jiosaavn","track_id":track.provider_track_id}),
                    ),
                ] {
                    object.insert(key.into(), data);
                }
            }
        }
    }
    match value {
        Value::Array(items) => items.retain_mut(|item| apply(item, online, linked)),
        Value::Object(fields) => {
            for child in fields.values_mut() {
                if !apply(child, online, linked) {
                    *child = Value::Null;
                }
            }
        }
        _ => (),
    }
    true
}
