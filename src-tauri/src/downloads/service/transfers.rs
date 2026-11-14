//! Running a transfer: start, poll, complete and cancel a download.
use super::*;

impl DownloadService {
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
}
