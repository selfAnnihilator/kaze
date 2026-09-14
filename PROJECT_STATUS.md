# Project Status

## Current Development Phase
**Phase 15: Cloud Synchronization, Non-Destructive Access Gating & Sync Tombstones (Complete, Fully Audited & Production-Ready)**

## Architecture Summary
- **Backend**: Rust 2021 modular monolith running on Tokio async runtime.
- **Cloud Infrastructure**: Cloudflare Worker + Cloudflare D1 (serverless SQLite) for authoritative cloud-backed authentication and user data synchronization.
- **Security Posture**: PBKDF2-HMAC-SHA256 (600,000 iterations), server-side token hashing (`token_hash` in D1), client OS credential keyring storage with restricted file fallback, IP rate limiting, timing discrepancy defense, and sole authoritative cloud account authority.
- **Session Management**: 30-day idle inactivity timeout, 90-day absolute hard ceiling, D1 write throttling (30 minutes), opaque 256-bit bearer tokens, multi-device listing and revocation (`logout-all`, `revoke_session`), and offline continuation.
- **Cloud Synchronization & Access Gating**: Non-destructive logout preserving local SQLite data while gating UI queries to Smart Mixes; deterministic Pull -> Reconcile -> Push synchronization sequence; liked (`+1`) and disliked (`-1`) song feedback synchronization; sync tombstones (`sync_tombstones`) tracking explicit deletions.
- **IPC & Desktop Shell**: Tauri v2 with React 19 + TypeScript + Vite.
- **Database**: SQLite 3 with WAL mode, managed through `sqlx` and embedded migrations.
- **Core Pattern**: Central Processor for validated command execution + Tokio broadcast Event Bus for decoupled notifications.
- **Audio Engine**: Thread-safe `AudioBackend` trait abstraction implemented via `RodioAudioBackend` (rodio/cpal) and `MockAudioBackend` (in-memory for headless testing).
- **Metadata**: `lofty` for tag extraction, with provider abstractions for MusicBrainz, Cover Art Archive, and Spotify.
- **Boundary Containment**: Enforces that scans operate strictly inside configured roots and their child directories without traversing outside or across unapproved symlinks.
- **History & Ranking**: Event-driven `HistoryService` tracking playback sessions and meaningful-play thresholds (>= 30s or >= 50%), with `RankingEngine` for multi-factor time-decayed scoring across rolling windows.
- **Taste & Recommendations**: Local offline recommendation engine with dual-window affinity modeling (artists, genres, eras), transparent factor explainability breakdown, repetition dampening, and automated smart mix generation (`Daily`, `OnRepeat`, `ForgottenFavorites`, `Genre`, `Artist`, `LateNight`, `Discovery`).
- **External Providers**: Async `MetadataProvider` trait with leaky-bucket rate limiting (MusicBrainz 1 req/s), local artwork caching (`CoverArtArchiveProvider`), optional Client Credentials flow (`SpotifyProvider`), and coordinator for track metadata enrichment.
- **Discovery & Matching**: Multi-factor fuzzy track matching (`FuzzyTrackMatcher`) with string normalization, Jaro-Winkler similarity, and duration delta tolerance. Download wishlist manager with status transitions (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`). External discovery coordinator linking external candidate tracks to local library ownership status.
- **Soulseek & Downloads**: Pluggable `DownloadProvider` trait, Slskd daemon REST bridge (`SoulseekProvider`), deterministic `MockDownloadProvider`, download queueing, progress broadcasting, and automatic library import pipeline upon transfer completion.
- **Desktop Shell & Frontend**: React 19 + TypeScript + Vite desktop GUI hosted in Tauri v2, with typed IPC bridge (`execute_command`, `execute_query`), streaming `backend-event` integration, persistent player bar, onboarding wizard, and dedicated views for Library, Artists, Albums, Playlists, Discovery, Wishlist, Downloads, and Settings.

## Current Working Features
- Complete documentation suite and Architecture Decision Records (`docs/adr/0001` through `0014`).
- Complete SQLite relational schema (19 entities + FTS5 full-text search) with embedded migrations.
- Full `Command`, `Event`, and `Query` catalogs with typed `serde` serialization (100% command execution coverage).
- `AppError` taxonomy with `thiserror`.
- `AppConfig` supporting OS-standard directories, audio settings, ranking weights, history thresholds, and download configuration.
- `EventBus` backed by `tokio::sync::broadcast` with graceful zero-subscriber dispatching.
- `CoreProcessor` command router and query execution coordinator.
- SQLite connection manager with WAL mode, foreign keys, and 5-second busy timeout.
- Fully operational headless test suite with 35 integration and unit tests passing.
- Complete React 19 desktop GUI with sub-second production bundle generation (`dist/index.html`).
- **All 8 Primary Views**:
  - `LibraryView`: Searchable table with instant playback, enqueueing, and like/dislike rating.
  - `ArtistsView`: Artist collection cards with indexed track statistics.
  - `AlbumsView`: Album grid with cover art and release year metadata.
  - `PlaylistsView`: Custom playlists and smart mix generators (`Daily`, `On Repeat`, `Forgotten Favorites`, `Discovery`, `Late Night`).
  - `DiscoveryView`: External recommendations with ownership badges (`In Library`, `Likely Owned`, `Alternate Version`, `Missing Track`).
  - `WishlistView`: Missing music wishlist with status workflows (`WANT`, `DOWNLOADED`, `ALREADY_OWN`, `IGNORE`).
  - `DownloadsView`: Active/completed transfers progress tracking, download cancellation, and Soulseek network search.
  - `SettingsView`: Music folder management, audio defaults, external metadata toggles, and Slskd connection settings.
- **NowPlayingBar**: Real-time position scrub slider, volume, repeat (`off`, `one`, `all`), shuffle, track details, and instant feedback.
- **OnboardingModal**: System audio root verification and strict directory containment guarantee.
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
- **Soulseek / Slskd Download Integration**: Search query dispatch, user-initiated download actions, progress event emission, cancelation, and task tracking (`DownloadService`).
- **Automated Library Import & Wishlist Completion**: Automatically indexes completed downloads into the local library and transitions linked wishlist records to `DOWNLOADED`.

## Partially Implemented Features
- None (Phases 1 through 8 fully realized and verified).

## Known Broken Features
- None.

## Important Files & Modules
- `docs/ARCHITECTURE.md` - Overall system structure and threading model.
- `docs/BACKEND.md` - Modular monolith structure and service interfaces.
- `docs/DATABASE.md` - Complete SQLite schema and optimization PRAGMAs.
- `docs/EVENTS.md` - Exhaustive Command, Event, and Query catalogs.
- `docs/DISCOVERY.md` - Fuzzy matching specifications and wishlist architecture.
- `docs/DOWNLOADS.md` - Download architecture, Soulseek/Slskd integration, and auto-import.
- `src-tauri/src/downloads/` - `DownloadProvider`, `SoulseekProvider`, `MockDownloadProvider`, `DownloadService`.
- `src-tauri/src/discovery/` - `FuzzyTrackMatcher`, `WishlistManager`, `DiscoveryCoordinator`.
- `src-tauri/src/providers/` - `MusicBrainzProvider`, `CoverArtArchiveProvider`, `SpotifyProvider`, `ProviderCoordinator`.
- `src-tauri/src/core/` - `CoreProcessor`, `EventBus`, `Command`, `Event`, `Query`, `AppError`.
- `src-tauri/src/playback/` - `PlaybackService`, `PlaybackQueue`, `AudioBackend`.
- `src-tauri/src/library/` - `LibraryService`, `LibraryScanner`, `LibraryWatcher`.
- `src-tauri/src/database/` - Connection pooling, models, migrations, and repositories.
- `src-tauri/tests/` - 8 integration test suites (`foundation_tests.rs`, `library_tests.rs`, `playback_tests.rs`, `history_tests.rs`, `recommendation_tests.rs`, `provider_tests.rs`, `discovery_tests.rs`, `download_tests.rs`).

## Current Blockers
- None.

## Next Recommended Task
Begin **Phase 9: Frontend & Desktop Shell**: initialize Tauri v2 desktop shell with React + TypeScript, create thin presentation views (Home/Library, Player Bar, Smart Mixes, Discovery, Wishlist, Downloads, Settings), wire typed Tauri IPC invocations to the backend `CoreProcessor`, and subscribe to real-time `EventBus` broadcasts.
