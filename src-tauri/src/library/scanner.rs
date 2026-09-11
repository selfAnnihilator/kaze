use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::repositories::{
    LibraryFolderRepository, ScannedMetadata, TrackRepository,
};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Set of supported audio file extensions.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "aac", "wav",
];

/// Checks if a given file has a supported audio extension.
pub fn is_supported_audio_format(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            SUPPORTED_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Enforces strict path containment: ensures candidate path is inside root path.
/// Does not allow directory traversal or escaping outside the designated root.
pub fn is_within_boundary(root: &Path, candidate: &Path) -> bool {
    let canonical_root = match root.canonicalize() {
        Ok(r) => r,
        Err(_) => return false,
    };
    let canonical_candidate = match candidate.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            // For files that might not yet be canonicalized or parent checks
            if let Some(parent) = candidate.parent() {
                if let Ok(cp) = parent.canonicalize() {
                    return cp.starts_with(&canonical_root);
                }
            }
            return false;
        }
    };
    canonical_candidate.starts_with(&canonical_root)
}

/// Scanner statistics outcome.
#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    pub added_tracks: usize,
    pub updated_tracks: usize,
    pub unchanged_tracks: usize,
    pub removed_tracks: usize,
    pub duration_ms: u64,
}

pub struct LibraryScanner {
    track_repo: Arc<dyn TrackRepository>,
    folder_repo: Arc<dyn LibraryFolderRepository>,
    event_bus: Arc<EventBus>,
    is_cancelled: Arc<AtomicBool>,
}

impl LibraryScanner {
    pub fn new(
        track_repo: Arc<dyn TrackRepository>,
        folder_repo: Arc<dyn LibraryFolderRepository>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            track_repo,
            folder_repo,
            event_bus,
            is_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }

    pub fn reset_cancel(&self) {
        self.is_cancelled.store(false, Ordering::SeqCst);
    }

    /// Recursively scans a designated folder, strictly bounded to root and its subdirectories.
    pub async fn scan_folder(
        &self,
        folder_id: &str,
        folder_path_str: &str,
        incremental: bool,
    ) -> AppResult<ScanSummary> {
        let start_time = Instant::now();
        self.reset_cancel();

        let root_path = PathBuf::from(folder_path_str);
        if !root_path.exists() || !root_path.is_dir() {
            return Err(AppError::Library(format!(
                "Directory does not exist or is not a folder: {}",
                folder_path_str
            )));
        }

        let canonical_root = root_path.canonicalize().map_err(|e| {
            AppError::Library(format!("Failed to canonicalize folder path: {}", e))
        })?;

        info!(
            folder = %canonical_root.display(),
            incremental = incremental,
            "Initiating library directory scan with strict boundary containment"
        );

        let _ = self.event_bus.publish(Event::LibraryScanStarted {
            folder_path: canonical_root.to_string_lossy().to_string(),
        });

        let mut discovered_files: HashSet<String> = HashSet::new();
        let mut added_tracks = 0;
        let mut updated_tracks = 0;
        let mut unchanged_tracks = 0;
        let mut scanned_count = 0;

        // Walk directory: follow_links is false to prevent escaping boundaries via symlinks
        let walker = WalkDir::new(&canonical_root)
            .follow_links(false)
            .into_iter();

        for entry_res in walker {
            if self.is_cancelled.load(Ordering::SeqCst) {
                info!("Library scan cancelled by user or processor");
                break;
            }

            let entry = match entry_res {
                Ok(e) => e,
                Err(err) => {
                    warn!("Skipping unreadable filesystem entry: {}", err);
                    continue;
                }
            };

            let path = entry.path();
            // Strict containment verification: path must be inside canonical_root
            if !path.starts_with(&canonical_root) {
                warn!(
                    path = %path.display(),
                    root = %canonical_root.display(),
                    "Skipping path outside root boundary"
                );
                continue;
            }

            if !entry.file_type().is_file() || !is_supported_audio_format(path) {
                continue;
            }

            let canonical_file_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => path.to_path_buf(),
            };

            // Re-verify canonical path stays inside root
            if !canonical_file_path.starts_with(&canonical_root) {
                warn!(
                    path = %canonical_file_path.display(),
                    "Canonical file path escaped boundary; skipping"
                );
                continue;
            }

            let file_path_str = canonical_file_path.to_string_lossy().to_string();
            discovered_files.insert(file_path_str.clone());
            scanned_count += 1;

            if scanned_count % 50 == 0 {
                let _ = self.event_bus.publish(Event::LibraryScanProgress {
                    scanned_files: scanned_count,
                    total_files: 0,
                    current_file: file_path_str.clone(),
                });
            }

            // File metadata for incremental check
            let fs_meta = match std::fs::metadata(&canonical_file_path) {
                Ok(m) => m,
                Err(err) => {
                    warn!(file = %file_path_str, "Could not read file metadata: {}", err);
                    continue;
                }
            };

            let file_size = fs_meta.len() as i64;
            let modified_timestamp = fs_meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // Incremental check: if file size & mtime match, skip re-parsing tags!
            if incremental {
                if let Ok(Some(existing)) = self.track_repo.find_by_path(&file_path_str).await {
                    if existing.file_size == file_size
                        && existing.modified_timestamp == modified_timestamp
                    {
                        unchanged_tracks += 1;
                        continue;
                    }
                }
            }

            // Parse audio tags with lofty
            match Self::extract_metadata(&canonical_file_path, file_size, modified_timestamp) {
                Ok(parsed_meta) => {
                    let is_update = self
                        .track_repo
                        .find_by_path(&file_path_str)
                        .await
                        .ok()
                        .flatten()
                        .is_some();

                    if let Err(err) = self.track_repo.save_scanned_track(parsed_meta).await {
                        warn!(file = %file_path_str, "Failed to persist track to database: {}", err);
                    } else if is_update {
                        updated_tracks += 1;
                    } else {
                        added_tracks += 1;
                    }
                }
                Err(err) => {
                    warn!(file = %file_path_str, "Failed to extract audio metadata: {}", err);
                }
            }
        }

