# Convention audit: download work and project baseline (2026-11-14)

Range: `5bb8619..3ddb111` (branch `feat/recommendations-round`):
- `bcad66c`: test credential isolation;
- `6d56748`: Local/Downloads split;
- `c6fb1f2`: remove a download;
- `3ddb111`: download refactor.

Also covered: a whole-project baseline for [CONVENTIONS.md](../CONVENTIONS.md). Auditor: Claude (agent), using [CONVENTIONS_AUDIT.md](../CONVENTIONS_AUDIT.md).

## Result

**Pass for the download work, with legacy debt recorded.** Every new module of the download work meets the limits. The violations it left are all growth inside files that were already oversized (hubs and large services). They are now frozen by the ratchet and scheduled for splitting.

One **compatibility violation** sits outside the download work but on this branch: four SQL migrations added in `399ef29` (2026-11-13) have no compatibility decision. Because of them, the installed Kaze 2.0.5 no longer opens a profile used by this build.

## Automated checks

- `npm run audit:conventions -- --base 5bb8619`: **no failures**. 53 source files changed. 28 of them are recorded legacy files, and none grew past its baseline.
- `npm run audit:conventions -- --all`: **no failures**. The baseline was recorded today: **89 legacy files** (28 over 500 lines, 61 with qlty function smells), **195 function smells** in total.
- `qlty check` on the changed files: **no issues** (clippy, trufflehog, osv-scanner).
- Rust: `cargo test --jobs 1 --offline --no-fail-fast` gives **367 passed, 14 ignored**; `cargo clippy --all-targets` is clean.
- Frontend: `npm test` passes **80/80** (including the new `auditConventions.test.mjs`); `tsc` and `npm run build` are clean.

## Checklist

| Item | Result | Evidence |
|---|---|---|
| New features in their own modules | ✅ | `downloads/service/{folder,removal}.rs`, `downloads/rows.rs`, `downloads/tags.rs`, `database/repositories/library_scope.rs`; `services/{downloadActions,confirmDialog}.ts`, `hooks/useDownloadAction.ts`, `hooks/app/useDownloadsSection.ts`, `components/modals/ConfirmModal.tsx`, `trackKind.ts` |
| Hubs gained wiring only | ⚠️ partly | `App.tsx` complexity went from 804 to 857 (event cases, playback placeholders, the `fetchPlaybackState` stale-result branch). Processor: `execute_query` 180 → 178, `dispatch_command` 173 → 175 (`RemoveDownload` arm, `scope` plumbing). |
| SQL only in repositories | ✅ | `release_to_online` and `relocate` are in `download_repo.rs`, scoped pages in `track_repo`/`album_repo`, rows in `downloads/rows.rs` |
| Render-only components | ✅ | `DownloadButton.tsx` (no qlty smells); logic in `describeDownload()` |
| `audit:conventions` passes | ✅ | see above |
| No new duplication | ✅ | qlty duplication in the changed files existed before this work |
| Optional IPC fields | ✅ | `scope: Option<String>` with `#[serde(default)]`; new `RemoveDownload` command; `DownloadsChanged` event |
| No new migration | ✅ for the download work / ❌ for the branch | Download work: none (guard test `no_new_migrations_without_a_compatibility_decision`). Branch: `20261112000000…030000` (`399ef29`) have no ADR; Kaze 2.0.5 panics with `migration 20261112000000 was previously applied but is missing`. |
| User files never deleted | ✅ | `ensure_owned_files` in `removal.rs`; moves are limited to task-owned files (`relocate_into_folder`); tests `a_file_outside_the_kaze_folder_is_never_deleted` and `only_kazes_own_downloads_move…` |
| Feature tests, temp dirs only | ✅ | `download_folder_tests.rs` (10), `download_tests.rs`, `songControls.test.mjs`, `downloadsView.test.mjs`. The account-test leak into the real profile was fixed (`use_isolated_store`). |
| Full suites pass, flaky tests named | ✅ | flaky: `active_scoring_tests::manual_for_you_refresh_rotates_relevant_songs_without_changing_taste` (exact float `assert_eq!` on a time-decayed score) |
| Performance rules | ✅ | no new App state for ticks; the confirm popup is store-driven; Downloads rows reuse the virtualized `LibraryView` |
| Docs / ADR | ❌ | `docs/DOWNLOADS.md` still describes the Soulseek flow; no ADR for the Kaze-owned folder or remove→online decisions |
| Commit hygiene | ✅ | conventional messages; the refactor is separate from the features |

