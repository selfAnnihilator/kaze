# Dependency Audit & License Compliance

All dependencies in this project are strictly free and open-source software (FOSS). Before adding any crate or library, its license, maintenance status, and necessity are verified.

---

## Rust Core Dependencies

| Crate | Version | License | Purpose | Repository | Replacement Options / Rationale |
|---|---|---|---|---|---|
| `tokio` | 1.x | MIT | Async runtime for non-blocking I/O, event dispatching, and background workers | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | Industry standard; async-std is alternative |
| `sqlx` | 0.8.x | MIT / Apache-2.0 | Async SQLite driver, connection pooling, and embedded migrations | [launchbadge/sqlx](https://github.com/launchbadge/sqlx) | rusqlite (synchronous only); diesel |
| `thiserror` | 2.x | MIT / Apache-2.0 | Ergonomic custom error derive macros for structured domain errors | [dtolnay/thiserror](https://github.com/dtolnay/thiserror) | Manual std::error::Error impls |
| `anyhow` | 1.x | MIT / Apache-2.0 | Flexible error handling for tests and top-level bootstrap | [dtolnay/anyhow](https://github.com/dtolnay/anyhow) | Standard Box<dyn Error> |
| `serde` | 1.x | MIT / Apache-2.0 | Generic serialization/deserialization framework for IPC, events, config | [serde-rs/serde](https://github.com/serde-rs/serde) | Essential for Rust ecosystem |
| `serde_json` | 1.x | MIT / Apache-2.0 | JSON support for IPC payloads and SQLite JSON columns | [serde-rs/json](https://github.com/serde-rs/json) | Standard JSON library |
| `tracing` | 0.1.x | MIT | Structured, contextual diagnostic logging | [tokio-rs/tracing](https://github.com/tokio-rs/tracing) | log crate (lacks structured spans) |
| `tracing-subscriber` | 0.3.x | MIT | Formatting and routing tracing spans to stdout / files | [tokio-rs/tracing](https://github.com/tokio-rs/tracing) | env_logger |
| `directories` | 6.x | MIT / Apache-2.0 | Standard OS directory discovery (Config, Data, Cache paths on Linux/Win/macOS) | [dirs-dev/directories-rs](https://github.com/dirs-dev/directories-rs) | dirs crate |
| `chrono` | 0.4.x | MIT / Apache-2.0 | Datetime representations, timestamps, and rolling window ranking queries | [chronotope/chrono](https://github.com/chronotope/chrono) | time crate |
| `uuid` | 1.x | MIT / Apache-2.0 | Unique identifier generation for tracks, playlists, sessions, and events | [uuid-rs/uuid](https://github.com/uuid-rs/uuid) | ulid |
| `walkdir` | 2.x | MIT / Unlicense | Fast recursive directory traversal for library scanning | [BurntSushi/walkdir](https://github.com/BurntSushi/walkdir) | std::fs (requires manual recursion) |
| `lofty` | 0.22.x | MIT / Apache-2.0 | Audio tag reader/writer for MP3, FLAC, OGG, OPUS, M4A, WAV | [Serial-ATA/lofty-rs](https://github.com/Serial-ATA/lofty-rs) | id3, metaflac (fragmented per format) |
| `rodio` | 0.20.x | MIT / Apache-2.0 | High-level audio playback and stream mixing | [RustAudio/rodio](https://github.com/RustAudio/rodio) | kira, soloud |
| `cpal` | 0.15.x | Apache-2.0 | Cross-platform low-level audio device output abstraction | [RustAudio/cpal](https://github.com/RustAudio/cpal) | Direct ALSA/WASAPI bindings |
| `notify` | 8.x | CC0-1.0 | Filesystem event monitoring for real-time library folder updates | [notify-rs/notify](https://github.com/notify-rs/notify) | Polling loop |
| `reqwest` | 0.12.x | MIT / Apache-2.0 | Async HTTP client for MusicBrainz, Cover Art Archive, Spotify API | [seanmonstar/reqwest](https://github.com/seanmonstar/reqwest) | ureq, surf |
| `strsim` | 0.11.x | MIT | String similarity algorithms (Jaro-Winkler, Levenshtein) for fuzzy track matching | [dguo/strsim-rs](https://github.com/dguo/strsim-rs) | fuzzy-matcher |
| `unicode-normalization` | 0.1.x | MIT / Apache-2.0 | Unicode canonical decomposition for accurate metadata comparison | [unicode-rs/unicode-normalization](https://github.com/unicode-rs/unicode-normalization) | None |

---

## Frontend Dependencies (Planned Phase 9)

| Package | License | Purpose |
|---|---|---|
| `react` & `react-dom` | MIT | User interface rendering |
| `@tauri-apps/api` | MIT / Apache-2.0 | Tauri IPC bridge (`invoke`, `listen`, `emit`) |
| `zustand` | MIT | Lightweight presentation state management |
| `lucide-react` | MIT | Lightweight icons for media player controls |
| `tailwindcss` | MIT | Utility-first styling |
