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
- Phase 10: Packaging, Verification, and Polishing (Completed).

### Recommended Next Step
Application is fully implemented, verified, and production-ready.

---

## 2026-09-11 (Phase 10: Packaging, Verification & Polish)

### Worked On
End-to-end binary compilation, headless daemon execution verification, production asset bundling, test suite audit, and comprehensive project documentation.

### Changes
- Built full desktop Tauri binary (`target/debug/music-player-app`) and headless CLI daemon (`target/debug/music-player-cli`).
- Executed `music-player-cli` headless test: confirmed SQLite database connection, automatic WAL mode configuration, migration execution, `CoreProcessor` command processing, and `EventBus` domain event broadcast.
- Verified frontend production bundle with `npm run build` (`dist/index.html` generated cleanly in ~1.23s).
- Verified full backend test suite with `cargo test`: all 32 unit & integration tests passed with 0 failures across foundation, library scanning, playback, history, recommendations, metadata providers, discovery matching, and downloads.
- Overhauled top-level `README.md` with detailed subsystem breakdown, architecture diagrams, build/run guides, supported audio formats, and documentation links.
- Updated `TODO.md`, `ROADMAP.md`, and `PROJECT_STATUS.md` reflecting 100% completion across all 10 project phases.

### Decisions
- Retain dual binary targets: `music-player-app` for desktop GUI users and `music-player-cli` for headless server or background daemon setups.
- Maintain exhaustive architectural documentation and 13 ADRs as living technical references.

### Problems
- None encountered. All builds, tests, and links verified clean.

### Remaining Work
- All phases completed. System is production-ready.

---

## 2026-09-14 (Phase 12: Cloud-Backed Authentication & Cloudflare D1 Synchronization)

### Worked On
Transition from local-only SQLite authentication to cloud-backed authentication and user data synchronization using Cloudflare Workers and Cloudflare D1 (serverless SQLite).

### System Inspection & Technical Findings
- **Existing Local System**: Evaluated `user_repo.rs`, `users` table, and SQLite password hashing. Current local implementation used single-round SHA-256 with UUID salt (`salt:sha256(salt:password)`).
- **Security Assessment**: Single-round SHA-256 is insufficient for cloud/web authentication. The server-side authentication in Cloudflare Workers requires modern password derivation. We adopt standard Web Crypto **PBKDF2-SHA256 (100,000 iterations)** with a cryptographically secure 16-byte random salt executed entirely inside the Cloudflare Worker V8 isolate.
- **Credential Protection**: Plaintext credentials are sent only over HTTPS/TLS to the Worker API; password hashes are never sent back to the desktop client.
- **Session Tokens**: Authenticated requests exchange a cryptographically secure 32-byte session token with server-side TTL in a D1 `sessions` table.
- **Local-First Resiliency**: Desktop client caches the user profile and session token locally in SQLite. Local playback history, local caching, and offline playback continue functioning without remote server dependency.
- **Synchronization Scope**: Designed Cloudflare D1 schema containing 8 tables: `users`, `sessions`, `songs`, `playlists`, `playlist_songs`, `song_stats`, `user_stats`, and `user_settings`.

### Changes
- Updated `docs/ARCHITECTURE.md`, `PROJECT_STATUS.md`, and `TODO.md` with Cloudflare Workers + D1 cloud architecture.
- Created `worker/` package:
  - `worker/schema.sql`: Full D1 database schema with `users`, `sessions`, `songs`, `playlists`, `playlist_songs`, `song_stats`, `user_stats`, `user_settings`.
  - `worker/wrangler.jsonc`: Cloudflare Worker configuration with D1 database binding `DB`.
  - `worker/package.json`: Worker configuration and dependencies.
  - `worker/src/index.ts`: Full Worker implementation featuring Web Crypto PBKDF2-SHA256 (100,000 iterations), 32-byte session tokens with 30-day expiration, `/api/auth/register`, `/api/auth/login`, `/api/auth/logout`, `/api/auth/me`, `/api/sync` (GET & POST).
  - `worker/README.md`: Local development and Cloudflare deployment instructions.
