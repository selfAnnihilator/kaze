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
Proceed to **Phase 3: Playback Engine**.
