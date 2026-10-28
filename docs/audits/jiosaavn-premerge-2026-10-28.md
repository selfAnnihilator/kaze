# JioSaavn pre-merge quality review — 2026-10-28

Current review status: all identified code defects are independently verified fixed at `3594e2c`; no remaining confirmed findings in the reviewed correction. See the final independent recheck below. Production rollout remains pending; earlier findings remain dated evidence.

## Independent final recheck at `3594e2c` — 2026-10-28

No remaining confirmed actionable findings in the collision correction. The earlier review hold is cleared for the identified code defects. This is an assessment of the local branch and the reviewed follow-up changes, not certification of a production deployment or every possible runtime behavior.

The exact reviewer reproduction that previously failed at `1fad2f4` now passes unchanged. A temporary integration test was created from `/tmp/kaze-review-cloud-collision-repro.rs`, run against the current implementation, then removed. Fresh log: `/tmp/kaze-collision-recheck/original-repro.log` (one passed). The new permanent tests cover both directions and payload orders, multiple legacy identities, repeated old/canonical payloads, absent and conflicting snapshots, offline rebinding/re-export, stable playlist/statistics/history references and rollback of source/alias/canonical write failures. Independent standards and spec inspections found no additional defect in `cloud/song_restore.rs`.

Fresh full Rust verification passed: **242 tests passed, zero failed, nine ignored**, across 28 result groups. Command: `cargo test --manifest-path src-tauri/Cargo.toml --lib --tests --offline --no-fail-fast` with disposable XDG directories under `/tmp/kaze-collision-recheck`, null ALSA output and local fixture-server permission. Log: `/tmp/kaze-collision-recheck/rust.log`. The additional original reproduction passed separately and is not included in 242. Branch whitespace validation also passes.

Qlty rerun against prod analyzed 120 files and returned **276 smells**, unchanged from the previous 276/118 scope. Restore orchestration complexity decreased from 44 to 39 after extraction; root App complexity remains 1500. Raw SARIF: `/tmp/kaze-collision-recheck-qlty.sarif`. Smells remain maintainability work, not newly confirmed merge blockers. No frontend or Worker implementation changed in this correction; their passing previous-recheck build/tests/typecheck evidence remains applicable, but was not rerun or represented as fresh. No live provider, native GUI, production D1 or release checks were performed.

Release sequencing still applies: migrate existing D1 databases, deploy the updated Worker, then distribute the desktop client, per `worker/README.md`. Nothing was merged, pushed, migrated, installed or deployed by this recheck. Original dated failures and prior hold recommendations below remain historical evidence.

## Independent recheck at `1fad2f4` — 2026-10-28

The original P1 source-snapshot omission and P2 disabled-import bypass are corrected in the covered cases. One additional P2 regression remains; recommendation: fix it before merging into `prod`. This is a review of `1fad2f4` against previous reviewed `39dc042`, with a refreshed branch-wide Qlty scan against local `prod`. No implementation fixes or deployments were made during this recheck.

### [P2] Legacy provider identities can abort cloud synchronization

Location: `src-tauri/src/cloud/sync_manager.rs:672–696`, specifically the newly propagated insert error at line 696.

A supported non-prefixed saved row can have `provider=jiosaavn`, `provider_id=song`, and stable Kaze ID `legacy:jio-song`. Another device can independently save the canonical `jiosaavn:song` row. The cloud Worker keys songs by Kaze ID/account and can hold both; desktop `external_tracks` additionally enforces unique `(provider, provider_id)`. Restore handles only ID conflicts and now propagates the provider-pair conflict, aborting sync before playlists/statistics are reconciled. Earlier code swallowed this insert error; that was not a complete alias solution, but it did not abort the entire sync at this point.

A temporary real Rust integration test seeded the legacy row plus a valid source snapshot on one migrated in-memory database, exported through `SyncManager`, and restored into another migrated database already holding the canonical row. `apply_remote_sync_payload` failed with SQLite code 2067: `UNIQUE constraint failed: external_tracks.provider, external_tracks.provider_id`. No network/audio was used. The test was removed afterward; session evidence is `/tmp/kaze-review-cloud-collision-repro.rs` and `/tmp/kaze-review-cloud-collision.log`.

Required correction: reconcile a provider-pair collision into the supported stable-alias/source-link representation while retaining playlist/history/statistics references. Continue propagating genuine persistence failures. Add coverage for both import orders with a non-prefixed `provider=jiosaavn` identity and a canonical row sharing one source. Current new round-trip fixtures use `provider=online` for their aliases, so they do not exercise this case. The historical non-prefixed identities are recognized by `ProviderCoordinator::persist` and documented in ADR 0016/project source-link history.

### Recheck verification

- Frontend production build passes; all 15 Node test files pass, including six actual Worker handler tests.
- Full Rust lib/tests/binaries pass: **235 passed, zero failed, nine ignored**, across 28 result groups, with disposable XDG storage, silent ALSA and permission for local fixture servers. The additional reviewer collision regression fails independently as described above.
- Worker TypeScript check passes against installed Workers types. Latest Cloudflare best-practices reference was consulted; latest types package retrieval was unavailable in restricted execution, so installed types were used.
- `git diff --check prod...HEAD` passes. No frontend implementation changed in this fix, so prior browser evidence was not rerun or represented as fresh.
- Updated `qlty smells --upstream prod --no-snippets --no-upgrade-check --sarif`: **276 findings across 118 analyzed files**, versus 269/113 at the previous review. Categories: file complexity 34, function complexity 73, returns 22, boolean logic 35, nesting 23, parameter count 15, similar code 68, identical code 6. Added regression files contribute to the changed scope; the increase is not seven confirmed bugs. App complexity remains 1500; restore-function complexity increased from 39 to 44. Raw output: `/tmp/kaze-jiosaavn-recheck-qlty.sarif`.

