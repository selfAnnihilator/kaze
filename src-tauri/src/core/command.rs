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
    /// One of the per-taste-cluster "Daily Mix N" playlists (1-based).
    DailyCluster(u8),
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
    /// Ordered list playback that stops at the end (Downloads Play all / Shuffle).
    /// `track_ids: None` plays every local library track in library order.
    PlayTrackList {
        track_ids: Option<Vec<String>>,
        start_track_id: Option<String>,
        #[serde(default)]
        shuffle: bool,
        /// With no `track_ids`: "local", "downloads" or absent (every local song).
        #[serde(default)]
        scope: Option<String>,
    },
    /// Shuffle (or restore order of) the remaining tracks of the active Downloads
    /// list. The current song is unaffected; no-op when another source is playing.
    SetArchiveShuffle {
        enabled: bool,
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
    TogglePlayPause,
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
        /// The user picked this song: it plays before the app's own queue.
        #[serde(default)]
        manual: bool,
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
        /// The user picked this song: it plays before the app's own queue.
        #[serde(default)]
        manual: bool,
    },
    DequeueTrack {
        track_id: String,
    },
    ClearQueue,
    ClearRemoteAudioCache,

    // --- Flow Controls ---
    StartFlow {
        #[serde(default)]
        familiarity_ratio: Option<f64>,
    },
    ContinueFlow,
    StopFlow,
    SetFlowSteering {
        familiarity_ratio: f64,
    },

    // --- Library & Onboarding Management ---
    SaveRecommendationSetup {
        #[serde(default)]
        expected_user_id: Option<String>,
        artists: Vec<String>,
        genres: Vec<String>,
        skipped: bool,
    },
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
    /// "Not this artist": blocks every credit of the track's artist everywhere.
    BlockArtist {
        track_id: String,
    },
    /// "Less like this": soft, decaying penalty on the track's artist and genre.
    LessLikeThis {
        track_id: String,
    },
    ClearEntityFeedback {
        kind: String,
        key: String,
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
    SearchDownloads {
        artist: String,
        title: String,
        album: Option<String>,
    },
    DownloadTrack {
        track_id: String,
    },
    /// Deletes Kaze's downloaded file; the song becomes an online song again.
    RemoveDownload {
        track_id: String,
        #[serde(default)]
        source_id: Option<String>,
    },
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
    FillLibraryGaps {
        #[serde(default)]
        retry: bool,
    },
    RemoveProviderEnrichment,
    SetMetadataPreference {
        key: String,
        enabled: bool,
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
    // --- Window / Lifecycle Settings ---
    SetCloseButtonBehavior {
        behavior: String,
    },
    SetInterfaceStyle {
        style: String,
    },
}

/// Result returned from command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum CommandResponse {
    Ok,
    EntityFeedback(Vec<crate::recommendations::entity_feedback::EntityFeedbackRow>),
    EntityId(String),
    MixGenerated {
        playlist_id: String,
        track_count: usize,
    },
    OnboardingCompleted {
        configured_folders: usize,
    },
    SearchResults(Vec<serde_json::Value>),
    DownloadStarted {
        task_id: String,
    },
    WishlistAdded {
        count: usize,
    },
    UserProfile(serde_json::Value),
    CloudSyncCompleted {
        synced_at: i64,
    },
    RemoteAudioCacheCleared {
        bytes_freed: u64,
        files_removed: usize,
    },
}
