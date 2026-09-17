use crate::core::error::{AppError, AppResult};
use crate::downloads::DownloadService;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Default maximum remote-audio cache size (1 GiB).
pub const DEFAULT_MAX_CACHE_BYTES: u64 = 1024 * 1024 * 1024;
/// Eviction target (~900 MiB) to avoid eviction thrashing around the limit.
pub const DEFAULT_EVICTION_TARGET_BYTES: u64 = 900 * 1024 * 1024;
/// Maximum allowable single file size to prevent one huge download from defeating the cache.
pub const DEFAULT_MAX_SINGLE_FILE_BYTES: u64 = 500 * 1024 * 1024;
/// Threshold for cleaning up abandoned partial (.part) files (24 hours).
pub const DEFAULT_STALE_PART_MAX_AGE_SECS: u64 = 24 * 3600;

/// Bounded cache configuration for remote audio streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAudioCacheConfig {
    pub max_size_bytes: u64,
    pub eviction_target_bytes: u64,
    pub max_single_file_bytes: u64,
    pub stale_part_max_age_secs: u64,
}

impl Default for RemoteAudioCacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: DEFAULT_MAX_CACHE_BYTES,
            eviction_target_bytes: DEFAULT_EVICTION_TARGET_BYTES,
            max_single_file_bytes: DEFAULT_MAX_SINGLE_FILE_BYTES,
            stale_part_max_age_secs: DEFAULT_STALE_PART_MAX_AGE_SECS,
        }
    }
}

/// Statistics for the remote audio disk cache.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteAudioCacheStats {
    pub total_size_bytes: u64,
    pub file_count: usize,
    pub max_size_bytes: u64,
    pub partial_file_count: usize,
}

/// Result of an LRU cache eviction pass.
#[derive(Debug, Clone, Default)]
pub struct EvictionResult {
    pub total_before: u64,
    pub total_after: u64,
    pub bytes_evicted: u64,
    pub files_evicted: usize,
}

/// Result of clearing the remote audio cache.
#[derive(Debug, Clone, Default)]
pub struct ClearCacheResult {
    pub bytes_freed: u64,
    pub files_removed: usize,
}

/// RAII Drop Guard ensuring partial (`.part`) files are removed promptly
/// if a download future is cancelled, timed out, or encounters an error.
pub struct PartFileCleanupGuard {
    pub part_path: PathBuf,
    keep: bool,
}

impl PartFileCleanupGuard {
    pub fn new(part_path: PathBuf) -> Self {
        Self {
            part_path,
            keep: false,
        }
    }

    pub fn disarm(&mut self) {
        self.keep = true;
    }
}

impl Drop for PartFileCleanupGuard {
    fn drop(&mut self) {
        if !self.keep {
            if let Err(e) = std::fs::remove_file(&self.part_path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!(path = %self.part_path.display(), error = %e, "Failed to remove partial audio cache file on drop");
                }
            } else {
                debug!(path = %self.part_path.display(), "Cleaned up cancelled or failed partial audio file");
            }
        }
    }
}

/// Manages streaming audio resolution, bounded disk caching, LRU eviction, and cancellation safety.
pub struct StreamPlaybackManager {
    cache_dir: PathBuf,
    legacy_cache_dir: PathBuf,
    pool: Option<SqlitePool>,
    download_service: Option<Arc<DownloadService>>,
    http_client: reqwest::Client,
    config: Arc<RwLock<RemoteAudioCacheConfig>>,
    active_playing_path: Arc<RwLock<Option<PathBuf>>>,
    active_downloads: Arc<tokio::sync::Mutex<HashSet<PathBuf>>>,
    download_waiters: Arc<tokio::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Notify>>>>,
    eviction_lock: Arc<tokio::sync::Mutex<()>>,
}

