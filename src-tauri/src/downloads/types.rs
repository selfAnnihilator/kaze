use serde::{Deserialize, Serialize};

/// Lifecycle states of a download task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DownloadStatus::Queued => "QUEUED",
            DownloadStatus::Downloading => "DOWNLOADING",
            DownloadStatus::Completed => "COMPLETED",
            DownloadStatus::Failed => "FAILED",
            DownloadStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "QUEUED" => DownloadStatus::Queued,
            "DOWNLOADING" => DownloadStatus::Downloading,
            "COMPLETED" => DownloadStatus::Completed,
            "FAILED" => DownloadStatus::Failed,
            "CANCELLED" => DownloadStatus::Cancelled,
            _ => DownloadStatus::Queued,
        }
    }
}

/// A search result returned by a download provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSearchResult {
    pub id: String,
    pub provider: String,
    pub username: String,
    pub filename: String,
    pub file_size: i64,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub format: String,
    pub slots_free: bool,
    pub speed_bps: u64,
}

/// Progress metrics for an active download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub task_id: String,
    pub bytes_downloaded: i64,
    pub total_bytes: i64,
    pub speed_bps: u64,
    pub status: DownloadStatus,
}
