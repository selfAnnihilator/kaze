# Backend Architecture & Service Specifications

## 1. Modular Monolith Design

The backend is structured as a cohesive modular monolith written in Rust. Each domain service is encapsulated in its own module with defined interfaces, internal state models, and explicit public methods. Services communicate via method invocation through the `CoreProcessor` or asynchronously across the `EventBus`.

```text
src-tauri/src/
├── core/
│   ├── mod.rs               // Core module exports
│   ├── processor.rs         // Central CoreProcessor coordinating all services
│   ├── command.rs           // Strongly typed Command definitions
│   ├── query.rs             // Read-only Query definitions
│   ├── event.rs             // Domain Event definitions
│   ├── event_bus.rs         // Tokio broadcast event dispatcher
│   └── error.rs             // Comprehensive Error types via thiserror
│
├── playback/
│   ├── mod.rs
│   ├── service.rs           // PlaybackService orchestrating playback logic
│   ├── backend.rs           // AudioBackend trait & RodioAudioBackend
│   ├── queue.rs             // Playback queue, shuffle, and repeat state
│   └── stream.rs            // StreamPlaybackManager: bounded LRU cache & remote stream resolution
│
├── library/
│   ├── mod.rs
│   ├── service.rs           // LibraryService: folder management & scans
│   ├── scanner.rs           // Recursive directory scanner with lofty extraction
│   └── watcher.rs           // notify-based filesystem watcher
│
├── database/
│   ├── mod.rs               // SqlitePool initialization & migrations
│   ├── models.rs            // Database row representations
│   └── repositories/        // Repository traits and implementations
│       ├── mod.rs
│       ├── track_repo.rs
│       ├── artist_repo.rs
│       ├── album_repo.rs
│       ├── playlist_repo.rs
│       ├── history_repo.rs
│       └── stats_repo.rs
│
├── history/
│   ├── mod.rs
│   └── service.rs           // HistoryService: session tracking & play threshold evaluation
│
├── statistics/
│   ├── mod.rs
│   └── service.rs           // StatisticsService: rolling window aggregations
│
├── ranking/
│   ├── mod.rs
│   └── engine.rs            // Multi-signal weighted scoring engine
│
├── recommendations/
│   ├── mod.rs
│   ├── taste.rs             // Short-term vs long-term taste profiling
│   ├── scoring.rs           // Recommendation scoring and explainability factors
│   ├── local.rs             // Offline library-based recommendation generator
│   ├── discovery.rs         // External recommendation coordinator
│   └── mixes.rs             // Smart temporary playlist generation (Daily Mix, etc.)
│
├── metadata/
│   ├── mod.rs
│   ├── provider.rs          // MetadataProvider trait
│   ├── embedded.rs          // Lofty embedded tag provider
│   ├── musicbrainz.rs       // MusicBrainz & Cover Art Archive client
│   └── spotify.rs           // Spotify Web API client (read-only discovery)
│
├── downloads/
│   ├── mod.rs
│   ├── provider.rs          // DownloadProvider trait
│   ├── wishlist.rs          // Wishlist management
│   └── soulseek.rs          // Local Soulseek client integration
│
├── config/
│   ├── mod.rs
│   └── settings.rs          // AppSettings and persistent configuration
│
└── tasks/
    ├── mod.rs
    └── scheduler.rs         // Background worker task management
```

---

## 2. CoreProcessor Interface & Lifecycle

The `CoreProcessor` owns or maintains shared handles (`Arc`) to all underlying services. It serves as the sole gateway for incoming commands and queries from the IPC layer.

```rust
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    playback_service: Arc<PlaybackService>,
    library_service: Arc<LibraryService>,
    history_service: Arc<HistoryService>,
    stats_service: Arc<StatisticsService>,
    recommendation_engine: Arc<RecommendationEngine>,
    metadata_coordinator: Arc<MetadataCoordinator>,
    download_service: Arc<DownloadService>,
    config: Arc<RwLock<AppConfig>>,
}

impl CoreProcessor {
    pub async fn dispatch_command(&self, cmd: Command) -> Result<CommandResponse, AppError>;
    pub async fn execute_query(&self, query: Query) -> Result<QueryResponse, AppError>;
    pub fn event_bus(&self) -> Arc<EventBus>;
}
```