impl StreamPlaybackManager {
    pub fn new(
        base_cache_dir: PathBuf,
        pool: Option<SqlitePool>,
        download_service: Option<Arc<DownloadService>>,
    ) -> Self {
        Self::with_config(base_cache_dir, pool, download_service, RemoteAudioCacheConfig::default())
    }

    pub fn with_config(
        base_cache_dir: PathBuf,
        pool: Option<SqlitePool>,
        download_service: Option<Arc<DownloadService>>,
        config: RemoteAudioCacheConfig,
    ) -> Self {
        let stream_cache_dir = base_cache_dir.join("remote-audio");
        let legacy_cache_dir = base_cache_dir.join("stream_cache");
        let http_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            cache_dir: stream_cache_dir,
            legacy_cache_dir,
            pool,
            download_service,
            http_client,
            config: Arc::new(RwLock::new(config)),
            active_playing_path: Arc::new(RwLock::new(None)),
            active_downloads: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            download_waiters: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            eviction_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Returns the active remote-audio cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Computes the unique cache key for a track.
    pub fn compute_cache_key(track_id: &str, artist: &str, title: &str) -> String {
        format!("{}:{}:{}", track_id, artist.trim().to_lowercase(), title.trim().to_lowercase())
    }

    /// Computes the final disk cache path for a cache key.
    pub fn get_cache_path(&self, key: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        self.cache_dir.join(format!("{}.audio", hash))
    }

    /// Computes the temporary `.part` file path for a target cache path.
    pub fn get_part_path(target_path: &Path) -> PathBuf {
        let file_name = target_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("temp");
        target_path.with_file_name(format!("{}.part", file_name))
    }

    /// Updates the active playing path to protect it from eviction.
    pub async fn set_active_playing_path(&self, path: Option<PathBuf>) {
        *self.active_playing_path.write().await = path;
    }

    /// For testing purposes: register an active in-flight download path.
    #[doc(hidden)]
    pub async fn mark_active_download_for_test(&self, path: PathBuf) {
        self.active_downloads.lock().await.insert(path);
    }

    /// For testing purposes: unregister an active in-flight download path.
    #[doc(hidden)]
    pub async fn unmark_active_download_for_test(&self, path: &Path) {
        self.active_downloads.lock().await.remove(path);
    }

    /// Retrieves all protected paths (currently playing file, active downloads, and their .part files).
    async fn get_protected_paths(&self) -> HashSet<PathBuf> {
        let mut protected = HashSet::new();
        if let Some(ref path) = *self.active_playing_path.read().await {
            protected.insert(path.clone());
        }
        let active = self.active_downloads.lock().await;
        for path in active.iter() {
            protected.insert(path.clone());
            protected.insert(Self::get_part_path(path));
        }
        protected
    }

    /// Validates whether a cached file exists and has sufficient bytes to be a usable audio file.
    pub fn is_valid_cache_entry(path: &Path) -> bool {
        if let Ok(meta) = std::fs::metadata(path) {
            meta.is_file() && meta.len() > 8192
        } else {
            false
        }
    }

    /// Touches a file's last modified timestamp to update its LRU status.
    pub fn touch_cache_entry(path: &Path) {
        let now = std::time::SystemTime::now();
        let times = std::fs::FileTimes::new().set_modified(now);
        if let Ok(file) = std::fs::File::options().write(true).open(path) {
            let _ = file.set_times(times);
        } else if let Ok(file) = std::fs::File::open(path) {
            let _ = file.set_times(times);
        }
    }

    /// Updates the cache configuration.
    pub async fn set_config(&self, new_config: RemoteAudioCacheConfig) {
        *self.config.write().await = new_config;
    }

    /// Returns the current cache configuration.
    pub async fn get_config(&self) -> RemoteAudioCacheConfig {
        self.config.read().await.clone()
    }

    /// Resolves an audio track to a playable local file path and duration.
    ///
    /// Checks in order:
    /// 1. Local matched file on disk (from library).
    /// 2. Downloaded audio file in downloads folder.
    /// 3. Valid cached audio in remote-audio cache (or migrated from legacy cache).
    /// 4. Remote full-audio stream resolution (via yt-dlp / composite provider).
    /// 5. Remote preview URL fallback (via iTunes / Deezer / Audius).
    pub async fn resolve_and_prepare_audio(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        preview_url: Option<&str>,
    ) -> AppResult<(PathBuf, f64)> {
        info!(track_id, title, artist, "Resolving stream audio for playback");

        // 1. Check if there is an already-matched local track in the database
        if let Some(pool) = &self.pool {
            let local_row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT matched_local_track_id FROM external_tracks WHERE id = ?",
            )
            .bind(track_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some((Some(matched_id),)) = local_row {
                if !matched_id.is_empty() && !matched_id.starts_with("online:") && !matched_id.starts_with("itunes:") {
                    let path_row: Option<(String, f64, String)> = sqlx::query_as(
                        "SELECT file_path, duration_secs, format FROM tracks WHERE id = ?",
                    )
                    .bind(&matched_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                    if let Some((file_path, dur, format)) = path_row {
                        let path = PathBuf::from(&file_path);
                        if format != "online" && path.exists() {
                            info!(%matched_id, path = %file_path, "Using matched local library file for online track");
                            return Ok((path, dur));
                        }
                    }
                }
            }
        }

        // 2. Check if a downloaded audio file exists in the download directory
        if let Some(ref dl) = self.download_service {
            let dl_dir = dl.download_dir();
            let sanitized_title = sanitize_component(title);
            let sanitized_artist = sanitize_component(artist);

            for ext in &["mp3", "flac", "m4a", "ogg", "opus"] {
                let candidate1 = dl_dir.join(format!("{} - {}.{}", sanitized_artist, sanitized_title, ext));
                if candidate1.exists() {
                    info!(path = %candidate1.display(), "Using downloaded file from download folder");
                    return Ok((candidate1, 210.0));
                }
                let candidate2 = dl_dir.join(format!("{}.{}", sanitized_title, ext));
                if candidate2.exists() {
                    info!(path = %candidate2.display(), "Using downloaded file from download folder");
                    return Ok((candidate2, 210.0));
                }
            }
        }

        // 3. Check if cached in stream disk cache (Cache Hit)
        let cache_key = Self::compute_cache_key(track_id, artist, title);
        let cache_path = self.get_cache_path(&cache_key);

        if Self::is_valid_cache_entry(&cache_path) {
            debug!(path = %cache_path.display(), "Cache hit for remote audio stream");
            Self::touch_cache_entry(&cache_path);
            let dur = self.get_known_duration(track_id).await.unwrap_or(210.0);
            return Ok((cache_path, dur));
        }

        // Check legacy cache dir if present
        let legacy_path = self.legacy_cache_dir.join(
            cache_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(""),
        );
        if Self::is_valid_cache_entry(&legacy_path) {
            let _ = tokio::fs::create_dir_all(&self.cache_dir).await;
            if tokio::fs::rename(&legacy_path, &cache_path).await.is_ok() {
                debug!(from = %legacy_path.display(), to = %cache_path.display(), "Migrated legacy cache entry to remote-audio");
                Self::touch_cache_entry(&cache_path);
                let dur = self.get_known_duration(track_id).await.unwrap_or(210.0);
                return Ok((cache_path, dur));
            }
        }

        // Handle duplicate in-flight downloads for the exact same cache path
        let waiter_notify = {
            let mut active = self.active_downloads.lock().await;
            let mut waiters = self.download_waiters.lock().await;
            if active.contains(&cache_path) {
                debug!(path = %cache_path.display(), "Download already active for this cache key, subscribing to completion");
                Some(waiters.entry(cache_path.clone()).or_insert_with(|| Arc::new(tokio::sync::Notify::new())).clone())
            } else {
                active.insert(cache_path.clone());
                None
            }
        };

        if let Some(notify) = waiter_notify {
            notify.notified().await;
            if Self::is_valid_cache_entry(&cache_path) {
                Self::touch_cache_entry(&cache_path);
                let dur = self.get_known_duration(track_id).await.unwrap_or(210.0);
                return Ok((cache_path, dur));
            }
            // If previous download failed, acquire slot and attempt fresh fetch
            let mut active = self.active_downloads.lock().await;
            active.insert(cache_path.clone());
        }

        // Ensure active download is unregistered when function completes or drops
        struct DownloadSlotGuard {
            target_path: PathBuf,
            active_downloads: Arc<tokio::sync::Mutex<HashSet<PathBuf>>>,
            download_waiters: Arc<tokio::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Notify>>>>,
        }

        impl Drop for DownloadSlotGuard {
            fn drop(&mut self) {
                let target = self.target_path.clone();
                let active_clone = self.active_downloads.clone();
                let waiters_clone = self.download_waiters.clone();
                tokio::spawn(async move {
                    active_clone.lock().await.remove(&target);
                    if let Some(notify) = waiters_clone.lock().await.remove(&target) {
                        notify.notify_waiters();
                    }
                });
            }
        }

        let _slot_guard = DownloadSlotGuard {
            target_path: cache_path.clone(),
            active_downloads: self.active_downloads.clone(),
            download_waiters: self.download_waiters.clone(),
        };

        // 4. Resolve stream URL
        // If a direct preview_url is provided (e.g. from iTunes/Discovery recommendations),
        // prioritize it for instant, pristine 256kbps preview playback (~300ms download).
        let direct_preview = preview_url.filter(|p| !p.trim().is_empty()).map(|p| p.to_string());
        let mut candidates: Vec<(String, f64)> = Vec::new();

        if let Some(ref prev) = direct_preview {
            candidates.push((prev.clone(), 30.0));
        }

        // Secondary / alternative: resolve full track audio via download service if available
        // If no preview was provided, this becomes the primary choice.
        if candidates.is_empty() {
            if let Some(ref dl) = self.download_service {
                match dl.resolve_full_track_audio(artist, title).await {
                    Ok(Some((stream_url, dur, _))) => {
                        info!(%stream_url, dur, "Resolved full track stream URL");
                        candidates.push((stream_url, dur));
                    }
                    Ok(None) => {
                        debug!("Full track stream not resolved by provider, falling back");
                    }
                    Err(e) => {
                        warn!(error = %e, "Error resolving full track stream from download service");
                    }
                }
            }
        }

        // Also check DB for cached preview URL if still empty
        if candidates.is_empty() {
            if let Some(pool) = &self.pool {
                let db_prev: Option<(Option<String>, Option<f64>)> = sqlx::query_as(
                    "SELECT preview_url, duration_secs FROM external_tracks WHERE id = ?",
                )
                .bind(track_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((Some(p), dur)) = db_prev {
                    if !p.trim().is_empty() {
                        candidates.push((p, dur.unwrap_or(30.0)));
                    }
                }
            }
        }

        if candidates.is_empty() {
            return Err(AppError::Playback(format!(
                "No streamable audio source available for \"{}\" by \"{}\"",
                title, artist
            )));
        }

        // 5. Download and cache the stream (Cache Miss with fallback across candidates)
        let mut last_error = None;
        let mut successful_duration = 210.0;

        for (stream_url, dur) in candidates {
            info!(url = %stream_url, target = %cache_path.display(), "Attempting to download remote audio stream to cache");
            match self.download_and_cache(&stream_url, &cache_path).await {
                Ok(()) => {
                    successful_duration = dur;
                    last_error = None;
                    break;
                }
                Err(err) => {
                    warn!(url = %stream_url, error = %err, "Failed to download stream candidate, checking for alternative candidate");
                    last_error = Some(err);
                }
            }
        }

        if let Some(err) = last_error {
            return Err(err);
        }

        // Trigger asynchronous background eviction if needed (never blocks playback startup)
        let manager_clone = self.clone_for_bg();
        tokio::spawn(async move {
            let _ = manager_clone.enforce_cache_limits().await;
        });

        Ok((cache_path, successful_duration))
    }

    /// Downloads the remote audio stream to a `.part` file, verifies size, and atomically renames it.
    async fn download_and_cache(&self, url: &str, target_path: &Path) -> AppResult<()> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| AppError::Io(format!("Failed to create remote-audio cache dir: {}", e)))?;

        let part_path = Self::get_part_path(target_path);
        let mut guard = PartFileCleanupGuard::new(part_path.clone());

        let mut request = self
            .http_client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            );

        // If streaming from googlevideo/youtube, send Range: bytes=0- to bypass Google CDN throttling
        if url.contains("googlevideo.com") || url.contains("youtube.com") {
            request = request.header(reqwest::header::RANGE, "bytes=0-");
        }

        let response = request
            .send()
            .await
            .map_err(|e| AppError::Playback(format!("Failed to connect to audio stream: {}", e)))?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(AppError::Playback(format!(
                "Audio stream returned HTTP {}",
                status
            )));
        }

