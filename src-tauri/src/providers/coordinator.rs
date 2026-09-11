use super::cover_art_archive::CoverArtArchiveProvider;
use super::musicbrainz::MusicBrainzProvider;
use super::spotify::SpotifyProvider;
use super::types::ExternalTrackMetadata;
use super::MetadataProvider;
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use sqlx::{FromRow, SqlitePool};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct TrackEnrichmentRow {
    id: String,
    title: String,
    artist_name: Option<String>,
    album_id: Option<String>,
    album_title: Option<String>,
    musicbrainz_track_id: Option<String>,
    spotify_id: Option<String>,
    has_cover_art: i64,
}

pub struct ProviderCoordinator {
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
        let cover_art = Arc::new(CoverArtArchiveProvider::new(cache_dir));
        let spotify = Arc::new(SpotifyProvider::new(spotify_client_id, spotify_client_secret));

        Self {
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

    /// Enriches a local track by querying external providers and persisting IDs/artwork.
    pub async fn enrich_track(&self, track_id: &str) -> AppResult<bool> {
        let row = sqlx::query_as::<_, TrackEnrichmentRow>(
            "SELECT t.id, t.title, a.name as artist_name, t.album_id, al.title as album_title,
                    t.musicbrainz_track_id, t.spotify_id, t.has_cover_art
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE t.id = ?"
        )
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let track = match row {
            Some(r) => r,
            None => return Err(AppError::NotFound(format!("Track {} not found", track_id))),
        };

        let artist_str = track.artist_name.as_deref().unwrap_or("Unknown");
        let album_str = track.album_title.as_deref();
        let mut updated = false;

        // 1. MusicBrainz Enrichment
        let mut mb_release_id = None;

        if track.musicbrainz_track_id.is_none() {
            debug!(track = %track.title, artist = %artist_str, "Searching MusicBrainz for track");
            match self.musicbrainz.search_track(&track.title, artist_str, album_str).await {
                Ok(recs) if !recs.is_empty() => {
                    let best = &recs[0];
                    mb_release_id = best.album_id.clone();

                    sqlx::query("UPDATE tracks SET musicbrainz_track_id = ? WHERE id = ?")
                        .bind(&best.provider_track_id)
                        .bind(track_id)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| AppError::Database(e.to_string()))?;

                    updated = true;
                    info!(track_id = %track_id, mb_id = %best.provider_track_id, "Enriched track with MusicBrainz ID");
                }
                Ok(_) => debug!(track = %track.title, "No MusicBrainz matches found"),
                Err(e) => warn!(error = %e, "MusicBrainz enrichment failed"),
            }
        }

        // 2. Cover Art Archive Enrichment
        if track.has_cover_art == 0 {
            if let Some(rel_id) = mb_release_id {
                match self.cover_art.fetch_cover_art(&rel_id).await {
                    Ok(Some(_bytes)) => {
                        let cached_path = self.cover_art.get_cached_path(&rel_id);
                        let path_str = cached_path.to_string_lossy().to_string();

                        sqlx::query("UPDATE tracks SET has_cover_art = 1 WHERE id = ?")
                            .bind(track_id)
                            .execute(&self.pool)
                            .await
                            .map_err(|e| AppError::Database(e.to_string()))?;

                        if let Some(alb_id) = &track.album_id {
                            let _ = sqlx::query("UPDATE albums SET cover_art_path = ? WHERE id = ?")
                                .bind(&path_str)
                                .bind(alb_id)
                                .execute(&self.pool)
                                .await;
                        }

                        updated = true;
                        info!(track_id = %track_id, path = %path_str, "Enriched track with Cover Art Archive artwork");
                    }
                    Ok(None) => debug!(release_id = %rel_id, "No artwork found in Cover Art Archive"),
                    Err(e) => warn!(error = %e, "Cover Art Archive fetch failed"),
                }
            }
        }

        // 3. Spotify Enrichment (optional)
        if track.spotify_id.is_none() && self.spotify.is_available() {
            match self.spotify.search_track(&track.title, artist_str, album_str).await {
                Ok(spotify_recs) if !spotify_recs.is_empty() => {
                    let best = &spotify_recs[0];
                    sqlx::query("UPDATE tracks SET spotify_id = ? WHERE id = ?")
                        .bind(&best.provider_track_id)
                        .bind(track_id)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| AppError::Database(e.to_string()))?;

                    updated = true;
                    info!(track_id = %track_id, spotify_id = %best.provider_track_id, "Enriched track with Spotify ID");
                }
                Ok(_) => (),
                Err(e) => warn!(error = %e, "Spotify enrichment failed"),
            }
        }

        Ok(updated)
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
