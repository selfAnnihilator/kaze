use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub client_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudUser {
    pub id: String,
    pub username: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSessionMetadata {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    #[serde(default)]
    pub client_version: Option<String>,
    #[serde(default)]
    pub created_at: Option<i64>,
    pub idle_expires_at: i64,
    pub absolute_expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub user: Option<CloudUser>,
    pub token: Option<String>,
    pub session: Option<RemoteSessionMetadata>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub client_version: String,
    pub created_at: i64,
    pub last_used_at: i64,
    pub idle_expires_at: i64,
    pub absolute_expires_at: i64,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    pub success: bool,
    #[serde(default)]
    pub sessions: Vec<SessionInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionExpiredReason {
    IdleTimeout,
    AbsoluteTimeout,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", content = "payload")]
#[serde(rename_all = "snake_case")]
pub enum AuthSessionState {
    SignedOut,
    Authenticating,
    OnlineAuthenticated { user: CloudUser, session_id: String },
    OfflineAuthenticated { user: CloudUser, session_id: String },
    SessionExpired { reason: SessionExpiredReason },
    CloudUnavailable,
    SyncPaused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSong {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub provider: Option<String>,
    pub provider_id: Option<String>,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPlaylist {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub is_smart_mix: i64,
    pub mix_type: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudPlaylistSong {
    pub id: String,
    pub user_id: String,
    pub playlist_id: String,
    pub song_id: String,
    pub position: i64,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSongStat {
    pub id: String,
    pub user_id: String,
    pub song_id: String,
    pub play_count: i64,
    pub total_time_listened: f64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub last_played_at: Option<i64>,
    pub manual_like: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudUserStat {
    pub id: String,
    pub user_id: String,
    pub year: i64,
    pub month: i64,
    pub total_seconds: f64,
    pub top_songs_json: String,
    pub top_artists_json: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudUserSetting {
    pub id: String,
    pub user_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncPayload {
    #[serde(default)]
    pub songs: Vec<CloudSong>,
    #[serde(default)]
    pub playlists: Vec<CloudPlaylist>,
    #[serde(default)]
    pub playlist_songs: Vec<CloudPlaylistSong>,
    #[serde(default)]
    pub song_stats: Vec<CloudSongStat>,
    #[serde(default)]
    pub user_stats: Vec<CloudUserStat>,
    #[serde(default)]
    pub user_settings: Vec<CloudUserSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPullResponse {
    pub success: bool,
    pub user_id: Option<String>,
    pub synced_at: Option<i64>,
    pub data: Option<SyncPayload>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPushResponse {
    pub success: bool,
    pub user_id: Option<String>,
    pub synced_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CloudSessionMetadata {
    pub user_id: String,
    pub username: String,
    pub session_id: String,
    pub device_id: String,
    pub device_name: String,
    pub idle_expires_at: i64,
    pub absolute_expires_at: i64,
    pub last_cloud_validation_at: Option<i64>,
    pub worker_url: String,
    pub synced_at: Option<i64>,
    pub authenticated_before: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncStatus {
    pub connected: bool,
    pub worker_url: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub session_id: Option<String>,
    pub device_name: Option<String>,
    pub idle_expires_at: Option<i64>,
    pub absolute_expires_at: Option<i64>,
    pub last_synced_at: Option<i64>,
    pub session_state: Option<AuthSessionState>,
}
