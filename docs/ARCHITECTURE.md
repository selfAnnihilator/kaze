# Architecture Overview: Intelligent Local Music Player

## 1. System Philosophy & Objectives

The application is a **free, open-source, local-first desktop music player** engineered with a heavy backend emphasis. The core mission is to provide an offline-capable, highly responsive, privacy-respecting audio player that delivers smart music recommendations and deep listening statistics without relying on paid infrastructure, remote servers, or proprietary cloud dependencies.

### Core Architectural Principles
* **Local-First & Offline-Centric**: Core functionality—audio playback, library indexing, metadata extraction, search, history logging, statistics, and local recommendations—operates 100% offline without remote network access.
* **Backend-Heavy Modular Monolith**: All application state, domain logic, data processing, recommendation algorithms, ranking models, and audio decoding reside within the Rust backend.
* **Thin Presentation Layer**: The frontend (React + TypeScript running inside Tauri) acts purely as a presentation and user input shell. It issues strongly typed commands/queries and subscribes to backend event streams. It executes zero audio decoding, ranking, or database queries.
* **Command & Event Separation**: User actions are submitted as discrete **Commands** to a **Central Processor**. State transitions and system occurrences are published across an **Event Bus** as strongly typed **Events**.
* **Provider Abstraction**: External metadata and discovery integrations (Spotify, MusicBrainz, Cover Art Archive, Soulseek) are isolated behind strict provider traits, ensuring zero runtime coupling to third-party availability.

---

## 2. High-Level Architecture Diagram

```text
 ┌──────────────────────────────────────────────────────────────┐
 │                     Frontend (Tauri Webview)                 │
 │            React 19 + TypeScript + Zustand Stores             │
 └──────────────┬───────────────────────────────▲───────────────┘
                │ Typed IPC Commands / Queries  │ Event Stream
                ▼                               │ (Tauri Events)
 ┌──────────────────────────────────────────────┴───────────────┐
 │                      CoreProcessor                           │
 │     - Command Router, Validator & Task Coordinator           │
 └──────┬──────────────────────┬──────────────────────┬─────────┘
        │                      │                      │
        ▼                      ▼                      ▼
 ┌──────────────┐       ┌──────────────┐       ┌──────────────┐
 │   Playback   │       │   Library    │       │   Playlist   │
 │   Service    │       │   Service    │       │   Service    │
 │ (rodio/cpal) │       │   (lofty)    │       │ (Static/Mix) │
 └──────┬───────┘       └──────┬───────┘       └──────┬───────┘
        │                      │                      │
        └──────────────────────┼──────────────────────┘
                               ▼
 ┌──────────────────────────────────────────────────────────────┐
 │                         EventBus                             │
 │   (tokio::sync::broadcast channel for internal event pub/sub)│
 └──────┬──────────────────────┬──────────────────────┬─────────┘
        │                      │                      │
        ▼                      ▼                      ▼
 ┌──────────────┐       ┌──────────────┐       ┌──────────────┐
 │  Statistics  │       │    Taste     │       │Recommendation│
 │  & History   │──────>│   Profile    │──────>│    Engine    │
 │   Service    │       │    Engine    │       │ (Local/Disc) │
 └──────┬───────┘       └──────────────┘       └──────┬───────┘
        │                                             │
        ▼                                             ▼
 ┌──────────────┐                              ┌──────────────┐
 │  SQLite /    │                              │   External   │
 │    sqlx      │                              │  Providers   │
 │ Repositories │                              │ (MB / Spot)  │
 └──────────────┘                              └──────────────┘
```

---

## 3. Core Component Responsibilities

### 3.1 Central Processor (`CoreProcessor`)
The `CoreProcessor` is the central orchestrator of the entire system.
* Receives command requests originating from Tauri IPC or internal triggers.
* Validates inputs, schema requirements, and preconditions.
* Dispatches commands to the appropriate service domain.
* Tracks global orchestration state (e.g., scan in progress, playback state snapshot).
* Emits state changes onto the system event bus.

### 3.2 Playback Service
* Directly manages audio output devices using `rodio` and `cpal`.
* Maintains play/pause/seek states, volume attenuation, playback queue, repeat, and shuffle logic.
* Computes precise continuous playback timestamps.
* Emits fine-grained events: `TrackStarted`, `TrackPaused`, `TrackFinished`, `PlaybackPositionChanged`, etc.
* Audio backend is encapsulated behind an `AudioBackend` trait to isolate device interaction.

### 3.3 Library Service & Scanner
* Registers, manages, and incrementally scans configured filesystem music directories.
* Uses `lofty` to extract comprehensive ID3v2, Vorbis, FLAC, and MP4 tags (artist, album, track number, disc, genre, year, duration, bitrate, sample rate, cover art, MusicBrainz tags).
* Implements file change detection via timestamp and size comparisons, backed by `notify` for filesystem events.
* Maintains SQLite library tables and FTS5 search index.

