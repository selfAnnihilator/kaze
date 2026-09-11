# Project Status

## Current Development Phase
**Phase 7: Discovery & Missing Music Matching (Completed)** -> **Phase 8: Soulseek Integration (Active)**

## Architecture Summary
- **Backend**: Rust 2021 modular monolith running on Tokio async runtime.
- **IPC & Desktop Shell**: Tauri v2 with React + TypeScript.
- **Database**: SQLite 3 with WAL mode, managed through `sqlx` and embedded migrations.
- **Core Pattern**: Central Processor for validated command execution + Tokio broadcast Event Bus for decoupled notifications.
- **Audio Engine**: Thread-safe `AudioBackend` trait abstraction implemented via `RodioAudioBackend` (rodio/cpal) and `MockAudioBackend` (in-memory for headless testing).
- **Metadata**: `lofty` for tag extraction, with provider abstractions for MusicBrainz, Cover Art Archive, and Spotify.
- **Boundary Containment**: Enforces that scans operate strictly inside configured roots and their child directories without traversing outside or across unapproved symlinks.
- **History & Ranking**: Event-driven `HistoryService` tracking playback sessions and meaningful-play thresholds (>= 30s or >= 50%), with `RankingEngine` for multi-factor time-decayed scoring across rolling windows.
- **Taste & Recommendations**: Local offline recommendation engine with dual-window affinity modeling (artists, genres, eras), transparent factor explainability breakdown, repetition dampening, and automated smart mix generation (`Daily`, `OnRepeat`, `ForgottenFavorites`, `Genre`, `Artist`, `LateNight`, `Discovery`).
- **External Providers**: Async `MetadataProvider` trait with leaky-bucket rate limiting (MusicBrainz 1 req/s), local artwork caching (`CoverArtArchiveProvider`), optional Client Credentials flow (`SpotifyProvider`), and coordinator for track metadata enrichment.
- **Discovery & Matching**: Multi-factor fuzzy track matching (`FuzzyTrackMatcher`) with string normalization, Jaro-Winkler similarity, and duration delta tolerance. Download wishlist manager with status transitions (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`). External discovery coordinator linking external candidate tracks to local library ownership status.

## Current Working Features
- Complete documentation suite and Architecture Decision Records (`docs/adr/0001` through `0011`).
- Complete SQLite relational schema (17 entities + FTS5 full-text search) with embedded migrations.
- Full `Command`, `Event`, and `Query` catalogs with typed `serde` serialization.
- `AppError` taxonomy with `thiserror`.
- `AppConfig` supporting OS-standard directories, audio settings, ranking weights, and history thresholds.
- `EventBus` backed by `tokio::sync::broadcast` with graceful zero-subscriber dispatching.
- `CoreProcessor` command router and query execution coordinator.
- SQLite connection manager with WAL mode, foreign keys, and 5-second busy timeout.
- Fully operational headless test suite with 27 integration tests passing.
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
- **Listening History Logging**: Asynchronous session recording decoupled from audio playback.
- **Meaningful Play Thresholds**: Evaluates qualified listens (`listened_seconds >= 30.0 || percentage >= 50.0% || completed`).
- **Multi-Window Rolling Rankings**: Dynamic aggregations (`Today`, `Last7Days`, `Last30Days`, `Last6Months`, `LastYear`, `AllTime`) across Tracks, Artists, Albums, and Genres.
- **Multi-Factor Ranking Formula**: Blends play count, duration, completion rate, time-decay recency, and user preference boosts (likes/dislikes).
- **User Preference Management**: Feedback system with `LikeTrack`, `DislikeTrack`, and `RemoveTrackFeedback` commands.
- **Dual-Window Taste Profiling**: Calculates short-term (14-day) and long-term (90-day) musical affinities for artists, genres, and release eras.
- **Explainability Scoring Algorithm**: Evaluates candidate tracks across 5 weighted factors with transparent human-readable explanations.
- **Repetition Fatigue Avoidance**: Exponential dampening for tracks played within 24 hours (-50%), 3 days (-30%), or 7 days (-15%).
- **Controlled Entropy & Exploration**: Surfaces unplayed local tracks that align with top affinities (+20% discovery bonus).
- **Smart Mix Generators**: Auto-generates and persists dynamic playlists (`Daily`, `OnRepeat`, `ForgottenFavorites`, `Genre`, `Artist`, `LateNight`, `Discovery`).
- **Playlist Management**: Full playlist CRUD and track reordering via `SqlitePlaylistRepository`.
- **MusicBrainz Integration**: Free, rate-limited (1 req/s) canonical metadata retrieval for recordings, releases, and artists.
- **Cover Art Archive Provider**: Automatic album artwork downloading and permanent local disk caching (`cache_dir/artwork/`).
- **Spotify Web API Client**: Client Credentials OAuth authentication with token caching and graceful offline degradation when unconfigured.
- **Provider Coordinator & Track Enrichment**: Background track metadata enrichment with database updates and status notifications.
- **Fuzzy Track Matching Engine**: Classifies match confidence (`EXACT_MATCH`, `LIKELY_MATCH`, `POSSIBLE_MATCH`, `NOT_FOUND`) combining Jaro-Winkler string similarity and duration difference tolerances.
- **Download Wishlist Management**: CRUD operations, state transitions (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`), and status filtering (`WishlistManager`).
- **External Discovery Coordinator**: Evaluates unowned tracks against user taste profile and surfaces discovery recommendations with explainability reasons.

## Partially Implemented Features
- None (Phases 1, 2, 3, 4, 5, 6, and 7 fully realized and verified).

## Known Broken Features
- None.

## Important Files & Modules
- `docs/ARCHITECTURE.md` - Overall system structure and threading model.
- `docs/BACKEND.md` - Modular monolith structure and service interfaces.
- `docs/DATABASE.md` - Complete SQLite schema and optimization PRAGMAs.
- `docs/EVENTS.md` - Exhaustive Command, Event, and Query catalogs.
- `docs/DISCOVERY.md` - Fuzzy matching specifications and wishlist architecture.
- `src-tauri/src/discovery/` - `FuzzyTrackMatcher`, `WishlistManager`, `DiscoveryCoordinator`.
- `src-tauri/src/providers/` - `MusicBrainzProvider`, `CoverArtArchiveProvider`, `SpotifyProvider`, `ProviderCoordinator`.
- `src-tauri/src/core/` - `CoreProcessor`, `EventBus`, `Command`, `Event`, `Query`, `AppError`.
- `src-tauri/src/playback/` - `PlaybackService`, `PlaybackQueue`, `AudioBackend`.
- `src-tauri/src/library/` - `LibraryService`, `LibraryScanner`, `LibraryWatcher`.
- `src-tauri/src/database/` - Connection pooling, models, migrations, and repositories.
- `src-tauri/tests/` - 7 integration test suites (`foundation_tests.rs`, `library_tests.rs`, `playback_tests.rs`, `history_tests.rs`, `recommendation_tests.rs`, `provider_tests.rs`, `discovery_tests.rs`).

## Current Blockers
- None.

## Next Recommended Task
Begin **Phase 8: Soulseek Integration**: define `DownloadProvider` trait, implement local client/daemon integration via IPC or local RPC, dispatch search queries from wishlist items, and monitor incoming download folders for auto-import into local library.