### Lifecycle Flow: Command Execution
1. Frontend sends command via Tauri IPC (e.g. `invoke("dispatch_command", { cmd: ... })`).
2. `CoreProcessor::dispatch_command` validates preconditions (e.g., track exists in database).
3. The designated domain service executes the command logic.
4. If state changes, the service emits an event onto the `EventBus`.
5. The `EventBus` broadcasts the event to all internal background listeners and directly forwards it to the Tauri webview via `app_handle.emit()`.
6. `CoreProcessor` returns a structured `CommandResponse` or typed `AppError` to the caller.

---

## 3. Error Handling Architecture

Errors are unified using `thiserror` to create descriptive, strongly typed domain error hierarchies.

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("Playback error: {0}")]
    Playback(String),

    #[error("Library error: {0}")]
    Library(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Metadata provider error: {0}")]
    Metadata(String),

    #[error("Recommendation error: {0}")]
    Recommendation(String),

    #[error("External API error ({provider}): {message}")]
    ExternalApi { provider: String, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}
```

**Resilience Contract**: External API errors (e.g., Spotify network timeout or HTTP 429) map to `AppError::ExternalApi` and are logged as warnings. They must never crash the application or prevent local audio playback.

---

## 4. Background Task System

Background operations (directory scanning, metadata fetching, taste recalculation) run on the Tokio thread pool using lightweight async worker loops. No heavy external message brokers (Redis/RabbitMQ/Kafka) are employed.

* **Task Prioritization**:
  1. *Immediate UI tasks* (Command dispatch, playback adjustments) - prioritized latency < 10ms.
  2. *Interactive background tasks* (Local search queries, single-track metadata lookup).
  3. *Batch background tasks* (Recursive directory scanning, rolling statistics aggregation) - yield frequently via `tokio::task::yield_now()` to prevent starving I/O.

---

## 5. Cloud & Session Management Subsystem (`cloud/`)

Located in `src-tauri/src/cloud/`, this subsystem manages cloud authentication, credential protection, and hybrid session lifecycle:

```text
src-tauri/src/cloud/
├── mod.rs               // Module root re-exports
├── client.rs            // CloudClient: HTTP client communicating with Cloudflare Worker
├── credentials.rs       // Secure OS keyring and 0600 file credential storage
├── device.rs            // Device ID generation (UUID v4) and OS friendly naming
├── models.rs            // Data models: CloudUser, SessionInfo, AuthSessionState, SyncPayload
└── sync_manager.rs      // SyncManager: SQLite sync staging, bidirectional push/pull
```

### 5.1 Hybrid Session Lifecycle Model
* **Idle Inactivity Timeout**: 30 days (`now < idle_expires_at`).
* **Absolute Hard Ceiling**: 90 days (`now < absolute_expires_at`).
* **Server-Side Write Throttling**: The Cloudflare Worker updates `last_used_at` and `idle_expires_at` in D1 at most once every 30 minutes on authenticated traffic, capping D1 writes.
* **Token Security**: 256-bit cryptographically secure random bearer tokens. The raw bearer token is stored exclusively in OS credential storage (`keyring` / `0600` file) and sent via HTTP `Authorization: Bearer <token>`. The server stores only the SHA-256 hash in D1; local SQLite stores only non-secret metadata.
* **Multi-Device Revocation**:
  - `Command::Logout`: Revokes the current session on the Worker.
  - `Command::LogoutAll`: Revokes all sessions for the authenticated user.
  - `Command::RevokeSession { session_id }`: Revokes an arbitrary remote session owned by the user.
  - `Query::ListSessions`: Fetches active user sessions with device names and activity timestamps.
* **Non-Destructive Operations & Access Gating**: User logout or session revocation never deletes local audio files, playlists, playlist tracks, track statistics, play history, or download tasks. Only authentication tokens and session rows are purged. Queries (`Query::GetPlaylists`) are access-gated: unauthenticated callers receive Smart Mixes only, while authenticated callers receive their custom playlists and synchronized library items.

### 5.2 Deterministic Synchronization Engine (`SyncManager`)
* **Deterministic Flow (Pull -> Reconcile -> Push)**:
  1. **Pull First**: Fetches the remote `SyncPayload` from `GET /api/sync` on Cloudflare D1.
  2. **Reconcile Locally**: Inserts/updates remote playlists, playlist track associations, and track rating stats into local SQLite without deleting items absent locally unless flagged by a tombstone.
  3. **Push Changes**: Gathers local changes (including newly created playlists, track memberships, play stats, and pending tombstones) and pushes to `POST /api/sync`.
  4. **Clean Tombstones**: Upon successful push confirmation, clears local tombstones.
  5. **Reactive Event Emission**: Emits `Event::PlaylistsUpdated` onto the `EventBus` to notify the frontend to refresh playlist collections and memberships.

### 5.3 Sync Tombstones (`sync_tombstones`)
* Distinguishes between "item not found locally" vs "item intentionally deleted by user".
* Local deletion commands (`DeletePlaylist`, `RemoveTrackFromPlaylist`) record a tombstone in `sync_tombstones`.
* During Pull reconciliation, remote items matching active local tombstones are skipped.
* During Push, tombstones are transmitted in `deleted_playlists` and `deleted_playlist_songs` arrays to purge records from Cloudflare D1.

### 5.4 Song Feedback & Rating Sync
* Synchronizes `manual_like` (+1 liked, -1 disliked, 0 neutral) across devices.
* All songs with `manual_like != 0` or `play_count > 0` are extracted into sync payloads even if not part of a playlist.
* D1 updates use `manual_like = excluded.manual_like`, enabling accurate transitions to disliked (-1) and unrated (0).

## 6. Profile Service & Avatar Processing Subsystem (`src-tauri/src/profile/`)

```
src-tauri/src/profile/
└── mod.rs             // ProfileService: image normalization, WebP encoding, and disk caching
```

### 6.1 Client-Side Normalization Pipeline
* **Input Formats**: JPEG, PNG, WebP parsed via the Rust `image` crate (features: `jpeg`, `png`, `webp`).
* **Validation Bounds**: Enforces maximum 5 MB raw file size and strictly prevents zero-byte payloads.
* **Centered Square Crop**: Identifies shortest side (`min(width, height)`) and computes centered offset `(width - square) / 2` and `(height - square) / 2` using `crop_imm`.
* **Lanczos3 Resizing**: Downscales or upscales cleanly to exact 256×256 pixels using `imageops::FilterType::Lanczos3`.
* **WebP Encoding**: Encodes the normalized buffer to WebP bytes (~20–100 KB target size).

### 6.2 Local Disk Cache & Offline Loading
* **Cache Location**: `<cache_dir>/avatars/{user_id}.webp`.
* **Instant Retrieval**: Returns base64 data URL (`data:image/webp;base64,...`) directly from local disk for sub-millisecond offline UI rendering.
* **Synchronization & Update**:
  - `Command::UploadAvatar`: Prompts native file dialog or takes optional path, normalizes to 256×256 WebP, uploads via Worker to Cloudinary (`POST /api/profile/avatar`), caches to local disk, updates SQLite metadata (`avatar_public_id`, `avatar_url`, `avatar_version`, `avatar_updated_at`), and emits `Event::UserProfileUpdated`.
  - `Command::RemoveAvatar`: Calls `DELETE /api/profile/avatar` on Cloudflare Worker (destroying asset in Cloudinary), purges local cached file, clears SQLite metadata, and emits `Event::UserProfileUpdated`.
  - `Query::GetProfile` & `Query::GetAvatar`: Resolves user profile and cached avatar data URL with offline fallback.
* **Cloudflare Worker Storage Abstraction (`worker/src/storage/avatar.ts`)**:
  - `AvatarStorage` interface decoupling storage from endpoint logic.
  - `CloudinaryAvatarStorage`: Handles signed SHA-1 uploads and deletions targeting `music-player/avatars/{user_id}`, returning `public_id`, `secure_url`, `version`.
  - Secure credential isolation: `CLOUDINARY_CLOUD_NAME`, `CLOUDINARY_API_KEY`, and `CLOUDINARY_API_SECRET` are stored as Worker environment secrets.


