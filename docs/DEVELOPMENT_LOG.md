# AI Development Log

## 2026-09-11

### Worked On
Initial project inception, architectural specification, database schema formalization, communication protocol design, and ADR documentation for the backend-first intelligent local music player.

### Changes
- Created `docs/ARCHITECTURE.md` specifying the backend-heavy modular monolith, threading model, and component interactions.
- Created `docs/BACKEND.md` detailing the module layout, `CoreProcessor` lifecycle, error hierarchies, and background task system.
- Created `docs/DATABASE.md` defining the complete SQLite relational schema (17 entities + FTS5 virtual table) and database PRAGMA performance tuning.
- Created `docs/EVENTS.md` cataloging all `Command`, `Event`, and `Query` variants with payload semantics.
- Created ADRs:
  - `0001-use-rust.md`: Rust for backend core logic.
  - `0002-use-tauri.md`: Tauri for thin presentation shell and typed IPC.
  - `0003-use-sqlite.md`: SQLite with `sqlx` for zero-maintenance local ACID persistence.
  - `0004-modular-monolith.md`: Modular monolith avoiding premature microservices.
  - `0005-central-processor-and-event-bus.md`: Central Processor + Tokio broadcast EventBus.
- Created `PROJECT_STATUS.md`, `ROADMAP.md`, `TODO.md`, and `docs/DEPENDENCIES.md`.

### Decisions
- Locked technology stack: Rust, Tauri, React + TypeScript, SQLite with `sqlx`.
- Prohibited business logic from residing in the frontend; all audio playback, scanning, ranking, and recommendation computation strictly lives in Rust.
- Encapsulated audio playback behind an `AudioBackend` trait and metadata extraction behind a `MetadataReader` / `MetadataProvider` trait.
- Non-blocking event broadcasting: EventBus gracefully succeeds with 0 receivers when no UI or internal components are actively subscribed.

### Problems
- Initial `sqlx` compilation error resolved by enabling the `macros` feature flag in `Cargo.toml`.
- Replaced missing `use std::str::FromStr;` import for `SqliteConnectOptions`.

### Phase 1 Implementation Summary
- Initialized Cargo workspace with `music-player-backend` crate.
- Configured SQLite embedded migrations with 17 relational tables and FTS5 search.
- Built `CoreProcessor` command dispatcher and query execution subsystem.
- Built `EventBus` backed by `tokio::sync::broadcast`.
- Validated via automated test suite in `tests/foundation_tests.rs` (all 5 tests passing).
- Validated via standalone headless CLI execution (`cargo run --bin music-player-cli`).

### Remaining Work
- Phase 2: Local music scanning with `lofty`, incremental indexing, and SQLite FTS5 search.
- Phase 3: Audio playback engine with `rodio` & `cpal`.
- Phases 4 - 9 as mapped in `ROADMAP.md`.

### Recommended Next Step
Proceed to Phase 2.

---

## 2026-09-11 (Phase 2: Local Music Library)

### Worked On
Local music library discovery, system default audio directory lookup, onboarding choice workflow, strict boundary containment traversal, multi-format metadata extraction with `lofty`, incremental scan caching via mtime and size, and SQLite FTS5 search.

### Changes
- Implemented `LibraryService::get_default_music_dir()` discovering system audio folders via `directories::UserDirs::audio_dir()`.
- Implemented `GetOnboardingStatus` query and `CompleteOnboarding` command letting users accept default directories or choose custom locations.
- Implemented `is_within_boundary()` and `WalkDir::follow_links(false)` to strictly ensure scans never escape outside designated root directories.
- Implemented multi-format audio scanning (`LibraryScanner`) supporting MP3, FLAC, OGG, OPUS, M4A, and WAV files.
- Integrated `lofty` tag reading for titles, artists, albums, track numbers, release years, audio properties (bitrate, sample rate), and embedded artwork indicators.
- Implemented incremental scan checks skipping re-parsing when `file_size` and `modified_timestamp` match database records.
- Implemented automatic pruning of deleted files from disk during scans.
- Added database migration `20260911000001_fts_triggers.sql` for automatic SQLite FTS5 search index synchronization.
- Implemented repositories: `TrackRepository`, `ArtistRepository`, `AlbumRepository`, `SettingsRepository`.
- Implemented `LibraryWatcher` using `notify` for filesystem change events.
- Created `docs/LIBRARY.md` and ADR `0006-strict-boundary-containment-and-incremental-scanning.md`.
- Implemented comprehensive integration test suite `tests/library_tests.rs` (all 9 tests passing).

### Decisions
- Strictly disallow symlink traversal outside library boundaries to preserve privacy and prevent unintended system scanning.
- Require onboarding confirmation before initiating any default directory scans.
- Keep FTS5 table automatically synchronized via SQLite triggers rather than error-prone manual application updates.

