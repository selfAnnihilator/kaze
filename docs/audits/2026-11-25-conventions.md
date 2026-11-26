# Convention audit for Android library and likes (2026-11-25)

Range `HEAD` plus working tree.

## Result

The frontend build and all 28 frontend test files passed. The convention audit
found no size, design-token, or migration violations. The complexity check and
on-device verification are pending.

## Automated checks

| Check | Result |
| --- | --- |
| `npm run audit:conventions` | Seven source files checked. No convention failures. qlty unavailable. |
| `npm test` | 28 of 28 test files passed. |
| `npm run build -- --configLoader runner` | TypeScript and Vite build passed. |
| `git diff --check` | Passed. |
| `qlty check` and `qlty smells` | Blocked because qlty cannot initialize its log file on the read-only filesystem. |

A temporary Node preload used writable output files and closed stdin for child
processes where the environment's default pipes failed. Repository test and
audit scripts ran without source edits.

## Checklist

| Area | Status | Evidence |
| --- | --- | --- |
| Structure and limits | Pass | Mobile shell wires `mobileLikeActions.ts`. The playlist predicate lives in `playlistCollection.ts`. The size and style audit passed. |
| Duplication and complexity | Pending | qlty could not initialize. |
| Compatibility and safety | Pass | All application edits are in the mobile presentation. No IPC, SQL, migration, Rust backend, or file operation changes. |
| Regression tests | Pass | `mobileLikeSignIn.test.mjs` checks guest toasts and signed-in forwarding. `mobileLibraryPlaylists.test.mjs` checks smart mix filtering, including a saved copy with the same name. Both use fixtures. |
| Performance | Pass | Mobile playlist rows use `WindowedList`. Playback progress, App state, blur, and motion were untouched. |
| Docs and history | Pass | Android notes updated. No architectural or compatibility decision. No commit created. |
| Wiki checkpoint | Blocked | The canonical wiki is outside the writable workspace. |

## Android taste setup follow-up

Android Music Taste adds song language selection and allows every offered
genre. The shared engine uses profile-scoped language choices and qualified
listening evidence in For You and Flow. The desktop taste interface uses its
existing picker. Run-time ranking follows the original path for profiles with
no language choices.

| Check | Result |
| --- | --- |
| Convention audit | 28 source files checked since HEAD, including the library and likes work. Size, style, and migration checks passed. qlty could not open its log on the read-only filesystem. |
| Frontend | TypeScript and Vite build passed. All 29 frontend test files passed. |
| Rust | 11 tests passed across discovery, taste setup and the new language regressions. The four existing taste setup tests requiring a local listener were blocked by the sandbox and excluded from the successful rerun. |
| Android ARM64 | `cargo check --offline --target aarch64-linux-android -p music-player-backend --lib` passed. |
| Compatibility | Optional IPC field. Setup revision and language changes are saved together in the existing schema. No migration. See ADR 0036. |
| Structure and performance | Language selection and catalog seeding live in feature modules. Catalog searches run at most three at a time. No playback state, animation or desktop UI edits. |
| On-device verification | Installed and launched an in-place ARM64 test build on the Realme RMX3471. Confirmed the first-run screen showed Hindi, Malayalam, and Tamil selected together. |

## Android Discover and artist picker follow-up

| Area | Evidence |
| --- | --- |
| Structure and limits | Catalog queries are handled in `discovery/mobile_catalog.rs`. Provider parsing, artist suggestions and the Android picker use dedicated modules. The size, design-token and migration checks passed. qlty remained unavailable. |
| Compatibility | Added mobile catalog queries with optional refresh, search limit and artist suggestion fields. Returning Android sign-in inserts a skipped taste marker only when the account has no local setup. No schema migration. |
| Performance | Search remains debounced. Regional charts load on selection, artist seeds load in batches of three, and search playlist rows use the existing windowed list. |
| Tests | Frontend suite, Android target check and focused Rust regressions cover language chart parsing, playlist relevance, artist choices and returning-account taste preservation. |
| Live verification | Catalog behavior in this follow-up was checked with fixtures. The on-device run above predates this follow-up. |
