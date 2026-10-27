use super::{strict_match, ExternalTrackMetadata, ProviderCoordinator};
use crate::{
    core::{AppError, AppResult, Event, EventBus},
    database::repositories::{ScannedMetadata, SqliteTrackRepository, TrackRepository},
};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify};

/// One cancellable worker; persistent negative results prevent repeat searches on every scan.
pub struct GapEnrichmentService {
    pool: SqlitePool,
    providers: Arc<ProviderCoordinator>,
    cache_dir: PathBuf,
    bus: Arc<EventBus>,
    operation: Mutex<()>,
    manual: AtomicBool,
    pending_updates: AtomicBool,
    wake: Notify,
}

impl GapEnrichmentService {
    pub fn new(
        pool: SqlitePool,
        providers: Arc<ProviderCoordinator>,
        cache_dir: PathBuf,
        bus: Arc<EventBus>,
    ) -> Self {
        Self {
            pool,
            providers,
            cache_dir,
            bus,
            operation: Mutex::new(()),
            manual: AtomicBool::new(false),
            pending_updates: AtomicBool::new(false),
            wake: Notify::new(),
        }
    }

    pub async fn enabled(&self, key: &str, default: bool) -> bool {
        sqlx::query_scalar::<_, String>("SELECT value FROM application_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(default)
    }

    pub async fn request_fill(&self, retry: bool) -> AppResult<()> {
        if retry {
            sqlx::query("UPDATE track_enrichment SET retry_after = 0, state = CASE WHEN state='applying' THEN state ELSE 'pending' END WHERE state != 'matched'").execute(&self.pool).await?;
        }
        self.manual.store(true, Ordering::Release);
        self.wake.notify_one();
        Ok(())
    }

    pub async fn run(self: Arc<Self>) {
        let mut events = self.bus.subscribe();
        let mut busy_until = tokio::time::Instant::now();
        let mut processed = 0;
        loop {
            // Events interrupt network requests: a newly selected playback gets priority.
            let work = async {
                tokio::time::sleep_until(busy_until).await;
                if !self.manual.load(Ordering::Acquire)
                    && !self.enabled("auto_fill_metadata", true).await
                {
                    return Ok(false);
                }
                self.process_next().await
            };
            tokio::select! {
                result = work => {
                    match result {
                        Ok(true) => {
                            processed += 1;
                            if processed >= 32 { self.publish_updates(); processed = 0; }
                            tokio::time::sleep(Duration::from_millis(350)).await; continue;
                        }
                        Ok(false) => {
                            if processed > 0 { self.publish_updates(); processed = 0; }
                            self.manual.store(false, Ordering::Release);
                        }
                        Err(e) => { tracing::warn!(error = %e, "Metadata gap worker will retry"); }
                    }
                }
                event = relevant_event(&mut events) => {
                    match event {
                        Ok(Event::PlaybackBuffering { .. } | Event::OnlinePlaybackRequested { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(20),
                        Ok(Event::PlaybackStarted { .. } | Event::PlaybackError { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(2),
                        Ok(Event::LibraryScanCompleted { .. }) => (),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => continue,
                    }
                    continue;
                }
                _ = self.wake.notified() => continue,
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(60)) => (),
                _ = self.wake.notified() => (),
                event = relevant_event(&mut events) => match event {
                    Ok(Event::PlaybackBuffering { .. } | Event::OnlinePlaybackRequested { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(20),
                    Ok(Event::PlaybackStarted { .. } | Event::PlaybackError { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(2),
                    Err(_) if self.pool.is_closed() => break,
                    _ => (),
                },
            }
        }
    }

    fn publish_updates(&self) {
        if self.pending_updates.swap(false, Ordering::AcqRel) {
            let _ = self.bus.publish(Event::LibraryMetadataUpdated);
        }
    }

    async fn process_next(&self) -> AppResult<bool> {
        let _guard = self.operation.lock().await;
        let id: Option<String> = sqlx::query_scalar(
            "SELECT t.id FROM tracks t LEFT JOIN track_enrichment e ON t.id = e.track_id
             WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'
             AND (t.has_cover_art = 0 OR t.year IS NULL OR t.track_number IS NULL OR t.artist_id IS NULL OR t.album_id IS NULL OR EXISTS (SELECT 1 FROM track_source_metadata s WHERE s.track_id=t.id AND json_extract(s.metadata_json,'$.title_is_fallback')=1))
             AND (e.track_id IS NULL OR (e.state != 'matched' AND e.retry_after <= ?)) ORDER BY t.id LIMIT 1")
            .bind(chrono::Utc::now().timestamp()).fetch_optional(&self.pool).await?;
        let Some(id) = id else {
            return Ok(false);
        };
        match self.enrich_inner(&id).await {
            Ok(true) => {
                self.pending_updates.store(true, Ordering::Release);
            }
            Ok(false) => (),
            Err(e) => {
                self.record_attempt(&id, "failed", 3600).await?;
                tracing::debug!(track_id = %id, error = %e, "Enrichment deferred");
            }
        }
        Ok(true)
    }

    async fn record_attempt(&self, id: &str, state: &str, seconds: i64) -> AppResult<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("INSERT INTO track_enrichment(track_id,provider,state,retry_after,attempted_at) VALUES (?,'jiosaavn',?,?,?) ON CONFLICT(track_id) DO UPDATE SET state=CASE WHEN track_enrichment.state='applying' THEN track_enrichment.state ELSE excluded.state END,retry_after=excluded.retry_after,attempted_at=excluded.attempted_at")
            .bind(id).bind(state).bind(now + seconds).bind(now).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn enrich(&self, id: &str) -> AppResult<bool> {
        let _guard = self.operation.lock().await;
        let result = self.enrich_inner(id).await;
        if matches!(result, Ok(true)) {
            let _ = self.bus.publish(Event::LibraryMetadataUpdated);
        }
        result
    }

    async fn enrich_inner(&self, id: &str) -> AppResult<bool> {
        let repo = SqliteTrackRepository::new(self.pool.clone());
        let Some(track) = repo.find_by_id(id).await? else {
            return Err(AppError::NotFound(id.into()));
        };
        if track.format == "online" {
            return Ok(false);
        }
        let matched: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM track_enrichment WHERE track_id=? AND state='matched'",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        if matched != 0 {
            return Ok(false);
        }
        let source: Option<String> = sqlx::query_scalar(
            "SELECT metadata_json FROM track_source_metadata WHERE track_id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let source: ScannedMetadata = if let Some(json) = source {
            serde_json::from_str(&json).map_err(|e| AppError::Metadata(e.to_string()))?
        } else {
            // Existing libraries acquire actual tag provenance lazily, without delaying scanning.
            let path = PathBuf::from(&track.file_path);
            let size = track.file_size;
            let modified = track.modified_timestamp;
            let source = tokio::task::spawn_blocking(move || {
                crate::library::scanner::LibraryScanner::extract_metadata(&path, size, modified)
            })
            .await
            .map_err(|e| AppError::Metadata(e.to_string()))??;
            repo.save_scanned_track(source.clone()).await?;
            source
        };
        let applying: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM track_enrichment WHERE track_id=? AND state='applying'",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        if applying != 0 {
            repo.apply_stored_enrichment(id).await?;
            let result = sqlx::query(
                "UPDATE track_enrichment SET state='matched' WHERE track_id=? AND state='applying'",
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
            return Ok(result.rows_affected() > 0);
        }
        if source.artist.is_none() && source.album.is_none() {
            self.record_attempt(id, "insufficient_evidence", 7 * 86400)
                .await?;
            return Ok(false);
        }
        let query = format!(
            "{} {}",
            source.title,
            source
                .artist
                .as_deref()
                .or(source.album.as_deref())
                .unwrap_or("")
        );
        let mut provider_failed = false;
        let mut candidates = Vec::new();
        if self.enabled("enable_jiosaavn", true).await {
            match self.providers.jiosaavn.search_metadata(&query, 25).await {
                Ok(found) => candidates = found,
                Err(e) => {
                    provider_failed = true;
                    tracing::debug!(error = %e, "JioSaavn metadata unavailable; will retry later");
                }
            }
        }
        let matched = strict_match::unique_match(
            &source.title,
            source.artist.as_deref(),
            source.album.as_deref(),
            source.duration_secs,
            &candidates,
        )
        .cloned();
        let Some(mut matched) = matched else {
            self.record_attempt(
                id,
                if provider_failed {
                    "temporary_failure"
                } else {
                    "no_unique_match"
                },
                if provider_failed { 3600 } else { 7 * 86400 },
            )
            .await?;
            return Ok(false);
        };
        if matched.provider == "jiosaavn" {
            let detail = self
                .providers
                .jiosaavn
                .detail(&matched.provider_track_id, false)
                .await?;
            let detailed = super::jiosaavn::parse_track(&detail)?;
            if strict_match::unique_match(
                &source.title,
                source.artist.as_deref(),
                source.album.as_deref(),
                source.duration_secs,
                std::slice::from_ref(&detailed),
            )
            .is_none()
            {
                self.record_attempt(id, "detail_mismatch", 7 * 86400)
                    .await?;
                return Ok(false);
            }
            matched = detailed;
        }
        let album_art: Option<String> = sqlx::query_scalar("SELECT al.cover_art_path FROM tracks t JOIN albums al ON al.id=t.album_id WHERE t.id=?")
            .bind(id).fetch_optional(&self.pool).await?.flatten();
        let has_album_art = album_art.is_some_and(|p| std::path::Path::new(&p).is_file());
        let artwork =
            if !source.has_cover_art && !has_folder_art(&source.file_path) && !has_album_art {
                self.cache_artwork(&matched).await?
            } else {
                None
            };
        let evidence = serde_json::json!({"title":strict_match::normalized(&source.title),"artist":source.artist,"album":source.album,"duration_delta_secs":matched.duration_secs.map(|d| d-source.duration_secs),"unique":true,"fields": {"title":source.title_is_fallback,"artist":source.artist.is_none(),"album":source.album.is_none(),"year":source.year.is_none(),"track_number":source.track_number.is_none(),"artwork":artwork.is_some()}});
        sqlx::query("INSERT INTO track_enrichment(track_id,provider,provider_id,metadata_json,evidence_json,cover_art_path,state,attempted_at) VALUES (?,?,?,?,?,?,'applying',?) ON CONFLICT(track_id) DO UPDATE SET provider=excluded.provider,provider_id=excluded.provider_id,metadata_json=excluded.metadata_json,evidence_json=excluded.evidence_json,cover_art_path=excluded.cover_art_path,state='applying',retry_after=0,attempted_at=excluded.attempted_at")
            .bind(id).bind(&matched.provider).bind(&matched.provider_track_id).bind(serde_json::to_string(&matched).unwrap()).bind(evidence.to_string()).bind(artwork.as_ref().map(|p| p.to_string_lossy().to_string())).bind(chrono::Utc::now().timestamp()).execute(&self.pool).await?;
        // Compare against the latest source snapshot, so a concurrent rescan always wins.
        repo.apply_stored_enrichment(id).await?;
        sqlx::query(
            "UPDATE track_enrichment SET state='matched' WHERE track_id=? AND state='applying'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(true)
    }

    async fn cache_artwork(&self, track: &ExternalTrackMetadata) -> AppResult<Option<PathBuf>> {
        let bytes = if let Some(url) = &track.cover_art_url {
            let parsed = reqwest::Url::parse(url).map_err(|e| AppError::Metadata(e.to_string()))?;
            if parsed.scheme() != "https" {
                return Ok(None);
            }
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .build()
                .map_err(|e| AppError::Network(e.to_string()))?;
            let mut response = client
                .get(url)
                .send()
                .await
                .map_err(|e| AppError::Network(e.without_url().to_string()))?
                .error_for_status()
                .map_err(|e| AppError::Network(e.without_url().to_string()))?;
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|e| AppError::Network(e.to_string()))?
            {
                if bytes.len() + chunk.len() > 5 * 1024 * 1024 {
                    return Err(AppError::Metadata("Artwork exceeds 5 MiB".into()));
                }
                bytes.extend_from_slice(&chunk);
            }
            bytes
        } else {
            vec![]
        };
        if bytes.is_empty() {
            return Ok(None);
        }
        let mut reader = image::ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|e| AppError::Metadata(e.to_string()))?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|e| AppError::Metadata(format!("Invalid artwork: {e}")))?;
        let key = format!(
            "{:x}",
            Sha256::digest(format!("{}:{}", track.provider, track.provider_track_id).as_bytes())
        );
        let path = self
            .cache_dir
            .join("enrichment-artwork")
            .join(format!("{key}.image"));
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let temp = path.with_extension("part");
        tokio::fs::write(&temp, bytes).await?;
        tokio::fs::rename(temp, &path).await?;
        Ok(Some(path))
    }

    pub async fn remove_all(&self) -> AppResult<()> {
        let _guard = self.operation.lock().await;
        self.manual.store(false, Ordering::Release);
        let sources: Vec<String> = sqlx::query_scalar("SELECT s.track_id FROM track_source_metadata s JOIN track_enrichment e ON e.track_id=s.track_id WHERE e.metadata_json IS NOT NULL").fetch_all(&self.pool).await?;
        sqlx::query("DELETE FROM track_enrichment")
            .execute(&self.pool)
            .await?;
        let repo = SqliteTrackRepository::new(self.pool.clone());
        for id in sources {
            repo.apply_stored_enrichment(&id).await?;
        }
        // Suppress automatic refill until the user explicitly retries or re-enables it.
        crate::database::repositories::SettingsRepository::set_setting(
            &crate::database::repositories::SqliteSettingsRepository::new(self.pool.clone()),
            "auto_fill_metadata",
            "false",
        )
        .await?;
        let dir = self.cache_dir.join("enrichment-artwork");
        if dir.is_dir() {
            tokio::fs::remove_dir_all(dir).await?;
        }
        let _ = self.bus.publish(Event::LibraryMetadataUpdated);
        Ok(())
    }
}

fn has_folder_art(file: &str) -> bool {
    std::path::Path::new(file).parent().is_some_and(|p| {
        [
            "cover.jpg",
            "folder.jpg",
            "album.jpg",
            "cover.png",
            "folder.png",
            "front.jpg",
            "front.png",
        ]
        .iter()
        .any(|n| p.join(n).is_file())
    })
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
                | Event::LibraryScanCompleted { .. }),
            ) => return Ok(e),
            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(e) => return Err(e),
        }
    }
}
