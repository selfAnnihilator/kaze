use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LibraryFolderRecord {
    pub id: String,
    pub path: String,
    pub added_at: i64,
    pub last_scanned_at: Option<i64>,
    pub enabled: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArtistRecord {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
    pub musicbrainz_id: Option<String>,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlbumRecord {
    pub id: String,
    pub title: String,
    pub normalized_title: String,
    pub artist_id: Option<String>,
    pub album_artist: Option<String>,
    pub release_year: Option<i64>,
    pub total_tracks: Option<i64>,
    pub cover_art_path: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GenreRecord {
    pub id: String,
    pub name: String,
    pub normalized_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackRecord {
    pub id: String,
    pub file_path: String,
    pub file_size: i64,
    pub modified_timestamp: i64,
    pub file_hash: Option<String>,
    pub title: String,
    pub normalized_title: String,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    pub genre_id: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub duration_secs: f64,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub format: String,
    pub has_cover_art: i64,
    pub musicbrainz_track_id: Option<String>,
    pub spotify_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart_mix: i64,
    pub mix_type: Option<String>,
    pub generation_reason: Option<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaybackHistoryRecord {
    pub id: String,
    pub track_id: String,
    pub started_at: i64,
    pub ended_at: i64,
    pub seconds_listened: f64,
    pub percentage_listened: f64,
    pub completed: i64,
    pub skipped: i64,
    pub source: String,
    pub playlist_id: Option<String>,
    pub recommendation_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackStatisticsRecord {
    pub track_id: String,
    pub play_count: i64,
    pub total_time_listened: f64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub last_played_at: Option<i64>,
    pub manual_like: i64,
    pub playlist_addition_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WishlistItemRecord {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub external_track_id: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExternalTrackRecord {
    pub id: String,
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: Option<f64>,
    pub cover_art_url: Option<String>,
    pub match_status: String,
    pub matched_local_track_id: Option<String>,
    pub created_at: i64,
}
