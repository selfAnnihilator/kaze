//! The remote-audio cache: its limits, stats and maintenance (LRU eviction,
//! stale partial files, clearing) and the paths that must never be evicted.
use super::*;

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


impl StreamPlaybackManager {
    /// Retrieves all protected paths (currently playing file, active downloads, and their .part files).
    pub(super) async fn get_protected_paths(&self) -> HashSet<PathBuf> {
        let mut protected = HashSet::new();
        if let Some(ref path) = *self.active_playing_path.read().await {
            protected.insert(path.clone());
        }
        let active = self.active_downloads.lock().await;
        for path in active.iter() {
            protected.insert(path.clone());
            protected.insert(Self::get_part_path(path));
        }
        if let Some(preparation) = self.preparation.lock_or_recover().as_ref() {
            protected.extend(preparation.paths.lock_or_recover().iter().cloned());
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

    pub(super) fn spawn_cache_eviction(&self) {
        let manager = self.clone_for_bg();
        tokio::spawn(async move { let _ = manager.enforce_cache_limits().await; });
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
                                if age >= max_age
                                    && tokio::fs::remove_file(&path).await.is_ok() {
                                        removed_count += 1;
                                        info!(path = %path.display(), age_secs = age.as_secs(), "Removed stale partial audio cache file");
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
}