The server rollout remains a release prerequisite: apply `worker/migrations/0004_song_recording_source.sql` once to an existing D1 schema, deploy the updated Worker, then distribute the desktop client. This documented prerequisite is separate from the remaining code defect. No live D1/provider validation, production migration, deployment or release was performed. Cloud-only backups that already lost their source snapshots require an upload from an intact device.

Standards recheck: one confirmed P2 regression. Spec recheck: no remaining confirmed violation beyond the standards finding; disabled-import behavior and Worker transport pass their new regressions. The two original findings are closed in code, with production rollout pending.

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

## Collision reconciliation follow-up — 2026-10-28

Corrected the additional P2 in the local working tree. The pre-existing independent recheck above is preserved. No commit, merge, push, database migration or deployment was performed for this correction.

Desktop cloud ingestion now delegates to a private `cloud/song_restore.rs` helper. Within one transaction it normalizes incoming noncanonical JioSaavn identities to `online` / stable Kaze ID, preserves their exact source through `online_track_sources`, and moves any existing noncanonical owner out of a canonical provider pair before inserting the canonical row. Existing saved sources remain authoritative. Missing snapshots are constructed from explicit saved provider IDs and metadata, including local language/year when available, with the reconciliation timestamp. A complete incoming snapshot for the same local ID/source is retained when no local snapshot exists. No provider requests or title matching occur.

Canonical IDs in older payloads labelled cloud/online or missing the provider label are also reconciled before `ensure_online_track` can promote them. Alias conversion, source persistence and the incoming external insert share the transaction; genuine failures propagate and roll back the changes. Subsequent registration and repeated cloud payloads retain both stable IDs and their references. Physical local tracks remain excluded. The existing cloud format and schema are unchanged; no additional Worker migration/deployment is introduced by this correction. The prior source-sync rollout prerequisite remains.

Before correction, the permanent two-device and multi-alias regressions failed with SQLite 2067 on `external_tracks.provider, external_tracks.provider_id`. After correction, **70 affected Rust tests passed, zero failed, one manual D-Bus test ignored**: library 39, cloud recording 12, cloud sync 4, JioSaavn 15. Command: `cargo test --manifest-path src-tauri/Cargo.toml --lib --test cloud_recording_tests --test cloud_sync_tests --test jiosaavn_tests --offline`, using disposable XDG storage, silent ALSA and permission for the existing local fixtures. Whitespace check passed. Logs: `/tmp/kaze-collision-before.log` and `/tmp/kaze-collision-rust-final.log`.

The twelve recording tests include the original five checks plus cross-device restore in either direction, fresh payloads in either order, multiple noncanonical provider rows, missing/present snapshots, offline exact binding and serialized re-export, repeated registration/sync, playlist/feedback/statistics continuity and existing history preservation. They also cover matching incoming full snapshots, conflicting pre-existing selections, fallback metadata/timestamps, and injected source/alias/canonical write failures with rollback. Test fixture playlists use distinct names to obey the existing per-account name constraint; the initial same-name fixture failure was unrelated to recording identity. Frontend/Worker source is unchanged and prior build/Worker/browser checks were not represented as fresh runs. No live cloud/provider or cross-platform runtime certification was performed.

## Production server rollout — 2026-10-28

Completed the user-authorized D1 migration and Worker deployment after `3594e2c`. This supersedes earlier pending-server-rollout statements as dated history; desktop distribution remains separate.

The configured production database lacked all three source columns. Applied the three ALTER statements in `worker/migrations/0004_song_recording_source.sql`, verified `provider`, `provider_id`, `online_source_json` as TEXT and an unchanged 375-song baseline. Saved the pre-migration recovery bookmark before execution. Do not reapply this migration to the now-upgraded database.

Worker version `06cb1863-202b-4de9-b7cb-e7923473434e` is active at 100% in deployment `b2144eef-83e2-429d-bc17-3019ab216a2a`, published at `2026-10-28T13:10:31Z`, at [the production endpoint](https://soundflow-cloud-worker.abhi-atlas-2026.workers.dev). Existing D1 and Cloudinary secret bindings are unchanged; deployment preserved environment variables with `--keep-vars`. Prior Worker version `733b9dba-1bf2-467c-965f-5141c6951cae` is retained for code rollback, with the additive schema compatible.

Fresh checks: actual Worker/SQLite test file, Worker TypeScript check, deployment dry run, remote schema/count, active version and binding checks. Live isolated verification passed health, account/session creation, authenticated POST/GET source-snapshot roundtrip for three stable IDs sharing a recording and a subsequent older-client upload without the source object. Exact provider identities and snapshots survived. Deleted the temporary user and verified its account, song and session counts are zero. No existing account data or credentials were used for the live fixture. An initial Python health probe returned 403; curl and a probe using Kaze's user agent returned 200, and the full authenticated probe using that user agent passed. The initial 403 cause is unconfirmed; no access/security configuration was changed.

Sanitized operational outcomes are in the canonical wiki. Session-local schema/deployment/verification artifacts use `/tmp/kaze-cloud-rollout-*`; they are not release artifacts or stored credentials. No desktop distribution, Git push or merge was performed.
