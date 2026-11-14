//! Cover art: short URLs for the webview and the resized images behind them
//! (see `library::artwork`).
use super::*;
use crate::library::artwork::{self, LocalArt, ThumbnailCache};

/// What a track's cover resolved to, kept for the session.
#[derive(Clone, Default)]
pub struct ResolvedArt {
    /// What the webview loads: an https URL or a `kaze-art:` URL.
    pub url: Option<String>,
    /// The local source behind a `kaze-art:` URL.
    pub local: Option<LocalArt>,
}

/// Resizing is CPU-bound; two at a time keeps fast scrolling from saturating
/// every core.
fn resize_permits() -> &'static tokio::sync::Semaphore {
    static PERMITS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    PERMITS.get_or_init(|| tokio::sync::Semaphore::new(2))
}

impl CoreProcessor {
    pub(super) async fn load_track_artwork(&self, track_id: &str) -> AppResult<Option<String>> {
        Ok(self.resolve_track_art(track_id).await?.url)
    }

    async fn resolve_track_art(&self, track_id: &str) -> AppResult<ResolvedArt> {
        if let Some(cached) = self.cover_art_cache.read().await.get(track_id) {
            return Ok(cached.clone());
        }
        let resolved = self.find_track_art(track_id).await?;
        self.cover_art_cache
            .write()
            .await
            .insert(track_id.to_string(), resolved.clone());
        Ok(resolved)
    }

    async fn find_track_art(&self, track_id: &str) -> AppResult<ResolvedArt> {
        // Source-bound online artwork belongs to the exact displayed recording.
        // Never fall through to unrelated enrichment artwork for an online alias.
        let online_cover: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE((SELECT json_extract(metadata_json,'$.cover_art_url') FROM online_track_sources WHERE track_id=?),
             (SELECT cover_art_url FROM external_tracks WHERE id=? AND provider='jiosaavn'))")
            .bind(track_id).bind(track_id).fetch_one(&self.db_pool).await?;
        if let Some(url) = online_cover.filter(|url| url.starts_with("https://")) {
            return Ok(ResolvedArt { url: Some(url), local: None });
        }
        if self.online_catalog.source_id(track_id).await?.is_some() {
            return Ok(ResolvedArt::default());
        }
        let local = match self.library_service.track_repo().find_by_id(track_id).await? {
            Some(track) => {
                let path = std::path::PathBuf::from(&track.file_path);
                tokio::task::spawn_blocking(move || LocalArt::find(&path))
                    .await
                    .unwrap_or(None)
            }
            None => None,
        };
        let local = match local {
            Some(local) => Some(local),
            None => sqlx::query_scalar::<_, Option<String>>(
                "SELECT cover_art_path FROM track_enrichment WHERE track_id = ? AND state = 'matched'",
            )
            .bind(track_id)
            .fetch_optional(&self.db_pool)
            .await?
            .flatten()
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_file())
            .map(LocalArt::File),
        };
        Ok(ResolvedArt {
            url: local.as_ref().map(|_| artwork::track_url(track_id, artwork::version())),
            local,
        })
    }

    /// The image behind a `kaze-art:` URL, resized to `size`, and its MIME type.
    pub async fn track_artwork_image(
        &self,
        track_id: &str,
        size: u32,
    ) -> Option<(Vec<u8>, &'static str)> {
        let local = self.resolve_track_art(track_id).await.ok()?.local?;
        let dir = self.config.read().await.cache_dir.join("artwork");
        let _permit = resize_permits().acquire().await.ok()?;
        tokio::task::spawn_blocking(move || {
            let original = local.read()?;
            Some(ThumbnailCache::new(dir).get(original, size))
        })
        .await
        .ok()
        .flatten()
    }

    /// Trims the resized-image cache in the background.
    pub fn prune_artwork_cache(&self) {
        let config = self.config.clone();
        tokio::spawn(async move {
            let dir = config.read().await.cache_dir.join("artwork");
            let _ = tokio::task::spawn_blocking(move || ThumbnailCache::new(dir).prune()).await;
        });
    }
}
