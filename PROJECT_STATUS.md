# Project Status

## Current Development Phase
**Phase 3: Playback Engine (Completed)** -> **Phase 4: Listening History & Statistics (Ready to Start)**

## Architecture Summary
- **Backend**: Rust 2021 modular monolith running on Tokio async runtime.
- **IPC & Desktop Shell**: Tauri v2 with React + TypeScript.
- **Database**: SQLite 3 with WAL mode, managed through `sqlx` and embedded migrations.
- **Core Pattern**: Central Processor for validated command execution + Tokio broadcast Event Bus for decoupled notifications.
- **Audio Engine**: Thread-safe `AudioBackend` trait abstraction implemented via `RodioAudioBackend` (rodio/cpal) and `MockAudioBackend` (in-memory for headless testing).
- **Metadata**: `lofty` for tag extraction, with provider abstractions for MusicBrainz/Cover Art Archive/Spotify.
- **Boundary Containment**: Enforces that scans operate strictly inside configured roots and their child directories without traversing outside or across unapproved symlinks.

## Current Working Features
- Complete documentation suite and Architecture Decision Records (`docs/adr/0001` through `0007`).
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
- **Audio Playback Engine**: `AudioBackend` trait abstraction with `RodioAudioBackend` for native cross-platform audio and `MockAudioBackend` for headless/CI testing.
- **Playback Controls**: Complete play, pause, resume, stop, seek, volume attenuation, and mute commands.
- **Queue Management**: Full queue sequencing with `enqueue` (play next or append), removal, clearing, and index navigation (`NextTrack`, `PreviousTrack`).
- **Repeat & Shuffle**: `RepeatMode` (`Off`, `One`, `All`) and reversible random permutation `shuffle`.
- **Position Ticker & Auto-Advance**: 250ms throttled `PlaybackPositionChanged` events and automatic track advancement upon stream finish.

## Partially Implemented Features
- None (Phases 1, 2, and 3 fully realized and verified).

## Known Broken Features
- None.

## Important Files & Modules
- `docs/ARCHITECTURE.md` - Overall system structure and threading model.
- `docs/BACKEND.md` - Modular monolith structure and service interfaces.
- `docs/DATABASE.md` - Complete SQLite schema and optimization PRAGMAs.
- `docs/EVENTS.md` - Exhaustive Command, Event, and Query catalogs.
- `docs/LIBRARY.md` - Library scanning, boundary containment, and metadata extraction.
- `docs/PLAYBACK.md` - Playback service, AudioBackend trait, and queue design.
- `src-tauri/src/core/` - `CoreProcessor`, `EventBus`, `Command`, `Event`, `Query`, `AppError`.
- `src-tauri/src/playback/` - `PlaybackService`, `PlaybackQueue`, `AudioBackend`, `RodioAudioBackend`, `MockAudioBackend`.
- `src-tauri/src/library/` - `LibraryService`, `LibraryScanner`, `LibraryWatcher`.
- `src-tauri/src/database/` - Connection pooling, models, migrations, and repositories.
- `src-tauri/tests/` - Integration test suites (`foundation_tests.rs`, `library_tests.rs`, `playback_tests.rs`).

## Current Blockers
- None.

## Next Recommended Task
Begin **Phase 4: Listening History & Statistics**: implement `playback_history` session logging, rule-based meaningful-play threshold evaluation (>= 30s or >= 50% of track), rolling window aggregations (Today, 7D, 30D, 6M, 1Y, All-Time), and multi-factor ranking algorithms.
