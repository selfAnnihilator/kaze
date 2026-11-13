# Flow session steering

Flow reacted to listening only through long-term taste, which moves slowly by design (ADR 0023). Skipping two Rock songs in a row changed nothing until history absorbed it, and queued tracks were never re-ranked. A radio should notice "not this right now" within one or two skips.

Decision:

| Area | Rule |
| --- | --- |
| Signal | HistoryService publishes `ListenOutcome` for every finalized play session, reusing `Engagement::from_listening` (early skip, qualified, near-complete). |
| State | `SessionSteer` lives in the active Flow, in memory only. Keys are canonical artist, genre and identity language. Skip −1, full +1, qualified partial +0.4; ×0.85 decay per outcome; at most 64 keys. Reset by `StartFlow`; dropped with the active Flow. |
| Triggers | 2 consecutive skips sharing a genre/language/artist → away; 3 consecutive skips → away; 3 consecutive full listens sharing a genre/language → toward. A 2-outcome cooldown follows each trigger. |
| Scoring | `0.18·tanh(g/2) + 0.08·tanh(l/2) + 0.12·tanh(min artist/2)`, negatives doubled, clamped to [−0.30, +0.15], added after the skip penalty. Genres at ≤ −1.5 get no online exploration. |
| Queue | A trigger regenerates 15 generated upcoming slots under `flow_operation` with generation checks. Current track, position and manual entries stay. Steering and refills pass the same session state. |
| UI | Explanations carry `session_style` and `session_adjustment`; only the positive note is shown. No new frontend state. |

Consequences: two quick skips of a style visibly change what comes next, and a run of finished songs gently leans Flow toward that style without letting one artist flood it (the asymmetric cap plus the shared diversifier). Session evidence never writes to long-term taste; history keeps doing that. A strongly preferred style can still reappear once the decay window passes.

Tests: `src-tauri/src/recommendations/session.rs` unit tests, plus `flow_tests.rs` (`session_skips_steer_away_from_genre`, `session_full_listens_steer_toward_genre`, `rapid_rock_skips_rerank_upcoming_flow`, `session_resets_on_start_flow`) and `tests/homeFlowUx.test.mjs`.
