# Listening-time preference and score eligibility

Automatic preference must be earned through actual listening, rather than selecting a song, a seek position or an end-of-file flag. The policy is shared by local/discovery recommendation scoring, Flow, taste affinities and automatic repeat/favorite eligibility. Explicit likes/dislikes remain immediate user intent; Not interested remains a strict recommendation exclusion.

| Evidence within one listening session | Preference weight |
| --- | ---: |
| Intentional skip before 10 seconds | -0.75 |
| Below the positive listening threshold | 0 |
| Qualified partial listen | +0.35 |
| At least 75% actually listened | +0.75 |
| At least 95% actually listened | +1.0 |

Positive eligibility starts at `min(30 seconds, max(10 seconds, 25% of duration))`. Ordinary songs therefore need 30 seconds. A 60-second song needs 15 seconds; clips shorter than ten seconds earn no automatic positive preference. Unknown-duration audio needs 30 seconds and cannot claim a percentage-based full listen. The 95% boundary tolerates decoder/container timing differences; a verified completion also requires actually reaching the end. Repeated qualified listens add a bounded song-level bonus, and explicit likes add the existing stronger preference signal. Neutral attempts do not dilute or inflate an existing song's engagement average.

`recommendations/engagement.rs` owns the thresholds and SQL eligibility helpers. Rust history accumulates monotonic playing intervals, excluding pauses and active buffering. Position updates and seek offsets do not add listening time. Native EOF now reports no duration-derived listening estimate; HistoryService uses elapsed listening, rather than manufacturing a duration-sized full listen. EOF without enough heard time is neutral. Matching terminal playback errors finalize neutrally; leaving a buffering stream also avoids a skip penalty. Shutdown/recovery retain their non-skip behavior. Stale session errors and duplicate end/stop events cannot modify another session.

`SongSignals` classifies account-scoped history on read and aggregates the resulting evidence across source aliases/recording families. Local session evidence takes precedence over legacy permissive counters; any unmatched aggregate remainder is retained conservatively. Unrepresented aggregate-only cloud/imported counters remain compatible. Raw lifetime/device counters and listening time are not rewritten. Old records that already inflated `seconds_listened` to a full duration cannot be reconstructed reliably; the new EOF path prevents further inflation. Former completion flags alone no longer prove preference when their stored actual seconds contradict them.

The shared scorer uses engagement for performance and applies a stronger quick-skip penalty. Repeated meaningful listens increase a song's own score. Flow and Home use qualified familiarity instead of counting brief attempts as known music. On Repeat qualifies sessions before limiting candidates, so many brief previews cannot crowd real repeated songs out of the pool. Repeat/Forgotten Favorites final filtering also checks qualified listening or explicit curation; existing favorite-folder intent is retained.

Score-based track/artist/album/genre rankings use read-only qualified-time projections. Raw listening seconds and recorded counters remain available as usage statistics; short neutral sessions do not add ranking points, including after a previous qualified listen of the same song. Queries preserve account/window boundaries and unmatched cumulative totals. This is separate from the legacy configurable usage play-count thresholds, which remain compatible.

TasteProfileChanged is published only after a successful history commit with non-neutral evidence. App continues to debounce personal refreshes; no frontend listening timer or playback-position state was introduced. Taste/Discovery recompute from current history, and Home's picks are process-local. A one-time migration expires only generated smart mixes to avoid serving old scoring for another week. Manual playlists, IDs, files, history and cumulative totals are preserved.

Verification covers threshold edges, short/unknown durations, non-finite evidence, graded affinity, neutral-session invariance, explicit feedback, legacy counters, seek-to-end/EOF, terminal errors, buffering exits, repeat preselection before its limit, migration scope and neutral ranking-time eligibility. The old partial-listen test now expects the baseline tier, and the repeat fixture uses a genuinely full 360-second listen instead of labelling a 200-second excerpt complete.
