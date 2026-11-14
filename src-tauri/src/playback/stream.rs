//! Resolving online songs to playable audio: exact JioSaavn identity, the
//! complete cache, then a progressive stream. Cache upkeep lives in `cache`.
use crate::core::sync::{MutexExt, RwLockExt};
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

use crate::core::event_bus::EventBus;
use crate::playback::decoder::{PlayerSource, SymphoniaSource};
use crate::playback::progressive::{ProgressiveStreamReader, ProgressiveStreamState};

mod cache;
mod progressive;

pub use cache::{
    ClearCacheResult, EvictionResult, PartFileCleanupGuard, RemoteAudioCacheConfig, RemoteAudioCacheStats,
    DEFAULT_EVICTION_TARGET_BYTES, DEFAULT_MAX_CACHE_BYTES, DEFAULT_MAX_SINGLE_FILE_BYTES,
    DEFAULT_STALE_PART_MAX_AGE_SECS,
};

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
        if let Some(state) = self.state.lock_or_recover().as_ref() {
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

    /// Resolves an audio track to a playable local file path and duration.
    ///
    /// Online audio uses exact JioSaavn identity, its complete cache, then its stream.
    /// Local library files are handled by PlaybackService before this resolver.
    /// No preview URL is treated as playable audio.
    pub fn set_event_bus(&self, bus: Arc<EventBus>) {
        *self.event_bus.write_or_recover() = Some(bus);
    }

    /// Synchronous signal: callers must invoke this before operation locks/draining.
    pub fn request_cancel(&self) {
        if let Some(preparation) = self.preparation.lock_or_recover().as_ref() {
            preparation.cancel();
        }
    }

    fn begin_preparation(&self) -> Arc<Preparation> {
        let preparation = Preparation::new();
        if let Some(old) = self.preparation.lock_or_recover().replace(preparation.clone()) {
            old.cancel();
        }
        preparation
    }

    pub async fn cancel_active_stream(&self) {
        let preparation = self.preparation.lock_or_recover().take();
        if let Some(preparation) = preparation {
            preparation.cancel();
            preparation.drain().await;
        }
    }

    pub async fn can_seek_current(&self) -> bool {
        self.preparation.lock_or_recover().as_ref()
            .and_then(|p| p.state.lock_or_recover().clone())
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
        preparation.paths.lock_or_recover().insert(cache_path.clone());

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
