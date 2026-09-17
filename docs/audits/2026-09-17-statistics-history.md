# Phase 21 statistics and listening-history correctness audit

Audited 2026-09-17 against the existing, uncommitted Phase 21 working tree. Production SQLite was inspected read-only. No production database repair, Worker deployment, playback streaming change, MPRIS work, or UI redesign was performed. Small UI wording changes describe the actual summary sources.

**Verdict: not ready for an unconditional long-term reliability sign-off.** The three-layer separation is sound, and the targeted changes below repair several real defects. Lifetime multi-device merging and playback-event accounting still have correctness gaps. Passing tests establish the listed contracts, not absence of the remaining defects.

## PASS — verified behavior after the targeted fixes

- Normal native start → finish/stop/replacement uses one `ActiveSession`; `Option::take()` prevents repeated finish/stop events from consuming it twice. A UUID is now allocated at session creation and reused at persistence.
- History, track lifetime counters, daily counters, and existing artist/genre affinity updates now commit in one SQLite transaction. A duplicate session UUID is a no-op. An injected daily INSERT failure rolls back history and lifetime writes; retrying that record succeeds once.
- An account change after HistoryService has processed the start event does not reassign the active session. Guest sessions remain `default`. All three tables use the captured owner. Ordinary login, restore, and sync do not invoke guest claiming; automatic claiming on registration has also been removed.
- Lifetime and all-time songs/artists come only from `track_statistics`; daily totals are never added to lifetime. Summary timeline queries, available years, and earliest timeline date now come only from `daily_user_stats`, including past years.
- Daily key is `(user_id, device_id, stat_date)` locally and in D1. Cumulative rows merge by per-field maximum, not addition. A: 3600 → 4200, B: 1000 produces 5200. Repeated push/pull is idempotent, including equal timestamps and backwards clocks.
- The Worker derives user identity from the validated bearer session. Real Worker handler tests with a SQLite-backed D1 adapter prove that a supplied `user_id=B` in A's push cannot write B's rows; A's GET cannot read B's data.
- Empty daily payloads contain no deletion operation. Pull → reconcile → snapshot → push is preserved; absent local aggregate rows do not erase D1 data.
- Fresh-install simulation restores lifetime, songs/artists, daily timeline, graph and top days without raw history. Local reconciliation preserves distinct device rows. Offline sessions accumulate into one current cumulative snapshot per device/date.
- `SyncPayload` has no raw playback history, session UUIDs, playback ticks, or exact per-session source context. Song metadata, aggregates, settings, playlists, and the pre-existing yearly aggregate envelope remain in the payload.
- Device ID lives in local `application_settings`, is excluded from synced settings, and survives normal restarts/upgrades/logout/login. Concurrent initializers now return the same stored ID. True data removal creates a new ID; restored old device rows keep their own keys.
- Indexes cover daily `(user_id,stat_date)`, the daily primary key, and track `(user_id,track_id)`. No indexes were added. Daily query size grows with days × devices, not playback ticks.

## ISSUES — remaining actual correctness bugs

### P1: Lifetime sync loses independent offline contributions

`src-tauri/src/cloud/sync_manager.rs` merges track counts/time with `MAX`; `worker/src/index.ts` does the same for `song_stats`. Starting from shared 100 seconds, A adds 30 and B adds 40 offline: snapshots 130 and 140 merge to **140, not 170**. A Worker regression documents this observed behavior. Daily device rows preserve both contributions, so lifetime and daily views can diverge for new listening too, independently of the older lifetime baseline.

Fixing this requires per-device per-track cumulative components or another explicit operation/delta protocol. Applying SUM to existing snapshots would instead duplicate restored totals. No schema/protocol redesign was made during this audit.

### P1: Position is not elapsed listening time; EOF is not necessarily completion

`HistoryService` keeps `max_position_secs`. Stop and replacement persist that maximum; natural completion uses `PlaybackService`'s full `duration`. Seeking forward inflates listened time, seeking back/replaying undercounts it, and buffering is not explicitly measured. Any backend EOF is published as `completed=true` with the full duration, including a potentially premature stream end. The existing meaningful-play rule is >=30 seconds OR >=50% OR completed; skipped means non-meaningful stop/replacement, not every Next action.

