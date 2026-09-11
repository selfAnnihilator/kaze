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
- Phase 5: Smart local recommendations, taste profiles, and temporary smart mix generators (Completed).
- Phase 6: External metadata providers (MusicBrainz, Cover Art Archive, Spotify).

### Recommended Next Step
Proceed to **Phase 5: Smart Local Recommendations & Mixes**.

---

## 2026-09-11 (Phase 5: Smart Local Recommendations & Mixes)

### Worked On
Taste profile modeling (short-term vs long-term dual window), transparent scoring engine with explainability factor breakdown, repetition dampening, diversity constraints, dynamic smart mix generation (`Daily`, `OnRepeat`, `ForgottenFavorites`, `Genre`, `Artist`, `LateNight`, `Discovery`), and playlist/recommendation persistence.

### Changes
- Implemented `TasteProfileEngine` in `src-tauri/src/recommendations/taste.rs` computing normalized affinities across artists, genres, and eras (60% short-term 14d + 40% long-term 90d).
- Implemented `ScoringEngine` in `src-tauri/src/recommendations/scoring.rs` calculating candidate scores with explicit likes (+25%), dislikes (exclusion), repetition penalties (-50% <24h, -30% <72h, -15% <7d), and discovery picks (+20%).
- Implemented `LocalRecommender` in `src-tauri/src/recommendations/local.rs` evaluating candidate tracks and enforcing artist diversity (maximum 2 tracks per artist per mix).
- Implemented `SmartMixGenerator` in `src-tauri/src/recommendations/mixes.rs` building dynamic smart playlists.
- Implemented `SqlitePlaylistRepository` and `SqliteRecommendationRepository` in `src-tauri/src/database/repositories/` persisting playlists, tracks, sessions, and user affinities.
- Wired commands (`GenerateSmartMix`, `CreatePlaylist`, `DeletePlaylist`, `AddTrackToPlaylist`, `RemoveTrackFromPlaylist`) and queries (`GetTasteProfile`, `GetLocalRecommendations`, `GetSmartMixes`, `GetPlaylists`, `GetPlaylistTracks`) into `CoreProcessor`.
- Created `docs/RECOMMENDATIONS.md` and ADR `0009-offline-recommendations-and-smart-mixes.md`.
- Implemented comprehensive integration test suite in `tests/recommendation_tests.rs` (all 19 workspace tests passing).

### Decisions
- Maintain 100% offline functionality without external neural models or remote recommendations for local library mixes.
- Provide human-readable explainability strings alongside every recommendation.
- Use `track_statistics.manual_like` for user preference state.

### Problems
- Initial candidate scoring without artist/genre matches produced empty reasons arrays. Resolved by adding a library catalog exploration explanation reason to ensure every recommended candidate has a clear reason.

### Remaining Work
- Phase 6: External metadata providers (MusicBrainz, Cover Art Archive, Spotify) (Completed).
- Phase 7: Discovery recommendations, fuzzy matching, and wishlist management.

### Recommended Next Step
Proceed to **Phase 6: External Metadata Providers**.

---

## 2026-09-11 (Phase 6: External Metadata Providers)

### Worked On
Modular async external metadata provider trait, leaky-bucket rate limiting for MusicBrainz, Cover Art Archive integration with local disk image caching, optional Spotify Web API client with Client Credentials authentication and token caching, and Provider Coordinator for track metadata enrichment.