### 3.4 History & Statistics Service
* Logs granular listening history sessions (seconds listened, completion percentage, skip indicators).
* Evaluates rule-based "meaningful plays" (e.g., >= 30 seconds or >= 50% duration).
* Aggregates rolling statistics over standard time windows (Today, 7D, 30D, 6M, 1Y, All-Time).
* Computes weighted multi-factor rankings for tracks, artists, albums, and genres.

### 3.5 Taste Profile & Recommendation Engine
* Derives short-term and long-term user affinity scores across artists, genres, and eras.
* Produces offline Local Recommendations from the library using collaborative/content-based scoring.
* Generates temporary smart mixes (Daily Mix, On Repeat, Forgotten Favorites, Genre Mixes).
* Interfaces with discovery providers (MusicBrainz, Spotify) for recommendations outside the library.
* Emits transparent, human-readable explanations for every generated recommendation.

### 3.6 External Providers (Metadata & Downloads)
* Implements the `MetadataProvider` trait for MusicBrainz, Cover Art Archive, and Spotify.
* Enforces caching and graceful fallback: external API downtime never degrades local playback.
* Implements `DownloadProvider` for optional local Soulseek integration via documented local client interfaces/IPC.

### 3.7 Cloud-Backed Authentication & Cloudflare D1 Synchronization
* **Authoritative Cloud Account System**: Cloudflare Worker (`worker/`) bound to Cloudflare D1 acts as the sole authoritative account system. Dual local password authorities are eliminated; local SQLite never stores or checks password hashes.
* **Server-Side Security & PBKDF2 600,000 Iterations**:
  - Hashing: Web Crypto API PBKDF2-HMAC-SHA256 with 600,000 iterations and a cryptographically secure 16-byte random salt per user (exceeding OWASP password hashing recommendations). Web Crypto executes natively in C++ inside Cloudflare Workers V8 isolates with zero WASM/cold-start overhead and predictable resource bounds.
  - Timing Discrepancy Defense: Login requests for non-existent users perform a dummy PBKDF2 verification (`DUMMY_HASH`) to prevent timing side-channel attacks and user enumeration. Generic `"Invalid username or password"` responses are strictly enforced.
  - Rate Limiting: D1-backed sliding-window rate limiting on `/api/auth/login` (5 req/min per IP) and `/api/auth/register` (3 req/min per IP) returning HTTP 429 Too Many Requests with `Retry-After`.
  - Input Validation: Server-side validation enforcing 3–50 character alphanumeric/punctuation usernames and 8–128 character passwords.
* **Hardened Hybrid Session Architecture**:
  - **Opaque Cryptographic Tokens**: 256-bit cryptographically secure random bearer tokens generated via `crypto.getRandomValues`. The raw token is returned to the client once upon registration/login and is never stored in D1 or SQLite.
  - **D1 Storage**: Stores only SHA-256 hash (`token_hash`) along with session metadata: `id`, `user_id`, `device_id`, `device_name`, `client_version`, `created_at`, `last_used_at`, `idle_expires_at`, `absolute_expires_at`, `revoked_at`, `revoked_reason`.
  - **Hybrid Expiry & Validation**:
    - **Idle Window**: 30-day inactivity timeout (`now < idle_expires_at`).
    - **Hard Ceiling**: 90-day absolute expiration from session creation (`now < absolute_expires_at`).
    - **Revocation**: Strictly requires `revoked_at IS NULL`.
    - **D1 Write Throttling**: Updates `last_used_at` and `idle_expires_at` in D1 at most once every 30 minutes (`now - last_used_at >= 1800`), sliding the idle timeout forward by `min(now + 30 days, absolute_expires_at)` without write amplification.
  - **Client-Side Storage**:
    - Secure OS Keyring: Raw bearer token persisted in native OS credential storage (`keyring` crate) with restricted `0600` file fallback.
    - Local SQLite (`cloud_sessions`): Stores non-secret metadata only (`session_id`, `device_id`, `device_name`, `idle_expires_at`, `absolute_expires_at`, `last_cloud_validation_at`, `worker_url`).
  - **Device Identity**: Persistent random UUID v4 generated on first launch (`application_settings` key `"device_id"`) and non-invasive friendly device name derived from the operating system.
  - **Multi-Device Session Control**:
    - `POST /api/auth/logout`: Revokes the current session.
    - `POST /api/auth/logout-all`: Revokes all active sessions for the user.
    - `GET /api/auth/sessions`: Lists active sessions with `is_current: true` flag.
    - `DELETE /api/auth/sessions/:id`: Revokes specific session with user ownership validation.
    - Scheduled background cleanup bounded to prune revoked/expired sessions older than 30 days.
* **Client Session Lifecycle & Offline Continuation**:
  - Strongly typed state machine: `SignedOut`, `Authenticating`, `OnlineAuthenticated`, `OfflineAuthenticated`, `SessionExpired { reason }`, `CloudUnavailable`, `SyncPaused`.
  - An authenticated client device with `authenticated_before = 1` retains offline continuation as `OfflineAuthenticated` when within local `idle_expires_at` and `absolute_expires_at` windows without extending cloud validity offline.
