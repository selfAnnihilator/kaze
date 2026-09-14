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
    // --- Playback State Changes ---
    PlaybackStarted {
        track_id: String,
        title: String,
        artist: String,
        duration_secs: f64,
        source: String,
    },
    PlaybackPaused {
        track_id: String,
        position_secs: f64,
    },
    PlaybackResumed {
        track_id: String,
        position_secs: f64,
    },
    PlaybackStopped,
    PlaybackPositionChanged {
        position_secs: f64,
        duration_secs: f64,
    },
    PlaybackSeeked {
        position_secs: f64,
    },
    PlaybackVolumeChanged {
        volume: f32,
        is_muted: bool,
    },
    QueueUpdated {
        items: Vec<QueueItem>,
        current_index: Option<usize>,
        queue_track_ids: Vec<String>,
    },
    PlaybackError {
        message: String,
    },

    // --- History & Track Milestones ---
    TrackFinished {
        track_id: String,
        seconds_listened: f64,
        completed: bool,
    },
    TrackSkipped {
        track_id: String,
        seconds_listened: f64,
        percentage_listened: f64,
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
    SmartMixGenerated {
        playlist_id: String,
        mix_type: String,
        track_count: usize,
    },
    RecommendationsRefreshed {
        session_id: String,
        recommendation_count: usize,
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

    // --- Auth & Session Lifecycle ---
    SessionChanged {
        user: Option<serde_json::Value>,
    },
    UserLoggedOut,
    PlaylistsUpdated,
    UserProfileUpdated {
        user: serde_json::Value,
    },
}
