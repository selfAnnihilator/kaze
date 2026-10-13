# Kaze Home + Flow UX refinement — 2026-10-05

This pass refines the existing uncommitted implementation. Home's layout, library-first Flow selection/scoring, PlaybackService queue ownership, playback/history semantics, Linux MPRIS and focused Space handling remain intact. No commit or push was made.

## Changed files in this pass

- `src/App.tsx`: connects Home controls to the existing pause/resume handler; routes explicit navigation through history; restores and guards collection/library/search requests; presents typed track evidence.
- `src/components/views/HomeView.tsx`: authoritative Flow and For You play/pause labels/icons/actions; friendly explanations in rows and modal.
- `src/components/views/LibraryView.tsx`: restores its debounced search input from the visited destination and cancels pending input on restoration.
- `src/navigationHistory.ts`, `src/hooks/useNavigationHistory.ts`: destination stack/index, branch truncation, stable Back/Forward functions and guarded Alt shortcuts.
- `src/localTrackMatch.ts`: shared current-track identity using exact/matched IDs or guarded online metadata aliases.
- `src/flowExplanation.ts`, `src/components/TrackExplanation.tsx`, `src/types.ts`: deterministic frontend copy and shared evidence presentation.
- `src-tauri/src/recommendations/flow.rs`, `src-tauri/src/recommendations/local.rs`: additive structured explanation evidence; existing ranking and legacy reason fields retained.
- `src-tauri/src/playback/service.rs`: retains the structured selection evidence alongside existing generated-entry ownership; returns an existing-queue fact for manual entries.
- `src-tauri/src/core/query.rs`, `src-tauri/src/core/processor.rs`: `GetFlowTrackReason` returns structured evidence instead of prose.
- `tests/homeFlowUx.test.mjs`, `tests/homeFlowRender.test.mjs`, `src-tauri/tests/flow_tests.rs`: focused regressions and actual Home rendering checks.
- This report. Earlier uncommitted audit changes are preserved.

## Navigation design

The history entry is the single routing state: view, collection metadata/tracks, lyrics visibility, library query/album ID, and submitted online query/results. Visits append after truncating the forward branch; equivalent visits replace the current entry. Back/Forward change the index without appending. History is bounded to 100 entries. Collection hydration/edits and debounced library input update the current entry without adding intermediate pages.

Sidebar views/playlists, album links, discovery collections/search, player origins, lyrics and collection Back use this model. Collection Back shares the same Back operation as Alt+Left. Future controls can use the hook's `back`, `forward`, `canGoBack`, and `canGoForward`.

Only Alt+ArrowLeft/ArrowRight are handled. Ctrl/Meta/Shift conflicts, already-handled events, editable controls and dialogs are left alone. `preventDefault` occurs only when a history step succeeds. Normal arrows and the existing focused Space/MPRIS paths are untouched. Navigation itself dispatches no playback commands.

The existing album presentation is still a title-filtered Library view; history additionally records its album ID. This pass does not add dedicated album or artist pages. Collection/search/library requests use effect cancellation to stop late responses overwriting a newer destination; an unfinished restored collection/search is loaded again.

## Playback synchronization

Flow derives Pause/Resume from the runtime-active session, current session track ID and authoritative `PlaybackState.is_playing`. Pause and Resume call the same existing handler as the player/Space; neither calls `StartFlow`. Inactive/unrelated Flow shows Start. Stop retains the audited backend pruning behavior.

For You compares each recommendation with `PlaybackState.current_track`. Exact local/external/matched IDs are preferred; catalog aliases can use the existing normalized title/full artist-credit matching with duration guards. Distinct local files are not conflated by metadata. Only the current row toggles Pause/Resume; other rows use existing `handlePlayTrack`. Backend pause/resume events already update this same state, including external MPRIS controls. No extra playing boolean or progress state was introduced.

## Explanation model

`FlowExplanation` carries a primary category and verified liked/unplayed status, recorded play count, days since last play, artist name/affinity, selection-time steering and timestamp, or existing-queue provenance. Generation and current-track queries use the same evidence helper. Playback keeps the facts captured at selection time; copy distinguishes those snapshots from current facts.

The frontend owns all new prose and renders a title, body and at most two secondary facts. Examples include “A fresh pick from your library”, “One of your favorites”, “Time to rediscover this” and “More from [artist]”. Artist affinity includes listening and feedback, so it does not claim frequent listening. Missing timestamps never imply age; missing history with positive plays/skips never implies unplayed. Month wording is approximate, derived from recorded days. No relative low-play ranking is available, so none is invented. Unknown/cold-start evidence falls back safely to “From your library”. Existing backend legacy reason strings remain for compatibility/tests and are not displayed by Home/player explanation surfaces.

## Verification

- `npm run build`: passed.
- `cargo check --bin kaze --manifest-path src-tauri/Cargo.toml`: passed.
- `node --test --test-isolation=none tests/*.test.mjs`: 25 passed. Covers actual Home rendering, moving active row, all explanation cases, ID aliases, restored album/collection/search destinations, branching and shortcut guards.
- Relevant backend integration suites: Flow 17, playback 7, history 3, recommendation 5 (32 total), passed.
- Backend library tests: 32 passed, including all six MPRIS tests.
- `git diff --check`: clean.
- Reasoned the requested Start -> Pause -> Resume -> Stop and Home -> album -> another view -> Back -> Back -> Forward -> Back -> new destination sequences. The backend regression confirms Pause/Resume retains the current track, queue, steering/session settings and typed selection evidence.

No audible desktop playback or physical global-key test was performed in this pass. Those existing paths were preserved and covered by relevant state/backend checks. The full online/download integration suite was not rerun; this pass does not touch provider or download execution. The previous audit records its unrelated environment/live-network limitations.