This audit left playback streaming/monitor behavior unchanged. Reliable elapsed listening requires active-play accounting that excludes pauses, seeks and buffering.

### P1: Completion monitor retains stale track identity

`src-tauri/src/playback/service.rs:107-149` initializes `current_track_id_cache` only when empty and clears it only on natural completion. Manual Next/Previous/replacement/Stop does not reset that local cache. After A → B, EOF can emit `TrackFinished(A)` while history owns B. The history match then ignores B's completion; the following replacement/stop can log B as incomplete using its last position. Very short tracks may finish before the monitor ever observes a playing tick.

Events carry track IDs, not session IDs. A delayed completion for an earlier play of the same track cannot be distinguished from a later play. The new persistence UUID protects retries of one record; it does not solve event identity at the producer.

### P1: No awaited shutdown finalization or durable pending-session recovery

`src-tauri/src/app.rs` has no exit/close handler draining history or committing the active session. Normal exit and crashes can lose the current session. A database failure now rolls back atomically and is logged, but the consumed session is not queued for retry. Event-channel lag is now logged and the listener continues; events already dropped remain unrecoverable.

### P1: Legacy repair migration is installation-specific and unsafe as a general upgrade

`20260917000000_repair_legacy_user_stats.sql` chooses a hardcoded canonical UUID or the oldest user, then claims all guest history. Only two hardcoded track collisions are merged. A minimal SQLite reproduction with another colliding track fails with `UNIQUE constraint failed: track_statistics.user_id, track_statistics.track_id`. Artist/genre/preference collisions are likewise not generally merged. It can also assign legitimate guest history to an arbitrary first user.

This migration was already present/applied in the user's Phase 21 work. It was not rewritten: changing an applied SQLx migration's checksum can break startup, and choosing historical ownership automatically would be unsafe. It needs a separate release/migration decision before distribution to other installations.

### P2: Start ownership is captured at event consumption, not audio start

`HistoryService::on_playback_started` reads mutable `current_user` asynchronously after the start event was published. Switching accounts before that event is consumed can still assign the session to the new account. Startup session restoration can race similarly. The tested A → B case waits until the start is processed; it does not prove this boundary race absent. Capture identity in the playback-start producer/session contract to close it.

### P2: Legacy recording command remains a second entry point

`Command::RecordPlaybackSession` remains callable, though the current frontend has no call sites. It accepts no session UUID, generates a new one per call, uses the user active at command time, and reconstructs start time from listened duration. Its writes are now atomic, but repeated calls can duplicate a session and account switching can choose the wrong owner. Retire it or require explicit session identity before supporting legacy callers.

### P2: Local detailed-history query is not user scoped

`SqliteHistoryRepository::get_recent_history` filters no `user_id`. Summary queries are scoped, but granular local history can mix users if exposed to an account-specific screen/API. Local-only storage does not itself enforce account isolation.

## RISKS — valid behavior with important limits

- **Midnight:** existing behavior assigns the whole 23:58–00:08 session to the start day; it is not split. This remains intentional and is tested. The start-day label is now captured when the session begins instead of reinterpreting the timestamp in the timezone at finalization.
- **Travel/DST/timezone:** each new session uses its then-current local calendar label. Existing daily labels remain stable and devices in different zones sum equal labels. DST does not require a fixed 24-hour day for this scheme. There is no historical timezone/offset field, so backfill cannot recover the original timezone after travel. System-clock errors at session start can still misdate a session. No automatic relabeling or splitting was introduced.
- **Snapshot merge:** MAX is appropriate only for nondecreasing counters. It intentionally cannot apply decreases/corrections. Restoring an old database with the same device ID and listening before pulling can hide the newly added increment beneath a newer cloud maximum. Cloning app data onto two machines also clones the device component and can lose contributions. Distinct installations must retain distinct IDs; restore should reconcile before new listening.
- **Backfill:** SQL migration/helper use INSERT OR IGNORE and leave raw history unchanged; replay with the same device/date keys is idempotent. Missing device IDs use `legacy_device` during migration; later helper execution under a new real device can duplicate those same historical days. The helper is not called by normal startup. Existing partial daily rows are not repaired by INSERT OR IGNORE. Backfill counts >=30 seconds or completion, omitting the native >=50% meaningful-play rule for short tracks.
- **Sync concurrency:** snapshots are not read in one multi-table read transaction, so one push can briefly contain different cutoffs for lifetime and daily tables. All rows are sent every cycle; there is no dirty flag cleared by this sync. With monotonic daily merge, a later local increment remains eligible on the next cycle. Older equal-timestamp pulls can no longer erase it. Several existing non-daily reconcile/read errors remain swallowed and can make a sync appear successful despite partial work.
- **Retention:** raw sessions are only local. Earliest `daily_user_stats` date means earliest aggregate timeline availability, not proof that raw session detail exists on a fresh device.
- **Power failure:** the new transaction prevents partial three-table commits. Existing WAL + synchronous NORMAL can still lose recent committed transactions after power loss; abrupt app exit can lose an uncommitted active session.
- **Cloud verification boundary:** actual Worker source was executed with a SQLite D1 adapter. Production D1 contents, deployed Worker version, service quotas, and real transport interruptions were not verified or modified.
- **UI verification boundary:** live read-only `StatsRepository::get_stats_overview` agrees with direct SQL and the source metric bindings. A rendered native Stats window was not visually exercised.

