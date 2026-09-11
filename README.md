# SoundFlow — Intelligent Local-First Music Player

A **free, open-source, local-first desktop music player** engineered with a heavy backend emphasis. Built with **Rust**, **Tauri v2**, **React 19**, **TypeScript**, and **SQLite**.

The player is designed from the ground up to provide a rich, private, and intelligent listening experience centered around **locally owned music**, with optional decentralized P2P acquisition via Soulseek and external metadata enrichment via MusicBrainz and Spotify.

---

## Core Philosophy & Key Features

* **100% Local-First & Offline-Capable**: Audio playback, library scanning, FTS5 search, listening history, multi-window rankings, taste profiling, smart temporary mixes, and local recommendations run entirely offline without internet connectivity or remote servers.
* **Backend-Heavy Modular Monolith**: High-performance Rust backend manages audio decoding, database queries, metadata extraction, taste profile affinity modeling, recommendation scoring, and P2P transfers.
* **Thin Declarative GUI**: Modern, high-contrast dark-mode desktop interface built with Tauri v2, React 19, and Vite. Zero business logic or file system traversal in JavaScript.
* **Zero Cost / No Subscriptions**: Operates with zero paid APIs, subscription fees, hosted servers, or proprietary cloud dependencies.
* **Strict Boundary Containment**: Scans stay strictly within user-approved root directories and their child directories. External symlinks and directories outside the approved root are discarded.
* **Decentralized Music Acquisition**: Pluggable `DownloadProvider` integrating with local Soulseek / Slskd daemons to search, queue, download, and automatically import missing music directly into your collection.
* **Explainable Recommendations**: Transparent multi-factor recommendation engine explaining why each track is suggested with dual-window affinity, repetition fatigue dampening, and discovery bonuses.

---

## Architecture Overview

```text
+-------------------------------------------------------------------------+
|                               TAURI SHELL                               |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  |                       REACT 19 FRONTEND                           |  |
|  |                                                                   |  |
|  |  +----------------+  +-----------------------------------------+  |  |
|  |  |   Sidebar.tsx  |  | Active View:                            |  |  |
|  |  |                |  | - LibraryView (Tracks & FTS5 Search)    |  |  |
|  |  | - Library      |  | - ArtistsView / AlbumsView              |  |  |
|  |  | - Artists      |  | - PlaylistsView (Smart Mixes)           |  |  |
|  |  | - Albums       |  | - DiscoveryView (External Recs)         |  |  |
|  |  | - Playlists    |  | - WishlistView (Track Acquisition)      |  |  |
|  |  | - Smart Mixes  |  | - DownloadsView (Soulseek Transfers)    |  |  |
|  |  | - Discovery    |  | - SettingsView (Folders, Audio, Config) |  |  |
|  |  | - Wishlist     |  +-----------------------------------------+  |  |
|  |  | - Downloads    |  +-----------------------------------------+  |  |
|  |  | - Settings     |  | NowPlayingBar.tsx (Scrub, Vol, Likes)   |  |  |
|  |  +----------------+  +-----------------------------------------+  |  |
|  +-------------------------------------------------------------------+  |
|                                |         ^                              |
|           execute_command(...) |         | Tauri "backend-event"        |
|             execute_query(...) |         |                              |
|                                v         |                              |
|  +-------------------------------------------------------------------+  |
|  |                   RUST BACKEND (CoreProcessor)                    |  |
|  |                                                                   |  |
|  |  - PlaybackEngine (Rodio/CPAL)   - EventBus (Tokio broadcast)     |  |  |
|  |  - LibraryScanner (Lofty/FTS5)   - Discovery & Matching           |  |  |
|  |  - TasteEngine & SmartMixes      - Soulseek / Slskd Service       |  |  |
|  +-------------------------------------------------------------------+  |
+-------------------------------------------------------------------------+
```

---

## Application Subsystems

