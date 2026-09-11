# Project Status

## Current Development Phase
**Phase 1: Foundation (Completed)** -> **Phase 2: Local Music Library (Ready to Start)**

## Architecture Summary
- **Backend**: Rust 2021 modular monolith running on Tokio async runtime.
- **IPC & Desktop Shell**: Tauri v2 with React + TypeScript.
- **Database**: SQLite 3 with WAL mode, managed through `sqlx` and embedded migrations.
- **Core Pattern**: Central Processor for validated command execution + Tokio broadcast Event Bus for decoupled notifications.
- **Audio Engine**: High-level abstraction wrapping `rodio` and `cpal`.
- **Metadata**: `lofty` for tag extraction, with provider abstractions for MusicBrainz/Cover Art Archive/Spotify.

## Current Working Features
- Complete documentation suite and Architecture Decision Records (`docs/adr/0001` through `0005`).
- Complete SQLite relational schema (17 entities + FTS5 full-text search) with embedded migrations.
- Full `Command`, `Event`, and `Query` catalogs with typed `serde` serialization.
- `AppError` taxonomy with `thiserror`.
- `AppConfig` supporting OS-standard directories, audio settings, ranking weights, and history thresholds.
- `EventBus` backed by `tokio::sync::broadcast` with graceful zero-subscriber dispatching.
- `CoreProcessor` command router and query execution coordinator.
- SQLite connection manager with WAL mode, foreign keys, and 5-second busy timeout.
- Fully operational headless test suite and CLI binary verification.

## Partially Implemented Features
- None (Phase 1 fully realized and verified).

## Known Broken Features
- None.

## Important Files & Modules
- `docs/ARCHITECTURE.md` - Overall system structure and threading model.
- `docs/BACKEND.md` - Modular monolith structure and service interfaces.
- `docs/DATABASE.md` - Complete SQLite schema and optimization PRAGMAs.
- `docs/EVENTS.md` - Exhaustive Command, Event, and Query catalogs.
- `src-tauri/src/core/` - `CoreProcessor`, `EventBus`, `Command`, `Event`, `Query`, `AppError`.
- `src-tauri/src/database/` - Connection pooling, models, migrations, and repositories.
- `src-tauri/src/config/` - Persistent settings and defaults.
- `src-tauri/tests/foundation_tests.rs` - Test suite.

## Current Blockers
- None.

## Next Recommended Task
Begin **Phase 2: Local Music Library**: implement recursive filesystem scanner using `walkdir` and `lofty`, incremental change detection (mtime & SHA-256 hash), Track/Artist/Album repositories, and SQLite FTS5 search.