        let config = self.config.read().await.clone();
        if let Some(content_length) = response.content_length() {
            if content_length > config.max_single_file_bytes {
                return Err(AppError::Playback(format!(
                    "Remote audio source ({} bytes) exceeds maximum allowable single cache file size ({} bytes)",
                    content_length, config.max_single_file_bytes
                )));
            }
        }

        let mut file = tokio::fs::File::create(&part_path)
            .await
            .map_err(|e| AppError::Io(format!("Failed to create partial cache file: {}", e)))?;

        let mut downloaded_bytes: u64 = 0;
        let mut mut_resp = response;
        loop {
            let chunk_opt = tokio::time::timeout(std::time::Duration::from_secs(25), mut_resp.chunk())
                .await
                .map_err(|_| AppError::Playback("Audio stream download timed out (no data received for 25s)".to_string()))?
                .map_err(|e| AppError::Playback(format!("Stream read error: {}", e)))?;

            let chunk = match chunk_opt {
                Some(c) => c,
                None => break,
            };

            downloaded_bytes += chunk.len() as u64;
            if downloaded_bytes > config.max_single_file_bytes {
                return Err(AppError::Playback(format!(
                    "Remote audio stream exceeded maximum single cache file size ({} bytes)",
                    config.max_single_file_bytes
                )));
            }
            file.write_all(&chunk)
                .await
                .map_err(|e| AppError::Io(e.to_string()))?;
        }