* **Access Gating & Non-Destructive Logout**:
  - Authentication strictly governs **access**, not data existence.
  - Logging out (`Logout`, `LogoutAll`) purges bearer tokens from keyring/fallback storage and resets memory session states (`current_user = None`).
  - User-owned SQLite data (custom playlists, playlist tracks, track likes/dislikes, play history, listening statistics) is **never deleted** on logout.
  - UI queries are strictly access-gated: signed-out users receive only algorithmic Smart Mixes; authenticated users receive their personal playlists and cloud-synchronized content.
* **Deterministic Cloud Synchronization Flow (Pull -> Reconcile -> Push)**:
  - **Sequence**: On login or startup, the client establishes `ActiveUser`, **Pulls** remote changes first, **Reconciles** them into local SQLite, **Pushes** local modifications and pending tombstones, and emits `Event::PlaylistsUpdated` to trigger reactive frontend refreshes.
  - **Liked and Disliked Songs**: Synchronizes `manual_like` ratings (`1` = liked, `-1` = disliked, `0` = neutral) across devices. Rated songs (even if not in any playlist) are collected in sync payloads. Worker upserts use `manual_like = excluded.manual_like`, enabling transitions between liked, disliked, and neutral states.
  - **Sync Tombstones**: Explicit deletions (`DeletePlaylist`, `RemoveTrackFromPlaylist`) record entries in the `sync_tombstones` SQLite table. Local absence without a tombstone never deletes remote data during pull reconciliation. Remote deletions are executed when tombstones are pushed, and tombstones are safely cleared upon confirmed sync acknowledgment.
  - **Selective Synchronization Scope**: Synchronizes essential user entities only: `songs`, `playlists`, `playlist_songs`, `song_stats`, `user_stats`, and `user_settings`.

### 3.8 User Profile & Cloudinary Avatar Storage
* **Decoupled Metadata & Cloud Binary Storage**:
  - D1 user metadata: Stores only non-binary profile metadata (`display_name`, `avatar_public_id`, `avatar_url`, `avatar_version`, `avatar_key`, `avatar_updated_at`). Zero image binaries or base64 strings are stored in D1 to prevent database bloating and respect edge D1 quotas.
  - Storage Abstraction (`AvatarStorage`): The Cloudflare Worker defines a pluggable `AvatarStorage` interface with `CloudinaryAvatarStorage` (default, credit-card free) and `R2AvatarStorage` (optional fallback).
  - Cloudinary Storage: Stores normalized WebP avatars under public ID prefix `music-player/avatars/{user_id}` with `overwrite = true` and `invalidate = true`. Uploads and deletions are cryptographically signed using SHA-1 on the Worker.
  - Credentials Security Boundary: `CLOUDINARY_CLOUD_NAME`, `CLOUDINARY_API_KEY`, and `CLOUDINARY_API_SECRET` are stored strictly as Cloudflare Worker secrets. The desktop client and frontend never see or store the API secret.
* **Client-Side Image Normalization**:
  - Input images (JPEG, PNG, WebP) are processed client-side before upload using the Rust `image` crate.
  - Transformation: Decodes image bytes, performs a centered square crop (`crop_imm`), resizes to 256×256 pixels using Lanczos3 filtering (`FilterType::Lanczos3`), and encodes to WebP format.
  - Target output: 256×256 WebP (~20–100 KB). Payload limit enforced at 5 MB.
* **Local Disk Cache & Offline Resilience**:
  - Normalized avatars are saved locally to `<cache_dir>/avatars/{user_id}.webp`.
  - On application startup or offline mode, the cached avatar is immediately served via base64 data URL with zero network latency.
  - Startup checks compare `avatar_updated_at` / `avatar_version` against local cache to prevent redundant re-downloads when images are unchanged.
  - Modifying or deleting avatars requires an active internet connection and valid authenticated session; unauthorized or offline changes are rejected gracefully.
  - Fallback avatar: First letter of username rendered over a dynamic gradient circle when no avatar is configured.
* **Profile Navigation & Search Bar Visibility**:
  - Stats section renamed to "Profile" across navigation sidebar and view hierarchy.
  - Profile photo edit is accessible directly via a pencil icon positioned on the profile circle.
  - Global top search bar is hidden when viewing the Profile section to prioritize user identity and account settings.

---

## 4. Threading & Concurrency Model

1. **Main UI Thread**: Runs the Tauri webview and desktop window event loop.
2. **Tokio Async Runtime**: Manages asynchronous I/O (database queries via `sqlx`, filesystem notifications via `notify`, external HTTP calls via `reqwest`, background workers).
3. **Dedicated Audio Thread**: `rodio` and `cpal` operate in a high-priority native audio thread to guarantee glitch-free, low-latency audio rendering independent of CPU-bound file scanning or database operations.
4. **Internal Event Bus**: Backed by `tokio::sync::broadcast` channels, decoupling command handling from asynchronous event listeners.
