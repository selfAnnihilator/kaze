use super::{DownloadProgress, DownloadProvider, DownloadSearchResult, DownloadStatus};
use crate::{
    core::{AppError, AppResult},
    providers::jiosaavn::JioSaavnProvider,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncWriteExt,
    sync::{Mutex, Semaphore},
};

struct Transfer {
    progress: DownloadProgress,
    handle: tokio::task::JoinHandle<()>,
}
pub struct JioSaavnDownloadProvider {
    provider: Arc<JioSaavnProvider>,
    client: reqwest::Client,
    transfers: Arc<Mutex<HashMap<String, Transfer>>>,
    slots: Arc<Semaphore>,
}
struct PartialFile(PathBuf);
impl Drop for PartialFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
impl JioSaavnDownloadProvider {
    pub fn new(provider: Arc<JioSaavnProvider>) -> Self {
        Self {
            provider,
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(180))
                .build()
                .expect("valid download client"),
            transfers: Arc::new(Mutex::new(HashMap::new())),
            slots: Arc::new(Semaphore::new(4)),
        }
    }
}
pub(super) fn safe_name(value: &str) -> String {
    // Bound UTF-8 bytes, not characters, so multilingual filenames fit normal filesystems.
    let mut name = String::new();
    for c in value
        .chars()
        .filter(|c| c.is_alphanumeric() || " -_().".contains(*c))
    {
        if name.len() + c.len_utf8() > 80 {
            break;
        }
        name.push(c);
    }
    let name = name.trim_matches('.').trim();
    if name.is_empty() {
        "Untitled".into()
    } else {
        name.into()
    }
}
#[async_trait]
impl DownloadProvider for JioSaavnDownloadProvider {
    fn name(&self) -> &'static str {
        "jiosaavn"
    }
    fn is_available(&self) -> bool {
        true
    }
    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        Ok(self
            .provider
            .search(query, 25)
            .await?
            .into_iter()
            .map(|track| DownloadSearchResult {
                id: format!("jiosaavn:{}", track.provider_track_id),
                provider: "jiosaavn".into(),
                username: track.artist_name.clone(),
                filename: format!(
                    "{} - {} ({}).m4a",
                    safe_name(&track.artist_name),
                    safe_name(&track.title),
                    safe_name(&track.provider_track_id)
                ),
                file_size: 0,
                bitrate: None,
                sample_rate: None,
                format: "m4a".into(),
                slots_free: true,
                speed_bps: 0,
            })
            .collect())
    }
    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        directory: &Path,
    ) -> AppResult<String> {
        let id = result
            .id
            .strip_prefix("jiosaavn:")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| AppError::Validation("Choose a JioSaavn song".into()))?;
        if result.provider != "jiosaavn"
            || Path::new(&result.filename)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(result.filename.as_str())
        {
            return Err(AppError::Validation("Invalid download selection".into()));
        }
        let (url, _) = self.provider.resolve(id, false).await?;
        let task_id = uuid::Uuid::new_v4().to_string();
        let destination = directory.join(&result.filename);
        if destination.exists() {
            return Err(AppError::Validation(
                "This song is already downloaded".into(),
            ));
        }
        let partial = directory.join(format!(".{task_id}.part"));
        let task_key = task_id.clone();
        let tasks = self.transfers.clone();
        let client = self.client.clone();
        let slots = self.slots.clone();
        let mut transfers = self.transfers.lock().await;
        if transfers.len() >= 256 {
            transfers.retain(|_, task| !task.handle.is_finished());
        }
        if transfers
            .values()
            .filter(|task| !task.handle.is_finished())
            .count()
            >= 32
        {
            return Err(AppError::Validation("Too many pending downloads".into()));
        }
        let progress = DownloadProgress {
            task_id: task_id.clone(),
            bytes_downloaded: 0,
            total_bytes: 0,
            speed_bps: 0,
            status: DownloadStatus::Queued,
        };
        let handle = tokio::spawn(async move {
            let _cleanup = PartialFile(partial.clone());
            let transfer: AppResult<()> = async {
                let _slot = slots
                    .acquire()
                    .await
                    .map_err(|_| AppError::Internal("Download service stopped".into()))?;
                let mut response = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| AppError::Network(e.without_url().to_string()))?
                    .error_for_status()
                    .map_err(|e| AppError::Network(e.without_url().to_string()))?;
                const MAX: u64 = 128 * 1024 * 1024;
                let length = response.content_length().unwrap_or(0);
                if length > MAX
                    || response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .is_some_and(|v| v.starts_with("text/") || v.contains("json"))
                {
                    return Err(AppError::Validation(
                        "Invalid audio download response".into(),
                    ));
                }
                let mut file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&partial)
                    .await
                    .map_err(|e| AppError::Io(e.to_string()))?;
                let mut bytes = 0u64;
                let started = Instant::now();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(|e| AppError::Network(e.without_url().to_string()))?
                {
                    bytes += chunk.len() as u64;
                    if bytes > MAX {
                        return Err(AppError::Validation("Audio download too large".into()));
                    }
                    file.write_all(&chunk)
                        .await
                        .map_err(|e| AppError::Io(e.to_string()))?;
                    if let Some(task) = tasks.lock().await.get_mut(&task_key) {
                        task.progress.bytes_downloaded = bytes as i64;
                        task.progress.total_bytes = length as i64;
                        task.progress.speed_bps =
                            (bytes as f64 / started.elapsed().as_secs_f64().max(0.1)) as u64;
                        task.progress.status = DownloadStatus::Downloading;
                    }
                }
                if bytes == 0 || (length > 0 && bytes != length) {
                    return Err(AppError::Network("Incomplete audio download".into()));
                }
                file.flush()
                    .await
                    .map_err(|e| AppError::Io(e.to_string()))?;
                drop(file);
                // Create the final name without overwriting a concurrently saved file.
                tokio::fs::hard_link(&partial, &destination)
                    .await
                    .map_err(|e| AppError::Io(e.to_string()))?;
                Ok(())
            }
            .await;
            if let Some(task) = tasks.lock().await.get_mut(&task_key) {
                task.progress.status = if transfer.is_ok() {
                    DownloadStatus::Completed
                } else {
                    DownloadStatus::Failed
                };
            }
        });
        transfers.insert(task_id.clone(), Transfer { progress, handle });
        Ok(task_id)
    }
    async fn get_progress(&self, id: &str) -> AppResult<Option<DownloadProgress>> {
        Ok(self
            .transfers
            .lock()
            .await
            .get(id)
            .map(|task| task.progress.clone()))
    }
    async fn cancel(&self, id: &str) -> AppResult<()> {
        if let Some(task) = self.transfers.lock().await.get_mut(id) {
            task.handle.abort();
            task.progress.status = DownloadStatus::Cancelled;
        }
        Ok(())
    }
}

#[cfg(test)]
mod filename_tests {
    use super::*;
    #[test]
    fn multilingual_names_stay_within_filesystem_byte_limit() {
        let name = safe_name(&"संगीत".repeat(100));
        assert!(name.len() <= 80);
        assert!(name.chars().all(|c| c.is_alphanumeric()));
        assert!(!safe_name("../").is_empty());
    }
}