- Implemented Rust Cloud Sync Client and Database persistence in `src-tauri`:
  - `src-tauri/migrations/20260914000001_cloud_sync.sql`: Added `cloud_sessions` table for local caching of active cloud session tokens and timestamps.
  - `src-tauri/src/cloud/models.rs`: Data transfer models for auth and batch data sync.
  - `src-tauri/src/cloud/client.rs`: `CloudClient` utilizing `reqwest` for communication with Cloudflare Worker.
  - `src-tauri/src/cloud/sync_manager.rs`: `SyncManager` providing bidirectional payload extraction and insertion between local SQLite and D1 payloads, with `ensure_online_track` foreign-key protection.
  - `src-tauri/src/core/command.rs`: Added `SyncCloudData`, `SetCloudServerUrl`, and `CloudSyncCompleted`.
  - `src-tauri/src/core/query.rs`: Added `GetCloudSyncStatus` and `CloudSyncStatus`.
  - `src-tauri/src/core/processor.rs`: Integrated `CloudClient` and `SyncManager` into `SignUp`, `Login`, and `Logout` handlers with graceful offline local fallback and automatic session restoration on app startup.
  - `src-tauri/tests/cloud_sync_tests.rs`: Unit test validating remote payload application, database persistence, and local sync payload preparation.
- Updated Frontend:
  - `src/types.ts`: Added cloud sync commands, query, and `CloudSyncStatus` interface.
  - `src/components/views/SettingsView.tsx`: Added "Cloud & Sync" section with session status indicator, configurable Cloudflare Worker endpoint, and "Sync Now" button.
  - `src/App.tsx`: Wired `cloudSyncStatus`, `handleSyncCloud`, and `handleSetCloudUrl`.
- Verification:
  - `npm run build` in root succeeds with code 0 (TypeScript and Vite build).
  - `npx tsc --noEmit` in `worker/` succeeds with code 0.
  - `cargo check` in `src-tauri` succeeds with code 0.
  - `cargo test` in `src-tauri` succeeds with all 33 unit and integration tests passing.

### Decisions
- Password hashing is executed strictly on the server-side inside the Cloudflare Worker isolate; the desktop client never receives or stores password hashes.
- Database queries never bypass the Worker API to access D1 directly.
- Offline-first resilience: Cached sessions and local SQLite allow the app to function seamlessly even when offline.
- Unrelated parts of the system remain intact and untouched.

---

## 2026-09-14 (Phase 13: Security Review & Authentication Hardening)

### Worked On
Comprehensive security audit and hardening of the Cloudflare Worker authentication and synchronization subsystem, eliminating dual local password authorities, upgrading password derivation to 600,000 PBKDF2 iterations, securing token storage with OS keyrings and cryptographic hashing, implementing sliding-window rate limiting, mitigating timing side-channels, and verifying offline session continuity.

### Security Hardening Decisions & Implementations

1. **PBKDF2-HMAC-SHA256 with 600,000 Iterations**:
   - **Rationale & Decision**: Increased PBKDF2-HMAC-SHA256 from 100,000 to **600,000 iterations** with a 16-byte random salt, fully compliant with current OWASP password hashing recommendations. While Argon2id was considered, the Web Crypto API (`crypto.subtle.deriveBits`) natively implements PBKDF2 in C++ inside Cloudflare Workers V8 isolates with zero WASM/cold-start overhead, guaranteed memory safety, and predictable execution bounds without bundling third-party WASM binaries.
   - **Implementation**: Updated `hashPassword` in `worker/src/index.ts` to `iterations: 600000`.

2. **Single Authoritative Account System (Elimination of Dual Authorities)**:
   - **Problem**: Previously, if the cloud worker was unreachable during signup/login, the client fell back to creating independent local password hashes in SQLite, creating split-brain credentials.
   - **Resolution**: Removed all local password creation/verification fallbacks from `Command::SignUp` and `Command::Login`. The Cloudflare Worker is the sole authority for account creation and credential validation.
   - If the Worker is unreachable during login or registration, the client returns a clear network error rather than creating an unverified local password.
   - Local SQLite stores `password_hash = ''`.

3. **Offline Support Model**:
   - Offline support is strictly defined as allowing a user who has previously authenticated on the device to continue using all local features, listening to music, managing playlists, and viewing stats while disconnected.
   - Session metadata in SQLite (`cloud_sessions`) is validated on startup (`expires_at > now`). If valid, `current_user` is restored without network requests and without prompting for a password.

