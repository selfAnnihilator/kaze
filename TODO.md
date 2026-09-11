# Project Task Tracking

## Current (Phase 4: Listening History & Statistics)
- [ ] Playback history database logging (`playback_history` table).
- [ ] Meaningful-play threshold evaluation (>= 30s or >= 50%).
- [ ] Incremental track statistics updates (play count, skip count, completion count).
- [ ] Multi-window aggregated statistics (Today, 7D, 30D, 6M, 1Y, All-Time).
- [ ] Multi-factor ranking engine (play count, listening duration, completion rate, recency, likes, skips).
- [ ] Ranking calculations and history verification tests.

## Next (Phase 5: Smart Local Recommendations & Mixes)
- [ ] User taste profile engine: short-term vs long-term affinity.
- [ ] Local offline recommendation engine with explainability factor breakdown.
- [ ] Smart temporary mix generator (Daily Mix, On Repeat, Forgotten Favorites, Genre Mixes, Artist Radios).
- [ ] Repetition avoidance and controlled entropy/randomness.
- [ ] Recommendation scoring tests.

## Later (Phases 6 - 9)
- [ ] External metadata providers (MusicBrainz, Spotify) (Phase 6).
- [ ] Discovery recommendations, fuzzy track matcher, and wishlist (Phase 7).
- [ ] Soulseek client integration (Phase 8).
- [ ] Tauri React/TS frontend and presentation layer (Phase 9).

## Completed
- [x] Architectural design and technical philosophy specifications.
- [x] System architecture diagram and module layout (`docs/ARCHITECTURE.md`, `docs/BACKEND.md`).
- [x] Exhaustive SQLite schema with 17 relational entities and FTS5 search (`docs/DATABASE.md`).
- [x] Command, Event, and Query catalogs with serialization contracts (`docs/EVENTS.md`).
- [x] Architecture Decision Records (`docs/adr/0001` through `0007`).
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
- [x] Phase 2 Local Music Library:
  - [x] Default system music directory discovery (`UserDirs::audio_dir()`).
  - [x] Onboarding state management (`GetOnboardingStatus`, `CompleteOnboarding`).
  - [x] Strict boundary containment (`is_within_boundary`, no traversal outside approved root folders, `follow_links(false)`).
  - [x] Recursive filesystem scanner (`WalkDir`) with multi-format audio support (MP3, FLAC, OGG, OPUS, M4A, WAV).
  - [x] Comprehensive audio metadata extraction using `lofty` (`TaggedFileExt`).
  - [x] Incremental scanning using cached `file_size` and `modified_timestamp` to skip unchanged files.
  - [x] Pruning of deleted files from SQLite database during rescans.
  - [x] Automatic SQLite FTS5 full-text search sync triggers (`20260911000001_fts_triggers.sql`).
  - [x] Repositories for Track, Artist, Album, Settings, and Folder.
  - [x] Directory monitoring with `notify` in `LibraryWatcher`.
  - [x] Comprehensive integration tests in `tests/library_tests.rs`.
- [x] Phase 3 Playback Engine:
  - [x] `AudioBackend` trait abstraction isolating audio driver implementations.
  - [x] `RodioAudioBackend` for native cross-platform audio using `rodio` and `cpal`.
  - [x] `MockAudioBackend` for deterministic headless and CI testing.
  - [x] `PlaybackQueue` with queue items, reordering, enqueue (play next / append), and removal.
  - [x] Repeat modes: `RepeatMode::Off`, `RepeatMode::One`, `RepeatMode::All`.
  - [x] Reversible shuffle mode preserving original track index mappings.
  - [x] `PlaybackService` coordinating audio decoding, position ticking (250ms), and auto-advance.
  - [x] Full suite of playback controls: play, pause, resume, stop, seek, volume, toggle mute, next, previous.
  - [x] Comprehensive integration tests in `tests/playback_tests.rs` (all 12 tests passing).
