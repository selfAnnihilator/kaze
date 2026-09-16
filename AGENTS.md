# Kaze Developer & AI Agent Guidelines

## Critical Performance Guardrails (Non-Negotiable)

During Phase 18, a major performance audit and optimization pass reduced renderer private RAM by 61 MB (-18.6%), total private RAM by 84 MB (-16.1%), and eliminated UI jitter and playback re-renders. 

When modifying, redesigning, or styling the frontend UI, **ALL** agents and contributors must strictly obey these performance rules:

### 1. Root `App.tsx` State Isolation
- **NEVER** reintroduce high-frequency values (audio position ticks, seeking values, continuous playback timers, mouse coordinates) into the root `App` component state.
- Playback progress updates (4–10 Hz) must **strictly** use the pub-sub emitter in `src/services/playbackProgress.ts` and `usePlaybackProgress` hook within isolated leaf components (such as `NowPlayingProgressBar`).
- Root `App.tsx`, the sidebar, and the top navigation bar must maintain **0 re-renders per second** during audio playback.

### 2. Backdrop Blur & Compositor Budget
- WebKitGTK on Linux allocates expensive offscreen GPU surfaces for `backdrop-filter: blur(...)`.
- **NEVER** add `backdrop-filter` or `filter: blur(...)` to:
  - Repeated list or grid items (e.g. track rows, album cards, discovery grid tiles, queue items).
  - Badges, pills, or small tags.
  - Elements with opaque or near-opaque backgrounds (e.g. `#181512`, `#24201b`, `rgba(..., 0.95)`).
- Limit `backdrop-filter` strictly to full-window modal backdrops where visually indispensable, keeping `blur <= 8px`.

### 3. List Virtualization
- Any track list, search results view, or collection view that can exceed 50 items **MUST** use windowed row virtualization (via `react-window` 2.x `List` or equivalent).
- **NEVER** map thousands of unvirtualized DOM nodes (e.g., `tracks.map(...)`). Bounded DOM nodes must remain proportional to viewport height (~20–30 DOM rows).

### 4. Animation & Transition Performance
- Only animate GPU-composited properties: `transform` (translate, scale, rotate) and `opacity`.
- **NEVER** animate layout-triggering properties: `width`, `height`, `top`, `left`, `bottom`, `right`, `margin`, `padding`.

### 5. Input Debouncing
- All search inputs, text filters, and query triggers must be debounced by 150ms–250ms before triggering state changes or IPC commands.

### 6. Component Memoization
- Leaf and container components (`NowPlayingBar`, `Sidebar`, `GlobalTopSearchBar`, `LibraryView`) must remain wrapped in `React.memo` with stable props and memoized callbacks (`useCallback`, `useMemo`).

---

## Unified Playback Architecture
- All audio playback (local files, remote streams, preview URLs, downloaded/cached audio) is handled by the **Rust backend (`PlaybackService`)**.
- WebKit / frontend must **NEVER** instantiate HTML5 `new Audio(...)` or `<audio>` elements.
- The player maintains **one queue, one state machine, one history path, one stats path, and one event stream**.
