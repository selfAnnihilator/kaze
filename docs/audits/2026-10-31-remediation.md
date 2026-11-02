# Kaze QA remediation — 2026-10-31

Subsequent [quality resolution follow-up](2026-10-31-quality-followup.md) updates the dependency/cache dispositions and quality totals. The measurements and check totals below retain the completed behavioral-remediation checkpoint.

Baseline: [full-app audit](2026-10-31-kaze-qa.md), checkout `5efffb9`. Changes are local, without commits, release publication, database migrations, or personal-profile modifications. Existing `.qlty/` and `rea-examples/` work was preserved; the existing Qlty configuration received explicit Rust version pins.

## Completed behavior changes

| Priority / status | Before | After and evidence |
|---|---|---|
| P1 frontend account isolation — fixed | A delayed old-account playlist response could replace a new account’s private list after SessionChanged | Playlist/membership requests use account and request generations; switching clears old lists, cached private collections and their navigation back stack. Initial account hydration cannot supersede a later session event. Two synthetic accounts reproduce the race before and reject it afterward in both interfaces. Actual live authentication switching remains separately unverified |
| P2 catalogue completeness — fixed | General browsing stopped at 1,000 tracks / 200 albums; fixture ignored page limits | New GetLibraryTracksPage/GetLibraryAlbumsPage return `{items,total}`. First pages contain 200 tracks/100 albums; both virtual presentations load near the visible end, deduplicate IDs, preserve failed-page contents and offer retry. Totals use the same local-file filter as rows, with account-scoped likes and stable title/ID ordering. Existing array APIs remain available |
| P2 playlist creation — fixed | Dialog cleared and closed before command result; rejection lost the name and escaped as an unhandled promise | Callback returns Promise<string/null>; pending guards prevent duplicate submission/dismissal. ID confirms success, null retains draft, rejection shows an accessible error. A failed post-create list refresh keeps the confirmed playlist visible and offers a separate Refresh playlists action; retry never creates a second playlist |
| P2 playlist membership — fixed | Add/remove failures were caught only in the console | Rejection reaches the modal; authoritative membership remains unchanged, error is announced and selecting the playlist retries. If creation succeeds but initial membership fails, retry targets the created playlist rather than creating another |
| P2 folder onboarding recovery — fixed | A rejected CompleteOnboarding closed the setup dialog and emitted an unhandled error | Close occurs after success. Dialog retains folder choices, disables resubmission while pending, announces the error and accepts retry. Controlled permission rejection reproduced both interfaces before the fix |
| P2 interrupted download recovery — fixed | Persisted DOWNLOADING jobs survived provider restart even though the provider’s in-memory transfer no longer existed, blocking normal direct-song retry | First download-list access reconciles missing jobs to FAILED with an interrupted/retry explanation. Conditional status update preserves concurrently finished/cancelled tasks. Audio files are retained. Real Rust service regression demonstrates failure before and successful retry afterward |
| Startup motion contract — repaired | Native reduced-motion repetitions intermittently observed running color transitions during stylesheet arrival | Entire startup subtree permits only transform/opacity transitions; reduced-motion startup disables transitions/animations on the root and descendants. Thirty ephemeral WebKitGTK launches (15/style) pass computed-style and active-animation checks; normal/reduced Chromium startup scenarios pass. No claim that the original fleeting color transition was a material visible defect |

Paged queries read rows and count in one SQLite transaction. General-library rows include physical local paths and exclude online placeholders; likes remain scoped to the current account. Albums require at least one corresponding local row and count only those rows. Paging stores general pages separately from search/album-detail results, rejects obsolete generations and serializes IPC requests per collection. Loaded collections survive ordinary navigation; like/unlike edits update both cached general rows and scoped album rows. Library metadata/scan/account changes invalidate the relevant generation. No audio progress state or timer was added to App.

The folder UI uses text paths, not an OS folder picker. Native picker cancellation is therefore not an existing flow to verify; editing/clearing a path without submitting issues no folder command. The controlled permission case verifies UI command failure, not every filesystem ACL configuration.

Affected code: catalogue query contracts and handlers in `src-tauri/src/core/{query,processor}.rs`, repository snapshot queries in `src-tauri/src/database/repositories/{track,album}_repo.rs`, paging in `src/hooks/useLibraryPage.ts`, `src/App.tsx` and windowed presentations; playlist confirmation/error handling in `src/components/views/PlaylistsView.tsx` and `src/components/modals/AddToPlaylistModal.tsx`; account guards/reset in `src/App.tsx` and `src/hooks/useNavigationHistory.ts`; onboarding in `src/components/OnboardingModal.tsx`; interrupted jobs in `src-tauri/src/downloads/service.rs` and `database/repositories/download_repo.rs`. The named regression scripts trace those user actions through the same callbacks and Rust repositories.

