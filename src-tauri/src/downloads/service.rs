use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus, DownloadTags};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::models::DownloadTaskRecord;
use crate::database::repositories::{DownloadRepository, OnlineRecording};
use crate::discovery::WishlistManager;
use crate::library::{parse_download_stem, LibraryService};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

mod folder;
mod removal;

use folder::{canonical_file, file_exists, in_folder, is_download_file, source_of};

const MISSING_MESSAGE: &str = "File was moved or deleted. Download it again.";

/// Everything needed to finish a transfer, shared by the service and its pollers.
#[derive(Clone)]
struct Finisher {
    repo: Arc<dyn DownloadRepository>,
    event_bus: Arc<EventBus>,
    library_service: Arc<LibraryService>,
    wishlist_manager: Arc<WishlistManager>,
    download_dir: PathBuf,
    auto_import: bool,
}

impl Finisher {
    /// The single completion path: import the file, mark the task, notify.
    async fn finish(&self, task: &DownloadTaskRecord) -> AppResult<()> {
        self.finish_with(task, true).await
    }

    /// `announce` is false when quietly re-adopting files found on disk.
    async fn finish_with(&self, task: &DownloadTaskRecord, announce: bool) -> AppResult<()> {
        let path = task
            .destination_path
            .clone()
            .unwrap_or_else(|| self.download_dir.join(&task.filename).to_string_lossy().into());
        if let Ok(meta) = std::fs::metadata(&path) {
            let _ = self.repo.update_progress(&task.id, meta.len() as i64).await;
        }
        self.import(Path::new(&path)).await;
        self.repo
            .update_status(&task.id, "COMPLETED", Some(&path), None, Some(Utc::now().timestamp()))
            .await?;
        if let Some(wl_id) = &task.wishlist_id {
            let _ = self.wishlist_manager.update_status(wl_id, "DOWNLOADED").await;
        }
        info!(task_id = %task.id, file = %path, "Download completed");
        if announce {
            let _ = self.event_bus.publish(Event::DownloadCompleted {
                task_id: task.id.clone(),
                file_path: path,
            });
        }
        Ok(())
    }

    /// Adds just this file to the library, never a full rescan.
    async fn import(&self, path: &Path) {
        if !self.auto_import {
            return;
        }
        if let Err(error) = self.library_service.import_file(path).await {
            warn!(%error, file = %path.display(), "Could not import downloaded song");
        }
    }

    async fn fail(&self, task_id: &str, message: &str) {
        let _ = self
            .repo
            .update_status(task_id, "FAILED", None, Some(message), None)
            .await;
        let _ = self.event_bus.publish(Event::DownloadFailed {
            task_id: task_id.to_string(),
            error: message.to_string(),
        });
    }
}

pub struct DownloadService {
    repo: Arc<dyn DownloadRepository>,
    provider: Arc<dyn DownloadProvider>,
    event_bus: Arc<EventBus>,
    library_service: Arc<LibraryService>,
    download_dir: PathBuf,
    finisher: Finisher,
    search_cache: Arc<RwLock<HashMap<String, DownloadSearchResult>>>,
    direct_gate: tokio::sync::Mutex<()>,
    recovery: tokio::sync::OnceCell<()>,
}

