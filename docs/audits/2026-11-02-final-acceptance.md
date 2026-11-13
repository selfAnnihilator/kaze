# Final checkout acceptance — 2026-11-02

Result: **PASSED within the Linux fixture scope below**. No commit, publication, installed-app replacement, public API change or migration. Unrelated work was preserved.

Candidate commit: `5efffb96a34fac6ee5882331cce5e2b4fcc6362b`. The dirty checkout is identified by the source-manifest SHA-256 **`66b379114a094cc660ca9eb62e785ba6cd74c0bea3310f6a5fe5df6eabbf43ab`**, not by the commit alone. Each gate JSON records this fingerprint before and after execution. Tracked files plus relevant untracked build/source/configuration files are hashed; generated evidence, caches, unrelated REA examples and audit-output documents are excluded from the executable candidate. The working-tree patches, untracked source archives, toolchain outputs and per-file manifests are retained.

Embedded native binary SHA-256: `07c1e794b73f9ea72e04e74849cdcfb8d9f705ac09a25a10bed11dbb6dc3707d`. Native tests used copied binaries, disposable profiles/synthetic media and private D-Bus. The duration harness owns a private Xvfb display; other native harnesses exercise Niri and focus/close only their own processes.

## Acceptance gates

All rows link raw logs and command/exit/fingerprint records. The candidate column resolves to the exact complete source fingerprint. A passed quality disposition does not turn its raw findings exit code into zero.