## FIXES MADE

1. Added `HistoryRepository::record_session`: atomic history/lifetime/daily/affinity transaction; conflict-on-session-ID no-op; failure rollback. Routed native and legacy recording through it.
2. Captured native session UUID and start-day label at session creation. Subscribed to events synchronously before spawning the consumer; log lag and continue instead of silently terminating the history listener.
3. Replaced timestamp-gated daily replacement with componentwise maximum locally and in Worker SQL. Daily reconcile errors propagate, and local daily import uses the requested account identity.
4. Made persistent device initialization safe against concurrent first callers.
5. Changed Past 7 Days to today plus six prior local dates; current year/month use local calendar; selected-year totals always read daily rows; removed yearly-cache/raw-history fallbacks from Stats summaries.
6. Represented the first timeline day as local midnight so the frontend's local-date rendering does not display the prior day west of UTC. Updated date assertions accordingly.
7. Removed implicit guest claiming during both local and cloud registration. Existing login/restore behavior already avoided it.
8. Corrected year-panel wording to daily totals and all-time rankings. No layout or performance changes.
9. Added regression/evidence tests and made the existing production-mutating migration test opt-in. Added a separate explicitly opt-in read-only live audit.

## Full path and end-path matrix

`PlaybackService` publishes start → `HistoryService` captures owner/UUID/day → position events update maximum → matching finish, stop, or replacement takes the session → `record_session` inserts raw history and updates track/daily totals in one transaction.

| Requested audit | Result / evidence |
|---|---|
| 1. Full write path | Common owner and atomic table writes verified; elapsed-time semantics remain incorrect with seeks. |
| 2. Exactly once | Normal duplicate finish/stop and duplicate persistence retry tested. End-to-end guarantee blocked by missing session identity, shutdown and dropped events. |
| Natural completion | Consumes matching active track once; stale monitor ID and premature EOF remain issues. |
| Next / Previous / another track | Successful new start finalizes old session once as stop/replacement. Failure before new start does not guarantee a final event for old history. |
| Stop | Takes current session once; uses last maximum position, missing the final sub-tick interval. |
| Shutdown | No awaited finalization; active session can be lost. |
| Stream failure / cancellation | Cancellation itself emits no history-finalization event; a later stop/new start or monitor EOF determines behavior. No streaming changes made. |
| Logout / account switch | Does not finalize; already-captured owner retained. Start-consumption race remains. |
| 3. Lifetime vs daily | Separate authorities, never summed. |
| 4–5. Multi-device / timestamps | Daily 5200 example, repeat cycles, stale/equal/future timestamps tested. Lifetime MAX undercount reproduced. |
| 6. Day boundary | Start day receives all time; start label pinned. DST/travel limitations documented above. |
| 7–8. Guest / switching | Native ownership and registration isolation tested; producer/consumer boundary race remains. |
| 9. Device stability | Stored local ID verified; concurrent init test; old restored device components retained. |
| 10. Fresh install | Empty history restore tests verify aggregates/rankings/earliest date. |
| 11. Offline → online | Accumulate three sessions, snapshot, reconcile twice tested; no live disconnect test. |
| 12. Cloud → local | Distinct device rows retained; 5200 example tested. |
| 13. Empty local | Worker empty push preserves aggregates; no daily delete path. |
| 14. Backfill | Same-key idempotence tested; ownership/changed-device/short-track limits recorded. |
| 15. Stats metrics | Mapping below; past-year stale source fixed. |
| 16. Raw history | No raw sessions in serialization or Worker tables/push paths. |
| 17. Tenant boundary | Authenticated Worker GET/POST adversarial user ID tests pass. |
| 18. Races | Daily regression protection fixed; no dirty-clear loss; cross-table snapshot timing and event races remain. |
| 19. Crash safety | Injected failure proves atomic rollback; active-session recovery remains absent. |
| 20. Indexes | Existing composite keys/indexes suffice; no new indexes. |
| 21. Live totals | Read-only SQL plus live repository query agree; rendered UI not checked. |
| 22. Tests | Results below; simulations distinguished from production verification. |

