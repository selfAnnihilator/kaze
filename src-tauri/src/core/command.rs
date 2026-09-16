use serde::{Deserialize, Serialize};

/// Supported repeat modes for audio playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    One,
    All,
}

/// Supported smart mix categories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartMixType {
    Daily,
    OnRepeat,
    Genre(String),
    Artist(String),
    ForgottenFavorites,
    LateNight,
    Discovery,
}

/// Status markers for download wishlist items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WishlistStatus {
    Want,
    Ignore,
    AlreadyOwn,
    Downloaded,
}

/// Commands represent actions and mutations dispatched to the CoreProcessor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum Command {
    // --- Playback Controls ---
    PlayTrack {
        track_id: String,
        source: Option<String>,
    },
    PlayOnlineTrack {
        track_id: String,
        title: String,
        artist: String,
        album: Option<String>,
        duration_secs: Option<f64>,
        cover_art_url: Option<String>,
        preview_url: Option<String>,
        source: Option<String>,
    },
    PlayQueueIndex {
        index: usize,
    },
    Pause,
    Resume,
    Stop,
    NextTrack,
    PreviousTrack,
    Seek {
        position_secs: f64,
    },
    SetVolume {
        volume: f32, // 0.0 to 1.0
    },
    ToggleMute,
    SetRepeatMode {
        mode: RepeatMode,
    },
    SetShuffle {
        enabled: bool,
    },
    EnqueueTrack {
        track_id: String,
        play_next: bool,
    },
    EnqueueOnlineTrack {
        track_id: String,
        title: String,
        artist: String,
        album: Option<String>,
        duration_secs: Option<f64>,
        cover_art_url: Option<String>,
        preview_url: Option<String>,
        play_next: bool,
    },
    DequeueTrack {
        track_id: String,
    },
    ClearQueue,
    ClearRemoteAudioCache,

    // --- Library & Onboarding Management ---
    CompleteOnboarding {
        music_folders: Vec<String>,
        start_scan: bool,
    },
    ResetOnboarding,
    AddLibraryFolder {
        path: String,
    },
    RemoveLibraryFolder {
        folder_id: String,
    },
    ScanLibrary {
        folder_id: Option<String>,
        incremental: bool,
    },
    CancelScan,

    // --- Playlist Operations ---
    CreatePlaylist {
        name: String,
        description: Option<String>,
    },
    DeletePlaylist {
        playlist_id: String,
    },
    RenamePlaylist {
        playlist_id: String,
        name: String,
    },
    EnsureLikedSongsPlaylist,
    AddTrackToPlaylist {
        playlist_id: String,
        track_id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        artist: Option<String>,
        #[serde(default)]
        album: Option<String>,
        #[serde(default)]
        duration_secs: Option<f64>,
        #[serde(default)]
        cover_art_url: Option<String>,
        #[serde(default)]
        preview_url: Option<String>,
    },
    RemoveTrackFromPlaylist {
        playlist_id: String,
        track_id: String,
    },
    GenerateSmartMix {
        mix_type: SmartMixType,
    },

    // --- User Feedback & Taste ---
    LikeTrack {
        track_id: String,
    },
    DislikeTrack {
        track_id: String,
    },
    RemoveTrackFeedback {
        track_id: String,
    },

    // --- Discovery & Wishlist ---
    AddToWishlist {
        title: String,
        artist: String,
        album: Option<String>,
        external_id: Option<String>,
    },
    AddMissingToWishlist {
        tracks: Vec<serde_json::Value>,
    },
    UpdateWishlistStatus {
        wishlist_id: String,
        status: WishlistStatus,
    },
    SearchSoulseek {
        artist: String,
        title: String,
        album: Option<String>,
    },
    LaunchSoulseek {
        search_query: Option<String>,
        #[serde(default)]
        filter_query: Option<String>,
    },
    ImportSoulseekDownloads,
    StartDownload {
        search_result_id: String,
        wishlist_id: Option<String>,
    },
    CancelDownload {
        task_id: String,
    },
    PollDownloadProgress {
        task_id: String,
    },
    TriggerMetadataRefresh {
        track_id: String,
    },
    // --- User Profile & Auth ---
    SignUp {
        username: String,
        password: String,
    },
    Login {
        username: String,
        password: String,
    },
    Logout,
    LogoutAll,
    RevokeSession {
        session_id: String,
    },
    UploadAvatar {
        #[serde(default)]
        file_path: Option<String>,
    },
    RemoveAvatar,
    // --- Cloud Authentication & Sync ---
    SyncCloudData,
    SetCloudServerUrl {
        url: String,
    },
    // --- Playback Session Tracking ---
    RecordPlaybackSession {
        track_id: String,
        title: String,
        artist: Option<String>,
        album: Option<String>,
        duration_secs: f64,
        seconds_listened: f64,
        completed: bool,
        skipped: bool,
        source: String,
    },
}

/// Result returned from command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum CommandResponse {
    Ok,
    EntityId(String),
    QueuedIndex(usize),
    ScanStarted { task_id: String },
    MixGenerated { playlist_id: String, track_count: usize },
    OnboardingCompleted { configured_folders: usize },
    SearchResults(Vec<serde_json::Value>),
    DownloadStarted { task_id: String },
    SoulseekLaunched { message: String },
    SoulseekImported { imported_count: usize },
    WishlistAdded { count: usize },
    UserProfile(serde_json::Value),
    CloudSyncCompleted { synced_at: i64 },
    RemoteAudioCacheCleared { bytes_freed: u64, files_removed: usize },
}
