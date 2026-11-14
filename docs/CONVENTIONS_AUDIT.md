# Convention audit

How to audit a change, a branch or the whole project against
[CONVENTIONS.md](CONVENTIONS.md). Save each audit as `docs/audits/<YYYY-MM-DD>-conventions.md`
using the template at the end.

## 1. Automated checks

```bash
npm run audit:conventions -- --base <ref>   # the change since <ref> (includes uncommitted files)
npm run audit:conventions -- --all          # every source file (legacy backlog)
qlty check <changed files>                  # clippy, secret scan (trufflehog), dependency CVEs (osv)
cd src-tauri && cargo clippy --all-targets && cargo test --jobs 1 --offline --no-fail-fast
npm test && npx tsc --noEmit -p . && npm run build
```

`audit:conventions` fails on:
- a source file over 500 lines;
- new qlty function smells (high complexity, many parameters or returns, deep nesting);
- a legacy file that grew (`scripts/conventions-baseline.json`);
- a new SQL migration.

Paste its table into the report.

## 2. Measuring what a change added

`qlty smells --upstream <ref>` lists **every** smell in changed files, including old ones. To see what a branch added, compare it with its base:

```bash
S=$(mktemp -d)
git archive <base> src src-tauri/src .qlty/qlty.toml | tar -x -C "$S"
(cd "$S" && git init -q && git add -A && git -c user.email=a@a -c user.name=a commit -qm base)
(cd "$S" && echo <files> | tr ' ' '\n' | xargs qlty smells --no-snippets --quiet --no-duplication)
echo <files> | tr ' ' '\n' | xargs qlty smells --no-snippets --quiet --no-duplication
```

Compare the per-function `count =` values. In zsh, pass file lists through `xargs`, because `$VAR` is not split into words.

## 3. Review checklist

Mark each item ✅ / ❌ / n/a, with evidence (file:line, command output).

**Structure**
- [ ] Each new feature lives in its own module (Rust child module or domain file; frontend actions / hook / component split).
- [ ] Hubs (`App.tsx`, the processor, `types.ts`, command/query/event) gained wiring only.
- [ ] SQL lives in repositories; UI row shaping isn't inline in the processor.
- [ ] Components are render-only; logic sits in hooks and pure functions.

**Limits**
- [ ] `audit:conventions` passes. Legacy files shrank or stayed the same.
- [ ] No new duplicated blocks.

**Compatibility and safety**
- [ ] New IPC fields are optional with defaults; existing commands are unchanged.
- [ ] No new migration, or there is an ADR with the compatibility decision.
- [ ] No deletes or moves of user files outside Kaze's own folder.
- [ ] `qlty check` passes (no secrets, no vulnerable dependencies, clippy clean).

**Tests**
- [ ] The feature has backend and/or frontend tests; a fix has a regression test.
- [ ] Tests use temp dirs and in-memory databases only; login tests isolate credentials.
- [ ] The full suites pass. Known flaky tests are named, not hidden.

**Performance (AGENTS.md)**
- [ ] No new App state for high-frequency values; App render guards still pass.
- [ ] New lists are virtualized; no blur on rows; only transform/opacity are animated.

**Docs and history**
- [ ] Domain doc updated; ADR for decisions.
- [ ] One feature per commit, conventional messages; refactors separate from behaviour changes.
- [ ] Wiki checkpoint done.

## 4. Report template

```markdown
# Convention audit: <scope> (<YYYY-MM-DD>)

Range: `<base>..<head>`. Auditor: <name/agent>.

## Result
<pass / fail, one paragraph>

## Automated checks
<audit:conventions table, qlty check result, test counts>

## Checklist
<the checklist above with ✅/❌/n/a and evidence>

## Violations
| # | File | Rule | Metric | Fix / owner |
|---|---|---|---|---|

## Complexity added or removed
| Function / file | Before | After |
|---|---|---|
```