impl DownloadService {
    pub fn new(
        repo: Arc<dyn DownloadRepository>,
        provider: Arc<dyn DownloadProvider>,
        event_bus: Arc<EventBus>,
        library_service: Arc<LibraryService>,
        wishlist_manager: Arc<WishlistManager>,
        download_dir: PathBuf,
        auto_import: bool,
    ) -> Self {
        let finisher = Finisher {
            repo: repo.clone(),
            event_bus: event_bus.clone(),
            library_service: library_service.clone(),
            wishlist_manager,
            download_dir: download_dir.clone(),
            auto_import,
        };
        Self {
            repo,
            provider,
            event_bus,
            library_service,
            download_dir,
            finisher,
            search_cache: Arc::new(RwLock::new(HashMap::new())),
            direct_gate: tokio::sync::Mutex::new(()),
            recovery: tokio::sync::OnceCell::new(),
        }
    }

    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }

    pub fn provider(&self) -> Arc<dyn DownloadProvider> {
        self.provider.clone()
    }

    /// Dispatches a search query to the download network and caches the results.
    pub async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        info!(query = %query, provider = self.provider.name(), "Searching download network");
        let results = self.provider.search(query).await?;

        let mut cache = self.search_cache.write().await;
        for res in &results {
            cache.insert(res.id.clone(), res.clone());
        }

        Ok(results)
    }

    pub async fn search_track(
        &self,
        artist: &str,
        title: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        let results = self.provider.search_track(artist, title).await?;
        let mut cache = self.search_cache.write().await;
        for result in &results {
            cache.insert(result.id.clone(), result.clone());
        }
        Ok(results)
    }

    /// Searches the network for a specific wishlist item.
    pub async fn search_wishlist_item(
        &self,
        wishlist_id: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        let item = self
            .finisher
            .wishlist_manager
            .get_by_id(wishlist_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Wishlist item {} not found", wishlist_id))
            })?;

        self.search_track(&item.artist, &item.title).await
    }

    /// Download the displayed recording; never search its title again.
    /// A file already on disk for this recording is adopted instead of re-fetched.
    pub async fn download_track(
        &self,
        track: &crate::providers::ExternalTrackMetadata,
    ) -> AppResult<DownloadTaskRecord> {
        let _guard = self.direct_gate.lock().await;
        let source = track.provider_track_id.as_str();
        let result = DownloadSearchResult {
            id: format!("jiosaavn:{source}"),
            provider: "jiosaavn".into(),
            username: track.artist_name.clone(),
            filename: format!(
                "{} - {} ({}).m4a",
                super::jiosaavn::safe_name(&track.artist_name),
                super::jiosaavn::safe_name(&track.title),
                super::jiosaavn::safe_name(source)
            ),
            file_size: 0,
            bitrate: None,
            sample_rate: None,
            format: "m4a".into(),
            slots_free: true,
            speed_bps: 0,
            tags: Some(DownloadTags {
                title: track.title.clone(),
                artist: track.artist_name.clone(),
                album: track.album_title.clone(),
                year: track.year.and_then(|y| u32::try_from(y).ok()),
                source_id: Some(source.to_string()),
            }),
        };
        let tasks = self.list_downloads(None, 10000).await?;
        let mine = || {
            tasks.iter().filter(|task| {
                task.provider == result.provider && task.source_track_id.as_deref() == Some(source)
            })
        };
        if let Some(task) = mine().find(|task| {
            matches!(task.status.as_str(), "DOWNLOADING" | "QUEUED")
                || (task.status == "COMPLETED" && file_exists(task.destination_path.as_deref()))
        }) {
            return Ok(task.clone());
        }
        if let Some(path) = self.file_for_source(source, &result.filename).await {
            let existing = mine().next().cloned();
            return self.adopt(&path, source, existing, true).await;
        }
        self.search_cache
            .write()
            .await
            .insert(result.id.clone(), result.clone());
        self.start_download(&result.id, None).await
    }

    /// The saved file for a recording: its expected name, or any `… (source).m4a`.
    async fn file_for_source(&self, source: &str, filename: &str) -> Option<PathBuf> {
        let expected = self.download_dir.join(filename);
        if expected.is_file() {
            return Some(expected);
        }
        let mut entries = tokio::fs::read_dir(&self.download_dir).await.ok()?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !is_download_file(&path) {
                continue;
            }
            if source_of(&path).await.as_deref() == Some(source) {
                return Some(path);
            }
        }
        None
    }

    /// Records an existing file as this recording's completed download.
    async fn adopt(
        &self,
        path: &Path,
        source: &str,
        existing: Option<DownloadTaskRecord>,
        announce: bool,
    ) -> AppResult<DownloadTaskRecord> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let size = std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
        let task = match existing {
            Some(mut task) => {
                let new_path = path.to_string_lossy().to_string();
                // A renamed file keeps its library row (history, likes, playlists).
                if let Some(old) = task.destination_path.as_deref().filter(|old| *old != new_path && !Path::new(old).exists()) {
                    let new_track = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                    self.repo
                        .relocate(&task.id, &new_path, old, &new_track.to_string_lossy())
                        .await?;
                }
                task.destination_path = Some(new_path);
                task.filename = filename;
                task
            }
            None => {
                let named = parse_download_stem(
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or_default(),
                );
                let now = Utc::now().timestamp();
                let task = DownloadTaskRecord {
                    source_track_id: Some(source.to_string()),
                    id: format!("dl_{}", Uuid::new_v4()),
                    provider: "jiosaavn".into(),
                    provider_task_id: None,
                    title: named.as_ref().map(|n| n.title.clone()).unwrap_or_else(|| filename.clone()),
                    artist: named.map(|n| n.artist).unwrap_or_else(|| "Unknown Artist".into()),
                    album: None,
                    filename,
                    destination_path: Some(path.to_string_lossy().into()),
                    file_size: Some(size),
                    bytes_downloaded: size,
                    status: "COMPLETED".into(),
                    error_message: None,
                    wishlist_id: None,
                    created_at: now,
                    completed_at: Some(now),
                };
                self.repo.create_task(&task).await?;
                task
            }
        };
        info!(task_id = %task.id, file = %path.display(), "Adopted existing download");
        self.finisher.finish_with(&task, announce).await?;
        Ok(self.repo.get_task_by_id(&task.id).await?.unwrap_or(task))
    }

    /// Enqueues a download task based on a cached search result.
    pub async fn start_download(
        &self,
        search_result_id: &str,
        wishlist_id: Option<String>,
    ) -> AppResult<DownloadTaskRecord> {
        let result = {
            let cache = self.search_cache.read().await;
            cache.get(search_result_id).cloned().ok_or_else(|| {
                AppError::NotFound(format!(
                    "Search result {} expired or not found. Please search again.",
                    search_result_id
                ))
            })?
        };

        // Create download directory if not present
        if !self.download_dir.exists() {
            tokio::fs::create_dir_all(&self.download_dir)
                .await
                .map_err(|e| AppError::Io(e.to_string()))?;
        }

        // Start transfer on the provider
        let provider_task_id = self
            .provider
            .start_download(&result, &self.download_dir)
            .await?;

        let now = Utc::now().timestamp();
        let task_id = format!("dl_{}", Uuid::new_v4());
        let dest_path = self.download_dir.join(&result.filename);
        let dest_str = dest_path.to_string_lossy().to_string();

        // Extract title and artist if parseable from filename, else fallback
        let (artist, mut title) = parse_artist_title_from_filename(&result.filename);
        let source_track_id = (result.provider == "jiosaavn")
            .then(|| result.id.strip_prefix("jiosaavn:").map(str::to_string))
            .flatten();
        if let Some(source) = &source_track_id {
            if let Some(clean) = title.strip_suffix(&format!(" ({source})")) {
                title = clean.to_string();
            }
        }

        let task = DownloadTaskRecord {
            source_track_id,
            id: task_id.clone(),
            provider: result.provider.clone(),
            provider_task_id: Some(provider_task_id),
            title,
            artist,
            album: None,
            filename: result.filename.clone(),
            destination_path: Some(dest_str.clone()),
            file_size: Some(result.file_size),
            bytes_downloaded: 0,
            status: "DOWNLOADING".to_string(),
            error_message: None,
            wishlist_id,
            created_at: now,
            completed_at: None,
        };

        self.repo.create_task(&task).await?;

        info!(task_id = %task.id, filename = %task.filename, "Enqueued download task");
        let _ = self.event_bus.publish(Event::DownloadQueued {
            task_id: task.id.clone(),
            title: task.title.clone(),
            artist: task.artist.clone(),
        });

        // Spawn background polling task to track transfer progress, auto-import on completion,
        // and stream progress events without requiring client-side polling loops.
        let repo = self.repo.clone();
        let provider = self.provider.clone();
        let event_bus = self.event_bus.clone();
        let finisher = self.finisher.clone();
        let task_id_clone = task.id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            loop {
                interval.tick().await;
                let current_task = match repo.get_task_by_id(&task_id_clone).await {
                    Ok(Some(t)) => t,
                    _ => break,
                };

                if current_task.status != "DOWNLOADING" && current_task.status != "QUEUED" {
                    break;
                }

                let pt_id = match current_task.provider_task_id.as_deref() {
                    Some(id) => id,
                    None => break,
                };

                match provider.get_progress(pt_id).await {
                    Ok(Some(progress)) => {
                        let _ = repo
                            .update_progress(&task_id_clone, progress.bytes_downloaded)
                            .await;
                        let _ = event_bus.publish(Event::DownloadProgressChanged {
                            task_id: task_id_clone.clone(),
                            bytes_downloaded: progress.bytes_downloaded,
                            total_bytes: progress.total_bytes,
                            speed_bps: progress.speed_bps,
                        });

                        if progress.status == DownloadStatus::Completed {
                            if let Err(error) = finisher.finish(&current_task).await {
                                warn!(%error, "Could not record finished download");
                            }
                            break;
                        } else if progress.status == DownloadStatus::Failed {
                            finisher.fail(&task_id_clone, "Transfer failed").await;
                            break;
                        } else if progress.status == DownloadStatus::Cancelled {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "Error polling download progress");
                    }
                }
            }
        });

        Ok(task)
    }

    /// Polls transfer progress for an active task, advancing status when finished.
    pub async fn poll_task(&self, task_id: &str) -> AppResult<Option<DownloadProgress>> {
        let task = match self.repo.get_task_by_id(task_id).await? {
            Some(t) => t,
            None => return Ok(None),
        };

        if task.status != "DOWNLOADING" && task.status != "QUEUED" {
            return Ok(None);
        }

        let provider_id = match task.provider_task_id {
            Some(ref id) => id,
            None => return Ok(None),
        };

        let progress = match self.provider.get_progress(provider_id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        // Update database progress
        self.repo
            .update_progress(task_id, progress.bytes_downloaded)
            .await?;

        let _ = self.event_bus.publish(Event::DownloadProgressChanged {
            task_id: task_id.to_string(),
            bytes_downloaded: progress.bytes_downloaded,
            total_bytes: progress.total_bytes,
            speed_bps: progress.speed_bps,
        });

        if progress.status == DownloadStatus::Completed {
            self.finisher.finish(&task).await?;
        } else if progress.status == DownloadStatus::Failed {
            self.finisher.fail(task_id, "Transfer failed on network").await;
        }

        Ok(Some(progress))
    }

    /// Marks a download task completed, imports its file and updates its wishlist item.
    pub async fn complete_task(&self, _task_id: &str, task: &DownloadTaskRecord) -> AppResult<()> {
        self.finisher.finish(task).await
    }

    /// Cancels an active download task.
    pub async fn cancel_download(&self, task_id: &str) -> AppResult<()> {
        let task =
            self.repo.get_task_by_id(task_id).await?.ok_or_else(|| {
                AppError::NotFound(format!("Download task {} not found", task_id))
            })?;

        if let Some(ref pt_id) = task.provider_task_id {
            let _ = self.provider.cancel(pt_id).await;
        }

        self.repo
            .update_status(task_id, "CANCELLED", None, None, None)
            .await?;

        info!(task_id = %task_id, "Cancelled download task");
        Ok(())
    }

    /// Lists tracked download tasks after reconciling them with the disk.
    pub async fn list_downloads(
        &self,
        status_filter: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<DownloadTaskRecord>> {
        self.recovery
            .get_or_try_init(|| async {
                self.recover_interrupted().await?;
                if let Err(error) = self.relocate_into_folder().await {
                    warn!(%error, "Could not move downloads into the Kaze folder");
                }
                self.adopt_orphans().await
            })
            .await?;
        self.reconcile_files().await?;
        self.repo.list_tasks(status_filter, limit).await
    }

    /// Provider jobs live in this process. Unknown persisted jobs cannot resume.
    async fn recover_interrupted(&self) -> AppResult<()> {
        for task in self.repo.list_tasks(None, 10000).await? {
            if !matches!(task.status.as_str(), "DOWNLOADING" | "QUEUED") {
                continue;
            }
            let missing = match task.provider_task_id.as_deref() {
                Some(id) => self.provider.get_progress(id).await?.is_none(),
                None => true,
            };
            if missing {
                let message = "Download interrupted by an app restart. Retry this song.";
                if self.repo.fail_interrupted_task(&task.id, message).await? {
                    let _ = self.event_bus.publish(Event::DownloadFailed {
                        task_id: task.id,
                        error: message.into(),
                    });
                }
            }
        }
        Ok(())
    }

}

/// Helper parsing "Artist - Title.ext" or "Title.ext" from a filename string.
fn parse_artist_title_from_filename(filename: &str) -> (String, String) {
    let clean = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    if let Some((artist, title)) = clean.split_once(" - ") {
        (artist.trim().to_string(), title.trim().to_string())
    } else {
        ("Unknown Artist".to_string(), clean.to_string())
    }
}