## Stats metric source map

| Metric | Authoritative read |
|---|---|
| Lifetime listening / all-time plays | SUM of account `track_statistics` counters |
| All-time top songs | `track_statistics` joined to local/external metadata, weighted ranking |
| All-time top artists | `track_statistics` grouped by artist metadata |
| Today | Account daily rows with today's local date |
| Past 7 days | Account daily rows between local today − 6 and today inclusive |
| Month / graph | Daily rows grouped by date in selected month |
| Year | Daily rows in selected year; no yearly cache |
| Active days | Frontend count of positive daily graph points in selected month |
| Top days | Account daily rows summed across devices, ordered by seconds, top 5 |
| Earliest timeline / year options | MIN date / distinct years of account daily rows |
| Granular sessions | Local `playback_history`; current recent-history query is unscoped |

## LIVE DATA

Read-only snapshot for the requested canonical account and persistent device on 2026-09-17:

| Metric | Value |
|---|---:|
| Lifetime seconds | 85,116.219750398 |
| Lifetime hours | 23.6433943751 |
| Track-stat rows | 66 |
| Daily rows / devices | 2 / 1 |
| Earliest / latest date | 2026-09-16 / 2026-09-17 |
| September 16 | 30.0 seconds |
| Today (September 17) | 229.140668935 seconds |
| Past 7 days | 259.140668935 seconds |
| September | 259.140668935 seconds |
| 2026 | 259.140668935 seconds |
| Local playback-history rows for account | 46 |
| Active September dates | 2 |
| Top song / artist | PHONKY TOWN / Playaphonk |

The persistent device equals the user-supplied ID. The supplied earlier baseline was ~85,087.7 lifetime seconds / 200.6 today; current values are higher. This audit did not attribute that increase to a particular session. The Stats formatter truncates to minutes: expected display is **23h 38m lifetime**, **3m today**, **4m week/month/year**. This is source/repository agreement, not a screenshot assertion.

## TEST RESULTS

See final verification totals appended below. Reproducible full backend command uses temporary XDG directories and a temporary ALSA null device to avoid workstation audio dependencies; `--offline` controls Cargo dependency fetching, not HTTP calls made by tests. The initial restricted run's loopback, cache-write and audio failures cleared under these test conditions.

The additional Worker test runs the actual bundled Worker handler against SQLite through a D1-shaped adapter. It covers duplicate push, empty push, two devices, adversarial tenant IDs and stale timestamps; a second test documents the unresolved lifetime MAX undercount. It is not a deployed-D1 integration test.

- Full Rust backend: **125 passed, 0 failed, 4 ignored** (two live benchmarks, production migration, optional live read-only audit).
- Read-only live repository audit: **1 passed**, explicitly run separately.
- Frontend regression tests: **11 passed** across four files.
- Worker regression/evidence tests: **2 passed**.
- `npm run build`: **passed** (TypeScript and Vite).
- Existing whitespace warnings remain in pre-existing Phase 21 edits to BACKEND.md, DEVELOPMENT_LOG.md, stats_and_account_tests.rs and worker/schema.sql; no unrelated cleanup performed.
