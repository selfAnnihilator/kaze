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
    ClearQueue,

    // --- Library & Onboarding Management ---
    CompleteOnboarding {
        music_folders: Vec<String>,
        start_scan: bool,
    },
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
    AddTrackToPlaylist {
        playlist_id: String,
        track_id: String,
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
    UpdateWishlistStatus {
        wishlist_id: String,
        status: WishlistStatus,
    },
    SearchSoulseek {
        artist: String,
        title: String,
        album: Option<String>,
    },
    TriggerMetadataRefresh {
        track_id: String,
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
}