## Violations

| # | File | Rule | Metric | Fix |
|---|---|---|---|---|
| 1 | `src-tauri/migrations/20261112*.sql` (4) | Compatibility | Older versions fail to start | Add an ADR with the decision (e.g. ship a patch release with `set_ignore_missing(true)`, or declare no downgrade support). |
| 2 | `docs/DOWNLOADS.md`, `docs/adr/` | Docs | Outdated / missing | Rewrite DOWNLOADS.md for JioSaavn and Kaze-owned downloads; add an ADR "Kaze-owned download folder and remove-to-online". |
| 3 | `src/App.tsx` | Hub wiring only | 2420 lines, 8 smells, complexity 857 | Legacy split (see the backlog): events → `useBackendEvents`, likes → `useLikes`, views → `AppViews`, modals → `AppModals`. |
| 4 | `src-tauri/src/core/processor.rs` | Hub wiring only, ≤500 lines | 3704 lines, 12 smells | Legacy split into `core/processor/<domain>.rs` with routing by domain. |
| 5 | `downloads/service.rs` (575), `playback/queue.rs` (633), `components/NowPlayingBar.tsx` (525), `types.ts` (628) | ≤500 lines | slightly over | Split when next touched (ratchet). |
| 6 | `active_scoring_tests.rs:386` | Tests not flaky | exact float equality | Compare with a tolerance. |

## Legacy backlog (files over 500 lines, from the baseline)

Each file may only shrink. Split them in this order, one commit per file:

| File | Lines | qlty smells |
|---|---|---|
| `src-tauri/src/core/processor.rs` | 3704 | 12 |
| `src/App.tsx` | 2420 | 8 |
| `src-tauri/src/playback/service.rs` | 2335 | 5 |
| `src/components/views/StatsView.tsx` | 1786 | 5 |
| `src/components/views/DiscoveryView.tsx` | 1630 | 2 |
| `src/components/views/PlaylistsView.tsx` | 1539 | 2 |
| `src-tauri/src/recommendations/flow.rs` | 1307 | 6 |
| `src-tauri/src/discovery/coordinator.rs` | 1246 | 6 |
| `src-tauri/src/recommendations/mixes.rs` | 1221 | 4 |
| `src-tauri/src/database/repositories/stats_repo.rs` | 1172 | 2 |
| `src-tauri/src/mpris/service.rs` | 1130 | 1 |
| `src/components/views/OnlineSearchResultsSection.tsx` | 1078 | 3 |
| `src/components/views/CollectionDetailView.tsx` | 1042 | 5 |
| `src-tauri/src/playback/stream.rs` | 960 | 10 |
| `src-tauri/src/cloud/sync_manager.rs` | 866 | 2 |
| `src-tauri/src/providers/jiosaavn.rs` | 704 | 1 |
| `src/hooks/app/useCollections.ts` | 680 | 4 |
| `src/components/views/SettingsView.tsx` | 675 | 4 |
| `src-tauri/src/playback/queue.rs` | 633 | 0 |
| `src/types.ts` | 628 | 0 |
| `src-tauri/src/recommendations/taste.rs` | 621 | 1 |
| `src-tauri/src/database/repositories/track_repo.rs` | 609 | 8 |
| `src-tauri/src/history/service.rs` | 600 | 3 |
| `src-tauri/src/downloads/service.rs` | 575 | 2 |
| `src-tauri/src/database/repositories/playlist_repo.rs` | 556 | 0 |
| `src/components/NowPlayingBar.tsx` | 525 | 0 |
| `src/presentation/SpatialHome.tsx` | 516 | 1 |
| `src-tauri/src/providers/online_catalog.rs` | 510 | 2 |

Another 61 smaller files carry qlty function smells (see `scripts/conventions-baseline.json`).

## Complexity added or removed by the download work

qlty function complexity, base `5bb8619` → `3ddb111`:

| Function / file | Before | After |
|---|---|---|
| `downloads/service.rs` (file) | 67 | 72, plus `service/folder.rs` 59 and `service/removal.rs` (no smells) |
| `DownloadButton` | 48 | no smells (logic in `useDownloadAction`/`downloadActions`, also no smells) |
| `App` | 804 | 857 |
| `execute_query` | 180 | 178 |
| `dispatch_command` | 173 | 175 |
| `useLibraryPage` | 44, 5 params | 47, options object |
| `jiosaavn::start_download` | 20 | 18 |
