# Kaze project conventions

These rules apply to every change, by people and agents alike. They exist so that a
new feature never has to re-decide where code goes, and so that complexity can only
go down. Check a change with `npm run audit:conventions` and the checklist in
[CONVENTIONS_AUDIT.md](CONVENTIONS_AUDIT.md). The performance and playback rules in
[AGENTS.md](../AGENTS.md) still apply on top of these.

## 1. One feature, one module

A new feature gets its **own file(s)**. Shared hubs only *wire* it in, with a few lines each:
`src/App.tsx`, `src-tauri/src/core/processor/` (`mod.rs` and `routing.rs`), `src/types.ts`,
`core/command.rs`, `core/query.rs` and `core/event.rs`.

**Backend (Rust)**
- Put a domain feature in `src-tauri/src/<domain>/<feature>.rs`.
- Add behaviour to a large service as a **child module** `src-tauri/src/<domain>/<service>/<feature>.rs`. It starts with `use super::*;` and keeps its own `impl <Service>` block, so it can use the service's private fields. Make methods shared between child modules `pub(super)`. Example: `downloads/service/folder.rs` and `downloads/service/removal.rs`.
- **New IPC command or query**: add the variant to `core/command.rs` / `core/query.rs`, assign it a domain in `core/processor/routing.rs` (the match is exhaustive, so the compiler asks), and handle it in `core/processor/<domain>.rs` (`playback`, `library`, `collections`, `downloads`, `account`, `sessions`, `profile`, `cloud`, `recommendations`, `feedback`, `discovery`, `stats`, `settings`). A new domain gets a new file. Shared processor helpers sit next to their domain; construction and lifecycle live in `construction.rs` and `lifecycle.rs`.
- Database access lives in a repository (`database/repositories/<name>_repo.rs`), and SQL stays there.
- Shaping data for the UI gets its own module, for example `downloads/rows.rs`, not code inline in the processor.

**Frontend (React/TS)**

| Concern | Location | Example |
|---|---|---|
| IPC calls and side effects | `src/services/<feature>Actions.ts` | `downloadActions.ts` |
| Live/shared store | `src/services/<feature>.ts` (subscribe + `useSyncExternalStore`) | `downloadProgress.ts`, `confirmDialog.ts` |
| Component state and behaviour | `src/hooks/use<Feature>.ts`, deriving state in an exported pure function | `useDownloadAction.ts` (`describeDownload`) |
| UI | `src/components/<Feature>.tsx`, render only | `DownloadButton.tsx` |
| App wiring | `src/hooks/app/use<Feature>.ts`, called once from `App.tsx` | `useDownloadsSection.ts` |
| Modal / popup | `src/components/modals/<Name>Modal.tsx`, opened through a store, not App state | `ConfirmModal.tsx` |

## 2. Size and complexity limits

- A **file** has at most **500 lines**.
- A **function** has at most about 60 lines and **at most 5 parameters**. Use an options object when it needs more (`useLibraryPage(query, { size, identity, navigation, scope })`).
- **No qlty smell** on new or changed functions: no high complexity, many returns or deep nesting (`qlty smells`). Split long decision chains into a pure function or lookup tables (for example `LABELS` in `DownloadButton.tsx`).
- **Legacy files** listed in `scripts/conventions-baseline.json` are allowed above these limits, but they **may only shrink**. The audit fails if one grows in lines or smells. After you split a legacy file, record the files the legacy code moved into with `npm run audit:conventions -- --record <files>`. This also drops entries for deleted files. Use it only for moved legacy code, never for new feature code. Never raise a limit by hand.
- No duplicated blocks (qlty duplication). Extract a shared helper or component instead.

### Splitting an oversized file

Use these patterns. Each split is a separate `refactor(…)` commit that changes no behaviour.

