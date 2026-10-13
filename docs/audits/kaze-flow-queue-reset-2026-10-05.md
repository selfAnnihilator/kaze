# Fresh Flow queue semantics — 2026-10-05

## Root cause

`PlaybackService::start_flow` used `PlaybackQueue::play_independent(first, true)`. That method intentionally splits and reattaches the upcoming queue for manual playback, so old manual entries remained ahead of appended Flow recommendations. Start also returned `resume()` immediately when a Flow was active, conflating a new Start with Continue. The first generated song was played immediately, unnecessarily interrupting current audio.

## Implementation and contract

- `PlaybackQueue::clear_upcoming` truncates after the current item, or empties all pending entries if there is no current item. It preserves the current queue ID/index, Previous history and repeat configuration. Fresh Flow begins sequentially, matching the existing Start behavior for shuffle.
- Start creates a new runtime generation, drops the previous Flow ownership/evidence and clears **all** upcoming entries before local generation. It uses explicit or persisted steering, excludes a continuing current song, then appends up to 20 freshly owned generated entries. Ranking/scoring stays unchanged. An eligible song formerly manually queued may be independently selected again as a fresh recommendation; its old queue-entry identity/provenance is discarded.
- A song with an active playback/session token, including paused audio or an in-progress source resolution, retains its current queue ID and audio token. Start does not stop, restart, resume or emit playback/history events for that song. Paused audio stays paused. With no live current source, Start plays the first generated item through the existing `play_item` path. Empty generation returns a clear error, leaves current audio untouched and Flow inactive; the explicitly reset upcoming entries remain cleared.
- `ContinueFlow` is a distinct IPC command that requires an active Flow for the current user and calls existing `resume()`. It does not change generation, steering or queue. Home's Resume Flow uses it; Pause Flow keeps the player's existing pause path. For You and normal player controls retain their existing handlers.
- Stop Flow retains audited semantics: adaptive mode ends, generated upcoming entries are pruned, current audio and in-Flow manual choices remain. Explicit manual playback still exits Flow.
- Successful manual enqueue commits are ordered with Flow operations and record their unique queue-entry IDs in active Flow. These choices retain their normal `play_next`/append position and survive Continue and Stop. A later explicit new Start clears them with the rest of the old upcoming sequence.
- Reasons distinguish generated selection evidence, explicit choices queued **during** active Flow, and the song already playing at Start. The last case is `continuing_track`, presented as “Your current track continues”, without fabricated recommendation evidence or queue-choice provenance. Pre-Flow Previous history likewise does not become an in-Flow manual choice.

## Stale work protection

A separate atomic Flow generation counter advances at Start request arrival, before acquiring the existing operation mutex. Older Starts recheck their token around asynchronous generation/settings work before committing. Refills capture the active generation and validate both it and the latest request token before appending. Stop/manual exit invalidate Flow tokens. Queue mutations remain serialized; an older refill cannot append after a newer reset. The existing audio `play_generation` and `(session_id, queue_id)` tokens remain separate and unchanged for preserved current audio, so its natural completion remains valid. Runtime generation is exposed through `GetFlowSession`; it is not a second queue or persisted live session.

## Changed files for this pass

`src-tauri/src/playback/queue.rs`, `src-tauri/src/playback/service.rs`, `src-tauri/src/core/command.rs`, `src-tauri/src/core/processor.rs`, `src-tauri/src/recommendations/flow.rs`, `src/App.tsx`, `src/components/views/HomeView.tsx`, `src/types.ts`, `src/flowExplanation.ts`, `src-tauri/tests/flow_tests.rs`, `tests/homeFlowRender.test.mjs`, `tests/homeFlowUx.test.mjs`, and this report. Previous uncommitted audit/UX work is preserved. No commit or push.

## Verification

- `npm run build`: passed.
- `cargo check --bin kaze --manifest-path src-tauri/Cargo.toml`: passed.
- `node --test --test-isolation=none tests/*.test.mjs`: 26 passed.
- Relevant Rust suites: Flow 21, playback 7, history 3, recommendation 5 and library 34, all passed (70 total). Library coverage includes six MPRIS tests and two new upcoming-reset tests.
- New regression coverage: A continuing while B/C's stale upcoming entries disappear; current position and absence of extra playback events; X queued during Flow retaining position/provenance through Continue; Stop/manual playback/new Start clearing stale entries; Continue preserving generation/queue/settings; Start changing generation while leaving paused current audio paused; two overlapping Starts producing only one PlaybackStarted with the newest generation/settings; actual natural completion retaining the original session identity before the next Flow track starts.
- `git diff --check`: clean.

Native tests use the mock audio backend and isolated XDG directories documented in the prior audit. No live desktop audio, physical media-key or D-Bus/status-bar check was performed. Existing MPRIS implementation and normal Next/Previous/Play/Pause were not modified and relevant regressions pass.
