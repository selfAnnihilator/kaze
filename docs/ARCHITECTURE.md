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

---

## 4. Threading & Concurrency Model

1. **Main UI Thread**: Runs the Tauri webview and desktop window event loop.
2. **Tokio Async Runtime**: Manages asynchronous I/O (database queries via `sqlx`, filesystem notifications via `notify`, external HTTP calls via `reqwest`, background workers).
3. **Dedicated Audio Thread**: `rodio` and `cpal` operate in a high-priority native audio thread to guarantee glitch-free, low-latency audio rendering independent of CPU-bound file scanning or database operations.
4. **Internal Event Bus**: Backed by `tokio::sync::broadcast` channels, decoupling command handling from asynchronous event listeners.