**React view:**
- Move each section into `src/components/views/<view>/<Section>.tsx`.
- Sections read one typed `<View>Context` object (`<view>/context.ts`). The view builds it once, right before its `return`.
  Examples: `stats/`, `discovery/`, `playlists/`, `searchResults/`, `collection/`, `layout/appViews/`.
- Narrowing does not carry over into a section's callbacks:
  - copy the value into a local `const` (`const topResult = c.topResult;`);
  - or return `null` early (as `PlaylistDetailsModal` does);
  - or call with `?.` inside a guard that already exists.
- Move module-level subcomponents (cards, rows) into their own files.
- A route switch becomes a renderer table, with a fragment keyed by the route. This keeps the remount-on-change behaviour.
- Source-text tests (`tests/*.test.mjs`) must follow code that moves. Point them at the new file, or read the whole section folder.

**Rust:**
- Move method groups into child modules `<module>/<group>.rs`, each with `use super::*;` and its own `impl`. Methods shared between modules become `pub(super)`.
- Move public types to `<module>/types.rs`, then `pub use types::*;` so existing paths keep working.
- Moved code is one module deeper. A `super::x` path becomes `super::super::x`.
- Inline tests become `<module>/tests.rs`, declared with `#[cfg(test)] mod tests;`. Their paths don't change.
- A trait impl can't be split across files. Keep thin trait methods that call inherent `<name>_query` methods in child modules (`stats_repo/`).
  - Never give the inherent method the trait method's name: callers outside the module would resolve to it and fail the privacy check.
- zbus `#[interface]` methods may be `pub(super)`. Code that calls them must sit in the same module or a child module (`mpris/service/player.rs`).

A single function over 500 lines (`flow/candidates.rs`, `coordinator/for_you.rs`) stays in the ratchet until its logic is refactored. Moving it to another file is not enough.

## 3. Interfaces and compatibility

- New IPC fields (`Command` / `Query` / events) are optional with `#[serde(default)]`, so older frontends and saved state keep working. Add new commands; don't change existing ones.
- **Migrations are additive only** ([ADR 0031](adr/0031-additive-migrations-and-downgrade.md)): `CREATE … IF NOT EXISTS` and `ADD COLUMN`; never drop, rename or repurpose. Kaze ignores migrations from newer versions and backs up the profile before migrating. Every new migration still gets a line in ADR 0031's migration log, and the count in `src-tauri/tests/download_folder_tests.rs` must be updated; `audit:conventions` flags it so that this is deliberate.
- Keep the schema stable. Derive values from data you already have (for example, the Local/Downloads split comes from path prefixes).
- Files Kaze didn't create are never deleted or moved. Only files in the Kaze download folder are.

## 4. Tests

- Every feature has tests next to its domain:
  - `src-tauri/tests/<feature>_tests.rs` for backend behaviour;
  - `tests/<feature>.test.mjs` for frontend logic. Export pure functions so they can be tested directly.
- Tests use temporary directories and in-memory databases, **never the real profile**. Tests that log in call `cloud::credentials::use_isolated_store()`.
- A bug fix comes with a test that fails without the fix.

## 5. Performance (from AGENTS.md, non-negotiable)

- `App.tsx` has 0 re-renders per second during playback. High-frequency values go through pub-sub stores and leaf components.
- Lists that can exceed 50 rows are virtualized.
- No `backdrop-filter` on list items.
- Only animate `transform` and `opacity`.
- Debounce search input by 150–250 ms.

## 6. Change hygiene

- One feature or fix per commit, with a conventional commit message (`feat(scope): …`, `fix(…)`, `refactor(…)`, `test(…)`, `docs(…)`). A refactor never changes behaviour and is committed separately.
- Update the domain doc (`docs/<DOMAIN>.md`) when behaviour changes, and add an ADR in `docs/adr/` for each architectural or compatibility decision.
- Before committing: `npm run audit:conventions`, `cargo clippy --all-targets`, `cargo test`, `npm test`, `npx tsc --noEmit -p .`.
- Run the wiki knowledge checkpoint (see AGENTS.md).
