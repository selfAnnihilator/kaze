# Project Task Tracking

## Current (Phase 2: Local Music Library)
- [ ] Implement `LibraryFolder` scanning commands.
- [ ] Build recursive filesystem scanner using `walkdir` and `lofty`.
- [ ] Implement incremental scan cache with file modification timestamps and SHA256 hashes.
- [ ] Build Track, Artist, Album, Genre normalization and repositories.
- [ ] Implement SQLite FTS5 search indexer.
- [ ] Implement `notify`-based directory watcher.

## Next (Phase 3: Playback Engine)
- [ ] AudioBackend abstraction trait.
- [ ] Native audio implementation using `rodio` and `cpal`.
- [ ] Playback controls: play, pause, resume, stop, seek, volume attenuation, mute.
- [ ] Playback queue management: play next, enqueue, reorder, clear.
- [ ] Shuffle and Repeat modes (Repeat Off, Repeat One, Repeat All).
- [ ] High-frequency position tracking and playback event emission (`TrackStarted`, `PlaybackPaused`, etc.).
- [ ] Headless playback tests.

## Later (Phases 4 - 9)
- [ ] Playback history, meaningful-play detection, and ranking engine (Phase 4).
- [ ] Taste profiling and smart mix generator (Phase 5).
- [ ] External metadata providers (MusicBrainz, Spotify) (Phase 6).
- [ ] Discovery recommendations, fuzzy track matcher, and wishlist (Phase 7).
- [ ] Soulseek client integration (Phase 8).
- [ ] Tauri React/TS frontend and presentation layer (Phase 9).

## Completed
- [x] Architectural design and technical philosophy specifications.
- [x] System architecture diagram and module layout (`docs/ARCHITECTURE.md`, `docs/BACKEND.md`).
- [x] Exhaustive SQLite schema with 17 relational entities and FTS5 search (`docs/DATABASE.md`).
- [x] Command, Event, and Query catalogs with serialization contracts (`docs/EVENTS.md`).
- [x] Architecture Decision Records (`docs/adr/0001` through `0005`).
- [x] Project roadmap and lifecycle status tracking (`ROADMAP.md`, `PROJECT_STATUS.md`).
- [x] Phase 1 Foundation:
  - [x] Cargo workspace and core Rust crate (`music-player-backend`).
  - [x] Dependency license compliance audit (`docs/DEPENDENCIES.md`).
  - [x] Core domain error types (`AppError`, `AppResult`).
  - [x] Application configuration subsystem (`AppConfig`).
  - [x] Structured diagnostics with `tracing` and `tracing-subscriber`.
  - [x] SQLite initial schema migration SQL (`20260911000000_initial_schema.sql`).
  - [x] Database connection pooling (`sqlx`) with WAL mode and automatic migration execution.
  - [x] `EventBus` backed by `tokio::sync::broadcast` channel.
  - [x] Strongly typed `Command`, `Event`, `Query` serialization contracts.
  - [x] `CoreProcessor` command dispatcher and query execution coordinator.
  - [x] Unit and integration tests passing (`cargo test`).
