# Intelligent Local Music Player

A **free, open-source, local-first desktop music player** engineered with a heavy backend emphasis. Built with Rust, Tauri, React, and SQLite.

---

## Key Features & Philosophy

* **100% Local-First & Offline-Capable**: Full playback, library browsing, instant search, listening history, multi-window rankings, smart temporary mixes, and local recommendations operate completely without internet connectivity.
* **Backend-Heavy Modular Monolith**: High-performance Rust backend manages all audio decoding, database queries, metadata extraction, taste profile modelling, and ranking algorithms.
* **Thin Presentation Layer**: Modern, lightweight desktop interface using Tauri and React + TypeScript. Zero business logic in UI components.
* **Central Processor & Event Bus**: Strict Command-Query Responsibility Segregation (CQRS) and event-driven architecture.
* **Explainable Recommendations**: Transparent, multi-factor recommendation engine explaining why each track is recommended.
* **Zero Cost / No Paid Subscriptions**: Built strictly with free and open-source software; no cloud servers, paid APIs, or proprietary databases required.

---

## Architecture Overview

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

## Documentation Links

- [Architecture Guide](docs/ARCHITECTURE.md)
- [Backend Services & Lifecycles](docs/BACKEND.md)
- [Database Schema & Migrations](docs/DATABASE.md)
- [Command, Event & Query Catalog](docs/EVENTS.md)
- [Dependency Audit](docs/DEPENDENCIES.md)
- [Architecture Decision Records (ADRs)](docs/adr/)
- [Development Roadmap](ROADMAP.md)
- [Project Status](PROJECT_STATUS.md)
- [Task Tracking (TODO)](TODO.md)

---

## Development & Building

### Prerequisites
- **Rust toolchain** (1.80+): `rustc`, `cargo`
- **Node.js** (20+): `node`, `npm` or `pnpm`
- **ALSA development headers** (on Linux): `alsa-lib` / `libasound2-dev`

### Running Backend Tests
```bash
cargo test
```