| Subsystem | Technology | Description |
|---|---|---|
| **Audio Engine** | `rodio`, `cpal` | Native low-latency audio playback, position ticking (250ms), repeat modes, and gapless queueing. |
| **Library Scanner** | `lofty`, `notify` | Recursive directory scanner supporting MP3, FLAC, OGG, OPUS, M4A, WAV, with incremental caching and deleted file pruning. |
| **Search Engine** | SQLite `FTS5` | Sub-millisecond full-text search across titles, artists, albums, and genres with automatic SQL sync triggers. |
| **History & Stats** | `sqlx`, SQLite | Session logging, meaningful-play evaluation (>= 30s or >= 50%), and multi-window rolling rankings (`Today`, `7D`, `30D`, `6M`, `1Y`, `AllTime`). |
| **Taste & Mixes** | Custom Scoring | Dual-window affinity profiling, repetition fatigue penalty, and smart mix generation (`Daily`, `On Repeat`, `Forgotten Favorites`, `Discovery`, `Late Night`). |
| **Metadata Providers** | `reqwest`, `tokio` | Rate-limited MusicBrainz API (1 req/s), Cover Art Archive disk caching, and optional Spotify Web API client. |
| **Missing Music & Wishlist**| `FuzzyTrackMatcher` | Multi-factor fuzzy matching (Jaro-Winkler + duration tolerance) and track wishlist management (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`). |
| **P2P Transfers** | Slskd REST API | Decentralized Soulseek file searching, transfer tracking, cancelation, and automated library import on completion. |
| **Desktop Shell** | Tauri v2, React 19 | Lightweight desktop webview with typed IPC command/query dispatching and asynchronous event streaming. |

---

## Supported Audio Formats

* **FLAC** (`.flac`) — Free Lossless Audio Codec
* **MP3** (`.mp3`) — MPEG-1 Audio Layer III
* **Ogg Vorbis** (`.ogg`)
* **Opus** (`.opus`)
* **M4A / AAC** (`.m4a`, `.aac`) — Advanced Audio Coding
* **WAV** (`.wav`) — Waveform Audio File Format

---

## Development & Building

### Prerequisites
* **Rust toolchain** (1.80+): `rustc`, `cargo`
* **Node.js** (20+): `node`, `npm`
* **Native Development Libraries** (on Linux):
  ```bash
  # Debian / Ubuntu
  sudo apt-get install libasound2-dev libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
  ```

### Running Backend Tests
Execute the comprehensive test suite (32 unit & integration tests covering foundation, library scanning, playback, history, recommendations, metadata providers, discovery matching, and downloads):
```bash
cargo test
```

### Running the Headless Core Daemon
```bash
cargo run --bin music-player-cli
```

### Building the Desktop Frontend Bundle
```bash
npm run build
```

### Running the Full Tauri Desktop App in Development Mode
```bash
cargo run --bin music-player-app
```

---

## Documentation Links

* [Architecture Guide](docs/ARCHITECTURE.md)
* [Frontend Desktop Shell](docs/FRONTEND.md)
* [Backend Services & Lifecycles](docs/BACKEND.md)
* [Database Schema & Migrations](docs/DATABASE.md)
* [Command, Event & Query Catalog](docs/EVENTS.md)
* [Local Library & Boundary Containment](docs/LIBRARY.md)
* [Playback Engine](docs/PLAYBACK.md)
* [Ranking & Statistics](docs/RANKING.md)
* [Recommendations & Smart Mixes](docs/RECOMMENDATIONS.md)
* [External Metadata Providers](docs/METADATA_PROVIDERS.md)
* [Discovery & Missing Music Matching](docs/DISCOVERY.md)
* [Soulseek & Downloads](docs/DOWNLOADS.md)
* [Dependency License Audit](docs/DEPENDENCIES.md)
* [Architecture Decision Records (ADRs)](docs/adr/)
* [Development Roadmap](ROADMAP.md)
* [Project Status](PROJECT_STATUS.md)
* [Development Log](docs/DEVELOPMENT_LOG.md)
* [Task Tracking (TODO)](TODO.md)

---

## License

This project is licensed under the MIT License. All dependencies adhere to permissive open-source licenses (MIT, Apache 2.0, BSD).
