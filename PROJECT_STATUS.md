# Project Status

## Current Development Phase
**Phase 2: Local Music Library (Completed)** -> **Phase 3: Playback Engine (Ready to Start)**

## Architecture Summary
- **Backend**: Rust 2021 modular monolith running on Tokio async runtime.
- **IPC & Desktop Shell**: Tauri v2 with React + TypeScript.
- **Database**: SQLite 3 with WAL mode, managed through `sqlx` and embedded migrations.
- **Core Pattern**: Central Processor for validated command execution + Tokio broadcast Event Bus for decoupled notifications.
- **Audio Engine**: High-level abstraction wrapping `rodio` and `cpal`.
- **Metadata**: `lofty` for tag extraction, with provider abstractions for MusicBrainz/Cover Art Archive/Spotify.
- **Boundary Containment**: Enforces that scans operate strictly inside configured roots and their child directories without traversing outside or across unapproved symlinks.

## Current Working Features
- Complete documentation suite and Architecture Decision Records (`docs/adr/0001` through `0006`).
- Complete SQLite relational schema (17 entities + FTS5 full-text search) with embedded migrations.
- Full `Command`, `Event`, and `Query` catalogs with typed `serde` serialization.
- `AppError` taxonomy with `thiserror`.
- `AppConfig` supporting OS-standard directories, audio settings, ranking weights, and history thresholds.
- `EventBus` backed by `tokio::sync::broadcast` with graceful zero-subscriber dispatching.
- `CoreProcessor` command router and query execution coordinator.
- SQLite connection manager with WAL mode, foreign keys, and 5-second busy timeout.
- Fully operational headless test suite and CLI binary verification.
- **Default Music Directory Discovery**: Automatic system audio directory lookup via `directories::UserDirs::audio_dir()`.
- **Onboarding Workflow**: `GetOnboardingStatus` and `CompleteOnboarding` commands letting users confirm the default music folder or choose alternate directories.
- **Strict Boundary Containment**: Scans are strictly restricted to registered directories and their child subdirectories; symlinks pointing outside the boundary are discarded.
- **Recursive Directory Scanner**: Multi-format audio parsing (MP3, FLAC, OGG, OPUS, M4A, WAV) with `lofty`.
- **Incremental Scanning**: Compares file size and modified timestamps to skip unchanged tracks.
- **Deleted Track Pruning**: Automatically removes tracks from the database and FTS5 search index when deleted from disk.
- **Full-Text Search (FTS5)**: Fast keyword searching across title, artist, album, and genre with automatic SQL triggers.
- **Directory Monitoring**: Background directory watching using `notify`.

## Partially Implemented Features
- None (Phase 1 and Phase 2 fully realized and verified).

## Known Broken Features
- None.

## Important Files & Modules
- `docs/ARCHITECTURE.md` - Overall system structure and threading model.
- `docs/BACKEND.md` - Modular monolith structure and service interfaces.
- `docs/DATABASE.md` - Complete SQLite schema and optimization PRAGMAs.
- `docs/EVENTS.md` - Exhaustive Command, Event, and Query catalogs.
- `docs/LIBRARY.md` - Library scanning, boundary containment, and metadata extraction.
- `src-tauri/src/core/` - `CoreProcessor`, `EventBus`, `Command`, `Event`, `Query`, `AppError`.
- `src-tauri/src/library/` - `LibraryService`, `LibraryScanner`, `LibraryWatcher`.
- `src-tauri/src/database/` - Connection pooling, models, migrations, and repositories (`TrackRepository`, `ArtistRepository`, `AlbumRepository`, `SettingsRepository`, `LibraryFolderRepository`).
- `src-tauri/src/config/` - Persistent settings and defaults.
- `src-tauri/tests/` - Integration test suites (`foundation_tests.rs`, `library_tests.rs`).

## Current Blockers
- None.

## Next Recommended Task
Begin **Phase 3: Playback Engine**: implement `AudioBackend` trait with `rodio` and `cpal`, playback queue management, play/pause/seek/volume controls, shuffle/repeat modes, and playback position event streams.