| Gate | Acceptance | Evidence | Candidate |
| --- | --- | --- | --- |
| 01-frontend-build | passed | [log](2026-11-02-final-acceptance-evidence/01-frontend-build.log) · [command/exit](2026-11-02-final-acceptance-evidence/01-frontend-build.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 02-node-tests | passed | [log](2026-11-02-final-acceptance-evidence/02-node-tests.log) · [command/exit](2026-11-02-final-acceptance-evidence/02-node-tests.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 03-request-owners | passed | [log](2026-11-02-final-acceptance-evidence/03-request-owners.log) · [command/exit](2026-11-02-final-acceptance-evidence/03-request-owners.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 04-rust-full | passed | [log](2026-11-02-final-acceptance-evidence/04-rust-full.log) · [command/exit](2026-11-02-final-acceptance-evidence/04-rust-full.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 05-quality | passed — execution and reviewed comparison; raw exit 1 | [log](2026-11-02-final-acceptance-evidence/05-quality.log) · [command/exit](2026-11-02-final-acceptance-evidence/05-quality.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 06-python-qa | passed | [log](2026-11-02-final-acceptance-evidence/06-python-qa.log) · [command/exit](2026-11-02-final-acceptance-evidence/06-python-qa.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 07-python-packaging | passed | [log](2026-11-02-final-acceptance-evidence/07-python-packaging.log) · [command/exit](2026-11-02-final-acceptance-evidence/07-python-packaging.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 08-python-quality | passed | [log](2026-11-02-final-acceptance-evidence/08-python-quality.log) · [command/exit](2026-11-02-final-acceptance-evidence/08-python-quality.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 09-auth-recovery | passed | [log](2026-11-02-final-acceptance-evidence/09-auth-recovery.log) · [command/exit](2026-11-02-final-acceptance-evidence/09-auth-recovery.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 10-browser-five-defects | passed | [log](2026-11-02-final-acceptance-evidence/10-browser-five-defects.log) · [command/exit](2026-11-02-final-acceptance-evidence/10-browser-five-defects.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 11-browser-account-switch-ui | passed | [log](2026-11-02-final-acceptance-evidence/11-browser-account-switch-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/11-browser-account-switch-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 12-browser-qa-remediation | passed | [log](2026-11-02-final-acceptance-evidence/12-browser-qa-remediation.log) · [command/exit](2026-11-02-final-acceptance-evidence/12-browser-qa-remediation.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 13-browser-recovery-ui | passed | [log](2026-11-02-final-acceptance-evidence/13-browser-recovery-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/13-browser-recovery-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 14-browser-song-controls-ui | passed | [log](2026-11-02-final-acceptance-evidence/14-browser-song-controls-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/14-browser-song-controls-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 15-browser-search-flow-controls-ui | passed | [log](2026-11-02-final-acceptance-evidence/15-browser-search-flow-controls-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/15-browser-search-flow-controls-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 16-browser-flow-steering | passed | [log](2026-11-02-final-acceptance-evidence/16-browser-flow-steering.log) · [command/exit](2026-11-02-final-acceptance-evidence/16-browser-flow-steering.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 17-browser-flow-startup-ui | passed | [log](2026-11-02-final-acceptance-evidence/17-browser-flow-startup-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/17-browser-flow-startup-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 18-browser-buffering-ui | passed | [log](2026-11-02-final-acceptance-evidence/18-browser-buffering-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/18-browser-buffering-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 19-browser-player-identity-ui | passed | [log](2026-11-02-final-acceptance-evidence/19-browser-player-identity-ui.log) · [command/exit](2026-11-02-final-acceptance-evidence/19-browser-player-identity-ui.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 20-browser-navigation-memory | passed | [log](2026-11-02-final-acceptance-evidence/20-browser-navigation-memory.log) · [command/exit](2026-11-02-final-acceptance-evidence/20-browser-navigation-memory.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 21-browser-dual-interface | passed | [log](2026-11-02-final-acceptance-evidence/21-browser-dual-interface.log) · [command/exit](2026-11-02-final-acceptance-evidence/21-browser-dual-interface.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 22-browser-brutalist-parity | passed | [log](2026-11-02-final-acceptance-evidence/22-browser-brutalist-parity.log) · [command/exit](2026-11-02-final-acceptance-evidence/22-browser-brutalist-parity.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 23-browser-responsive-layout | passed | [log](2026-11-02-final-acceptance-evidence/23-browser-responsive-layout.log) · [command/exit](2026-11-02-final-acceptance-evidence/23-browser-responsive-layout.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 24-browser-interface-startup | passed | [log](2026-11-02-final-acceptance-evidence/24-browser-interface-startup.log) · [command/exit](2026-11-02-final-acceptance-evidence/24-browser-interface-startup.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 25-native-build | passed | [log](2026-11-02-final-acceptance-evidence/25-native-build.log) · [command/exit](2026-11-02-final-acceptance-evidence/25-native-build.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 26-native-duration-native | passed | [log](2026-11-02-final-acceptance-evidence/26-native-duration-native.log) · [command/exit](2026-11-02-final-acceptance-evidence/26-native-duration-native.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 27-native-playback-restore-native | passed | [log](2026-11-02-final-acceptance-evidence/27-native-playback-restore-native.log) · [command/exit](2026-11-02-final-acceptance-evidence/27-native-playback-restore-native.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 28-native-dual-interface-native | passed | [log](2026-11-02-final-acceptance-evidence/28-native-dual-interface-native.log) · [command/exit](2026-11-02-final-acceptance-evidence/28-native-dual-interface-native.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 29-native-tray-lifecycle | passed | [log](2026-11-02-final-acceptance-evidence/29-native-tray-lifecycle.log) · [command/exit](2026-11-02-final-acceptance-evidence/29-native-tray-lifecycle.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 30-native-single-instance | passed | [log](2026-11-02-final-acceptance-evidence/30-native-single-instance.log) · [command/exit](2026-11-02-final-acceptance-evidence/30-native-single-instance.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 31-native-autostart-native | passed | [log](2026-11-02-final-acceptance-evidence/31-native-autostart-native.log) · [command/exit](2026-11-02-final-acceptance-evidence/31-native-autostart-native.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 32-native-interface-startup-native | passed | [log](2026-11-02-final-acceptance-evidence/32-native-interface-startup-native.log) · [command/exit](2026-11-02-final-acceptance-evidence/32-native-interface-startup-native.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 33-webkit-reduced-startup | passed | [log](2026-11-02-final-acceptance-evidence/33-webkit-reduced-startup.log) · [command/exit](2026-11-02-final-acceptance-evidence/33-webkit-reduced-startup.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |
| 34-render-guardrails | passed | [log](2026-11-02-final-acceptance-evidence/34-render-guardrails.log) · [command/exit](2026-11-02-final-acceptance-evidence/34-render-guardrails.json) | [66b379114a09…](2026-11-02-final-acceptance-evidence/candidate-final.json) |

## Actual totals and scope

- Frontend build and embedded native build: passed. Node runner: **58 passed, 0 failed, 0 skipped**. Request-owner subscription/artwork checks: passed.
- Full Rust command: **263 passed, 0 failed, 11 ignored**. The guarded authentication test passed separately with synthetic credentials and an unavailable credential bus, leaving ten other ignored cases **not run**, not passed.
- Python QA support: **4 passed**; packaging: **7 passed**; quality-gate behavior: **3 passed**.
- **All 14 five-defect browser groups passed together in one execution** on this candidate. All 15 checked-in browser suites in the gate table completed, covering account/paging/playlist recovery, controls/queue/Flow/buffering/recording identity, navigation, both layouts, responsive windows and startup.
- All seven native harnesses completed, including three duration cases, local and bound-provider offline cached restoration, dual-interface continuity, tray/normal Quit, single-instance and startup. This is fixture-based Linux acceptance; no live provider login is inferred.
- Rendering probe passed its existing assertions. Supplemental JSON checks confirm zero App/sidebar/top-search playback and download-tick renders, one backend/update subscription after 20 navigation cycles, and zero lyric resync timers after leaving. Bounded-list assertions exercise 1,036 tracks (fewer than 35 rendered rows) and 201 albums in both styles. The owned Vite server was stopped.

## Quality comparison

Qlty completed with **174 findings** versus the reviewed **162** baseline: no removed baseline findings and **12 new** findings, all explicitly reviewed in `scripts/test-duration-native.py` (one B404 import, six B603 shell-free subprocess calls, five B607 local-tool PATH invocations). These are opted-in disposable test-harness operations, not product input paths. The comparison file records each line and its disposition. No rules, warnings or assertions were suppressed. Independent all-target Clippy and actual emitted scanner invocations succeeded; ripgrep exit 1 is its documented no-match result. Unchanged scanner inputs use Qlty cache. Raw quality exit remains **1 for findings**. See [full comparison](2026-11-02-final-acceptance-evidence/quality-comparison.json) and [SARIF](2026-11-02-final-acceptance-evidence/quality.sarif).

## Initial failures, repairs and reruns

The original frozen candidate was `5b12adc779da61c46bf81e3a41ca8fd0663701ecdc8142b9113fbd888ed9a83c`. Its full Rust gate failed in local library search with `no such table: tracks_fts`; the same failure reproduced in focused repeat 18. A new cancelled-acquisition regression failed before repair with zero FTS schema tables. SQLx documents that cancellation during idle health checks can drop the sole in-memory connection; the metadata worker uses cancellable work. Disabling that health check only for the one-connection ephemeral helper fixes the demonstrated schema-loss pattern. File-backed application pool configuration is unchanged. The regression passes, the original library target passes **20 consecutive final-candidate repeats**, and the complete final Rust/integrated pass was rerun.

The initial scanner run reported 263 findings, including 88 formatting diagnostics and one predictable temporary-output-path diagnostic in the new native harness. AST-preserving formatting/import splitting retains every assertion; the fallback output directory now uses a safely allocated temporary directory. The 12 remaining subprocess diagnostics have explicit fixture dispositions above. No assertion was removed or weakened. The original logs, first manifest/patch/archive and before-fix regression are retained under initial-run (local-only evidence, not committed). The cancellation-before record was conservatively marked invalidated because unrelated Python formatting changed during that Rust-focused run; its failing Rust log is preserved, and only final-candidate passes count toward acceptance.

## Tests not run and open release boundaries


- mpris::service::tests::mpris_private_bus_lifecycle_and_signals ... ignored, requires a private dbus-run-session; run scripts/test-mpris-session.sh
- catalog_playlist_and_exact_download_gate ... ignored, requires live JioSaavn catalog and audio download
- full_track_decoder_and_native_seek_gate ... ignored, requires live JioSaavn network and native audio output
- reported_bruno_mars_song_has_full_provider_audio ... ignored, requires live JioSaavn search and full audio
- saved_catalog_entries_play_the_exact_version_shown ... ignored, requires live JioSaavn and native audio output
- sia_artist_edition_keeps_artwork_and_download_on_the_verified_release ... ignored, requires live artist-edition metadata, cover and full audio
- unified_player_stream_cache_and_identity_gate ... ignored, requires live JioSaavn network and native audio output
- native_paused_stream_seeks_before_completion ... ignored, requires native audio output; run with a disposable ALSA null device
- read_only_live_stats_overview ... ignored, read-only local production audit; requires this workstation's database
- run_migration_on_live_database ... ignored, manual production migration with stale fixture totals; never run against live user data in the test suite

The suspend/resume tray-Quit hang and long-session memory growth remain **open**. Normal Quit and fixture playback do not close either issue. Linux is the acceptance platform. Live-provider authentication, Windows/macOS runtime behavior, installer/updater delivery and a long Wayland memory run remain separate release checks.

Machine-readable [acceptance summary](2026-11-02-final-acceptance-evidence/acceptance-summary.json) and [tool versions](2026-11-02-final-acceptance-evidence/toolchain-host.json) accompany this report.

Wiki checkpoint: `projects/kage.md` and `log.md` updated; validation passed (54 concepts). [Validation log](2026-11-02-final-acceptance-evidence/wiki-validation.log). Evidence hashes: [SHA256SUMS](2026-11-02-final-acceptance-evidence/SHA256SUMS.json).