4. **Cryptographic Token Storage in D1 (Server-Side)**:
   - **Problem**: Storing raw session tokens in D1 creates exposure if the database is accessed.
   - **Resolution**: Updated `worker/schema.sql` and `worker/src/index.ts` so the `sessions` table stores `token_hash TEXT PRIMARY KEY` (SHA-256 hash of the bearer token). Raw bearer tokens are never stored in D1. The Worker computes `hashToken(rawToken)` upon receiving Bearer headers to verify active sessions.

5. **Desktop Client Secure Token Storage**:
   - Integrated the Rust `keyring` crate (v3) to persist raw session bearer tokens in OS credential stores (macOS Keychain, Windows Credential Manager, Linux Secret Service).
   - In environments where the OS keyring is unavailable, locked, or headless, `src-tauri/src/cloud/credentials.rs` falls back to a restricted-permission private file (`0600` on Unix) in the user's application data directory.
   - SQLite `cloud_sessions` stores only non-sensitive session metadata (`user_id`, `username`, `expires_at`, `worker_url`, `synced_at`, `created_at`), never raw tokens.

6. **Rate Limiting on Authentication Endpoints**:
   - Implemented D1-backed sliding-window rate limiting on `/api/auth/login` (5 requests per minute per IP) and `/api/auth/register` (3 requests per minute per IP) in `worker/src/index.ts`.
   - Exceeding limits returns HTTP `429 Too Many Requests` with `Retry-After` headers and localized error messages.

7. **Timing Discrepancy Defense & Generic Error Responses**:
   - In `/api/auth/login`, requests for non-existent users trigger dummy PBKDF2-HMAC-SHA256 verification against a static dummy hash (`DUMMY_HASH`) to prevent timing side-channel attacks that could leak user registration status.
   - All invalid login attempts return the generic message `"Invalid username or password"`.

8. **Strict Input Validation**:
   - Server- and client-side validation enforces username length (3–50 chars, `^[a-zA-Z0-9_\-\.]+$`) and password length (8–128 chars). Malformed or oversize requests are rejected with HTTP 400.

9. **Session Expiry & Logout Verification**:
   - `Command::Logout` invokes Worker `/api/auth/logout` (deleting D1 session by `token_hash`), deletes the OS keyring credential, and purges local SQLite session metadata.
   - Expired sessions (`expires_at <= now`) are purged automatically upon startup and during sync operations.

### Verification
- `cargo test`: All 35 tests pass with 0 failures across 12 test suites.
- `npx tsc --noEmit` in `worker/`: Exits with code 0.
- `npm run build`: TypeScript and Vite bundle production build cleanly in 1.12s.

---

## [Phase 14] - Hardened Hybrid Session Management, Multi-Device Revocation & D1 Write Throttling

### Architecture & Hardening Overview
To ensure session management is robust, revocable, offline-friendly, and bounded for long-running desktop players without write amplification against Cloudflare D1, implemented a complete hybrid session lifecycle:

1. **Hybrid Expiry Logic (Idle Timeout + Absolute Ceiling)**:
   - **Idle Inactivity Window**: 30 days (`now < idle_expires_at`).
   - **Hard Absolute Ceiling**: 90 days (`now < absolute_expires_at`).
   - **Active Revocation**: Requires `revoked_at IS NULL`.
   - The session terminates when either timeout expires or if explicit revocation occurs.

2. **Server-Side D1 Write Throttling**:
   - To avoid write amplification and protect D1 limits during continuous playback and sync operations, `last_used_at` and `idle_expires_at` are refreshed at most once every 30 minutes (`now - last_used_at >= 1800`).
   - The sliding idle expiration is strictly capped: `min(now + 30 days, absolute_expires_at)`.

3. **Opaque Token & Hash Storage**:
   - Authentication produces 256-bit cryptographically secure random bearer tokens (32 bytes via `crypto.getRandomValues`).
   - Raw bearer tokens are returned exactly once to the client upon successful authentication.
   - D1 stores only SHA-256 hashes (`token_hash`) in the `sessions` table.
   - Client persists raw tokens in native OS credential storage (`keyring` / `0600` file) and stores non-secret session metadata in SQLite `cloud_sessions`.

4. **Device Identity**:
   - Each desktop installation creates a persistent UUID v4 on first launch (`application_settings` key `"device_id"`).
   - Inferred friendly OS names ("Linux Desktop", "macOS Laptop", "Windows PC") are attached to remote session records without fingerprinting hardware.

