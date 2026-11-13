# Auto-tuned exploration

Flow's explore share came only from the familiarity dial. Spotify adapts how much it explores to how a listener reacts to new music. Kaze never learned whether new finds landed.

Decision:

| Area | Rule |
| --- | --- |
| Slots | Playback Flow (`include_online`) writes every selected pick into `flow_slot_assignments(user_id, track_id, slot)`. A pick is `explore` when it is online or when its novelty term beats its familiar term; otherwise it is `familiar`. Enqueueing again overwrites the slot. The Home preview writes nothing. |
| Attribution | `HistoryService::finalize_active_session` stamps `playback_history.rec_slot` for `source='flow'` listens after a successful `record_session`. The slot lives in the database, not in playback memory, so it survives restarts and needs no new IPC. |
| Tuner | `recommendations/exploration.rs`. Uses the last 30 days of listens that have a `rec_slot`. A qualified listen (not skipped, and completed or ≥ 50 % heard) counts +1 and an early skip −1, mapped to 0..1. `bias = clamp((s_explore − s_familiar)·0.5, ±0.12)`. Below 30 listens in either slot, the bias is 0. Cached daily in `application_settings` as `flow_explore_bias:{user}` = `bias@computed_at`. |
| Apply | Novelty weight `0.55·(1+bias)`. Online share `clamp((1−ratio)·0.6 + bias, 0, 0.7)`, playback Flow only. |
| UI | `FlowExplanation.rec_slot` and `explore_bias` (optional, additive). An explore pick with a bias ≥ 0.03 gets the secondary line "Exploring a bit more — you've liked new finds." |

Alternatives:
- Storing slots in `ActiveFlow.reasons` and passing them to history through playback. This was rejected: it crosses services, is lost on restart and needs new plumbing.
- A multi-armed bandit over the dial position. This was rejected because it would override the user's explicit dial. A bounded nudge keeps the control honest.
- Counting plain completion only. This was rejected because the qualified/skip signal matches how taste is already learned.

Consequences:
- With no evidence the bias is 0, so behaviour is unchanged until there are about 60 Flow listens.
- A track can be re-labelled when it is enqueued again; the latest assignment wins for attribution.
- Non-Flow listens are never stamped, so library play does not skew the tuner.

Tests: `src-tauri/tests/exploration_tests.rs`
- Pure bias signs, bounds and minimums.
- Completed explore listens raise the bias and skipped ones lower it.
- Listens older than the window are ignored.
- Slot stamping and re-labelling.
- Daily cache reuse and recompute.

Frontend: `tests/homeFlowUx.test.mjs` checks the copy appears only for explore picks with a positive bias.
