# Convention audit: legacy refactor (2026-11-14)

Range: `39272c2..aef8df6` (branch `feat/recommendations-round`). Auditor: Claude (agent), using [CONVENTIONS_AUDIT.md](../CONVENTIONS_AUDIT.md). This report follows [the baseline audit](2026-11-14-conventions-baseline.md) and works through its legacy backlog.

## Result

**Pass.**
- Every file over 1000 lines in the backlog is split, except `App.tsx`, which is down from 2420 to 1135 lines.
- Each split is a separate `refactor(…)` commit and changes no behaviour.
- The one functional test change fixes a known flaky test (violation #6 of the baseline audit).
- Two baseline violations are still open: #1 (migrations with no ADR) and #2 (`docs/DOWNLOADS.md` and its ADR). Neither is a refactor item.

## Automated checks

- `npm run audit:conventions -- --all`: **no failures**.
- `qlty check` on the changed files: **no issues**.
- `cargo clippy --all-targets`: clean.
- `cargo test --jobs 1 --offline --no-fail-fast`: **367 passed, 14 ignored**.
- `npm test`: **80/80**.
- `tsc`, `npm run build` and the native `cargo build --features tauri/custom-protocol` are clean.
- The scoring test that was flaky passed 5 of 5 runs after the fix.

## Commits and sizes

| Commit | File | Lines before → after | Moved to |
|---|---|---|---|
| `967cfd9` | `core/processor.rs` | 3704 → 201 (`processor/mod.rs`) | `core/processor/<domain>.rs`, `routing.rs` |
| `e8ce2a4`, `e9e03a8` | `src/App.tsx` | 2420 → 1135 | `hooks/app/useBackendEvents.ts` + `backendEvents/`, `hooks/app/useLikes.ts`, `components/layout/AppViews.tsx` + `appViews/`, `modals/WhyTrackModal.tsx` |
| `d29e888` | `playback/service.rs` | 2335 → 354 | `playback/service/{flow,flow_adaptive,monitor,play,transport,queue_ops,state,bookmark,tracks}.rs` |
| `ca5dc7b` | `StatsView.tsx` | 1786 → 226 | `views/stats/` (8 sections, `RankedListCard`, `monthNames`) |
| `8f48262` | `DiscoveryView.tsx` | 1630 → 535 | `views/discovery/` (cards, 4 shelves, hero, offline, `ShelfHeader`) |
| `b5c343e` | `PlaylistsView.tsx` | 1539 → 302 | `views/playlists/` (2 sections, header, 5 modals) |
| `d172f9e` | `OnlineSearchResultsSection.tsx` | 1078 → 408 | `views/searchResults/` (header, 5 tabs, `CoverOrDisc`) |
| `20ad99c` | `CollectionDetailView.tsx` | 1042 → 161 | `views/collection/` (hero, toolbar, tracks, `types.ts`) |
| `c1321cf` | `recommendations/flow.rs` | 1307 → 263 | `flow/{types,candidates,home,explain}.rs` |
| `80320cc` | `discovery/coordinator.rs` | 1246 → 245 | `coordinator/{charts,starters,for_you}.rs` |
| `56809c8` | `recommendations/mixes.rs` | 1221 → 194 | `mixes/{online_tracks,build,families,generators}.rs` |
| `03e0bdf` | `stats_repo.rs` | 1172 → 387 | `stats_repo/{types,writes,ranked,overview,all_time}.rs` |
| `a43a65f` | `mpris/service.rs` | 1130 → 254 | `mpris/service/{root,player,metadata,tests}.rs` |
| `aef8df6` | `active_scoring_tests.rs` | — | tolerance instead of exact float equality |

Duplication removed:
- the 38-line "find or register online track" block in `play_track`/`enqueue_track` (now `tracks.rs::find_or_register_track`);
- the three 56-line Discover shelf headers (`ShelfHeader`);
- the three "cover or disc" blocks in search results (`CoverOrDisc`).

## Checklist

| Item | Result | Evidence |
|---|---|---|
| New code in its own modules | ✅ | see the table above |
| Hubs gained wiring only | ✅ | `App.tsx` lost 1285 lines; it now builds one `AppViews` context and calls hooks |
| SQL in repositories | ✅ | `stats_repo` queries stay in the repository. Legacy SQL in `playback/service/tracks.rs` was moved as-is (one copy instead of two) |
| Render-only components | ✅ | sections read a typed context and own no state |
| `audit:conventions` passes | ✅ | `--all`, no failures |
| No new duplication | ✅ | three duplicate groups removed; the 24-line MixCard/ChartCard similarity is older |
| IPC / migrations / user files | n/a | none touched |
| Tests | ✅ | source-text tests now point at the moved files; the download-state guard also covers `discovery/DiscoveryTrackCard.tsx`, `searchResults/*Tab.tsx` and `collection/CollectionTracks.tsx` |
| Performance rules | ✅ | no new App state; the route switch keeps remount-per-route through a keyed fragment; no new lists or blur |
| Commit hygiene | ✅ | one file per commit, conventional messages |

## Complexity

Function complexity from qlty, whole tree, base `39272c2` compared with `aef8df6`. `worker/` is outside the audited tree and excluded.

| Measure | Before | After |
|---|---|---|
| Largest function | 857 (`App`) | 356 (`App`) |
| Functions with complexity ≥ 100 | 12 | 9 |
| Sum of flagged function complexity | 6799 | 6791 |
| Files over 500 lines | 28 (31 268 lines) | 19 |
| Ratchet: legacy files / smells | 89 / 195 | 114 / 208 |

The ratchet's smell count rose even though complexity didn't. A split cuts one huge function into several that are each still over the threshold. For example, `PlaylistsView` at 231 became 98 + 42 + 25 + 21.

The trait delegates in `stats_repo` also repeat signatures with 6–7 parameters, so the "many parameters" smell is counted twice there. Removing it would need options structs on `StatsRepository`, which is an interface change.

## Violations and backlog

| # | File | Rule | Metric | Fix |
|---|---|---|---|---|
| 1 | `src-tauri/migrations/20261112*.sql` | Compatibility | no ADR | **closed** `8ec63e0`: ADR 0031, tolerate newer profiles + backup before migrating |
| 2 | `docs/DOWNLOADS.md`, `docs/adr/` | Docs | outdated | **closed** `f3fa1ab`: rewritten for Kaze-owned downloads, ADR 0032, old doc archived |
| 3 | `flow/candidates.rs::generate_candidates` (555), `coordinator/for_you.rs::get_discovery_recommendations` (530) | ≤ 500 lines | single functions | **closed** `9c0a565`, `24b5ee7`: staged pipelines (45 and 35 lines), snapshot test `35dab39` unchanged |
| 4 | `App.tsx` | Hub wiring only | 1135 lines, complexity 356 | **closed** `4df92a6`: 553 lines, complexity 87 |
| 5 | `playback/stream.rs` (960, 10 smells), `cloud/sync_manager.rs` (866), `useCollections.ts` (680), `SettingsView.tsx` (675), others 500–720 | ≤ 500 lines | ratchet | split when next touched |

## Follow-up (same day): backlog items #1–#4 closed

| Item | Before | After |
|---|---|---|
| Migrations | 2.0.5 panics on a newer profile; no decision recorded | `set_ignore_missing(true)`; `VACUUM INTO <db>.bak-before-<version>` before pending migrations; ADR 0031 (additive only); 3 tests in `migration_guard_tests.rs` |
| Downloads docs | Soulseek flow | `docs/DOWNLOADS.md` matches the code; ADR 0032; old text in `docs/archive/` |
| `generate_candidates` | one 555-line function | `flow/candidates/{inputs,rows,scoring,ordering}.rs`, all files qlty-clean |
| `get_discovery_recommendations` | one 529-line function | `coordinator/for_you/{inputs,scoring,rotation}.rs`, all files qlty-clean |
| `App.tsx` | 1135 lines, complexity 356 | 553 lines, complexity 87; 7 new hooks/components, all qlty-clean |

The safety net is `tests/scoring_snapshot_tests.rs`, written before the scoring refactors. It froze the Flow scores and the For You order and rotation, and stayed identical through both splits.

The one intended behaviour change is that a For You candidate with a NaN affinity is now treated as not relevant.

Checks at `4df92a6`:
- `cargo clippy --all-targets`: clean.
- `cargo test`: 372 passed, 14 ignored.
- `npm test`: 80/80.
- `tsc`, `npm run build` and the native `custom-protocol` build: clean.
- `npm run audit:conventions -- --all`: no failures.

Item #5 is still the ratchet backlog.
