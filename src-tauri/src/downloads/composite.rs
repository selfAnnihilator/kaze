use super::archive::ArchiveProvider;
use super::audius::AudiusProvider;
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
    archive: Arc<ArchiveProvider>,
    audius: Arc<AudiusProvider>,
}

impl CompositeDownloadProvider {
    pub fn new(ytdlp: Arc<YtDlpProvider>, soulseek: Arc<SoulseekProvider>) -> Self {
        Self {
            ytdlp,
            soulseek,
            archive: Arc::new(ArchiveProvider::new()),
            audius: Arc::new(AudiusProvider::new()),
        }
    }
}

#[async_trait]
impl DownloadProvider for CompositeDownloadProvider {
    fn name(&self) -> &'static str {
        "composite"
    }

    fn is_available(&self) -> bool {
        self.ytdlp.is_available()
            || self.soulseek.is_available()
            || self.archive.is_available()
            || self.audius.is_available()
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

    async fn search_track(
        &self,
        artist: &str,
        title: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        let query = format!("{} {}", artist, title);
        let (ytdlp, soulseek, archive, audius, stream_source) = tokio::join!(
            async {
                if self.ytdlp.is_available() {
                    self.ytdlp.search(&query).await
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                if self.soulseek.is_available() {
                    self.soulseek.search(&query).await
                } else {
                    Ok(Vec::new())
                }
            },
            self.archive.search_track(artist, title),
            self.audius.search_track(artist, title),
            async {
                if self.ytdlp.is_available() {
                    self.ytdlp.resolve_stream_url(&query).await
                } else {
                    Ok(None)
                }
            },
        );
        let mut results = Vec::new();
        for (name, outcome) in [
            ("yt-dlp", ytdlp),
            ("soulseek", soulseek),
            ("internet-archive", archive),
            ("audius", audius),
        ] {
            match outcome {
                Ok(mut matches) => results.append(&mut matches),
                Err(error) => {
                    tracing::warn!(provider = name, %error, "Download source search failed")
                }
            }
        }
        match stream_source {
            Ok(Some((_, _, Some(source)))) if !results.iter().any(|result| result.id == source.id) => {
                results.push(source);
            }
            Err(error) => tracing::warn!(provider = "yt-dlp-stream", %error, "Stream source search failed"),
            _ => {}
        }
        Ok(results)
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        if result.provider == "internet-archive" {
            self.archive.start_download(result, destination_dir).await
        } else if result.provider == "audius" {
            self.audius.start_download(result, destination_dir).await
        } else if result.provider == "yt-dlp" || result.id.starts_with("ytdlp_") {
            self.ytdlp.start_download(result, destination_dir).await
        } else {
            self.soulseek.start_download(result, destination_dir).await
        }
    }

    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>> {
        if let Some(prog) = self.archive.get_progress(provider_task_id).await? {
            return Ok(Some(prog));
        }
        if let Some(prog) = self.audius.get_progress(provider_task_id).await? {
            return Ok(Some(prog));
        }
        if let Some(prog) = self.ytdlp.get_progress(provider_task_id).await? {
            return Ok(Some(prog));
        }
        self.soulseek.get_progress(provider_task_id).await
    }

    async fn cancel(&self, provider_task_id: &str) -> AppResult<()> {
        let _ = self.archive.cancel(provider_task_id).await;
        let _ = self.audius.cancel(provider_task_id).await;
        let _ = self.ytdlp.cancel(provider_task_id).await;
        let _ = self.soulseek.cancel(provider_task_id).await;
        Ok(())
    }

    async fn resolve_stream_url(&self, query: &str) -> AppResult<Option<(String, f64, Option<DownloadSearchResult>)>> {
        if self.ytdlp.is_available() {
            self.ytdlp.resolve_stream_url(query).await
        } else {
            Ok(None)
        }
    }

    async fn resolve_stream_urls(&self, query: &str) -> AppResult<Vec<(String, f64, Option<DownloadSearchResult>)>> {
        if self.ytdlp.is_available() {
            self.ytdlp.resolve_stream_urls(query).await
        } else {
            Ok(Vec::new())
        }
    }
}
