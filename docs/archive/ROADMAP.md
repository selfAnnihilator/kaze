# Development Roadmap

This roadmap defines the sequential, incremental progression of the music player project.

---

## Phase 1 — Foundation (Completed)
- [x] Architectural documentation, database schema, event/command contracts, and ADRs.
- [x] Project initialization: Rust project structure, Cargo dependencies, and tooling.
- [x] Structured logging (`tracing` / `tracing-subscriber`).
- [x] Core error handling (`AppError` with `thiserror`).
- [x] Configuration subsystem (paths, audio preferences, ranking weights).
- [x] SQLite database connection pool (`sqlx`) with WAL mode and initial migrations.
- [x] `EventBus` implementation using Tokio broadcast channels.
- [x] `CoreProcessor` command dispatcher skeleton.
- [x] Foundation unit & integration tests.

---

## Phase 2 — Local Music Library (Completed)
- [x] Default system music directory lookup and onboarding status management.
- [x] Library folder registration and persistence.
- [x] Strict boundary containment (never scans outside root; discards unapproved symlinks).
- [x] Recursive filesystem scanner with `lofty` metadata extraction (MP3, FLAC, OGG, OPUS, M4A, WAV).
- [x] Incremental scan caching via file size and modified timestamps.
- [x] Automatic pruning of deleted files on folder rescan.
- [x] SQLite FTS5 full-text search index triggers and repository search method.
- [x] Repositories for Tracks, Artists, Albums, and Genres.
- [x] Filesystem watcher using `notify` for real-time folder monitoring.
- [x] Comprehensive integration tests in `tests/library_tests.rs`.

---

## Phase 3 — Playback Engine (Completed)
- [x] AudioBackend abstraction trait.
- [x] Native audio implementation using `rodio` and `cpal`.
- [x] Playback controls: play, pause, resume, stop, seek, volume attenuation, mute.
- [x] Playback queue management: play next, enqueue, reorder, clear.
- [x] Shuffle and Repeat modes (Repeat Off, Repeat One, Repeat All).
- [x] High-frequency position tracking and playback event emission (`TrackStarted`, `PlaybackPaused`, etc.).
- [x] Headless playback tests.

---

## Phase 4 — Listening History & Statistics (Completed)
- [x] Playback session recording with configurable meaningful play threshold (>= 30s or >= 50%).
- [x] Listening history database logging.
- [x] Incremental track statistics updates (play count, skip count, completion count).
- [x] Multi-window aggregated statistics (Today, 7D, 30D, 6M, 1Y, All-Time).
- [x] Multi-factor ranking engine (play count, listening duration, completion rate, recency, likes, skips).
- [x] Ranking calculations and history verification tests.

---

## Phase 5 — Smart Local Recommendations & Mixes (Completed)
- [x] User taste profile engine: short-term vs long-term affinity for artists, genres, and eras.
- [x] Local offline recommendation engine with explainability factor breakdown.
- [x] Smart temporary mix generator (Daily Mix, On Repeat, Forgotten Favorites, Genre Mixes, Artist Radios, Late Night, Discovery).
- [x] Repetition avoidance and controlled entropy/randomness.
- [x] Recommendation scoring tests.

---

## Phase 6 — External Metadata Providers (Completed)
- [x] MusicBrainz API integration for artist & album metadata enrichment.
- [x] Cover Art Archive provider for missing artwork fetching and local caching.
- [x] Spotify Web API client for discovery & metadata enrichment.
- [x] Rate limiting, token caching, and provider failover architecture.

---

## Phase 7 — Discovery & Missing Music Matching (Completed)
- [x] Discovery recommendations outside the local library (Spotify, MusicBrainz).
- [x] Fuzzy library matching engine (exact match, likely match, possible match, not found) with confidence scoring.
- [x] External tracks database table management and status transitions.
- [x] Download Wishlist management (Want, Ignore, Already Own, Downloaded).
- [x] Comprehensive integration tests in `tests/discovery_tests.rs`.

---

## Phase 8 — Soulseek Integration (Completed)
- [x] `DownloadProvider` trait definition and progress metrics.
- [x] Local Soulseek client integration via Slskd daemon REST API (`SoulseekProvider`).
- [x] Mock download provider for deterministic offline testing (`MockDownloadProvider`).
- [x] User-initiated search dispatch and queue management (`DownloadService`).
- [x] Download directory monitoring and auto-import into library upon completion.
- [x] Automatic linked wishlist transition to `DOWNLOADED`.
- [x] Comprehensive integration tests in `tests/download_tests.rs`.

---

## Phase 9 — UI Refinement & Desktop Shell (Completed)
- [x] Tauri v2 desktop shell initialization with React 19 + TypeScript + Vite.
- [x] Thin frontend views: Library, Artists, Albums, Playlists, Smart Mixes, Discovery, Wishlist, Downloads, Settings.
- [x] Now Playing bar with real-time scrub slider, volume, repeat, shuffle, and feedback controls.
- [x] Onboarding modal enforcing system audio directory inspection and strict boundary containment.
- [x] Typed IPC command/query dispatcher and reactive backend-event listener bridge.
- [x] Headless web fallback mode for decoupled UI development.

---

## Phase 10 — Packaging & Release Polish (Completed)
- [x] End-to-end integration workflows across live audio, scan, discovery, and transfers.
- [x] Native binary compilation for both GUI app (`music-player-app`) and headless daemon (`music-player-cli`).
- [x] Verified headless initialization, migration execution, and CoreProcessor command execution.
- [x] Production bundle compilation (`npm run build` exits 0 with sub-second bundle generation).
- [x] Comprehensive backend integration test verification (all 32 tests passing).
- [x] Exhaustive system documentation in `README.md`, `docs/`, and ADR suite (`0001` through `0013`).
