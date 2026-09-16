use super::types::{DownloadProgress, DownloadSearchResult};
use crate::core::error::AppResult;
use async_trait::async_trait;
use std::path::Path;

/// Contract for download network providers (e.g. Soulseek/Slskd or Mocks).
#[async_trait]
pub trait DownloadProvider: Send + Sync {
    /// Human-readable identifier of the provider.
    fn name(&self) -> &'static str;

    /// Checks if the underlying provider or daemon is reachable.
    fn is_available(&self) -> bool;

    /// Searches the provider network for matching audio files.
    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>>;

    async fn search_track(
        &self,
        artist: &str,
        title: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        self.search(&format!("{} {}", artist, title)).await
    }

    /// Enqueues a download for a specific search result to the local destination directory.
    /// Returns the provider's task/transfer ID.
    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String>;

    /// Polls current transfer progress from the provider.
    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>>;

    /// Cancels or aborts an active transfer.
    async fn cancel(&self, provider_task_id: &str) -> AppResult<()>;

    /// Resolves direct streamable audio URL and duration for full-song previewing.
    async fn resolve_stream_url(&self, _query: &str) -> AppResult<Option<(String, f64, Option<DownloadSearchResult>)>> {
        Ok(None)
    }
}