### Problems
- SQLite FTS5 content table join requires joining on `fts.rowid = t.rowid` rather than unindexed columns. Fixed in `SqliteTrackRepository::search_tracks`.

### Remaining Work
- Phase 3: Dedicated playback service with `rodio` & `cpal`, playback queue, position tracking, and shuffle/repeat.

### Recommended Next Step
Proceed to Phase 3.

---

## 2026-09-11 (Phase 3: Playback Engine)

### Worked On
Audio output abstraction, thread-safe Rodio/cpal backend integration, headless mock backend, playback queue state machine (enqueue, play next, append, remove, clear), repeat modes, reversible shuffle, 250ms throttled position ticks, and automatic queue advancement.

### Changes
- Created `AudioBackend` trait abstraction in `playback/backend.rs` isolating audio hardware interfaces.
- Implemented `RodioAudioBackend` for native cross-platform audio using `rodio` and `cpal`.
- Implemented `MockAudioBackend` for deterministic, soundcard-independent unit/integration testing in headless CI environments.
- Implemented `PlaybackQueue` supporting queue management, `RepeatMode` (`Off`, `One`, `All`), and reversible randomized `shuffle`.
- Implemented `PlaybackService` coordinating audio decoding, position monitoring loop (250ms ticks), track finish detection, and auto-advance.
- Wired all playback commands (`PlayTrack`, `PlayQueueIndex`, `Pause`, `Resume`, `Stop`, `Seek`, `SetVolume`, `ToggleMute`, `SetRepeatMode`, `SetShuffle`, `EnqueueTrack`, `ClearQueue`) and queries (`GetPlaybackState`) through `CoreProcessor`.
- Created `docs/PLAYBACK.md` and ADR `0007-playback-service-and-audio-backend-abstraction.md`.
- Implemented comprehensive integration test suite `tests/playback_tests.rs` (all 12 tests passing).

### Decisions
- Kept `rodio::OutputStream` detached on process startup to maintain thread-safe `Send + Sync` guarantees across background Tokio tasks.
- Provided `CoreProcessor::new_with_backend()` to permit test injection of `MockAudioBackend` without requiring physical audio hardware.
- Throttled periodic position notifications to 250ms to ensure smooth UI progress bar updates without saturating IPC channels.

### Problems
- `rodio::OutputStream` contains platform-dependent raw pointers on ALSA causing `!Send`. Resolved by detaching the stream for process lifetime and maintaining `Send`-compliant `OutputStreamHandle` and `Sink`.

### Remaining Work
- Phase 4: Listening history logging, meaningful-play detection, and multi-factor ranking (Completed).
- Phase 5: Smart local recommendations, taste profiles, and temporary smart mix generators.

### Recommended Next Step
Proceed to **Phase 4: Listening History & Statistics**.

---

## 2026-09-11 (Phase 4: Listening History & Statistics)

### Worked On
Listening history persistence, meaningful play evaluation, multi-factor ranking engine, user preference feedback (likes/dislikes), and stats aggregation across rolling time windows.

### Changes
- Implemented `HistoryService` in `src-tauri/src/history/service.rs` subscribing asynchronously to `EventBus` to track sessions, seek counts, and accumulated playback duration.
- Implemented strict meaningful play criteria: `listened_seconds >= 30.0 || percentage >= 50.0% || completed`.
- Implemented `SqliteHistoryRepository` recording listening history sessions and tracking user preference feedback.
- Implemented `SqliteStatsRepository` calculating multi-factor scores with time-decay recency, completion weighting, and preference boosts (+30% for likes, -80% for dislikes).
- Implemented `RankingEngine` supporting rolling windows (`Today`, `Last7Days`, `Last30Days`, `Last6Months`, `LastYear`, `AllTime`) for `Tracks`, `Artists`, `Albums`, and `Genres`.
- Wired commands (`LikeTrack`, `DislikeTrack`, `RemoveTrackFeedback`) and queries (`GetTopRankings`) into `CoreProcessor`.
- Created `docs/RANKING.md` and ADR `0008-meaningful-play-and-ranking-engine.md`.
- Implemented comprehensive integration test suite in `tests/history_tests.rs` (all 15 workspace tests passing).

### Decisions
- Asynchronously decouple history logging from the audio playback loop using the event bus to prevent I/O jitter.
- Define meaningful plays with a dual threshold (>=30s or >=50%) to prevent short previews from skewing ranking calculations while properly crediting short songs (<60s).
- Pre-filter disliked tracks from top ranking calculations unless explicitly queried.

### Problems
- None encountered; schema `track_stats`, `listening_history`, and `user_feedback` tables were fully provisioned in Phase 1 initial migration.

### Remaining Work
- Phase 5: Smart local recommendations, taste profiles, and temporary smart mix generators.

### Recommended Next Step
Proceed to **Phase 5: Smart Local Recommendations & Mixes**.