        // Detect and prune deleted tracks that existed under this folder
        let mut removed_tracks = 0;
        let prefix = canonical_root.to_string_lossy().to_string();
        if let Ok(existing_paths) = self.track_repo.get_all_paths_in_folder_prefix(&prefix).await {
            for existing_path in existing_paths {
                if !discovered_files.contains(&existing_path) {
                    debug!(file = %existing_path, "Track deleted from disk; pruning from library database");
                    if self.track_repo.delete_by_path(&existing_path).await.is_ok() {
                        removed_tracks += 1;
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Update folder's last_scanned_at
        let now_ts = chrono::Utc::now().timestamp();
        let _ = self.folder_repo.update_last_scanned(folder_id, now_ts).await;

        let summary = ScanSummary {
            added_tracks,
            updated_tracks,
            unchanged_tracks,
            removed_tracks,
            duration_ms,
        };

        info!(
            folder = %canonical_root.display(),
            ?summary,
            "Completed library directory scan"
        );

        let _ = self.event_bus.publish(Event::LibraryScanCompleted {
            added_tracks,
            updated_tracks,
            removed_tracks,
            duration_ms,
        });

        Ok(summary)
    }

    /// Extracts audio metadata from an audio file using lofty.
    pub fn extract_metadata(
        path: &Path,
        file_size: i64,
        modified_timestamp: i64,
    ) -> AppResult<ScannedMetadata> {
        let probe = Probe::open(path)
            .map_err(|e| AppError::Library(format!("Probe open error: {}", e)))?
            .guess_file_type()
            .map_err(|e| AppError::Library(format!("Format guess error: {}", e)))?;

        let format_str = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();

        let tagged_file = probe
            .read()
            .map_err(|e| AppError::Library(format!("Audio tag read error: {}", e)))?;

        let properties = tagged_file.properties();
        let duration_secs = properties.duration().as_secs_f64();
        let bitrate = properties.audio_bitrate();
        let sample_rate = properties.sample_rate();

        let primary_tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        let title = primary_tag
            .and_then(|t| t.title().as_deref().map(str::to_string))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown Title")
                    .to_string()
            });

        let artist = primary_tag
            .and_then(|t| t.artist().as_deref().map(str::to_string))
            .filter(|s| !s.trim().is_empty());

        let album = primary_tag
            .and_then(|t| t.album().as_deref().map(str::to_string))
            .filter(|s| !s.trim().is_empty());

        let genre = primary_tag
            .and_then(|t| t.genre().as_deref().map(str::to_string))
            .filter(|s| !s.trim().is_empty());

        let track_number = primary_tag.and_then(|t| t.track());
        let disc_number = primary_tag.and_then(|t| t.disk());
        let year = primary_tag.and_then(|t| t.year());

        let has_cover_art = primary_tag
            .map(|t| !t.pictures().is_empty())
            .unwrap_or(false);

        Ok(ScannedMetadata {
            file_path: path.to_string_lossy().to_string(),
            file_size,
            modified_timestamp,
            file_hash: None,
            title,
            artist,
            album,
            album_artist: None,
            genre,
            track_number,
            disc_number,
            year,
            duration_secs,
            bitrate,
            sample_rate,
            format: format_str,
            has_cover_art,
            musicbrainz_track_id: None,
        })
    }
}