5. **Multi-Device Revocation & Session Management Endpoints**:
   - `POST /api/auth/logout`: Revokes the current session (`revoked_at = now, revoked_reason = 'user_logout'`).
   - `POST /api/auth/logout-all`: Revokes all active sessions for the user.
   - `GET /api/auth/sessions`: Lists active user sessions with `is_current: true` for the active token.
   - `DELETE /api/auth/sessions/:id`: Revokes a specific session with user ownership validation.
   - Scheduled cleanup job running on Cloudflare Workers cron to prune revoked/expired sessions older than 30 days.

6. **Client State Machine & Offline Continuation**:
   - Strongly typed session states: `SignedOut`, `Authenticating`, `OnlineAuthenticated`, `OfflineAuthenticated`, `SessionExpired { reason }`, `CloudUnavailable`, `SyncPaused`.
   - On startup or offline use, devices with `authenticated_before = 1` and unexpired local timestamps enter `OfflineAuthenticated`, permitting uninterrupted local playback and library organization without extending cloud validity offline.

7. **Strict Non-Destructive Data Preservation**:
   - User logout, session expiration, and remote revocation delete authentication credentials and session records only.
   - Local audio files, playlists, play history, and downloads are preserved.

8. **Verification**:
   - `cargo test`: 34 unit and integration tests passing, including session lifecycle, data preservation, and offline continuation tests.
   - `npx tsc --noEmit`: Exits with code 0 in `worker/`.
   - `npm run build`: Production frontend build succeeds with zero errors.

---

## [Phase 15] - Cloud Synchronization, Non-Destructive Access Gating & Sync Tombstones

### Problem Statement & Architectural Regression
After the initial authentication persistence hardening, user playlists and tracks were failing to restore from Cloudflare D1 upon re-login. Investigation revealed:
1. **Destructive Logout Bug**: Logout was wiping user data from local SQLite (`delete_all_user_playlists`, `DELETE FROM playback_history`, `DELETE FROM yearly_stats_archive`), conflicting with local-first desktop player principles where logout must only terminate session access without destroying stored application data.
2. **Missing Deletion Distinctions**: Absence of a playlist or track locally was indistinguishable from an intentional deletion, risking remote data wipeout during reconciliation.
3. **Song Rating Sync Issues**: Worker D1 queries used `MAX(song_stats.manual_like, excluded.manual_like)`, preventing dislikes (`-1`) and neutral unliking (`0`) from syncing. Additionally, rated songs not added to any playlist were not collected into sync payloads.
4. **Serde Deserialization Mismatch**: Cloudflare Worker returned object or null representations for `user_stats` and `user_settings`, while client previously expected arrays, failing payload deserialization.

### Changes & Architecture Improvements
1. **Non-Destructive Logout & Access Gating**:
   - Reverted destructive SQL deletions in `Logout`, `LogoutAll`, and startup token checks.
   - Preserved all local user playlists, playlist tracks, track statistics, and play history across logouts.
   - Access-gated `Query::GetPlaylists`: Unauthenticated users receive algorithmic Smart Mixes only; authenticated users receive their personal playlists and cloud-synced items.
2. **Deterministic Synchronization Flow (Pull -> Reconcile -> Push)**:
   - Established deterministic sequence: On startup/login -> establish `ActiveUser` -> `GET /api/sync` (Pull) -> Reconcile locally -> `POST /api/sync` (Push) -> Clear pushed tombstones -> Emit `Event::PlaylistsUpdated`.
   - Added structured debug logging at each stage of the sync lifecycle.
3. **Sync Tombstones (`sync_tombstones` table)**:
   - Created SQLite migration `20260914000003_sync_tombstones.sql`.
   - Explicit deletions (`DeletePlaylist`, `RemoveTrackFromPlaylist`) record a tombstone.
   - Pull reconciliation skips tombstoned items.
   - Push transmits `deleted_playlists` and `deleted_playlist_songs` to Cloudflare D1, pruning remote rows, and clears local tombstones on success.
4. **Song Feedback & Rating Sync**:
   - Updated Worker D1 query to `manual_like = excluded.manual_like`.
   - `SyncManager::prepare_local_sync_payload` now includes all tracks where `manual_like != 0 OR play_count > 0`, ensuring rated standalone songs sync.
5. **Serde Resilience**:
   - Changed `user_stats` and `user_settings` in `SyncPayload` to `Option<serde_json::Value>` with `#[serde(default)]` across all cloud structs, aliased `total_seconds`.
