# Frontend & Desktop Shell Architecture

## Overview
The frontend of SoundFlow is a thin, reactive single-page application written in **React 19**, **TypeScript**, and styled with modern dark-mode CSS. It runs inside **Tauri v2** using native OS webviews (WebKitGTK on Linux) and communicates with the underlying Rust `CoreProcessor` via typed IPC commands, queries, and asynchronous event streaming.

```
+-------------------------------------------------------------------------+
|                               TAURI SHELL                               |
|                                                                         |
|  +-------------------------------------------------------------------+  |
|  |                       REACT 19 FRONTEND                           |  |
|  |                                                                   |  |
|  |  +----------------+  +-----------------------------------------+  |  |
|  |  |   Sidebar.tsx  |  | Active View:                            |  |  |
|  |  |                |  | - LibraryView (Tracks & Search)         |  |  |
|  |  | - Library      |  | - ArtistsView / AlbumsView              |  |  |
|  |  | - Artists      |  | - PlaylistsView (Smart Mixes)           |  |  |
|  |  | - Albums       |  | - DiscoveryView (Missing Music Recs)    |  |  |
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

## Component Hierarchy & Organization

* **`src/App.tsx`**: Top-level application coordinator. Holds reactive state for playback, library items, playlists, wishlist, downloads, and settings. Handles backend event listener registration and view routing.
* **`src/components/Sidebar.tsx`**: Navigation component for switching between views.
* **`src/components/NowPlayingBar.tsx`**: Persistent bottom playback bar featuring:
  - Track metadata, artist, album, format
  - Scrub slider with real-time millisecond-level position synchronization
  - Play/Pause, Next, Previous, Repeat (`off` / `one` / `all`), Shuffle
  - Volume slider and Mute toggle
  - Quick Like/Dislike feedback buttons
* **`src/components/OnboardingModal.tsx`**: Startup wizard enforcing system audio directory inspection, custom folder boundary selection, and user confirmation.
* **`src/components/views/LibraryView.tsx`**: Complete local track listing with live FTS5 search, table sorting, instant play, queueing, and feedback actions.
* **`src/components/views/ArtistsView.tsx`**: Artist collection overview with indexed track counters and search drill-down.
* **`src/components/views/AlbumsView.tsx`**: Album grid displaying release year, artist, and cover art thumbnails.
* **`src/components/views/PlaylistsView.tsx`**: Custom playlist creation and one-click smart mix generators (`Daily`, `On Repeat`, `Forgotten Favorites`, `Discovery`, `Late Night`).
* **`src/components/views/DiscoveryView.tsx`**: External track discovery cards displaying library ownership badges (`In Library`, `Likely Owned`, `Alternate Version`, `Missing Track`), recommendation reasons, "Add to Wishlist", and "Find on Soulseek" shortcuts.
* **`src/components/views/WishlistView.tsx`**: Management table for missing music, status workflows (`Want`, `Downloaded`, `Owned`, `Ignored`), manual track additions, and Soulseek search triggers.
* **`src/components/views/DownloadsView.tsx`**: Soulseek P2P transfers tab (progress bar, speed, status badges, cancellation) and interactive P2P network search tab.
* **`src/components/views/SettingsView.tsx`**: System configuration view for managing audio root folders, rescanning, audio engine defaults, external metadata toggles, and Slskd connection settings.

---

## Typed IPC Interface (`src/services/api.ts`)

Communication between frontend and backend is strictly typed and decoupled:

1. **`dispatchCommand(command: Command): Promise<any>`**:
   Invokes Tauri's `execute_command`. In headless or web browser mode, it logs the dispatched command and returns an optimistic response.
2. **`executeQuery(query: Query): Promise<QueryResponse>`**:
   Invokes Tauri's `execute_query`. In headless or web browser mode, it serves deterministic mock responses.
3. **`subscribeBackendEvents(callback: (event: any) => void): Promise<() => void>`**:
   Listens to `"backend-event"` emitted by Tauri's Rust event forwarder, updating the React UI reactively without polling.
