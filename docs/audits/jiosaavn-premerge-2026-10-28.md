# JioSaavn pre-merge quality review — 2026-10-28

## Blocker fix follow-up — 2026-10-28

Both findings below are fixed in the local working tree. The original review and its evidence remain below as history; the current implementation status supersedes its hold recommendation for these two findings. No merge, push, production migration or deployment has been performed.

- **P1:** `CloudSong.online_source` transports the exact JioSaavn ID, normalized metadata and link timestamp independently of the original provider pair. Desktop export reads `online_track_sources`; restore validates and inserts the link transactionally with the external song before presentation, preserving existing bindings and many aliases per recording. Source write failures propagate. Physical local tracks are skipped by online placeholder restoration.
- **Cloud boundary:** the Worker now persists original provider fields and the source object in D1, returns the object on pull, and retains existing identity/bindings when older clients omit them. Fresh schema and `worker/migrations/0004_song_recording_source.sql` are supplied. Apply the migration to existing installations before deploying the Worker, then release the desktop client; see `worker/README.md`. Previously lost source links require resync from an intact local library.
- **P2:** the playlist helper checks the existing availability policy before any provider call or catalog writes. Disabled imports return a Settings validation error; absent settings still default to enabled.

Verification: the new desktop source regression failed before correction on missing export, swallowed source-write failure and physical-track metadata overwrite. The disabled-import regression also failed before correction by successfully importing while disabled. After correction, affected Rust library/cloud/JioSaavn suites passed (63 tests including the final added validation regression; one manual D-Bus test ignored). All 15 Node test files, the production frontend build, Worker TypeScript check and whitespace check passed. Worker tests execute real upload/download handlers against SQLite and cover old-schema migration, multiple aliases/canonical rows, repeated uploads, old clients and tenant isolation. Desktop tests cover serialized fresh-database restoration, offline binding, snapshot re-export, playlist/feedback references, existing-binding retention, invalid identity rejection, source-write rollback and physical-file preservation. Playlist tests count local fixture requests and database rows, checking zero requests/writes while disabled and successful re-enable/default behavior.

Verification uses disposable databases/XDG storage and a local playlist fixture, with a silent ALSA test configuration. Restricted execution initially denied the local fixture socket; rerunning the affected Rust suites with local-server permission resolved this environment restriction. No live JioSaavn, production D1 or cross-platform release behavior was certified.

## Original review verdict and scope

Hold the merge until the two findings below are addressed. No implementation fixes were made by this review.

Reviewed `codex/jiosaavn-playback-experiment` at `39dc042643f1645dcfb61a8dbfaaff8a64a73466` against the merge-base with the user-confirmed target `prod` / `origin/prod`, local target `172c5bd1ef3b8f570247f90d4eba39f582aeec46`: 20 commits, 159 changed files. Remote refs were not fetched; this verdict describes the local revisions. The historical JioSaavn plan was interpreted through superseding ADRs 0015–0022 and the experiment's current contracts.

## Standards

### [P1] Converted aliases lose their exact recording during cloud restore

Location: `src-tauri/src/providers/online_catalog.rs:194–198`; affected export: `src-tauri/src/cloud/sync_manager.rs:379–405`.

The branch saves an exact recording in `online_track_sources` while deliberately preserving a legacy alias's original provider pair to support many aliases per source. Cloud export still reads only `external_tracks`, and cloud import does not reconstruct the source-link snapshot. A new device therefore cannot recover the exact selected JioSaavn ID. Presentation must search again, potentially selecting another release or omitting the song when unavailable/offline. The same-device restart coverage does not cover this boundary.

A temporary integration test exercised the real migrated databases, `SyncManager::prepare_local_sync_payload`, `apply_remote_sync_payload`, and `OnlineCatalog::source_id`, without network or audio. It seeded an online alias `itunes:old` linked to `exact_album_version` and gave it a like so it would sync. Before sync, source ID was `Some("exact_album_version")`; exported fields were `provider=online`, `provider_id=itunes:old`; after restore the source ID was `None`. The preservation assertion failed. The test was removed afterward; source and log are retained for this session at `/tmp/kaze-review-alias-sync-repro.rs` and `/tmp/kaze-review-alias-sync.log`.

Required correction: transport and restore the saved source identity/snapshot while keeping stable Kaze IDs and the many-to-one relationship. Cover a converted alias and a separately saved canonical provider row pointing to the same source. Do not resolve this by rewriting aliases into conflicting unique provider pairs. This follows ADR 0016's exact-recording contract and the plan's provider-identity cloud-roundtrip requirement.

No additional confirmed new frontend performance-guardrail violations were found in the inspected changes. Progress remains in leaf subscriptions; changed large lists retain windowing; pointer handling avoids high-frequency React state. This is static inspection plus existing regression checks, not a fresh native performance profile.

## Spec

### [P2] Playlist imports bypass the online-music availability setting

Location: `src-tauri/src/core/processor.rs:2746–2748`, helper at `3344–3352`.

