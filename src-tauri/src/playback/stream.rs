use crate::core::error::{AppError, AppResult};
use crate::downloads::DownloadService;
use crate::playback::resolution_cache::{parse_url_expiry, ResolutionCache, ResolvedEntry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::collections::HashSet;
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

use crate::core::event_bus::EventBus;
use crate::playback::decoder::{PlayerSource, SymphoniaSource};
use crate::playback::progressive::{ProgressiveStreamReader, ProgressiveStreamState};

/// Representation of prepared audio ready to be fed into the AudioBackend.
pub enum PreparedPlayback {
    LocalFile {
        path: PathBuf,
        duration: f64,
    },
    ProgressiveStream {
        source: PlayerSource,
        part_path: PathBuf,
        target_path: PathBuf,
        duration: f64,
        state: Arc<ProgressiveStreamState>,
    },
}

/// Representation of a resolved audio source prior to audio streaming/playback.
#[derive(Debug, Clone)]
pub enum ResolvedAudioSource {
    LocalFile {
        path: PathBuf,
        duration: f64,
    },
    RemoteStream {
        url: String,
        duration: f64,
        from_cache: bool,
    },
}

/// Owns every worker and cancellation signal from resolution through playback.
struct Preparation {
    cancelled: tokio::sync::watch::Sender<bool>,
    state: std::sync::Mutex<Option<Arc<ProgressiveStreamState>>>,
    paths: std::sync::Mutex<HashSet<PathBuf>>,
    producer: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    probe: tokio::sync::Mutex<Option<tokio::task::JoinHandle<AppResult<SymphoniaSource>>>>,
}

impl Preparation {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            cancelled: tokio::sync::watch::channel(false).0,
            state: std::sync::Mutex::new(None),
            paths: std::sync::Mutex::new(HashSet::new()),
            producer: tokio::sync::Mutex::new(None),
            probe: tokio::sync::Mutex::new(None),
        })
    }

    fn cancel(&self) {
        self.cancelled.send_replace(true);
        if let Some(state) = self.state.lock().unwrap().as_ref() {
            state.deactivate_playback();
            state.set_cancelled();
        }
    }

    async fn drain(&self) {
        if let Some(task) = self.producer.lock().await.take() { let _ = task.await; }
        if let Some(task) = self.probe.lock().await.take() { let _ = task.await; }
    }
}

