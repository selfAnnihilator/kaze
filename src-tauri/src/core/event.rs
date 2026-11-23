use serde::{Deserialize, Serialize};

/// Representation of an item within the active playback queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueItem {
    pub queue_id: String,
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: f64,
}

/// Events represent state changes and domain occurrences published across the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", content = "payload")]
pub enum Event {
    LibraryMetadataUpdated,
    RecommendationFeedbackChanged {
        user_id: String,
        track_id: String,
        title: String,
        track_ids: Vec<String>,
        disliked: bool,
        manual_like: i64,
    },
    EntityFeedbackChanged {
        user_id: String,
        rows: Vec<crate::recommendations::entity_feedback::EntityFeedbackRow>,
        cleared: bool,
    },
    // --- Playback State Changes ---
    PlaybackStarted {
        session_id: String,
        track_id: String,
        title: String,
        artist: String,
        duration_secs: f64,
        source: String,
    },
    PlaybackPaused {
        session_id: String,
        track_id: String,
        position_secs: f64,
    },
    PlaybackResumed {
        session_id: String,
        track_id: String,
        position_secs: f64,
    },
    PlaybackStopped {
        session_id: String,
    },
    OnlinePlaybackRequested {
        track_id: String,
        title: String,
        artist: String,
        album: Option<String>,
        duration_secs: f64,
    },
    PlaybackPositionChanged {
        session_id: String,
        position_secs: f64,
        duration_secs: f64,
    },
    PlaybackSeeked {
        position_secs: f64,
    },
    /// A source capability changed without changing playback state or position.
    PlaybackCapabilitiesChanged {
        track_id: String,
    },
    PlaybackVolumeChanged {
        volume: f32,
        is_muted: bool,
    },
    /// Shared (`Arc`) so each event-bus subscriber's copy is cheap; the JSON
    /// shape is unchanged.
    QueueUpdated {
        items: std::sync::Arc<Vec<QueueItem>>,
        current_index: Option<usize>,
        queue_track_ids: std::sync::Arc<Vec<String>>,
        /// Queue position of each upcoming entry, in play order (shuffle-aware).
        #[serde(default)]
        upcoming_indices: std::sync::Arc<Vec<usize>>,
        /// Songs the user queued that are still waiting, in the order they were added.
        #[serde(default)]
        user_queue_track_ids: std::sync::Arc<Vec<String>>,
        /// Playback source that owns the queue ("library", "flow", "archive", ...).
        #[serde(default)]
        source: String,
    },
    PlaybackError {
        session_id: String,
        message: String,
    },
    PlaybackBuffering {
        session_id: String,
        track_id: String,
    },
    /// A reader stalled/recovered within an already-started playback session.
    /// This does not change transport state or reset the playback position.
    PlaybackBufferingChanged {
        session_id: String,
        track_id: String,
        is_buffering: bool,
    },

    // --- History & Track Milestones ---
    TrackFinished {
        session_id: String,
        track_id: String,
        seconds_listened: f64,
        completed: bool,
    },
    /// Committed listening outcome for one finished play (skip vs full listen).
    /// Published after the history row is stored; Flow uses it for in-session steering.
    ListenOutcome {
        user_id: String,
        track_id: String,
        early_skip: bool,
        qualified: bool,
        near_complete: bool,
        weight: f64,
    },

    // --- Library Lifecycle ---
    LibraryScanStarted {
        folder_path: String,
    },
    LibraryScanProgress {
        scanned_files: usize,
        total_files: usize,
        current_file: String,
    },
    LibraryScanCompleted {
        added_tracks: usize,
        updated_tracks: usize,
        removed_tracks: usize,
        duration_ms: u64,
    },
    LibraryScanFailed {
        error: String,
    },

    // --- Recommendations & Mixes ---
    /// Published only after listening or explicit feedback has been committed.
    TasteProfileChanged {
        user_id: String,
    },
    SmartMixGenerated {
        playlist_id: String,
        mix_type: String,
        track_count: usize,
    },

    // --- Provider Status ---
    ProviderStatusChanged {
        provider: String,
        available: bool,
        message: Option<String>,
    },

    // --- Downloads Lifecycle ---
    DownloadQueued {
        task_id: String,
        title: String,
        artist: String,
    },
    DownloadProgressChanged {
        task_id: String,
        bytes_downloaded: i64,
        total_bytes: i64,
        speed_bps: u64,
    },
    DownloadCompleted {
        task_id: String,
        file_path: String,
    },
    DownloadFailed {
        task_id: String,
        error: String,
    },
    /// Files in the Kaze download folder changed (added, removed or renamed).
    DownloadsChanged,

    // --- Auth & Session Lifecycle ---
    SessionChanged {
        user: Option<serde_json::Value>,
    },
    UserLoggedOut,
    PlaylistsUpdated,
    /// A cloud sync merged other devices' listening data into this profile.
    CloudDataSynced,
    UserProfileUpdated {
        user: serde_json::Value,
    },
}