## Reproduction and before/after evidence

The two original defects retain their detailed [baseline reproductions](2026-10-31-kaze-qa.md#ranked-confirmed-defects). The latest [controlled regressions](2026-10-31-remediation-evidence/regressions-final.log) reach every required catalogue boundary and verify retained playlist drafts, cancellation, pending guards, refresh retry without duplicate creation, and both membership failure directions in both interfaces.

Additional confirmed recovery defects were reproduced independently before changing product code:

| Scenario / user impact | Reproduction | Expected versus original behavior | Evidence and regression |
|---|---|---|---|
| P1 account isolation: previous account's private list/collection remains visible | Load synthetic account A, hold its playlist response, open A's private playlist; emit SessionChanged for account B, then release A's response and try Back/Alt-left | Only B's private data and history should be available. Originally A's delayed list could win, and the open private collection/history survived switching | [Delayed-list failure](2026-10-31-remediation-evidence/account-before.log), [private-history failure](2026-10-31-remediation-evidence/private-history-before.log), [both-style passing regression](2026-10-31-remediation-evidence/account-switch-ui-acceptance.log); `scripts/test-account-switch-ui.mjs`. Synthetic accounts, not a live auth-provider claim |
| P2 onboarding: failed save hides the only recovery controls | Submit a folder; delay CompleteOnboarding and reject it with a controlled permission error | Keep the folder and dialog, announce failure and permit retry. Originally the modal closed before rejection | [Before](2026-10-31-remediation-evidence/onboarding-before.log), [after](2026-10-31-remediation-evidence/recovery-ui.log); `scripts/test-recovery-ui.mjs` |
| P2 interrupted transfer: restart leaves a permanently active job | Start a download through Rust's synthetic provider, recreate the service/provider against the same database, list jobs and retry the song | An absent provider job should become failed/retryable. Originally the persisted row stayed DOWNLOADING | [Before](2026-10-31-remediation-evidence/download-restart-before.log), [Rust recovery pass](2026-10-31-remediation-evidence/recovery-rust.log); `interrupted_transfer_becomes_retryable_after_provider_restart` |
| P2 feedback consistency introduced during paging integration | Like a track in exact album view, return to the cached general library, reopen the album | Both visible copies should reflect the same confirmed feedback. The initial paging integration updated only the active copy | [Failed integration reproduction](2026-10-31-remediation-evidence/cached-feedback-before.log), [final both-style pass](2026-10-31-remediation-evidence/regressions-final.log); fixed by updating both caches through a stable callback. This was an integration regression, not a newly established baseline defect |

The playlist-membership fixture rejects AddTrackToPlaylist and RemoveTrackFromPlaylist separately; the modal must keep confirmed membership and show retry feedback. The creation fixture separately rejects, returns null, delays success, and rejects only the subsequent GetPlaylists call. This distinguishes a failed creation from a saved playlist whose refresh failed.

## Verification repairs

- Browser fixture honors pagination and totals. `scripts/test-qa-remediation.mjs` covers track 1,036, album 201, bounded rows, failed subsequent pages, retry, delayed navigation, creation rejection/cancellation/success and both failed add/remove membership operations. Rust paging checks cover empty data, duplicate titles, exact page boundaries, complete pages and account-specific feedback.
- Checked-in playback assertions use authoritative TogglePlayPause while preserving actual playback checks. Album parity scrolling accounts for page growth rather than assuming every record was initially loaded.
- Native restore fixture uses filename-derived metadata for untagged WAVs and a bound synthetic JioSaavn identity with `full:v3:jiosaavn:<id>` cache data. Real offline Rust audio resumes at the saved position through MPRIS.
- Tray harness uses the current Music Folders label and completes lifecycle, crash/relaunch and journal checks.
- Both HTTP fixtures now bound header size, handle partial reads and EOF, and enforce a timeout. Intentional test assertions remain.
- Local rust-toolchain.toml, release jobs, Qlty plugin **and runtime** use verified Rust 1.98.1. Pinning only Qlty’s Clippy plugin still used its old Cargo; the runtime pin is necessary. `scripts/check-quality.sh` requires independent all-target Clippy and detects Qlty’s misclassified failed Clippy execution. Three command-double tests verify that failures cannot be hidden.
- Arch directory changes now stop on failure; seven toolchain tests include a missing-source guard. Its old fixture hardcoded version 2.0.2 and relied on the ignored cd failure; it now derives the recipe version. Generic makepkg-supplied-variable warnings are not blanket-suppressed.

## Security disposition

[Per-input triage](2026-10-31-security-triage.json) retains all original 50 dependency and 20 workflow results, including duplicates. It distinguishes version exposure, maintenance notices, reachable application code, conditional exceptions and unresolved transitive paths. This is targeted advisory triage, not an exhaustive new security scan or exploit validation.

Compatible lockfile patches: Rustls 0.23.46, Quinn-proto 0.11.18, Tokio 1.53.3, rustls-webpki 0.103.16, webpki-roots/webpki-root-certs 1.0.10. Source: individual [RustSec advisories](https://rustsec.org/advisories/RUSTSEC-2026-0285.html), [Tokio advisory](https://rustsec.org/advisories/RUSTSEC-2026-0386.html), and package-specific OSV references in the triage. The initially unmatched certificate CVE was resolved through RUSTSEC-2026-0381/0382/0383 aliases, not assumed to be a false positive.

Worker-only development dependencies use compatible overrides Undici 7.30.1 and Sharp 0.35.5. A fresh npm audit reports zero vulnerabilities and a local Wrangler dry-run bundle passes; no deployment occurred. These packages are development tools, not Worker runtime imports.

Release actions are pinned to verified upstream SHAs, checkouts do not retain credentials, and release tags pass through environment variables and an anchored version allowlist. Rust Clippy is an explicit gate before release builds; publishing was not executed.

Remaining exceptions are explicit:

- **GLib 0.18.5 / RUSTSEC-2024-0429:** affected GTK/WebKit dependency remains. No direct VariantStrIter call was found in application code; transitive reachability remains unresolved. The upstream fixed 0.20 API belongs to a different GTK dependency stack. No incompatible override or speculative vendoring was applied. [Advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html).
- **Six ZIP advisories:** ZIP 4.6.1 is outside the current Linux feature graph and used by the Windows updater after minisign verification. Current application code has no user ZIP import. This narrows exposure; it does not prove every Windows archive path safe. Fixed ZIP 9 requires an upstream-compatible updater change. A compromised trusted release/signing path remains a risk.
- **RSA advisory:** optional sqlx-mysql is present in the lockfile but absent from the shipped SQLite feature graph; no RSA signing/decryption path was established. Reassess if those features change.
- **Five maintenance notices:** paste, proc-macro-error, zbus_names, zvariant and zvariant_derive require upstream support/replacement decisions. They are not converted into demonstrated exploits or suppressed.
- **Four cache warnings:** release workflow accepts version-tag pushes/manual dispatch, not untrusted PR triggers. Build permissions are contents:read; write is scoped to the dependent publish job. No lower-trust cache writer was established. Trusted-runner/cache compromise is not disproven.

## Check results

The [remediation evidence directory](2026-10-31-remediation-evidence/) records final output separately from the historical audit. Counts below distinguish test cases, files, groups and scripts. Incomplete memory runs are excluded from acceptance.

- Frontend production build and 15 Node test files passed.
- Final complete locked/offline Rust suite: **259 passed, zero failed, 11 ignored** (ten existing opt-ins plus the new guarded auth test). The new authentication test was also explicitly run separately: one case passes both idle and absolute expiry, clearing identity, metadata and synthetic fallback token. It uses disposable XDG storage and an unavailable credential bus; it does not validate a live provider token. The ten other opt-ins remain outside this suite's acceptance.
- All-target Clippy completes successfully on Rust 1.98.1; pre-existing warnings are retained rather than mass-formatted/suppressed.
- Checked-in native [tray](2026-10-31-remediation-evidence/native-tray.log), [cached restoration](2026-10-31-remediation-evidence/native-restoration.log), [dual-interface](2026-10-31-remediation-evidence/native-dual-interface.log) and [startup](2026-10-31-remediation-evidence/native-startup.log) harnesses all pass on the final rebuilt embedded app. No temporary corrected copies are used.
- Browser integration run initially had three layout failures. Stable reruns of responsive and Spatial pass unchanged after development hot reloads stopped; Brutalist’s album scroll test was adapted to pagination and passes. Original playback harnesses pass their checked-in corrected assertions.
- Render probe confirms zero App/Sidebar/top-search renders across 100 playback ticks per style and download ticks, stable subscriptions, bounded list DOM and lyrics timer cleanup.
- Reduced-motion WebKit: thirty launch repetitions pass; browser normal/reduced startup suite passes.
- Worker audit: zero vulnerabilities; dry-run bundle passes. Seven packaging and three quality-gate tests pass.

The final whole-repository quality gate exits **1 for reported findings**, with successful independent all-target Clippy and a successful Qlty Clippy invocation (actual exit 0). It is a completed scan, not a clean quality gate. Existing configured exclusions still apply. [Summary](2026-10-31-remediation-evidence/quality-summary.json) and [SARIF](2026-10-31-remediation-evidence/quality-complete.sarif) retain the results.

| Source | Error | Warning | Note | Total | Disposition |
|---|---:|---:|---:|---:|---|
| Bandit | 337 | 0 | 0 | 337 | Primarily test assertions and controlled subprocess/server harnesses; assertions remain intentional. Static findings do not establish user-facing defects |
| Ruff | 0 | 222 | 0 | 222 | Mostly compact Python harness style; no mass formatting |
| Clippy | 0 | 48 | 0 | 48 | Execution succeeds; remaining warnings are retained for focused follow-up |
| Python Radar | 26 | 3 | 2 | 31 | Complexity, literals and harness maintainability follow-up |
| OSV | 0 | 14 | 0 | 14 | Individually triaged upstream/conditional dependency exceptions above |
| Shellcheck | 0 | 11 | 0 | 11 | Remaining packaging-variable/static shell findings; actionable source-directory failures repaired |
| Zizmor | 4 | 0 | 0 | 4 | Release cache exceptions above |
| **Total** | **367** | **298** | **2** | **667** | Severity is the tool's classification, not a confirmed product-bug ranking |

Live provider/account credentials, physical media keys, non-Linux runtimes, real disk-full hardware behavior and native allocation ownership remain outside the controlled acceptance evidence. Synthetic cached playback and controlled write failures are separately labeled. No live-account pass is inferred from fixtures.

## Native memory investigation

The original audit’s private-memory increase remains the baseline risk. The first attempted run was invalidated when a second inspector connection closed the measuring connection; another short run was stopped to refresh embedded assets. Neither is a passing soak or evidence of a memory fix. The repaired harness uses one inspector, queues sufficient silent media before measuring, warms up, samples renderer/app private memory plus RSS and swap, and cycles playback, navigation, style and tray visibility.

The completed acceptance run records **1,800 seconds after a 60-second warm-up, 60 samples, one unchanged renderer PID, uninterrupted MPRIS Playing**, navigation/style changes and actual tray hide/restore. Rust naturally advanced from t0 at 60.225s to t1 at 64.845s by the endpoint. [Raw measurements](2026-10-31-remediation-evidence/native-soak-complete.json), [supervisor completion](2026-10-31-remediation-evidence/native-acceptance-status.json) and [memory summary](2026-10-31-remediation-evidence/native-memory-summary.json) establish run completion; they do not establish flat memory.

| Measurement | Warmed start | Restored Spatial Home endpoint | Change |
|---|---:|---:|---:|
| Renderer private resident | 134.31 MiB | 198.49 MiB | **+64.18 MiB** |
| Renderer recorded swap | 0.00 MiB | 36.91 MiB | +36.91 MiB |
| All application private resident | 320.91 MiB | 239.59 MiB | -81.32 MiB |
| All application recorded swap | 19.92 MiB | 139.25 MiB | +119.33 MiB |

![Native private resident and swap measurements](2026-10-31-remediation-evidence/native-memory.png)

Renderer growth persists in equivalent Spatial Home cycles: 157.93 MiB private/15.75 MiB swap at 721s versus 175.69/31.43 at 1,442s. Swap includes process accounting effects and is not an exact measure of private retained allocations. The lower total resident endpoint must not be interpreted as a fix. Endpoint global listener counts are identical, tracked pending timers/RAF/intervals are zero, and the visible DOM remains bounded (594 versus 602 nodes; 14 versus 13 images after the natural song change). Those facts do not rule out native retention.

**Disposition: unresolved performance risk, independently reproduced growth; no demonstrated allocation owner or memory fix.** The fresh-process tick/heap diagnostic on the final build also grows, without swap in that probe. [Diagnostic data](2026-10-31-remediation-evidence/memory-ownership-diagnostic.json), [log](2026-10-31-remediation-evidence/memory-ownership-diagnostic.log), [summary](2026-10-31-remediation-evidence/memory-diagnostic-summary.json) and [final verification completion](2026-10-31-remediation-evidence/final-verification-status.json) retain the independent repeat.

| Fresh-process phase | Post-GC live JavaScript | Live nodes | Renderer private resident |
|---|---:|---:|---:|
| Warm | 5.338 MiB | 37,160 | 140.48 MiB |
| 5,000 profiled ticks | 4.576 MiB | 36,245 | 202.39 MiB |
| Further 5,000 ticks, counters disabled | 4.693 MiB | 36,236 | 236.66 MiB |
| Further 5,000 ticks, static seek background | 4.769 MiB | 36,256 | 268.66 MiB |

No growing live JavaScript object graph is demonstrated. Disabling counters or making the seek background static does not stop native resident growth in this ordered screen, so neither is established as its sole cause. Remaining instrumentation, inspector/snapshot allocations and native caches still confound attribution. This repeats the narrower growth signal independently; it is not another 30-minute playback run. Heaptrack, Valgrind and perf executables are unavailable here. Native allocation attribution or an inspector-free equivalent workload remains follow-up work; no speculative ownership rewrite was made.

The soak's copied binary contains the completed paging/account/download/backend, startup and cache-feedback changes. The later Refresh playlists action is outside the measured Home/Discovery/Tracks routes; final native UI checks use the rebuilt final bundle. A detached preliminary attempt ran 17.5 minutes before a harness-only scheduled-hide assumption failed because the window was already hidden; it is excluded from acceptance. Another supervised process ended without a completion record. The repaired harness tolerates pre-hidden state, checks renderer identity throughout, and runs under an independent bounded supervisor. The final completed log ends with a session-bus teardown message after the PASS record; supervisor exit is 0 and no continuity assertion failed.

Memory hypotheses, ranked before the focused probe: (1) navigation/paint caches warm and plateau during repeated equivalent phases; (2) test render/mutation instrumentation retains data, so disabling counters reduces growth; (3) changing native seek-bar painting resources account for growth with a stable JavaScript heap, so a static seek background reduces it; (4) application JavaScript retains objects, so post-GC heap bytes/object classes keep rising across tick bursts. The opt-in diagnostic samples heap classes after collection, then screens successive 5,000-tick bursts with profiling enabled, counters disabled, and a static seek background. These ordered phases are affected by cache warm-up and do not independently establish causality. A matching fresh-process repeat is required before fixing a suggested owner. Synthetic diagnostic ticks are separate from the soak's actual Rust playback.

## Repeatable checks

Run from the repository root with installed dependencies. Native GUI harnesses need the workstation's WebKitGTK/GI, Niri and a private D-Bus; use only copied embedded binaries and synthetic profiles. Browser checks use mocked IPC unless explicitly described otherwise.

```sh
npm run build
node --test tests/*.test.mjs
scripts/check-quality.sh --no-upgrade-check --sarif
python3 scripts/tests/test-quality-gate.py
python3 scripts/tests/test-arch-toolchain.py
KAZE_TEST_URL=http://127.0.0.1:5194 node scripts/test-qa-remediation.mjs
KAZE_TEST_URL=http://127.0.0.1:5194 node scripts/test-account-switch-ui.mjs
KAZE_TEST_URL=http://127.0.0.1:5194 node scripts/test-recovery-ui.mjs
KAZE_TEST_URL=http://127.0.0.1:5194 KAZE_STARTUP_REPEATS=15 python3 scripts/test-startup-reduced-webkit.py
```

The browser scripts accept `KAZE_PLAYWRIGHT_MODULE` when Playwright is installed outside the project; they default to the portable package import. Start the local fixture server on the stated port. Rust tests/builds use `CARGO_BUILD_JOBS=1`, locked dependencies, and disposable XDG data/cache/config paths. The opt-in auth case additionally requires `KAZE_PRIVATE_AUTH_TEST=1` and an unavailable private credential bus; do not point it at a personal credential store. The native soak command is `KAZE_PRIVATE_PERF_TEST=1 KAZE_ASSERT_PERFORMANCE=1 dbus-run-session -- python3 scripts/profile-frontend-native.py COPIED_BINARY OUTPUT.json --soak-only --soak-seconds 1800`. Do not connect a second inspector during measurement.
