# Convention audit: Android update restart prompt (2026-11-26)

Scope: restart permission preflight, native dialog, bridge, regression tests,
and `docs/ANDROID_UPDATES.md`. The earlier Android library, taste and catalog
working-tree changes are excluded from this fix.

## Result

The scoped audit, changed-function smell checks, qlty checks, Android plugin
compilation and tests, frontend tests, TypeScript, and Android ARM64 clippy pass.
The whole working tree has an unrelated artist-picker convention failure and
one Rust recommendation-scoring snapshot mismatch. No phone installation or
live update was performed.

## Automated checks

| Check | Result |
| --- | --- |
| Scoped `npm run audit:conventions` | Pass. HEAD snapshot with only the changed Android updater source: one audited source file, no failures. Plugin sources are outside this script's path filter and were checked directly with qlty. |
| Whole-tree `npm run audit:conventions` | 51 source files checked; fails on one smell in the pre-existing uncommitted `src/presentation/mobile/settings/useTasteArtistPicks.ts`. That file is untouched by this fix. |
| `qlty smells` on changed Rust and Kotlin files | Pass, no smells. |
| `qlty check` on changed Rust and Kotlin files | Pass, no issues. |
| `npm test` | All 186 tests pass. New pre-download/native-dialog wiring regressions failed before implementation. |
| `npx tsc --noEmit -p .` | Pass. |
| `:tauri-plugin-kaze-media:testDebugUnitTest --offline --no-daemon` | Native plugin compiles; nine JVM tests pass, including three new permission-choice tests. |
| `cargo clippy --all-targets --offline` | Pass with two warnings in the earlier uncommitted catalog/taste code. |
| Android ARM64 `cargo clippy --offline --target aarch64-linux-android -p music-player-backend --lib` | Pass with one warning in the earlier catalog code; isolated `target/android-adb-check` output directory and NDK 28 wrappers. |
| `cargo test --jobs 1 --offline --no-fail-fast` outside sandbox | One failing target: `scoring_snapshot_tests::flow_candidate_scoring_snapshot`, with changed recommendation score expectations. All other targets pass. This fix changes no scoring code, and its backend operation is Android-only. |
| `git diff --check` | Pass. |
| Wiki validation | Pass, 60 concepts. |

Initial sandbox runs blocked Gradle's cache locks, qlty's log, Node Git
subprocesses, and Rust test fixture listeners/network. Reruns outside the
sandbox resolved those environment failures. No project test assertions or
quality rules were disabled.

## Checklist

- Structure: native UI is in `UpdateRestartPrompt.kt`; the testable choice is
  in `UpdateRestartChoice.kt`; Rust calls a plugin preflight before downloading.
- Limits: every changed production file is below 500 lines, and qlty reports
  no smells in changed Rust/Kotlin functions.
- Compatibility: no existing IPC fields or SQL schema changed. The existing
  `kaze_update.overlay_asked` key keeps prior answers across upgrades.
- Safety: permission refusal or an unavailable Settings activity still allows
  an update; only the user can grant permission. APK hashing and installation
  permission checks are unchanged.
- Regression coverage: granted/unknown/answered states, answer persistence
  before routing, duplicate callbacks, dialog dismissal and pre-download wiring.
  Tests use synthetic state and source fixtures, never an app profile.
- Performance: no new renderer state, timers, lists, blur, custom style literals,
  or playback-progress updates. Native dialog actions run on the UI thread;
  the waiting plugin call runs on Rust's blocking pool.
- Documentation: domain behavior is in `docs/ANDROID_UPDATES.md`; the canonical
  wiki records the decision and verification limits. No migration or
  architectural compatibility ADR is needed.

## Remaining verification

Exercise OK, refusal, Settings return, remembered refusal, and granted access
on a phone using an isolated test installation. A shipped old APK must receive
this fix before its own updater can display the new dialog.