6. **Frontend Reactive Refresh**:
   - Wired `Event::PlaylistsUpdated` in `App.tsx` to automatically re-fetch playlists and memberships upon sync completion.
7. **Cloudflare Worker Deployment**:
   - Built and deployed updated worker to production (`version 538a04a1-d9e9-40e4-9a77-81a7d54a095d`).

### Verification
- `cargo test`: All 35 tests pass with 0 failures across all 13 test suites.
- `npm run build`: Production frontend build succeeds in 1.17s.

---

## [Phase 16] - User Profile & Cloudflare R2 Avatar Normalization

### Requirements & Objectives
1. Rename Stats section to "Profile" across navigation, views, and routing.
2. Hide persistent global search bar when navigating to Profile/Stats view.
3. Above listening stats, display dedicated User Profile Header Card with avatar on left, username, joined date ("Member since..."), and active cloud badge.
4. Profile photo customization using Cloudflare R2:
   - Optional, free-tier friendly, lightweight, secure, and offline resilient.
   - Client-side normalization: 256×256 WebP centered-square crop using Rust `image` crate with `Lanczos3` filter (target ~20-100 KB, 5MB file size limit).
   - Zero image binaries or base64 strings in D1; D1 stores metadata only (`avatar_key`, `avatar_updated_at`), while R2 stores the image.
   - Local caching at `<cache_dir>/avatars/{user_id}.webp` for instant offline rendering.
   - Fallback to first letter of username on dynamic gradient when no avatar is set.
   - Offline guard requiring internet connectivity for upload and removal actions.

### Implementation Details
1. **Remote Cloudflare D1 & Worker**:
   - Created migration `worker/migrations/0002_user_avatar.sql` adding `display_name`, `avatar_key`, `avatar_updated_at` to `users` table. Applied remotely to `soundflow-db`.
   - Updated `worker/src/index.ts` with `GET /api/profile`, `POST /api/profile/avatar`, `DELETE /api/profile/avatar`, and `GET /api/profile/avatar`.
   - Optional `PROFILE_IMAGES?: R2Bucket` binding with graceful 503 response if R2 bucket is not yet bound on Cloudflare dashboard.
   - Deployed worker to production (`version 6fdea612-094f-4bd3-867b-d3e495dd89fe`).
2. **Local SQLite Migration**:
   - Migration `src-tauri/migrations/20260914000004_user_avatar.sql` adding `display_name`, `avatar_key`, `avatar_updated_at` to `users`.
3. **Rust Profile Service & Image Processing**:
   - Added `image` crate (features: `jpeg`, `png`, `webp`) and `rfd` native file dialog to `Cargo.toml`.
   - Implemented `ProfileService` in `src-tauri/src/profile/mod.rs` with `normalize_avatar_image` (center square crop, Lanczos3 resize to 256×256, WebP encode) and local disk caching helpers.
   - Added unit tests in `src-tauri/src/profile/mod.rs` validating dimensions, WebP format, empty/oversized payload bounds, and disk caching roundtrips.
4. **Cloud Models, Client & Repositories**:
   - Extended `CloudUser`, `UserProfile`, and `UserRecord` with avatar fields.
   - Added `upload_avatar`, `delete_avatar`, `download_avatar`, and `get_profile` in `CloudClient`.
   - Added `update_avatar_metadata` to `SqliteUserRepository`.
5. **Core Processor Integration**:
   - Integrated `ProfileService` into `CoreProcessor`.
   - Implemented `Command::UploadAvatar`, `Command::RemoveAvatar`, `Query::GetProfile`, and `Query::GetAvatar`.
   - Startup session validation immediately loads cached avatar, downloads fresh remote avatar if available, and broadcasts `SessionChanged`.
   - Emits reactive `Event::UserProfileUpdated` on avatar changes.
6. **Frontend UI Integration**:
   - `Sidebar.tsx`: Renamed "Stats" to "Profile" with `User` icon; displays user avatar image in footer with fallback to letter avatar.
   - `App.tsx`: Hidden `GlobalTopSearchBar` on `currentView === "stats"`; listens for `UserProfileUpdated`.
   - `StatsView.tsx`: Added Profile Header Card above listening stats with 72px round avatar, hover camera overlay, "Change Photo", "Remove", username, joined date, and status indicators.

### Verification
- `cargo test`: All 36 tests pass with 0 failures across all 14 test suites.
- `npm run build`: Production frontend build succeeds in 1.45s with zero errors.


