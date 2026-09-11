use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::models::DownloadTaskRecord;
use crate::database::repositories::DownloadRepository;
use crate::discovery::WishlistManager;
use crate::library::LibraryService;
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

pub struct DownloadService {
    repo: Arc<dyn DownloadRepository>,
    provider: Arc<dyn DownloadProvider>,
    event_bus: Arc<EventBus>,
    library_service: Arc<LibraryService>,
    wishlist_manager: Arc<WishlistManager>,
    download_dir: PathBuf,
    auto_import: bool,
    search_cache: Arc<RwLock<HashMap<String, DownloadSearchResult>>>,
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
        Self {
            repo,
            provider,
            event_bus,
            library_service,
            wishlist_manager,
            download_dir,
            auto_import,
            search_cache: Arc::new(RwLock::new(HashMap::new())),
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

    /// Searches the network for a specific wishlist item.
    pub async fn search_wishlist_item(&self, wishlist_id: &str) -> AppResult<Vec<DownloadSearchResult>> {
        let item = self
            .wishlist_manager
            .get_by_id(wishlist_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Wishlist item {} not found", wishlist_id)))?;

        let query = format!("{} {}", item.artist, item.title);
        self.search(&query).await
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
        let (artist, title) = parse_artist_title_from_filename(&result.filename);

        let task = DownloadTaskRecord {
            id: task_id.clone(),
            provider: self.provider.name().to_string(),
            provider_task_id: Some(provider_task_id),
            title,
            artist,
            album: None,
            filename: result.filename.clone(),
            destination_path: Some(dest_str),
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
            self.complete_task(task_id, &task).await?;
        } else if progress.status == DownloadStatus::Failed {
            self.repo
                .update_status(
                    task_id,
                    "FAILED",
                    None,
                    Some("Transfer failed on network"),
                    None,
                )
                .await?;
            let _ = self.event_bus.publish(Event::DownloadFailed {
                task_id: task_id.to_string(),
                error: "Transfer failed on network".to_string(),
            });
        }

        Ok(Some(progress))
    }

    /// Marks a download task completed, triggers auto-import into library,
    /// and updates any linked wishlist item to 'DOWNLOADED'.
    pub async fn complete_task(
        &self,
        task_id: &str,
        task: &DownloadTaskRecord,
    ) -> AppResult<()> {
        let now = Utc::now().timestamp();
        let dest_path_str = task.destination_path.clone().unwrap_or_default();

        self.repo
            .update_status(task_id, "COMPLETED", None, None, Some(now))
            .await?;

        info!(task_id = %task_id, file = %dest_path_str, "Download completed");
        let _ = self.event_bus.publish(Event::DownloadCompleted {
            task_id: task_id.to_string(),
            file_path: dest_path_str.clone(),
        });

        // 1. Update linked wishlist item status
        if let Some(ref wl_id) = task.wishlist_id {
            let _ = self
                .wishlist_manager
                .update_status(wl_id, "DOWNLOADED")
                .await;
            info!(wishlist_id = %wl_id, "Updated wishlist item status to DOWNLOADED");
        }

        // 2. Auto-import downloaded file into local library
        if self.auto_import {
            info!("Auto-importing newly downloaded music into library");
            let _ = self.library_service.scan_library(None, true).await;
        }

        Ok(())
    }

    /// Cancels an active download task.
    pub async fn cancel_download(&self, task_id: &str) -> AppResult<()> {
        let task = self
            .repo
            .get_task_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Download task {} not found", task_id)))?;

        if let Some(ref pt_id) = task.provider_task_id {
            let _ = self.provider.cancel(pt_id).await;
        }

        self.repo
            .update_status(task_id, "CANCELLED", None, None, None)
            .await?;

        info!(task_id = %task_id, "Cancelled download task");
        Ok(())
    }

    /// Lists tracked download tasks.
    pub async fn list_downloads(
        &self,
        status_filter: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<DownloadTaskRecord>> {
        self.repo.list_tasks(status_filter, limit).await
    }

    /// Fetches a download task by ID.
    pub async fn get_download(&self, task_id: &str) -> AppResult<Option<DownloadTaskRecord>> {
        self.repo.get_task_by_id(task_id).await
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
