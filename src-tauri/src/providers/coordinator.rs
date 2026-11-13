use super::{jiosaavn::JioSaavnProvider, spotify_embed::SpotifyEmbedClient, types::ExternalTrackMetadata};
use crate::{
    core::{error::AppResult, event::Event, event_bus::EventBus},
    database::{
        models::ExternalTrackRecord,
        repositories::{SqliteWishlistRepository, WishlistRepository},
    },
};
use sqlx::SqlitePool;
use std::sync::Arc;

/// The single online catalog and local-file metadata source.
pub struct ProviderCoordinator {
    pub jiosaavn: Arc<JioSaavnProvider>,
    /// Metadata-only playlist reader; tracks are matched onto JioSaavn.
    pub spotify: Arc<SpotifyEmbedClient>,
    pool: SqlitePool,
    event_bus: Arc<EventBus>,
}
impl ProviderCoordinator {
    pub async fn enabled(&self) -> bool {
        match sqlx::query_scalar::<_, String>(
            "SELECT value FROM application_settings WHERE key='enable_jiosaavn'",
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(value) => value.as_deref() != Some("false"),
            Err(_) => false,
        }
    }
    pub fn new(pool: SqlitePool, event_bus: Arc<EventBus>) -> Self {
        Self {
            pool,
            event_bus,
            jiosaavn: Arc::new(JioSaavnProvider::new()),
            spotify: Arc::new(SpotifyEmbedClient::new()),
        }
    }
    pub fn emit_status(&self) {
        let _ = self.event_bus.publish(Event::ProviderStatusChanged {
            provider: "jiosaavn".into(),
            available: true,
            message: Some("Music, artwork and catalog metadata".into()),
        });
    }
    /// Reuse any pre-existing saved provider identity, including cloud aliases.
    pub async fn persist(&self, track: ExternalTrackMetadata) -> AppResult<ExternalTrackRecord> {
        let snapshot = serde_json::to_string(&track)
            .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM external_tracks WHERE provider='jiosaavn' AND provider_id=?",
        )
        .bind(&track.provider_track_id)
        .fetch_optional(&self.pool)
        .await?;
        let row = ExternalTrackRecord {
            id: existing.unwrap_or_else(|| format!("jiosaavn:{}", track.provider_track_id)),
            provider: "jiosaavn".into(),
            provider_id: track.provider_track_id,
            title: track.title,
            artist: track.artist_name,
            album: track.album_title,
            duration_secs: track.duration_secs,
            cover_art_url: track.cover_art_url,
            preview_url: None,
            genre: None,
            match_status: "NOT_FOUND".into(),
            matched_local_track_id: None,
            created_at: chrono::Utc::now().timestamp(),
        };
        SqliteWishlistRepository::new(self.pool.clone())
            .upsert_external_track(&row)
            .await?;
        sqlx::query("UPDATE external_tracks SET language=COALESCE(?,language), release_year=COALESCE(?,release_year) WHERE id=?")
            .bind(track.language).bind(track.year).bind(&row.id).execute(&self.pool).await?;
        sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,?,?,?) ON CONFLICT(track_id) DO NOTHING")
            .bind(&row.id).bind(&row.provider_id).bind(snapshot).bind(chrono::Utc::now().timestamp()).execute(&self.pool).await?;
        Ok(row)
    }
}