/// Manages streaming audio resolution, bounded disk caching, LRU eviction, and cancellation safety.
pub struct StreamPlaybackManager {
    jiosaavn: Arc<crate::providers::jiosaavn::JioSaavnProvider>,
    pub catalog: Option<Arc<crate::providers::online_catalog::OnlineCatalog>>,
    cache_dir: PathBuf,
    pool: Option<SqlitePool>,
    http_client: reqwest::Client,
    config: Arc<RwLock<RemoteAudioCacheConfig>>,
    active_playing_path: Arc<RwLock<Option<PathBuf>>>,
    active_downloads: Arc<tokio::sync::Mutex<HashSet<PathBuf>>>,
    eviction_lock: Arc<tokio::sync::Mutex<()>>,
    event_bus: Arc<std::sync::RwLock<Option<Arc<crate::core::event_bus::EventBus>>>>,
    preparation: Arc<std::sync::Mutex<Option<Arc<Preparation>>>>,
    /// In-memory TTL-aware resolution cache for pre-resolved audio URLs.
    resolution_cache: Arc<ResolutionCache>,
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
        _download_service: Option<Arc<DownloadService>>,
        config: RemoteAudioCacheConfig,
    ) -> Self {
        let stream_cache_dir = base_cache_dir.join("remote-audio");
        let http_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .read_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let jiosaavn = Arc::new(crate::providers::jiosaavn::JioSaavnProvider::new());
        let catalog = pool.as_ref().map(|pool| crate::providers::online_catalog::OnlineCatalog::new(pool.clone(), jiosaavn.clone()));
        Self {
            jiosaavn,
            catalog,
            cache_dir: stream_cache_dir,
            pool,
            http_client,
            config: Arc::new(RwLock::new(config)),
            active_playing_path: Arc::new(RwLock::new(None)),
            active_downloads: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            eviction_lock: Arc::new(tokio::sync::Mutex::new(())),
            event_bus: Arc::new(std::sync::RwLock::new(None)),
            preparation: Arc::new(std::sync::Mutex::new(None)),
            resolution_cache: Arc::new(ResolutionCache::new()),
        }
    }

    /// Returns the active remote-audio cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Returns a reference to the in-memory resolution cache.
    pub fn resolution_cache(&self) -> &Arc<ResolutionCache> {
        &self.resolution_cache
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
        if let Some(preparation) = self.preparation.lock().unwrap().as_ref() {
            protected.extend(preparation.paths.lock().unwrap().iter().cloned());
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
    /// Online audio uses exact JioSaavn identity, its complete cache, then its stream.
    /// Local library files are handled by PlaybackService before this resolver.
    /// No preview URL is treated as playable audio.
    pub fn set_event_bus(&self, bus: Arc<EventBus>) {
        *self.event_bus.write().unwrap() = Some(bus);
    }

    /// Synchronous signal: callers must invoke this before operation locks/draining.
    pub fn request_cancel(&self) {
        if let Some(preparation) = self.preparation.lock().unwrap().as_ref() {
            preparation.cancel();
        }
    }

    fn begin_preparation(&self) -> Arc<Preparation> {
        let preparation = Preparation::new();
        if let Some(old) = self.preparation.lock().unwrap().replace(preparation.clone()) {
            old.cancel();
        }
        preparation
    }

    pub async fn cancel_active_stream(&self) {
        let preparation = self.preparation.lock().unwrap().take();
        if let Some(preparation) = preparation {
            preparation.cancel();
            preparation.drain().await;
        }
    }

    pub async fn can_seek_current(&self) -> bool {
        self.preparation.lock().unwrap().as_ref()
            .and_then(|p| p.state.lock().unwrap().clone())
            .is_none_or(|state| state.can_seek())
    }

    async fn finish_preparation<F>(&self, preparation: &Arc<Preparation>, work: F) -> AppResult<PreparedPlayback>
    where F: std::future::Future<Output = AppResult<PreparedPlayback>> {
        let mut cancelled = preparation.cancelled.subscribe();
        let result = tokio::select! {
            biased;
            _ = async { cancelled.wait_for(|value| *value).await.ok(); } =>
                Err(AppError::Playback("Playback preparation cancelled".into())),
            result = tokio::time::timeout(std::time::Duration::from_secs(30), work) =>
                result.unwrap_or_else(|_| Err(AppError::Playback("Playback preparation timed out after 30 seconds. Retry or choose Next.".into()))),
        };
        if result.is_err() {
            preparation.cancel();
            preparation.drain().await;
            warn!("Playback preparation ended without a source; workers drained");
        }
        result
    }

    async fn jiosaavn_identity(&self, track_id: &str, _title: &str, _artist: &str) -> AppResult<String> {
        if let Some(catalog) = &self.catalog {
            if let Some(source) = catalog.source_id(track_id).await? { return Ok(source); }
        }
        if let Some(id) = track_id.strip_prefix("jiosaavn:") { return Ok(id.into()); }
        Err(AppError::Playback("This song is not available from JioSaavn".into()))
    }

    async fn jiosaavn_enabled(&self) -> AppResult<bool> {
        if let Some(pool) = &self.pool {
            let value: Option<String> = sqlx::query_scalar("SELECT value FROM application_settings WHERE key = 'enable_jiosaavn'").fetch_optional(pool).await?;
            Ok(value.as_deref() != Some("false"))
        } else { Ok(true) }
    }

    async fn audio_candidates(&self, jio_id: &str, refresh: bool) -> AppResult<Vec<(String, f64, Option<crate::downloads::DownloadSearchResult>)>> {
        if !self.jiosaavn_enabled().await? {
            return Err(AppError::Playback("JioSaavn is disabled in Settings; this song is not cached".into()));
        }
        let (url, duration) = self.jiosaavn.resolve(jio_id, refresh).await?;
        info!(provider = "jiosaavn", song_id = jio_id, refresh, "Resolved JioSaavn audio source");
        Ok(vec![(url, duration, None)])
    }

    /// Resolves an audio track for playback, using progressive streaming on cache misses.
    pub async fn resolve_and_prepare_playback(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
    ) -> AppResult<PreparedPlayback> {
        let preparation = self.begin_preparation();
        self.finish_preparation(&preparation, self.resolve_preparation(track_id, title, artist, &preparation)).await
    }

    async fn resolve_preparation(&self, track_id: &str, title: &str, artist: &str, preparation: &Arc<Preparation>) -> AppResult<PreparedPlayback> {
        info!(track_id, title, artist, "Resolving audio for playback");
        let jio_id = self.jiosaavn_identity(track_id, title, artist).await?;

        // 3. Check if cached in stream disk cache (Cache Hit)
        let cache_key = format!("full:v3:jiosaavn:{jio_id}");
        let cache_path = self.get_cache_path(&cache_key);

        if Self::is_valid_cache_entry(&cache_path) && Self::is_decodable_audio(&cache_path).await {
            info!(provider = "jiosaavn", song_id = %jio_id, path = %cache_path.display(), "JioSaavn audio cache hit");
            Self::touch_cache_entry(&cache_path);
            let dur = self.full_cache_duration(&cache_path, track_id).await;
            return Ok(PreparedPlayback::LocalFile { path: cache_path, duration: dur });
        }

        if !self.jiosaavn_enabled().await? {
            return Err(AppError::Playback("JioSaavn is disabled in Settings; this song is not cached".into()));
        }

        // One preparation owns resolution and download, including retries.
        // Replacements cancel the old owner; no detached slot cleanup or
        // completion waiter may mutate a newer request's bookkeeping.
        preparation.paths.lock().unwrap().insert(cache_path.clone());

        // 4a. Check in-memory resolution cache before calling provider.
        let candidates = if let Some(resolved) = self.resolution_cache.get(&cache_key) {
            info!(track_id, "Resolution cache hit — skipping provider call");
            vec![(resolved.url.clone(), resolved.duration, None)]
        } else {
            let raw = self.audio_candidates(&jio_id, false).await?;
            // Store the first (best) candidate in the resolution cache.
            if let Some((url, dur, _)) = raw.first() {
                let expiry = parse_url_expiry(url);
                self.resolution_cache.insert(
                    cache_key.clone(),
                    ResolvedEntry::new(url.clone(), *dur, expiry),
                );
            }
            raw
        };

        if candidates.is_empty() {
            return Err(AppError::Playback(format!(
                "No full-song source available for \"{}\" by \"{}\"", title, artist
            )));
        }

        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| AppError::Io(format!("Failed to create remote-audio cache dir: {}", e)))?;

        let part_path = Self::get_part_path(&cache_path);
        let mut last_error = None;

        for (index, (stream_url, dur, _)) in candidates.into_iter().take(3).enumerate() {
            info!(candidate = index + 1, dur, "Trying full-song progressive source");
            match self.try_start_progressive(&stream_url, &part_path, &cache_path, track_id, dur, preparation).await {
                Ok(prepared) => return Ok(prepared),
                Err(e) => {
                    warn!(candidate = index + 1, error = %e, "Progressive stream candidate failed");
                    last_error = Some(e.to_string());
                    let _ = tokio::fs::remove_file(&part_path).await;
                }
            }
        }

        // If the candidate came from resolution cache and failed (e.g. expired URL),
        // invalidate cache entry and fall back to fresh provider search.
        {
            warn!(track_id, "Cached resolution candidate failed, invalidating resolution cache and retrying via provider");
            self.resolution_cache.invalidate(&cache_key);
            let fresh = self.audio_candidates(&jio_id, true).await?;
            if let Some((url, dur, _)) = fresh.first() {
                let expiry = parse_url_expiry(url);
                self.resolution_cache.insert(
                    cache_key.clone(),
                    ResolvedEntry::new(url.clone(), *dur, expiry),
                );
            }
            for (index, (stream_url, dur, _)) in fresh.into_iter().take(3).enumerate() {
                info!(candidate = index + 1, dur, "Trying fresh progressive source");
                match self.try_start_progressive(&stream_url, &part_path, &cache_path, track_id, dur, preparation).await {
                    Ok(prepared) => return Ok(prepared),
                    Err(e) => {
                        warn!(candidate = index + 1, error = %e, "Fresh progressive stream candidate failed");
                        last_error = Some(e.to_string());
                        let _ = tokio::fs::remove_file(&part_path).await;
                    }
                }
            }
        }

        Err(AppError::Playback(format!(
            "Could not play a full song for \"{}\" by \"{}\": {}",
            title, artist, last_error.unwrap_or_else(|| "all sources failed".to_string())
        )))
    }

    /// Dedicated source resolution API for pre-resolution and fast source lookup.
    ///
    /// Uses the same JioSaavn identity and cache namespace as playback.
    /// Older provider caches and download-folder filename matches are excluded.
    ///
    /// Background pre-resolution must NEVER call code that starts HTTP audio download,
    /// creates `.part` files, modifies playback state, or prepares Rodio.
    pub async fn resolve_source_only(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
    ) -> AppResult<ResolvedAudioSource> {
        info!(track_id, title, artist, "Pre-resolving audio source only");
        let jio_id = self.jiosaavn_identity(track_id, title, artist).await?;

        // 3. Complete disk-cache hit
        let cache_key = format!("full:v3:jiosaavn:{jio_id}");
        let cache_path = self.get_cache_path(&cache_key);

        if Self::is_valid_cache_entry(&cache_path) && Self::is_decodable_audio(&cache_path).await {
            debug!(path = %cache_path.display(), "Disk cache hit during source pre-resolution");
            Self::touch_cache_entry(&cache_path);
            let dur = self.full_cache_duration(&cache_path, track_id).await;
            return Ok(ResolvedAudioSource::LocalFile { path: cache_path, duration: dur });
        }

        if !self.jiosaavn_enabled().await? {
            return Err(AppError::Playback("JioSaavn is disabled in Settings; this song is not cached".into()));
        }

        // 4. In-memory resolution cache hit
        if let Some(resolved) = self.resolution_cache.get(&cache_key) {
            debug!(track_id, "Resolution memory cache hit during source pre-resolution");
            return Ok(ResolvedAudioSource::RemoteStream {
                url: resolved.url,
                duration: resolved.duration,
                from_cache: true,
            });
        }

        // 5. Provider resolution

        let raw = self.audio_candidates(&jio_id, false).await?;
        if let Some((url, dur, _)) = raw.first() {
            let expiry = parse_url_expiry(url);
            self.resolution_cache.insert(
                cache_key.clone(),
                ResolvedEntry::new(url.clone(), *dur, expiry),
            );
            Ok(ResolvedAudioSource::RemoteStream {
                url: url.clone(),
                duration: *dur,
                from_cache: false,
            })
        } else {
            Err(AppError::Playback(format!(
                "No full-song source available for \"{}\" by \"{}\"", title, artist
            )))
        }
    }

    /// Backwards-compatible resolution method returning final path and duration.
    pub async fn resolve_and_prepare_audio(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
    ) -> AppResult<(PathBuf, f64)> {
        match self.resolve_and_prepare_playback(track_id, title, artist).await? {
            PreparedPlayback::LocalFile { path, duration } => Ok((path, duration)),
            PreparedPlayback::ProgressiveStream { target_path, duration, .. } => Ok((target_path, duration)),
        }
    }

    async fn try_start_progressive(
        &self, stream_url: &str, part_path: &Path, target_path: &Path,
        track_id: &str, candidate_dur: f64, preparation: &Arc<Preparation>,
    ) -> AppResult<PreparedPlayback> {
        // Each attempt owns a unique path. An obsolete attempt cannot unlink a
        // newer request's file, even when both resolve to the same cache key.
        let part_path = part_path.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        preparation.paths.lock().unwrap().insert(part_path.clone());
        let response = tokio::time::timeout(std::time::Duration::from_secs(12),
            self.http_client.get(stream_url).header(reqwest::header::USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36").send()).await
            .map_err(|_| AppError::Playback("Audio stream connection timed out (12s)".into()))?
            .map_err(|_| AppError::Playback("Could not connect to audio source".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Playback(format!("Audio stream returned HTTP {}", response.status())));
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_owned();
        if content_type.starts_with("text/") || content_type.contains("json") {
            return Err(AppError::Playback("Audio source returned a page instead of audio".into()));
        }
        let max_size = self.config.read().await.max_single_file_bytes;
        let length = response.content_length().unwrap_or(0);
        if length > max_size { return Err(AppError::Playback("Audio source exceeds cache file limit".into())); }
        let cleanup = PartFileCleanupGuard::new(part_path.clone());
        // Keep creation and cleanup ownership in one synchronous step: dropping
        // a Tokio file-create future can otherwise create a file after cleanup.
        let file = tokio::fs::File::from_std(std::fs::File::create(&part_path)
            .map_err(|e| AppError::Io(e.to_string()))?);
        let state = Arc::new(ProgressiveStreamState::new(track_id.into(), self.event_bus.read().unwrap().clone()));
        state.content_length.store(length, std::sync::atomic::Ordering::Release);
        // Serialize registration with cancellation, including cancellation that
        // arrived while the HTTP headers were being fetched.
        {
            let mut slot = preparation.state.lock().unwrap();
            *slot = Some(state.clone());
            if *preparation.cancelled.borrow() { state.set_cancelled(); }
        }
        let mut cancelled = preparation.cancelled.subscribe();
        let producer_state = state.clone();
        let target = target_path.to_path_buf();
        let producer_path = part_path.clone();
        let manager = self.clone_for_bg();
        // Install the producer before any synchronous decoder can request bytes.
        *preparation.producer.lock().await = Some(tokio::spawn(async move {
            let mut cleanup = cleanup;
            let mut file = file;
            let mut response = response;
            let mut total = 0u64;
            let download: AppResult<()> = async {
                loop {
                    let chunk = tokio::select! {
                        biased;
                        _ = async { cancelled.wait_for(|value| *value).await.ok(); } =>
                            return Err(AppError::Playback("Stream cancelled".into())),
                        chunk = tokio::time::timeout(std::time::Duration::from_secs(25), response.chunk()) =>
                            chunk.map_err(|_| AppError::Playback("Audio download inactive for 25 seconds".into()))?
                                .map_err(|_| AppError::Playback("Audio download interrupted or truncated".into()))?,
                    };
                    let Some(chunk) = chunk else { break };
                    total += chunk.len() as u64;
                    if total > max_size { return Err(AppError::Playback("Audio exceeds cache file limit".into())); }
                    file.write_all(&chunk).await.map_err(|e| AppError::Io(e.to_string()))?;
                    file.flush().await.map_err(|e| AppError::Io(e.to_string()))?;
                    producer_state.update_downloaded(total);
                }
                if total == 0 || (length > 0 && total != length) {
                    return Err(AppError::Playback("Audio download empty or truncated".into()));
                }
                if producer_state.is_cancelled() { return Err(AppError::Playback("Stream cancelled".into())); }
                Ok(())
            }.await;
            drop(file); // Stop writing before either finalizing or removing the partial file.
            match download {
                Ok(()) => match tokio::fs::rename(&producer_path, &target).await {
                    Ok(()) => {
                        cleanup.disarm();
                        producer_state.set_completed(); // Real EOF wakes probing readers, too.
                        Self::touch_cache_entry(&target);
                        manager.spawn_cache_eviction();
                    }
                    Err(_) => producer_state.set_failed("Could not finalize audio cache".into()),
                },
                Err(error) => producer_state.set_failed(error.to_string()),
            }
        }));
        let reader = ProgressiveStreamReader::new(&part_path, state.clone())
            .or_else(|_| ProgressiveStreamReader::new(target_path, state.clone()))
            .map_err(|e| AppError::Io(e.to_string()))?;
        let mut hint = symphonia::core::probe::Hint::new();
        if content_type.contains("webm") { hint.with_extension("webm"); }
        else if content_type.contains("ogg") || content_type.contains("opus") { hint.with_extension("ogg"); }
        let desc = format!("progressive:{track_id}");
        let mut probe = preparation.probe.lock().await;
        *probe = Some(tokio::task::spawn_blocking(move || SymphoniaSource::from_media_source(Box::new(reader), hint, &desc)));
        let decoded = probe.as_mut().unwrap().await
            .map_err(|_| AppError::Playback("Audio decoder worker failed".into()))?;
        probe.take();
        let source = match decoded {
            Ok(source) => source,
            Err(error) => {
                state.set_cancelled();
                // End this candidate's producer before a retry owns a new file.
                if let Some(task) = preparation.producer.lock().await.take() {
                    task.abort(); // Async HTTP producer only; never abort the blocking probe.
                    let _ = task.await;
                }
                return Err(error);
            }
        };
        if state.is_cancelled() { return Err(AppError::Playback("Playback preparation cancelled".into())); }
        info!(track_id, "Playback preparation decoded initial audio");
        Ok(PreparedPlayback::ProgressiveStream {
            source: PlayerSource::Symphonia(source), part_path,
            target_path: target_path.into(), duration: candidate_dur, state,
        })
    }

    async fn is_decodable_audio(path: &Path) -> bool {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || SymphoniaSource::new(&path).is_ok()).await.unwrap_or(false)
    }

    async fn full_cache_duration(&self, path: &Path, track_id: &str) -> f64 {
        use rodio::Source;
        let path = path.to_path_buf();
        let decoded = tokio::task::spawn_blocking(move || SymphoniaSource::new(&path)
            .ok()
            .and_then(|source| source.total_duration())
            .map(|duration| duration.as_secs_f64())
            .filter(|duration| *duration > 30.0)).await.ok().flatten();
        if let Some(duration) = decoded { return duration; }
        self.get_known_duration(track_id).await.filter(|duration| *duration > 30.0).unwrap_or(210.0)
    }

    fn spawn_cache_eviction(&self) {
        let manager = self.clone_for_bg();
        tokio::spawn(async move { let _ = manager.enforce_cache_limits().await; });
    }

    /// Downloads the remote audio stream to a `.part` file, verifies size, and atomically renames it.
    #[allow(dead_code)]
    async fn download_and_cache(&self, url: &str, target_path: &Path) -> AppResult<()> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| AppError::Io(format!("Failed to create remote-audio cache dir: {}", e)))?;

        let part_path = Self::get_part_path(target_path);
        let mut guard = PartFileCleanupGuard::new(part_path.clone());

        let request = self
            .http_client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            );

        let response = request
            .send()
            .await
            .map_err(|e| {
                #[cfg(test)] eprintln!("fixture connection diagnostic: {e:?}");
                AppError::Playback(format!("Failed to connect to audio stream: {}", e))
            })?;

        let status = response.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(AppError::Playback(format!(
                "Audio stream returned HTTP {}",
                status
            )));
        }
        if response.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|content_type| content_type.starts_with("text/") || content_type.contains("json"))
        {
            return Err(AppError::Playback("Audio source returned a page instead of audio".to_string()));
        }

        let config = self.config.read().await.clone();
        let range_total = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            response.headers().get(reqwest::header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| {
                    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
                    if !range.starts_with("0-") { return None; }
                    total.parse::<u64>().ok()
                })
        } else { None };
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
        if let Some(expected_bytes) = range_total {
            if downloaded_bytes != expected_bytes {
                return Err(AppError::Playback(format!(
                    "Incomplete ranged audio stream: received {} of {} bytes",
                    downloaded_bytes, expected_bytes
                )));
            }
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

        for dir in &[&self.cache_dir] {
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

    /// Cleans stale partials and enforces limits without importing older provider audio.
    pub async fn startup_maintenance(&self) {
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

        for dir in &[&self.cache_dir] {
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

        for dir in &[&self.cache_dir] {
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
            jiosaavn: self.jiosaavn.clone(),
            catalog: self.catalog.clone(),
            cache_dir: self.cache_dir.clone(),
            pool: self.pool.clone(),
            http_client: self.http_client.clone(),
            config: self.config.clone(),
            active_playing_path: self.active_playing_path.clone(),
            active_downloads: self.active_downloads.clone(),
            eviction_lock: self.eviction_lock.clone(),
            event_bus: self.event_bus.clone(),
            preparation: self.preparation.clone(),
            resolution_cache: self.resolution_cache.clone(),
        })
    }
}

#[cfg(test)]
#[path = "stream_preparation_tests.rs"]
mod preparation_tests;
