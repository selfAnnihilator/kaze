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
        Ok(self.resolve_stream_urls(query).await?.into_iter().next())
    }

    /// Resolves streamable audio URL candidates using a staged approach:
    ///
    /// **Phase 1 – direct providers (Audius + Internet Archive in parallel)**
    ///   Both providers are queried concurrently.  If *either* returns a valid
    ///   candidate it is returned immediately; yt-dlp is never invoked.
    ///   Direct providers have permanent or very long-lived URLs and complete in
    ///   ~100–500ms.
    ///
    /// **Phase 2 – yt-dlp fallback**
    ///   Only reached when Phase 1 yields no candidates.  yt-dlp is the
    ///   universal fallback (8–20s) and returns signed, time-limited URLs.
    ///
    /// This design means yt-dlp is bypassed entirely for tracks that are
    /// available on Audius or Internet Archive, cutting resolution from ~12s to
    /// ~0.5s for those tracks.
    async fn resolve_stream_urls(&self, query: &str) -> AppResult<Vec<(String, f64, Option<DownloadSearchResult>)>> {
        use tokio::select;

        // -----------------------------------------------------------------
        // Phase 1: race Audius and Archive concurrently.
        // We use select! with two branches so the first provider to return a
        // non-empty result wins and we cancel the other immediately (by letting
        // its future drop).
        // -----------------------------------------------------------------
        let audius = self.audius.clone();
        let archive = self.archive.clone();
        let q = query.to_string();

        // Spawn both as independent tasks so we can cancel the winner's
        // "opponent" by simply dropping the losing future.
        let audius_fut = {
            let q2 = q.clone();
            async move { audius.resolve_stream_urls(&q2).await }
        };
        let archive_fut = {
            let q3 = q.clone();
            async move { archive.resolve_stream_urls(&q3).await }
        };

        // Pin both futures.
        tokio::pin!(audius_fut);
        tokio::pin!(archive_fut);

        // Keep track of which branch finished first.
        let mut audius_done = false;
        let mut archive_done = false;
        let mut audius_result: Vec<(String, f64, Option<DownloadSearchResult>)> = Vec::new();
        let mut archive_result: Vec<(String, f64, Option<DownloadSearchResult>)> = Vec::new();

        // We poll both futures until we have results from both or one winner.
        loop {
            select! {
                res = &mut audius_fut, if !audius_done => {
                    audius_done = true;
                    if let Ok(candidates) = res {
                        if !candidates.is_empty() {
                            // Direct Audius match — return immediately.
                            return Ok(candidates);
                        }
                        audius_result = Vec::new(); // empty, continue
                    }
                    if archive_done {
                        break; // Both finished, no direct hits.
                    }
                }
                res = &mut archive_fut, if !archive_done => {
                    archive_done = true;
                    if let Ok(candidates) = res {
                        if !candidates.is_empty() {
                            // Direct Archive match — return immediately.
                            return Ok(candidates);
                        }
                        archive_result = Vec::new(); // empty, continue
                    }
                    if audius_done {
                        break; // Both finished, no direct hits.
                    }
                }
            }
            if audius_done && archive_done {
                break;
            }
        }
        let _ = (audius_result, archive_result); // suppress unused warning

        // -----------------------------------------------------------------
        // Phase 2: fall back to yt-dlp (slower but universal).
        // -----------------------------------------------------------------
        if self.ytdlp.is_available() {
            self.ytdlp.resolve_stream_urls(query).await
        } else {
            Ok(Vec::new())
        }
    }
}
