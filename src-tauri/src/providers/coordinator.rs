use super::cover_art_archive::CoverArtArchiveProvider;
use super::musicbrainz::MusicBrainzProvider;
use super::spotify::SpotifyProvider;
use super::types::ExternalTrackMetadata;
use super::MetadataProvider;
use crate::core::error::AppResult;
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;


pub struct ProviderCoordinator {
    cache_dir: PathBuf,
    pub jiosaavn: Arc<super::jiosaavn::JioSaavnProvider>,
    pool: SqlitePool,
    event_bus: Arc<EventBus>,
    musicbrainz: Arc<MusicBrainzProvider>,
    cover_art: Arc<CoverArtArchiveProvider>,
    spotify: Arc<SpotifyProvider>,
}

impl ProviderCoordinator {
    pub fn new(
        pool: SqlitePool,
        event_bus: Arc<EventBus>,
        cache_dir: PathBuf,
        spotify_client_id: Option<String>,
        spotify_client_secret: Option<String>,
    ) -> Self {
        let musicbrainz = Arc::new(MusicBrainzProvider::new());
        let cover_art = Arc::new(CoverArtArchiveProvider::new(cache_dir.clone()));
        let spotify = Arc::new(SpotifyProvider::new(spotify_client_id, spotify_client_secret));

        Self {
            cache_dir: cache_dir.clone(),
            jiosaavn: Arc::new(super::jiosaavn::JioSaavnProvider::new()),
            pool,
            event_bus,
            musicbrainz,
            cover_art,
            spotify,
        }
    }

    pub fn with_providers(
        pool: SqlitePool,
        event_bus: Arc<EventBus>,
        musicbrainz: Arc<MusicBrainzProvider>,
        cover_art: Arc<CoverArtArchiveProvider>,
        spotify: Arc<SpotifyProvider>,
    ) -> Self {
        Self {
            cache_dir: cover_art.get_cached_path("probe").parent().and_then(|p| p.parent()).unwrap_or(std::path::Path::new(".")).to_path_buf(),
            jiosaavn: Arc::new(super::jiosaavn::JioSaavnProvider::new()),
            pool,
            event_bus,
            musicbrainz,
            cover_art,
            spotify,
        }
    }

    pub fn musicbrainz(&self) -> Arc<MusicBrainzProvider> {
        self.musicbrainz.clone()
    }

    pub fn cover_art(&self) -> Arc<CoverArtArchiveProvider> {
        self.cover_art.clone()
    }

    pub fn spotify(&self) -> Arc<SpotifyProvider> {
        self.spotify.clone()
    }

    /// Broadcasts availability status of all configured providers.
    pub fn emit_status(&self) {
        let mb_avail = self.musicbrainz.is_available();
        let _ = self.event_bus.publish(Event::ProviderStatusChanged {
            provider: "musicbrainz".to_string(),
            available: mb_avail,
            message: Some("Free metadata archive (1 req/s throttled)".to_string()),
        });

        let ca_avail = self.cover_art.is_available();
        let _ = self.event_bus.publish(Event::ProviderStatusChanged {
            provider: "cover_art_archive".to_string(),
            available: ca_avail,
            message: Some("Open cover artwork repository".to_string()),
        });

        let sp_avail = self.spotify.is_available();
        let _ = self.event_bus.publish(Event::ProviderStatusChanged {
            provider: "spotify".to_string(),
            available: sp_avail,
            message: if sp_avail {
                Some("Configured with Client Credentials".to_string())
            } else {
                Some("Unconfigured (add credentials in settings to enable)".to_string())
            },
        });
    }

    /// All enrichment uses the conservative, reversible gap-fill path.
    pub async fn enrich_track(self: &Arc<Self>, track_id: &str) -> AppResult<bool> {
        super::enrichment::GapEnrichmentService::new(self.pool.clone(), self.clone(), self.cache_dir.clone(), self.event_bus.clone())
            .enrich(track_id).await
    }

    /// Searches external tracks across available providers.
    pub async fn search_external(&self, query: &str, limit: u32) -> AppResult<Vec<ExternalTrackMetadata>> {
        let mut results = Vec::new();

        // MusicBrainz search
        if let Ok(mb_results) = self.musicbrainz.search_track(query, "", None).await {
            results.extend(mb_results.into_iter().take(limit as usize));
        }

        // Spotify search if available and need more results
        if results.len() < limit as usize && self.spotify.is_available() {
            if let Ok(sp_results) = self.spotify.search_track(query, "", None).await {
                results.extend(sp_results.into_iter().take(limit as usize - results.len()));
            }
        }

        Ok(results)
    }
}