`ImportJioSaavnPlaylist` directly calls `import_jiosaavn_playlist`, which requests `provider_coordinator.jiosaavn.playlist_reference` and persists its results without checking `enable_jiosaavn`. The provider itself does not read that setting. Disabling “Enable online music” therefore still allows this operation to contact JioSaavn and add external catalog records. Search and download handlers already reject the disabled state.

Required correction: use the existing availability check before requesting the playlist, and verify that disabled imports make no provider request or catalog writes. ADR 0017 states that source availability settings are honored. Confirmed by tracing the full handler/helper/provider path; no live playlist was imported during this review.

No other confirmed spec defect was found in the inspected paths. A remaining coverage gap is automatic import of real downloaded audio: the live acceptance gate sets automatic import to false, so it does not establish correct local metadata/source association after import. This is an unverified risk, not a third finding.

## Qlty analysis

Executed the requested command:

```sh
qlty smells --upstream prod --no-snippets --no-upgrade-check --sarif
```

It analyzed 113 files and returned 269 findings. A disposable `git archive prod` baseline analyzed the 76 supported pre-existing files from the same surviving changed-path selection and returned 283 findings. New branch files contribute only to the branch total; deleted files are absent from this comparison. These are scope-limited smell totals, not repository-wide measurements or counts of newly introduced bugs. Qlty's default test exclusions do not cover every repository script/fixture, so these totals include some test helpers.

| Smell | prod baseline | JioSaavn branch |
| --- | ---: | ---: |
| Function complexity | 70 | 72 |
| File complexity | 28 | 33 |
| Similar code | 72 | 67 |
| Identical code | 6 | 6 |
| Boolean logic | 51 | 35 |
| Nested control flow | 27 | 21 |
| Return statements | 15 | 20 |
| Parameter count | 14 | 15 |
| Total | 283 | 269 |

Actionable maintainability concerns, separate from merge-blocking functional defects:

- `App.tsx` function complexity increased from 1,372 to 1,500, about 9.3%. It remains the largest frontend concentration of state, event handling and interactions. Extract coherent event/account/search responsibilities while preserving leaf progress ownership and stable callbacks.
- `CoreProcessor::execute_query` decreased from 336 to 180, but `dispatch_command` increased from 160 to 168, and the new `load_track_artwork` helper scores 65. Source-availability policy should be shared across handlers; the import bypass demonstrates the cost of duplicated checks.
- Qlty also flags the new `OnlineCatalog::bind` at complexity 20. Review its source selection, persistence and presentation boundaries when correcting the cloud contract.

Raw session outputs: `/tmp/kaze-jiosaavn-qlty.sarif` and `/tmp/kaze-prod-qlty.sarif`. Qlty needed permission to create its local log; no Qlty configuration was changed or committed. Existing untracked `.qlty/` and `rea-examples/` were preserved.

## Verification and limits

Passed:

- `npm run build` (TypeScript and production frontend bundle).
- `node --test tests/*.test.mjs`: all 15 test files pass.
- Full Rust library/integration/binary test run: **228 passed, 0 failed, 9 ignored** across 27 result groups, with disposable XDG directories, silent ALSA output and permission for local fixture servers.
- `scripts/test-jiosaavn-ui.mjs`: both Spatial and Brutalist source selection/settings checks pass.
- `scripts/test-buffering-ui.mjs`: both interfaces pass stale-event/state-query and buffering/pause/progress checks.
- `scripts/test-recommendation-feedback-ui.mjs`: both interfaces pass exclusion/Allow again without playback changes.
- `git diff --check prod...HEAD`.

The additional reviewer-written alias-sync regression intentionally fails, demonstrating the P1 finding despite the existing suite passing. Initial restricted Rust attempts encountered read-only credential fallback storage, denied local fixture sockets and inaccessible audio; the final appropriately isolated run passed. These initial environment failures are not counted as product regressions.

Reproduction environment for the successful suite:

```sh
ALSA_CONFIG_PATH=/tmp/kaze-review-asound.conf \
XDG_DATA_HOME=/tmp/kaze-review-test-data2 \
XDG_CACHE_HOME=/tmp/kaze-review-test-cache2 \
XDG_CONFIG_HOME=/tmp/kaze-review-test-config2 \
cargo test --manifest-path src-tauri/Cargo.toml --lib --tests --offline --no-fail-fast
```

The temporary ALSA configuration uses `pcm.!default { type null }`. Cargo's `--offline` prevents dependency fetching, not HTTP inside tests. Optional live/manual gates remained ignored; the live profile migration test must remain opt-in. No fresh regional/provider compatibility certification, audible quality comparison, native WebKit/MPRIS continuity run, or cross-platform release build was performed. The running app/profile was not intentionally changed or restarted. Nothing was merged, pushed, installed or released.

Summary: Standards 1 confirmed finding; Spec 1 confirmed finding. The worst issue is loss of a converted alias's exact source during cloud restore.
