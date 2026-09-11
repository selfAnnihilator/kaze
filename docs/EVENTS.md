# Event & Command Architecture Catalog

## 1. Communication Paradigm

The system adheres to a strict Command-Query Responsibility Segregation (CQRS) and Event-Driven architecture across the IPC boundary:

```text
[ Frontend ] ── Command ──> [ CoreProcessor ] ──> [ Domain Service ]
                                                        │
                                                     Emits
                                                        ▼
[ Frontend ] <── Event ──── [ EventBus ] <──────────────┘
```

1. **Commands (`Command`)**: Intent to mutate state. Sent by frontend, handled by `CoreProcessor`, executed transactionally by domain services. Returns `Result<CommandResponse, AppError>`.
2. **Queries (`Query`)**: Read-only requests for state, library data, rankings, or recommendations. Dispatched directly to repositories or services. Returns `Result<QueryResponse, AppError>`.
3. **Events (`Event`)**: Fact-based notifications of state transitions that have occurred. Published over Tokio broadcast channel, consumed by internal services (e.g. HistoryService logging upon `TrackFinished`) and relayed to the Tauri webview.

---

## 2. Command Catalog

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum Command {
    // --- Playback Controls ---
    PlayTrack { track_id: String, source: Option<String> },
    PlayQueueIndex { index: usize },
    Pause,
    Resume,
    Stop,
    NextTrack,
    PreviousTrack,
    Seek { position_secs: f64 },
    SetVolume { volume: f32 }, // 0.0 to 1.0
    ToggleMute,
    SetRepeatMode { mode: RepeatMode }, // Off, One, All
    SetShuffle { enabled: bool },
    EnqueueTrack { track_id: String, play_next: bool },
    ClearQueue,

    // --- Library Management ---
    AddLibraryFolder { path: String },
    RemoveLibraryFolder { folder_id: String },
    ScanLibrary { incremental: bool },
    CancelScan,

    // --- Playlist Operations ---
    CreatePlaylist { name: String, description: Option<String> },
    DeletePlaylist { playlist_id: String },
    AddTrackToPlaylist { playlist_id: String, track_id: String },
    RemoveTrackFromPlaylist { playlist_id: String, track_id: String },
    GenerateSmartMix { mix_type: SmartMixType }, // Daily, OnRepeat, Genre, Artist, ForgottenFavorites

    // --- User Feedback & Taste ---
    LikeTrack { track_id: String },
    DislikeTrack { track_id: String },
    RemoveTrackFeedback { track_id: String },

    // --- Discovery & Wishlist ---
    AddToWishlist { title: String, artist: String, album: Option<String>, external_id: Option<String> },
    UpdateWishlistStatus { wishlist_id: String, status: WishlistStatus },
    SearchSoulseek { artist: String, title: String, album: Option<String> },
    TriggerMetadataRefresh { track_id: String },
}
```

---

## 3. Event Catalog

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    PlaybackPaused { track_id: String, position_secs: f64 },
    PlaybackResumed { track_id: String, position_secs: f64 },
    PlaybackStopped,
    PlaybackPositionChanged { position_secs: f64, duration_secs: f64 },
    PlaybackSeeked { position_secs: f64 },
    PlaybackVolumeChanged { volume: f32, is_muted: bool },
    QueueUpdated {
        items: Vec<QueueItem>,
        current_index: Option<usize>,
    },
    PlaybackError { message: String },

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
    LibraryScanStarted { folder_path: String },
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
    LibraryScanFailed { error: String },

    // --- Recommendations & Playlists ---
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
}
```

---

## 4. Query Catalog

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "query", content = "payload")]
pub enum Query {
    GetLibraryOverview,
    GetTracks { offset: u32, limit: u32, sort_by: Option<String>, ascending: bool },
    GetTrackById { track_id: String },
    GetArtists { offset: u32, limit: u32 },
    GetArtistDetails { artist_id: String },
    GetAlbums { offset: u32, limit: u32 },
    GetAlbumDetails { album_id: String },
    GetPlaylists,
    GetPlaylistTracks { playlist_id: String },
    SearchLibrary { query_text: String, limit: u32 },
    GetTopRankings { window: TimeWindow, entity: RankingEntity, limit: u32 },
    GetTasteProfile,
    GetSmartMixes,
    GetLocalRecommendations { limit: u32 },
    GetDiscoveryRecommendations { limit: u32 },
    GetWishlist { status: Option<WishlistStatus> },
    GetSettings,
    GetPlaybackState,
}
```
