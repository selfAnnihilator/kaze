# Convention audit: Android media shutdown (2026-11-26)

Scope: the kaze-media service, plugin lifecycle wiring, Rust Stop mapping,
isolated Android fixture/driver, and `docs/ANDROID_LIFECYCLE.md`.

## Result

The changed production sources pass their scoped convention and qlty checks.
Eight device cases pass on the Realme RMX3471 running Android 14, using a
separate silent QA package and the real service/plugin lifecycle callbacks.
The QA package is removed in cleanup. The main Kaze installation and profile
are not replaced or used by the automated tests.

## Reproduction and repair

Read-only phone diagnostics found an active Kaze media session with playback
state `NONE`. In the original service, applying an empty snapshot left an
active empty session; playing task dismissal retained a foreground service.
Both failures were reproduced with the real service in the isolated fixture
before changing it.

The service now releases native resources on a terminal snapshot and full
dismissal. Plugin finishing/task removal forwards the existing Rust Stop
command and blocks late updates until reopening. Home keeps background audio
active. The service is non-sticky and refuses empty system starts. External
controllers also expose Stop through the shared command route.

## Verification

| Check | Result |
| --- | --- |
| Scoped `npm run audit:conventions` | One changed Rust source in an isolated HEAD snapshot; no failures. Kotlin/plugin sources are outside that script's filter and are checked directly. |
| `qlty smells` | No smells in changed Rust, Kotlin, and Python functions, including the QA Activity. |
| `qlty check` | No issues in changed production sources and the test driver/Activity. |
| Android compile and JVM tests | Plugin and isolated QA APK compile. All nine existing plugin JVM tests pass. |
| Phone regression | All eight cases pass; [retained results](2026-11-26-android-lifecycle-evidence/native.json). |
| Rust media bridge tests | Four passed, including Stop dispatch to the shared backend. |
| Full Rust rerun | One failing target: `scoring_snapshot_tests::flow_candidate_scoring_snapshot`, with the same scoring mismatch as the updater checkpoint. All other targets pass. |
| `cargo clippy --all-targets --offline` | No errors; two existing warnings in the earlier catalog/taste changes. |
| Frontend | 189 Node tests pass; TypeScript and the production Vite build pass. Two mobile startup browser scenarios also pass; painted recovery screenshot inspected. |
| Whole-tree conventions | Still fails on the earlier uncommitted `settings/useTasteArtistPicks.ts` smell, outside this scope. |
| Whitespace | `git diff --check` passes. |

The full Rust rerun confirms the earlier unrelated scoring-snapshot mismatch
recorded in `2026-11-26-conventions.md`. These changes do not modify scoring.

## Checklist

- Structure: media behavior stays in the plugin/service; the shared Rust hub
  gains only a Stop mapping. The fixture has its own application ID and module.
- Compatibility: no new IPC fields, migrations, or schema changes. AppCompat is
  declared directly at the same version already used by the Android application
  because modern Tauri lifecycle callback signatures require it.
- Performance: no App state, progress timers, lists, blur, animations, or design
  values change. Session/focus/wake-lock teardown happens on the Android thread.
- Tests and safety: the driver checks APK package identity before installation,
  uses only synthetic state, captures disconnect errors, and limits cleanup to
  the QA package. Narrow Bandit annotations document argument-vector subprocess
  use by this local device driver; shell execution is explicitly disabled.
- Documentation: domain behavior, reproducible commands, and evidence live next
  to this report. Significant outcomes are captured in the canonical wiki.

## Execution notes

Gradle/Cargo caches disappeared during this session. Dependencies were restored
from official distributions/registries without changing project versions. The
project-pinned Gradle archive was verified against its official SHA-256 and
restored to the normal wrapper cache. The wrapper's version command then passed.
The
fixture needed direct compile dependencies for Android types appearing in
library API signatures. A full notification dump includes historical records;
the test uses `cmd notification list` to assert only active notification keys.
The runner preserves its report even if a device disconnect prevents cleanup.

No release was published and no APK was installed over the main Kaze app.
