# ADR 0001: Use Rust for Backend and Core Logic

## Context
The application is a backend-heavy, local-first intelligent music player requiring high-performance audio playback, fast recursive directory scanning, robust tag extraction, thread-safe asynchronous concurrency, and sophisticated recommendation and ranking algorithms without requiring runtime virtual machines or heavy garbage collection pauses.

## Options Considered
1. **Python**: Rapid prototyping and rich ML ecosystem, but high memory footprint, GIL limitations for concurrent audio/scanning, slower audio processing, and challenging desktop bundling.
2. **C++ (Qt)**: High performance and low overhead, but manual memory management, risk of memory corruption vulnerabilities, and steep build system complexity.
3. **Go**: Simple concurrency, but lacks mature high-level audio ecosystem (e.g. low-level ALSA/Pulse/WASAPI binding headaches) and higher binary sizes.
4. **Rust**: Zero-cost abstractions, memory safety without garbage collection pauses, top-tier audio crates (`rodio`, `cpal`, `lofty`), robust async ecosystem (`tokio`), excellent SQLite integration (`sqlx`), and native Tauri synergy.

## Decision
Use **Rust** as the primary language for all core backend logic, audio playback, library management, recommendation modeling, and data persistence.

## Reasoning
* Rust guarantees fear-free concurrency between background tasks (scanning, metadata indexing) and the real-time audio playback thread.
* Audio libraries `rodio` and `cpal` provide cross-platform desktop audio without external dependencies.
* Strong type system prevents runtime nil/null pointer crashes and enforces strict domain boundaries.

## Consequences
* High compilation safety and exceptional execution performance.
* Clean, type-safe IPC serialization via `serde`.
* Zero runtime garbage collection hiccups interfering with audio streams.
