use serde::{Deserialize, Serialize};

/// Normalized metadata for a track retrieved from an external provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTrackMetadata {
    pub provider: String,
    pub provider_track_id: String,
    pub title: String,
    pub artist_name: String,
    pub artist_id: Option<String>,
    pub album_title: Option<String>,
    pub album_id: Option<String>,
    pub duration_secs: Option<f64>,
    pub year: Option<i64>,
    pub track_number: Option<u32>,
    pub cover_art_url: Option<String>,
}

/// Normalized metadata for an artist retrieved from an external provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalArtistMetadata {
    pub provider: String,
    pub provider_artist_id: String,
    pub name: String,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub genres: Vec<String>,
}

/// Normalized metadata for an album retrieved from an external provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAlbumMetadata {
    pub provider: String,
    pub provider_album_id: String,
    pub title: String,
    pub artist_name: String,
    pub release_year: Option<i64>,
    pub total_tracks: Option<u32>,
    pub cover_art_url: Option<String>,
}