### Changes
- Defined `MetadataProvider` trait in `src-tauri/src/providers/mod.rs` for searching tracks, artists, albums, and fetching cover art.
- Implemented `MusicBrainzProvider` in `src-tauri/src/providers/musicbrainz.rs` with leaky-bucket rate limiting (enforcing 1 req/sec) and dedicated User-Agent.
- Implemented `CoverArtArchiveProvider` in `src-tauri/src/providers/cover_art_archive.rs` with automatic downloading and persistent disk caching in `{cache_dir}/artwork/{mbid}.jpg`.
- Implemented `SpotifyProvider` in `src-tauri/src/providers/spotify.rs` supporting OAuth Client Credentials flow, token caching with expiration, and graceful degradation when unconfigured.
- Implemented `ProviderCoordinator` in `src-tauri/src/providers/coordinator.rs` managing provider lifecycles, broadcasting status events (`ProviderStatusChanged`), and enriching local tracks (`musicbrainz_track_id`, `spotify_id`, `has_cover_art`).
- Enhanced `TrackDetail` model and repository queries with `musicbrainz_track_id` and `spotify_id`.
- Added `Network` and `Io` variants to `AppError`.
- Wired `Command::TriggerMetadataRefresh` in `CoreProcessor`.
- Created `docs/METADATA_PROVIDERS.md` and ADR `0010-external-metadata-providers-and-caching.md`.
- Implemented comprehensive integration test suite in `tests/provider_tests.rs` (all 23 workspace tests passing).

### Decisions
- Strictly throttle MusicBrainz requests to 1 req/sec to comply with community API guidelines.
- Cache cover art directly on disk to minimize network overhead and provide instant image loads.
- Ensure all providers degrade gracefully: offline or unconfigured states report availability cleanly and never block audio playback or local features.

### Problems
- `TrackDetail` previously lacked external ID columns, causing compiler errors in integration tests. Resolved by exposing `musicbrainz_track_id` and `spotify_id` on `TrackDetail` and updating repository queries.

### Remaining Work
- Phase 7: Discovery & Missing Music Matching (Completed).
- Phase 8: Soulseek Integration.
- Phase 9: Frontend & Desktop Shell.

### Recommended Next Step
Proceed to **Phase 7: Discovery & Missing Music Matching**.

---

## 2026-09-11 (Phase 7: Discovery & Missing Music Matching)

### Worked On
Fuzzy library track matching engine (`FuzzyTrackMatcher`), external candidate track ingestion and status tracking, download wishlist subsystem (`WishlistManager`), taste-aligned discovery recommendation coordinator (`DiscoveryCoordinator`), and integration into `CoreProcessor`.

### Changes
- Implemented `FuzzyTrackMatcher` in `src-tauri/src/discovery/matcher.rs`:
  - Multi-stage string normalization (stripping brackets, editions, featured artists, punctuation, and leading articles `the `, `a `, `an `).
  - Jaro-Winkler string similarity computation for title and artist.
  - Duration difference delta tolerance classification (`EXACT_MATCH`, `LIKELY_MATCH`, `POSSIBLE_MATCH`, `NOT_FOUND`).
  - Implemented `find_best_match` over local candidate collections.