        file.flush()
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;
        drop(file);

        if downloaded_bytes == 0 {
            return Err(AppError::Playback("Audio stream was empty".to_string()));
        }

        // Atomic rename from .part to final .audio
        tokio::fs::rename(&part_path, target_path)
            .await
            .map_err(|e| AppError::Io(format!("Failed to finalize cache file: {}", e)))?;

        // Succeeded: disarm cleanup guard
        guard.disarm();
        Self::touch_cache_entry(target_path);

        info!(bytes = downloaded_bytes, path = %target_path.display(), "Successfully cached remote stream");
        Ok(())
    }

    /// Enforces the bounded cache size using LRU eviction down to the eviction target.
    pub async fn enforce_cache_limits(&self) -> AppResult<EvictionResult> {
        let _lock = self.eviction_lock.lock().await;
        let config = self.config.read().await.clone();
        let protected = self.get_protected_paths().await;

        let mut entries = Vec::new();
        let mut total_size: u64 = 0;

        let mut read_dir = match tokio::fs::read_dir(&self.cache_dir).await {
            Ok(rd) => rd,
            Err(_) => return Ok(EvictionResult::default()),
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            // Critical boundary safety check: strictly within cache_dir
            if path.parent() != Some(&self.cache_dir) {
                continue;
            }

            let is_audio = path.extension().and_then(|ext| ext.to_str()) == Some("audio");
            if !is_audio {
                continue;
            }

            if let Ok(meta) = entry.metadata().await {
                if meta.is_file() {
                    let size = meta.len();
                    total_size += size;
                    let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    entries.push((path, size, mtime));
                }
            }
        }

        if total_size <= config.max_size_bytes {
            return Ok(EvictionResult {
                total_before: total_size,
                total_after: total_size,
                bytes_evicted: 0,
                files_evicted: 0,
            });
        }

        info!(
            total_size_mb = total_size / (1024 * 1024),
            max_size_mb = config.max_size_bytes / (1024 * 1024),
            target_size_mb = config.eviction_target_bytes / (1024 * 1024),
            "Remote audio cache exceeded limit, starting LRU eviction"
        );

        // Sort by mtime ascending (oldest first)
        entries.sort_by_key(|(_, _, mtime)| *mtime);

        let mut current_size = total_size;
        let mut bytes_evicted = 0;
        let mut files_evicted = 0;

        for (path, size, _) in entries {
            if current_size <= config.eviction_target_bytes {
                break;
            }

            // Never evict protected paths (active playback or actively downloading)
            if protected.contains(&path) {
                debug!(path = %path.display(), "Skipping protected path during cache eviction");
                continue;
            }

            if let Err(e) = tokio::fs::remove_file(&path).await {
                warn!(path = %path.display(), error = %e, "Failed to remove evicted cache file");
            } else {
                current_size = current_size.saturating_sub(size);
                bytes_evicted += size;
                files_evicted += 1;
                debug!(path = %path.display(), size, "Evicted LRU remote audio cache entry");
            }
        }

        info!(
            bytes_evicted_mb = bytes_evicted / (1024 * 1024),
            files_evicted,
            current_size_mb = current_size / (1024 * 1024),
            "Remote audio cache eviction completed"
        );

        Ok(EvictionResult {
            total_before: total_size,
            total_after: current_size,
            bytes_evicted,
            files_evicted,
        })
    }

    /// Cleans up stale `.part` files older than the given max age threshold.
    pub async fn clean_stale_partial_files(&self, max_age: std::time::Duration) -> usize {
        let mut removed_count = 0;
        let now = std::time::SystemTime::now();

        for dir in &[&self.cache_dir, &self.legacy_cache_dir] {
            if let Ok(mut read_dir) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = read_dir.next_entry().await {
                    let path = entry.path();
                    if path.parent() != Some(dir.as_path()) {
                        continue;
                    }
                    let is_part = path.extension().and_then(|s| s.to_str()) == Some("part");
                    if is_part {
                        if let Ok(meta) = entry.metadata().await {
                            let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                            if let Ok(age) = now.duration_since(mtime) {
                                if age >= max_age {
                                    if tokio::fs::remove_file(&path).await.is_ok() {
                                        removed_count += 1;
                                        info!(path = %path.display(), age_secs = age.as_secs(), "Removed stale partial audio cache file");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        removed_count
    }

    /// Migrates any valid entries from the legacy `stream_cache` directory into `remote-audio`.
    pub async fn migrate_legacy_cache(&self) {
        if !self.legacy_cache_dir.exists() || self.legacy_cache_dir == self.cache_dir {
            return;
        }

        let _ = tokio::fs::create_dir_all(&self.cache_dir).await;
        if let Ok(mut read_dir) = tokio::fs::read_dir(&self.legacy_cache_dir).await {
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("audio") {
                    if let Some(file_name) = path.file_name() {
                        let target = self.cache_dir.join(file_name);
                        if !target.exists() {
                            let _ = tokio::fs::rename(&path, &target).await;
                        }
                    }
                }
            }
        }
    }

    /// Performs background startup maintenance (migrating legacy cache, cleaning stale partials, enforcing limits).
    pub async fn startup_maintenance(&self) {
        self.migrate_legacy_cache().await;
        let max_age = std::time::Duration::from_secs(self.config.read().await.stale_part_max_age_secs);
        let stale_count = self.clean_stale_partial_files(max_age).await;
        if stale_count > 0 {
            info!(stale_count, "Cleaned up orphaned partial downloads on startup");
        }
        let _ = self.enforce_cache_limits().await;
    }

    /// Clears the remote-audio cache while preserving currently active playing and downloading files.
    pub async fn clear_cache(&self) -> AppResult<ClearCacheResult> {
        let _lock = self.eviction_lock.lock().await;
        let protected = self.get_protected_paths().await;
        let mut bytes_freed: u64 = 0;
        let mut files_removed: usize = 0;

        for dir in &[&self.cache_dir, &self.legacy_cache_dir] {
            if let Ok(mut read_dir) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = read_dir.next_entry().await {
                    let path = entry.path();
                    if path.parent() != Some(dir.as_path()) {
                        continue;
                    }
                    if protected.contains(&path) {
                        debug!(path = %path.display(), "Skipping protected active path during clear cache");
                        continue;
                    }
                    if let Ok(meta) = entry.metadata().await {
                        if meta.is_file() {
                            let size = meta.len();
                            if tokio::fs::remove_file(&path).await.is_ok() {
                                bytes_freed += size;
                                files_removed += 1;
                            }
                        }
                    }
                }
            }
        }

        info!(bytes_freed, files_removed, "Remote audio cache cleared");
        Ok(ClearCacheResult {
            bytes_freed,
            files_removed,
        })
    }

    /// Returns aggregate statistics for the remote audio cache.
    pub async fn get_cache_stats(&self) -> AppResult<RemoteAudioCacheStats> {
        let config = self.config.read().await.clone();
        let mut total_size_bytes: u64 = 0;
        let mut file_count: usize = 0;
        let mut partial_file_count: usize = 0;

        for dir in &[&self.cache_dir, &self.legacy_cache_dir] {
            if let Ok(mut read_dir) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(entry)) = read_dir.next_entry().await {
                    let path = entry.path();
                    if path.parent() != Some(dir.as_path()) {
                        continue;
                    }
                    if let Ok(meta) = entry.metadata().await {
                        if meta.is_file() {
                            let is_part = path.extension().and_then(|s| s.to_str()) == Some("part");
                            let is_audio = path.extension().and_then(|s| s.to_str()) == Some("audio");
                            if is_part {
                                partial_file_count += 1;
                                total_size_bytes += meta.len();
                            }
                            if is_audio {
                                file_count += 1;
                                total_size_bytes += meta.len();
                            }
                        }
                    }
                }
            }
        }

        Ok(RemoteAudioCacheStats {
            total_size_bytes,
            file_count,
            max_size_bytes: config.max_size_bytes,
            partial_file_count,
        })
    }

    async fn get_known_duration(&self, track_id: &str) -> Option<f64> {
        let pool = self.pool.as_ref()?;
        let row: Option<(Option<f64>,)> = sqlx::query_as(
            "SELECT duration_secs FROM external_tracks WHERE id = ?",
        )
        .bind(track_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        row.and_then(|(d,)| d).filter(|&d| d > 0.0)
    }

    fn clone_for_bg(&self) -> Arc<Self> {
        Arc::new(Self {
            cache_dir: self.cache_dir.clone(),
            legacy_cache_dir: self.legacy_cache_dir.clone(),
            pool: self.pool.clone(),
            download_service: self.download_service.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone(),
            active_playing_path: self.active_playing_path.clone(),
            active_downloads: self.active_downloads.clone(),
            download_waiters: self.download_waiters.clone(),
            eviction_lock: self.eviction_lock.clone(),
        })
    }
}

fn sanitize_component(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '/'
                || c == '\\'
                || c == ':'
                || c == '*'
                || c == '?'
                || c == '"'
                || c == '<'
                || c == '>'
                || c == '|'
            {
                '_'
            } else {
                c
            }
        })
        .collect()
}
