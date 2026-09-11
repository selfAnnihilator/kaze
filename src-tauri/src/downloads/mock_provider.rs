use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::AppResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

struct MockJob {
    bytes_downloaded: i64,
    total_bytes: i64,
    speed_bps: u64,
    status: DownloadStatus,
}

pub struct MockDownloadProvider {
    available: AtomicBool,
    canned_results: Arc<RwLock<Vec<DownloadSearchResult>>>,
    jobs: Arc<RwLock<HashMap<String, MockJob>>>,
}

impl MockDownloadProvider {
    pub fn new() -> Self {
        Self {
            available: AtomicBool::new(true),
            canned_results: Arc::new(RwLock::new(Vec::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_available(&self, available: bool) {
        self.available.store(available, Ordering::SeqCst);
    }

    pub async fn set_canned_results(&self, results: Vec<DownloadSearchResult>) {
        let mut guard = self.canned_results.write().await;
        *guard = results;
    }

    pub async fn complete_job(&self, job_id: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.bytes_downloaded = job.total_bytes;
            job.status = DownloadStatus::Completed;
        }
    }
}

#[async_trait]
impl DownloadProvider for MockDownloadProvider {
    fn name(&self) -> &'static str {
        "mock_soulseek"
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        if !self.is_available() {
            return Ok(Vec::new());
        }

        let canned = self.canned_results.read().await;
        if !canned.is_empty() {
            return Ok(canned.clone());
        }

        // Generate synthetic mock results matching query
        let results = vec![
            DownloadSearchResult {
                id: format!("mock_res_1_{}", query),
                provider: "mock_soulseek".to_string(),
                username: "audiophile_99".to_string(),
                filename: format!("Music/{}.flac", query),
                file_size: 35_000_000,
                bitrate: Some(980),
                sample_rate: Some(44100),
                format: "flac".to_string(),
                slots_free: true,
                speed_bps: 2_500_000,
            },
            DownloadSearchResult {
                id: format!("mock_res_2_{}", query),
                provider: "mock_soulseek".to_string(),
                username: "mp3_vault".to_string(),
                filename: format!("Shared/{}.mp3", query),
                file_size: 9_500_000,
                bitrate: Some(320),
                sample_rate: Some(44100),
                format: "mp3".to_string(),
                slots_free: false,
                speed_bps: 1_200_000,
            },
        ];

        Ok(results)
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        let job_id = format!("job_{}", uuid::Uuid::new_v4());

        // Create empty destination file to simulate beginning of write
        let dest_file = destination_dir.join(&result.filename);
        if let Some(parent) = dest_file.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&dest_file, b"MOCK_AUDIO_DATA").await;

        let job = MockJob {
            bytes_downloaded: 0,
            total_bytes: result.file_size,
            speed_bps: result.speed_bps,
            status: DownloadStatus::Downloading,
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), job);

        Ok(job_id)
    }

    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>> {
        let jobs = self.jobs.read().await;
        if let Some(job) = jobs.get(provider_task_id) {
            Ok(Some(DownloadProgress {
                task_id: provider_task_id.to_string(),
                bytes_downloaded: job.bytes_downloaded,
                total_bytes: job.total_bytes,
                speed_bps: job.speed_bps,
                status: job.status,
            }))
        } else {
            Ok(None)
        }
    }

    async fn cancel(&self, provider_task_id: &str) -> AppResult<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(provider_task_id) {
            job.status = DownloadStatus::Cancelled;
        }
        Ok(())
    }
}