- Implemented `SqliteWishlistRepository` in `src-tauri/src/database/repositories/wishlist_repo.rs` managing `wishlist` and `external_tracks` tables.
- Implemented `WishlistManager` in `src-tauri/src/discovery/wishlist.rs`:
  - Item creation, status transitions (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`), status filtering, retrieval, and deletion.
  - Foreign key safety: automatically ensures referenced external tracks exist in `external_tracks`.
- Implemented `DiscoveryCoordinator` in `src-tauri/src/discovery/coordinator.rs`:
  - Fuzzy matching of external candidate tracks against the local library.
  - Integration with user taste profile to discover external recommendations outside the local library.
  - Cross-referencing against wishlist to flag `in_wishlist = true/false`.
  - Prioritizing unowned music (`NotFound` > `PossibleMatch` > `LikelyMatch` > `ExactMatch`).
- Wired Commands and Queries in `CoreProcessor`:
  - `Command::AddToWishlist`
  - `Command::UpdateWishlistStatus`
  - `Query::GetWishlist`
  - `Query::GetDiscoveryRecommendations`
- Added `QueryResponse::Wishlist` and `QueryResponse::DiscoveryRecommendations`.
- Created `docs/DISCOVERY.md` and ADR `0011-fuzzy-library-matching-and-wishlist.md`.
- Implemented comprehensive integration test suite in `tests/discovery_tests.rs` (4 tests passing; all 27 workspace tests passing).

### Decisions
- Strip leading English articles ("the ", "a ", "an ") in normalization to prevent band name prefixes from causing false negatives.
- When adding to the wishlist with an `external_track_id`, auto-upsert a stub record in `external_tracks` if it does not yet exist to respect SQLite foreign key constraints.
- Prioritize unowned tracks in discovery recommendations while clearly flagging matching status and explainability reasons.

### Problems
- Initial SQLite foreign key violation when adding wishlist items with un-indexed external track IDs. Resolved by auto-upserting external track stubs in `WishlistManager::add_to_wishlist`.
- Missing `updated_at` column in `artists` test table insertion. Resolved by matching actual schema definition.

### Remaining Work
- Phase 8: Soulseek Integration (Completed).
- Phase 9: Tauri v2 desktop shell with React + TypeScript frontend.

### Recommended Next Step
Proceed to **Phase 8: Soulseek Integration**.

---

## 2026-09-11 (Phase 8: Soulseek Integration)

### Worked On
`DownloadProvider` trait abstraction, Slskd daemon REST API client (`SoulseekProvider`), `MockDownloadProvider` for offline testing, `download_tasks` SQLite schema and repository (`SqliteDownloadRepository`), `DownloadService` coordinating task queuing, progress tracking, and automated library import upon download completion.

### Changes
- Created SQLite migration `migrations/20260911000002_download_tasks.sql` for download task tracking.
- Created `DownloadTaskRecord` model in `src-tauri/src/database/models.rs`.
- Created `DownloadRepository` trait and `SqliteDownloadRepository` in `src-tauri/src/database/repositories/download_repo.rs`.
- Defined `DownloadProvider` trait in `src-tauri/src/downloads/traits.rs` (`search`, `start_download`, `get_progress`, `cancel`).
- Implemented `SoulseekProvider` in `src-tauri/src/downloads/soulseek.rs` interfacing with local Slskd daemon HTTP endpoints (`/api/v0/search`, `/api/v0/transfers/downloads`).
- Implemented `MockDownloadProvider` in `src-tauri/src/downloads/mock_provider.rs` supporting synthetic and canned search results and controllable transfer progression.
- Implemented `DownloadService` in `src-tauri/src/downloads/service.rs`:
  - Search caching and wishlist query translation (`search_wishlist_item`).
  - Transfer enqueueing and persistence in `download_tasks`.
  - Event emission (`DownloadQueued`, `DownloadProgressChanged`, `DownloadCompleted`, `DownloadFailed`).
  - Automated library indexing on completion via `LibraryService::scan_library(None, true)`.
  - Automatic linked wishlist transition from `WANT` to `DOWNLOADED`.
- Added `DownloadConfig` to `AppConfig` (`download_dir`, `slskd_host`, `slskd_port`, `slskd_api_key`, `auto_import`, `max_concurrent_downloads`).
- Wired Commands and Queries in `CoreProcessor`:
  - `Command::SearchSoulseek`
  - `Command::StartDownload`
  - `Command::CancelDownload`
  - `Command::PollDownloadProgress`
  - `Query::GetDownloads`
- Created `docs/DOWNLOADS.md` and ADR `0012-soulseek-and-download-provider-architecture.md`.
- Implemented comprehensive integration test suite in `tests/download_tests.rs` (5 tests passing; all 32 workspace tests passing).

### Decisions
- Strictly enforce user-initiated downloads: no automatic downloading without explicit user choice.
- Decouple protocol implementation behind `DownloadProvider` to allow Slskd, future backends, or mock providers.
- When downloads complete, trigger incremental library scanning and automatically mark the corresponding wishlist item as `DOWNLOADED`.

### Problems
- Unused import warnings during test compilation cleanly resolved.
- Replaced unreachable catch-all match arm in `processor.rs` after achieving 100% explicit command coverage.

### Remaining Work
- Phase 9: Tauri v2 desktop shell with React + TypeScript frontend (Completed).
- Phase 10: Final verification and release packaging.

### Recommended Next Step
Proceed to **Phase 9: Frontend & Desktop Shell**.

---

## 2026-09-11 (Phase 9: Frontend & Desktop Shell)

### Worked On
Tauri v2 desktop shell integration, React 19 + TypeScript + Vite frontend application, typed IPC command and query dispatching, asynchronous backend-event streaming, onboarding directory verification modal, sticky now playing bar, and comprehensive views across library, artists, albums, playlists, discovery, wishlist, downloads, and settings.

### Changes
- Configured Tauri v2 integration:
  - Updated `src-tauri/Cargo.toml` with `tauri = "2.0"` and `tauri-build = "2.0"`.
  - Created `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, and valid PNG icons in `src-tauri/icons/`.
  - Implemented `src-tauri/src/app.rs` with `execute_command` and `execute_query` Tauri command handlers and Tokio broadcast event forwarder (`app_handle.emit("backend-event", payload)`).
  - Configured project root `package.json` (React 19, TypeScript, Vite 6, `@tauri-apps/api` 2.1.1, `lucide-react`).
- Created frontend application architecture:
  - `src/types.ts`: Exhaustive domain interfaces, Command/Query unions matching Rust serde serialization.
  - `src/services/api.ts`: Dual-mode IPC client automatically detecting Tauri environment with graceful mock fallback for headless/browser execution.
  - `src/index.css`: Dark-themed modern layout with high contrast, responsive typography, and micro-interactions.
  - `src/components/Sidebar.tsx`: Navigation across 8 views with active state indicators.
  - `src/components/NowPlayingBar.tsx`: Sticky audio bar with scrub slider, volume, repeat (`off`/`one`/`all`), shuffle, track information, and like/dislike buttons.
  - `src/components/OnboardingModal.tsx`: Enforces system audio directory inspection, custom folder selection, and strict boundary containment.
  - `src/components/views/LibraryView.tsx`: Complete track table with live FTS5 search, sorting, queueing, and feedback.
  - `src/components/views/ArtistsView.tsx`: Grid of artists with track counts and direct library drill-down.
  - `src/components/views/AlbumsView.tsx`: Grid of albums with release years and cover art.
  - `src/components/views/PlaylistsView.tsx`: Custom playlist creation and smart mix generators (`Daily`, `On Repeat`, `Forgotten Favorites`, `Discovery`, `Late Night`).
  - `src/components/views/DiscoveryView.tsx`: Recommendations with match status badges (`In Library`, `Likely Owned`, `Alternate Version`, `Missing Track`), "Add to Wishlist", and "Find on Soulseek".
  - `src/components/views/WishlistView.tsx`: Missing music wishlist with status workflows (`WANT`, `DOWNLOADED`, `ALREADY_OWN`, `IGNORE`), custom additions, and Soulseek search triggers.
  - `src/components/views/DownloadsView.tsx`: Soulseek P2P transfers tracking with progress bars, cancellation, and network search.
  - `src/components/views/SettingsView.tsx`: Music folder management, audio engine defaults, external metadata toggles, and Slskd connection settings.
  - `src/App.tsx`: Central coordinator managing domain data, playback state, and event subscriptions.
  - `src/main.tsx`: React application entry point.
- Created `docs/FRONTEND.md` and ADR `0013-frontend-desktop-shell-architecture.md`.
- Verified build: `npm run build` exits 0 with sub-second production bundle; `cargo check` in `src-tauri` exits 0; `cargo test` exits 0 with all 32 integration tests passing.

### Decisions
- Maintain thin frontend: zero audio decoding, ranking algorithms, or file system traversal in JavaScript. All logic executes strictly in the Rust backend.
- Expose dual-mode API client to enable instant headless bundling and fast component development in browser without requiring Tauri webview binaries during test runs.
- Wire cross-view actions: user clicking "Find on Soulseek" in Discovery or Wishlist seamlessly navigates to Downloads with prepopulated search query.

### Problems
- Unused variables flagged by TypeScript `noUnusedLocals: true` during initial `npm run build`. Cleanly removed and verified.

### Remaining Work
- Phase 10: Packaging, Verification, and Polishing.

### Recommended Next Step
Proceed to **Phase 10: Packaging, Verification, and Polishing**.



