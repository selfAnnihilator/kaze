use super::types::{ExternalAlbumMetadata, ExternalArtistMetadata, ExternalTrackMetadata};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::Client;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, warn};

pub struct CoverArtArchiveProvider {
    client: Client,
    cache_dir: PathBuf,
    base_url: String,
}

impl CoverArtArchiveProvider {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self::with_base_url(cache_dir, "https://coverartarchive.org")
    }

    pub fn with_base_url(cache_dir: PathBuf, base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent("MusicPlayer/0.1.0 ( https://github.com/example/music-player )")
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self {
            client,
            cache_dir,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Resolves the local cache path for an artwork image by release MBID.
    pub fn get_cached_path(&self, release_mbid: &str) -> PathBuf {
        self.cache_dir.join("artwork").join(format!("{}.jpg", release_mbid))
    }
}

#[async_trait]
impl super::MetadataProvider for CoverArtArchiveProvider {
    fn name(&self) -> &'static str {
        "cover_art_archive"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search_track(
        &self,
        _title: &str,
        _artist: &str,
        _album: Option<&str>,
    ) -> AppResult<Vec<ExternalTrackMetadata>> {
        Ok(Vec::new())
    }

    async fn search_artist(&self, _name: &str) -> AppResult<Option<ExternalArtistMetadata>> {
        Ok(None)
    }

    async fn search_album(&self, _title: &str, _artist: &str) -> AppResult<Option<ExternalAlbumMetadata>> {
        Ok(None)
    }

    async fn fetch_cover_art(&self, release_id: &str) -> AppResult<Option<Vec<u8>>> {
        let cache_path = self.get_cached_path(release_id);

        // 1. Check if already cached locally on disk
        if cache_path.exists() {
            debug!(path = %cache_path.display(), "Cover art retrieved from local disk cache");
            let bytes = fs::read(&cache_path)
                .await
                .map_err(|e| AppError::Io(e.to_string()))?;
            return Ok(Some(bytes));
        }

        // 2. Fetch from remote Cover Art Archive
        let url = format!("{}/release/{}/front", self.base_url, release_id);
        let resp = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!(error = %e, release_id = %release_id, "Failed to connect to Cover Art Archive");
                return Ok(None);
            }
        };

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            debug!(release_id = %release_id, "No cover art available in Cover Art Archive");
            return Ok(None);
        }

        if !resp.status().is_success() {
            warn!(status = %resp.status(), release_id = %release_id, "Cover Art Archive returned error status");
            return Ok(None);
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?
            .to_vec();

        // 3. Save to local cache directory
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }
        let _ = fs::write(&cache_path, &bytes).await;
        debug!(path = %cache_path.display(), "Cached cover art locally");

        Ok(Some(bytes))
    }
}
