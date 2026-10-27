use super::{DownloadProgress, DownloadProvider, DownloadSearchResult};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use std::path::Path;

/// The JioSaavn trial excludes legacy search and download networks.
pub struct DisabledDownloadProvider;

fn disabled<T>() -> AppResult<T> {
    Err(AppError::Validation(
        "Older download providers are disabled during the JioSaavn playback trial".into(),
    ))
}

#[async_trait]
impl DownloadProvider for DisabledDownloadProvider {
    fn name(&self) -> &'static str { "disabled" }
    fn is_available(&self) -> bool { false }
    async fn search(&self, _: &str) -> AppResult<Vec<DownloadSearchResult>> { disabled() }
    async fn start_download(&self, _: &DownloadSearchResult, _: &Path) -> AppResult<String> { disabled() }
    async fn get_progress(&self, _: &str) -> AppResult<Option<DownloadProgress>> { disabled() }
    async fn cancel(&self, _: &str) -> AppResult<()> { disabled() }
}
