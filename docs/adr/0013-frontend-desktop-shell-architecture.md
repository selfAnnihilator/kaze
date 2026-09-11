# ADR 0013: Frontend Desktop Shell Architecture, IPC Bridge, and Component Design

## Context
Following the implementation of Phases 1 through 8, the Rust backend is feature-complete, containing all business logic, audio decoding, ranking algorithms, taste profiling, recommendation scoring, external metadata orchestration, fuzzy library matching, and Soulseek acquisition workflows.

Phase 9 requires building the desktop shell and graphical user interface. The core philosophy of the project dictates that:
1. The frontend must remain thin and declarative, primarily displaying backend state and dispatching typed commands/queries.
2. No heavy business logic, algorithmic scoring, or file I/O may reside in JavaScript/TypeScript.
3. The UI must be responsive, modern, dark-themed, and responsive to real-time events (playback position ticks, library scan status, download progress).
4. The system must support development both inside the Tauri desktop webview and within standard browser environments for fast iteration and automated bundle validation.

## Options Considered
1. **Electron with Node.js backend**:
   - Rejected due to high memory footprint, large binary size, and violation of the project's requirement to use Rust for all performance-critical subsystems and audio playback.
2. **Native Rust GUI (e.g. Slint, Iced, or egui)**:
   - Rejected due to ecosystem maturity constraints for complex responsive audio scrubbers, album art grids, and flexbox table styling compared to web standards.
3. **Tauri v2 + React 19 + TypeScript + Vite**:
   - Tauri v2 provides an ultra-lightweight desktop container leveraging the OS native webview (WebKitGTK on Linux, WebView2 on Windows, WebKit on macOS), compiling the Rust backend directly into the application process.
   - React 19 + TypeScript offers declarative state rendering with compile-time type safety matching the backend's serde JSON schemas.
   - Vite provides instant Hot Module Replacement (HMR) and sub-second production builds.

## Decision
Adopt **Tauri v2 with a React 19 + TypeScript + Vite frontend**, using an explicit typed IPC layer:
* **Two Primary IPC Channels**: `execute_command` for mutations and `execute_query` for data requests, serializing to and from Rust enum variants without wildcard fallback.
* **Reactive Event Bridge**: The Rust `EventBus` broadcasts internal events (`PlaybackPositionChanged`, `ScanProgress`, `DownloadProgress`, etc.) directly to the Tauri webview via `app_handle.emit("backend-event", payload)`. The frontend hooks into these events via `@tauri-apps/api/event`.
* **Universal API Layer (`src/services/api.ts`)**: Auto-detects whether the app is executing inside Tauri (`window.__TAURI_INTERNALS__`). If running in a web browser, it gracefully logs commands and serves mock responses, allowing `vite build` and component testing without requiring a live webview window.
* **Component Hierarchy**:
  - `App.tsx`: Central coordinator managing domain data, playback state, and event subscriptions.
  - `Sidebar.tsx`: Navigation across 8 views (Library, Artists, Albums, Playlists, Smart Mixes, Discovery, Wishlist, Downloads, Settings).
  - `NowPlayingBar.tsx`: Sticky audio control bar with scrub slider, volume, repeat, shuffle, and instant track liking/disliking.
  - `OnboardingModal.tsx`: Enforces system audio root directory inspection, user choice, and strict boundary containment.
  - Dedicated modular views: `LibraryView`, `ArtistsView`, `AlbumsView`, `PlaylistsView`, `DiscoveryView`, `WishlistView`, `DownloadsView`, `SettingsView`.

## Consequences
* The UI remains completely decoupled from low-level audio decoding and storage details.
* The frontend bundle is compact (~300 KB gzipped) and starts instantly.
* Full offline functionality is preserved.
