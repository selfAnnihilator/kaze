# ADR 0002: Use Tauri for Desktop Framework and IPC

## Context
A modern, responsive desktop interface is required for the music player. The UI must be thin, responsive, and decoupled from backend processing while allowing rich media display (album art, waveforms, smooth queues, responsive layouts).

## Options Considered
1. **Electron**: Mature web ecosystem, but heavy memory consumption (Chromium + Node.js runtime per window, often 200MB+ idle RAM), large distribution packages, and awkward integration with native background audio threads.
2. **Native GUI (Iced / Slint / egui)**: Fully written in Rust, but custom widget styling, font rendering, responsive layout animations, and media display ecosystems are still evolving and less flexible for desktop media applications.
3. **Tauri (v2)**: Uses the operating system's native webview (WebKitGTK on Linux, WebView2 on Windows), resulting in minimal memory overhead (typically <30-50MB), tiny binary sizes, and native Rust-to-frontend IPC via commands and events.

## Decision
Adopt **Tauri** with a thin **React + TypeScript** frontend for user interaction and visualization.

## Reasoning
* Memory footprint remains tiny and battery-friendly.
* Strict separation between Rust backend logic and React presentation.
* Fast, asynchronous typed IPC via `invoke` and event listeners.
* Web-standard styling allows modern, polished media player UX (glassmorphism, smooth animations, responsive grids).

## Consequences
* Frontend cannot access databases, filesystems, or audio devices directly; everything must travel through Tauri commands/events.
* Cross-platform webview rendering variations require testing on standard Linux/WebKitGTK and Windows/WebView2 platforms.
