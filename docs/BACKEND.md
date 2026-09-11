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
│   └── queue.rs             // Playback queue, shuffle, and repeat state
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
