use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaylistRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

mod sqlite;
pub use sqlite::SqlitePlaylistRepository;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistTrackDetail {
    pub id: String,
    pub file_path: String,
    pub file_size: i64,
    pub modified_timestamp: i64,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub genre_name: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub duration_secs: f64,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub format: String,
    pub has_cover_art: i64,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
    pub created_at: i64,
}

#[async_trait]
pub trait PlaylistRepository: Send + Sync {
    async fn create_playlist(&self, playlist: &PlaylistRecord) -> AppResult<()>;
    async fn get_playlist(&self, id: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn get_all_playlists(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mixes(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mixes_for_user(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mix_by_type(&self, mix_type: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_smart_mix(&self, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_smart_mix_for_user(&self, user_id: &str, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn delete_duplicate_smart_mixes(&self, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()>;
    async fn delete_duplicate_smart_mixes_for_user(&self, user_id: &str, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()>;
    async fn delete_playlist(&self, id: &str) -> AppResult<()>;
    async fn rename_playlist(&self, id: &str, name: &str) -> AppResult<()>;
    async fn ensure_liked_songs_playlist(&self, user_id: &str) -> AppResult<String>;
    async fn add_track(&self, playlist_id: &str, track_id: &str, position: Option<i64>) -> AppResult<()>;
    async fn remove_track(&self, playlist_id: &str, track_id: &str) -> AppResult<()>;
    async fn set_tracks(&self, playlist_id: &str, track_ids: &[String]) -> AppResult<()>;
    async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<PlaylistTrackDetail>>;
    async fn get_track_count(&self, playlist_id: &str) -> AppResult<i64>;
    async fn get_track_playlist_memberships(&self) -> AppResult<std::collections::HashMap<String, Vec<String>>>;
    async fn create_playlist_with_user(&self, playlist: &PlaylistRecord, user_id: &str) -> AppResult<()>;
    async fn get_user_playlists(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>>;
    async fn find_by_name_and_user(&self, name: &str, user_id: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn get_track_playlist_memberships_for_user(&self, user_id: &str) -> AppResult<std::collections::HashMap<String, Vec<String>>>;
    async fn delete_all_user_playlists(&self, user_id: Option<&str>) -> AppResult<()>;
    async fn reassign_orphan_playlists_to_user(&self, user_id: &str) -> AppResult<()>;
}
