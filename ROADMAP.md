# Development Roadmap

This roadmap defines the sequential, incremental progression of the music player project.

---

## Phase 1 — Foundation (Active)
- [x] Architectural documentation, database schema, event/command contracts, and ADRs.
- [ ] Project initialization: Rust project structure, Cargo dependencies, and tooling.
- [ ] Structured logging (`tracing` / `tracing-subscriber`).
- [ ] Core error handling (`AppError` with `thiserror`).
- [ ] Configuration subsystem (paths, audio preferences, ranking weights).
- [ ] SQLite database connection pool (`sqlx`) with WAL mode and initial migrations.
- [ ] `EventBus` implementation using Tokio broadcast channels.
- [ ] `CoreProcessor` command dispatcher skeleton.
- [ ] Foundation unit & integration tests.

---

## Phase 2 — Local Music Library
- [ ] Library folder registration and persistence.
- [ ] Recursive filesystem scanner with `lofty` metadata extraction (title, artist, album, genre, year, duration, track/disc numbers, bitrate, cover art detection).
- [ ] Incremental scan hashing and timestamp checks (avoid re-reading unmodified files).
- [ ] Normalized text generation and SQLite FTS5 search indexing.
- [ ] Repository implementations for Tracks, Artists, Albums, and Genres.
- [ ] Filesystem watcher using `notify` for real-time file addition/deletion.
- [ ] Scanner performance and integrity tests.

---

## Phase 3 — Playback Engine
- [ ] AudioBackend abstraction trait.
- [ ] Native audio implementation using `rodio` and `cpal`.
- [ ] Playback controls: play, pause, resume, stop, seek, volume attenuation, mute.
- [ ] Playback queue management: play next, enqueue, reorder, clear.
- [ ] Shuffle and Repeat modes (Repeat Off, Repeat One, Repeat All).
- [ ] High-frequency position tracking and playback event emission (`TrackStarted`, `PlaybackPaused`, etc.).
- [ ] Headless playback tests.

---

## Phase 4 — Listening History & Statistics
- [ ] Playback session recording with configurable meaningful play threshold (>= 30s or >= 50%).
- [ ] Listening history database logging.
- [ ] Incremental track statistics updates (play count, skip count, completion count).
- [ ] Multi-window aggregated statistics (Today, 7D, 30D, 6M, 1Y, All-Time).
- [ ] Multi-factor ranking engine (play count, listening duration, completion rate, recency, likes, skips).
- [ ] Ranking calculations and history verification tests.

---

## Phase 5 — Smart Local Recommendations & Mixes
- [ ] User taste profile engine: short-term vs long-term affinity for artists, genres, and eras.
- [ ] Local offline recommendation engine with explainability factor breakdown.
- [ ] Smart temporary mix generator (Daily Mix, On Repeat, Forgotten Favorites, Genre Mixes, Artist Radios).
- [ ] Repetition avoidance and controlled entropy/randomness.
- [ ] Recommendation scoring tests.

---

## Phase 6 — External Metadata Providers
- [ ] `MetadataProvider` trait architecture.
- [ ] MusicBrainz API client and Cover Art Archive integration.
- [ ] Read-only Spotify Web API client for public metadata and artwork.
- [ ] Provider priority cascade with persistent offline caching.
- [ ] Resilience verification: offline fallback when external APIs are unreachable.

---

## Phase 7 — Discovery & Missing Music Matching
- [ ] External discovery recommendation coordinator.
- [ ] Fuzzy library matching engine (exact match, likely match, possible match, not found) with confidence scoring.
- [ ] Download Wishlist management (Want, Ignore, Already Own, Downloaded).
- [ ] Match confirmation workflows.

---

## Phase 8 — Soulseek Integration
- [ ] `DownloadProvider` trait definition.
- [ ] Local Soulseek client integration via documented local interfaces/IPC.
- [ ] Search query dispatch and user-initiated download actions (no automated piracy).
- [ ] Download directory monitoring and auto-import into library.

---

## Phase 9 — UI Refinement & Desktop Polish
- [ ] Tauri v2 desktop shell initialization with React + TypeScript.
- [ ] Thin frontend views: Home, Library, Artists, Albums, Smart Mixes, Discover, Statistics, Wishlist, Settings.
- [ ] Now Playing bar with real-time waveform/position slider and volume controls.
- [ ] Full end-to-end integration and release packaging.
