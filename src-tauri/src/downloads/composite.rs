use super::soulseek::SoulseekProvider;
use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult};
use super::ytdlp::YtDlpProvider;
use crate::core::error::AppResult;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Composite download provider that unifies built-in local yt-dlp downloading
/// with Soulseek/Slskd network capabilities.
pub struct CompositeDownloadProvider {
    ytdlp: Arc<YtDlpProvider>,
    soulseek: Arc<SoulseekProvider>,
}

impl CompositeDownloadProvider {
    pub fn new(ytdlp: Arc<YtDlpProvider>, soulseek: Arc<SoulseekProvider>) -> Self {
        Self { ytdlp, soulseek }
    }
}

#[async_trait]
impl DownloadProvider for CompositeDownloadProvider {
    fn name(&self) -> &'static str {
        "composite"
    }

    fn is_available(&self) -> bool {
        self.ytdlp.is_available() || self.soulseek.is_available()
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        let mut results = Vec::new();

        // 1. Primary: built-in local yt-dlp search
        if self.ytdlp.is_available() {
            match self.ytdlp.search(query).await {
                Ok(mut ytdlp_results) => results.append(&mut ytdlp_results),
                Err(e) => tracing::warn!(error = %e, "yt-dlp search returned error"),
            }
        }

        // 2. Secondary: Soulseek network (if reachable/running)
        if self.soulseek.is_available() {
            if let Ok(mut slsk_results) = self.soulseek.search(query).await {
                results.append(&mut slsk_results);
            }
        }

        Ok(results)
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        if result.provider == "yt-dlp" || result.id.starts_with("ytdlp_") {
            self.ytdlp.start_download(result, destination_dir).await
        } else {
            self.soulseek.start_download(result, destination_dir).await
        }
    }

    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>> {
        if let Some(prog) = self.ytdlp.get_progress(provider_task_id).await? {
            return Ok(Some(prog));
        }
        self.soulseek.get_progress(provider_task_id).await
    }

    async fn cancel(&self, provider_task_id: &str) -> AppResult<()> {
        let _ = self.ytdlp.cancel(provider_task_id).await;
        let _ = self.soulseek.cancel(provider_task_id).await;
        Ok(())
    }

    async fn resolve_stream_url(&self, query: &str) -> AppResult<Option<(String, f64)>> {
        if self.ytdlp.is_available() {
            self.ytdlp.resolve_stream_url(query).await
        } else {
            Ok(None)
        }
    }
}
