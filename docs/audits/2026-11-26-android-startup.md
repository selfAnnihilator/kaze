# Convention audit: phone startup recovery (2026-11-26)

Scope: `src/main.tsx`, `src/services/startupSettings.ts`, their unit/browser
regressions, and `docs/ANDROID_STARTUP.md`. Earlier uncommitted library, taste,
and catalog changes are excluded.

## Result

The scoped conventions audit passes for both production sources. The entrypoint
shrinks; retry behavior is isolated in its own service module. No root App state,
SQL, IPC fields, presentation tokens, or playback behavior changes.

## Checks

| Check | Result |
| --- | --- |
| Scoped `npm run audit:conventions` | Two source files, no failures; isolated HEAD snapshot with only startup changes. |
| `qlty smells` | No smells in the changed production functions. |
| `npm test` | 189 passed, including three new entrypoint tests. The two recovery tests failed before the change. |
| TypeScript | `tsc --noEmit -p .` passes. |
| Browser regression | Automatic recovery after two failed reads, plus bounded failure/manual retry, both pass with the real mobile App. |
| Real phone | An installed 2.2.2 launch succeeded; the intermittent failure's original bridge/database error remains unknown. No production profile was modified for tests. |

The entire working tree's convention audit still finds a smell in the earlier
uncommitted `settings/useTasteArtistPicks.ts`. The startup files do not add a
violation. Full Rust-suite and unrelated scoring-snapshot results from the
updater checkpoint are recorded in `2026-11-26-conventions.md`; this startup fix
has no Rust changes.

Startup failures remain visible after bounded retries. Settings are neither
guessed nor overwritten; the app never starts in a default theme while waiting.
The active attempt's cancellation guard still prevents a late settings response
from reviving startup after an App-code loading failure.
