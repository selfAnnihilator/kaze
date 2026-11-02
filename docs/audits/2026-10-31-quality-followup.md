# Kaze quality resolution follow-up — 2026-10-31

This executes the six-step resolution plan against the **667-result baseline**, after the [behavioral remediation](2026-10-31-remediation.md). Historical evidence remains unchanged. No commit, release, external issue, installed-app replacement or personal-profile mutation is part of this pass.

## Dependency reconciliation

All **14 OSV inputs / 13 unique package-advisory pairs** map to the existing [70-record triage](2026-10-31-security-triage.json); the duplicate GLib input remains represented. Original input order, locations, versions and previous assessments are retained. Current public advisory records and Cargo reverse-dependency trees were rechecked rather than treating lockfile presence as runtime reachability.

`tauri-plugin-updater` now requires 2.14.1 and the lockfile resolves **ZIP 9.2.0**, using the updater's supported `^9` dependency rather than a forced major override. Its Tauri requirement remains `^2.12`, matching the existing desktop dependency. The update also changes Minisign Verify 0.2.5 to 0.3.0 and removes obsolete Infer/CFB/Arbitrary nodes; no database change is required. [Updater dependency metadata](https://crates.io/api/v1/crates/tauri-plugin-updater/2.14.1/dependencies).

| Advisory / matched component | Path and affected functionality | Current disposition |
|---|---|---|
| [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), GLib 0.18.5 (two inputs) | GTK/WebKit/Tauri uses GLib 0.18. `VariantStrIter` unsoundness is affected below 0.20; no direct application call was found | **Needs review.** Transitive reachability is unresolved. Fixed API belongs to a different GTK stack; no incompatible upgrade or speculative vendoring |
| [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html), Paste 1.0.15 | Lofty 0.22.4 uses this compile-time macro for audio metadata | **Maintenance exception.** No patched release. The previous generic GTK explanation was corrected to the actual Lofty path |
| [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html), Proc Macro Error 1.0.4 | GLib Macros 0.18.5 build dependency | **Maintenance exception.** No patched release; upstream macro migration needed |
| [CVE-2023-49092 / RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html), RSA 0.9.10 | Optional SQLx MySQL dependency is in the lockfile; current all-target feature graph has no RSA node. App explicitly enables SQLite with SQLx default features disabled | **Not applicable to current shipped feature graph.** No RSA private-key operation is established. Reassess if MySQL features change; no patched version is advertised |
| [RUSTSEC-2026-0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html), Zbus Names 4.3.4 | Zbus 5.19 / MPRIS, portal and single-instance dependencies | **Maintenance exception.** No patched release; active dependency remains recorded |
| [CVE-2026-115905](https://github.com/zip-rs/zip2/security/advisories/GHSA-m665-v7pm-7grj), ZIP 4.6.1 | Central-directory versus streaming-header differential; fixed in 9.0.5 | **Affected-version exposure removed:** Windows updater graph resolves ZIP 9.2.0 |
| [CVE-2026-115906](https://github.com/zip-rs/zip2/security/advisories/GHSA-8hxf-qr8p-62x4), ZIP 4.6.1 | AES-reader HMAC failure at premature EOF; fixed in 9.0.0. Updater does not call password-decryption APIs | **Affected-version exposure removed:** ZIP 9.2.0; no claim that an AES exploit was established in Kaze |
| [CVE-2026-115909](https://github.com/zip-rs/zip2/security/advisories/GHSA-q7v9-x93c-xqv3), ZIP 4.6.1 | Symlink-target allocation exhaustion in archive extraction; fixed in 9.0.0 | **Affected-version exposure removed:** ZIP 9.2.0 |
| [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), ZIP 4.6.1 | Unix privileged permission preservation; fixed in 9.0.0. Old ZIP dependency was Windows-only | **Affected-version exposure removed:** ZIP 9.2.0; no shipped Unix ZIP path inferred |
| [RUSTSEC-2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), ZIP 4.6.1 | Extracted symlink targets outside destination; fixed in 9.0.0 | **Affected-version exposure removed:** ZIP 9.2.0 |
| [RUSTSEC-2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html), ZIP 4.6.1 | Unix permissions applied through symlinks; fixed in 9.0.0 | **Affected-version exposure removed:** ZIP 9.2.0 |
| [RUSTSEC-2026-0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html), Zvariant 5.15.0 | Zbus and macros encode/decode D-Bus values | **Maintenance exception.** No patched release |
| [RUSTSEC-2026-0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html), Zvariant Derive 5.15.0 | Zvariant's compile-time derive dependency | **Maintenance exception.** No patched release |

The updater's native-installer and legacy-archive functionality remains enabled. The old signed-Windows-archive exception is superseded by the compatible dependency update. Linux compilation and target dependency metadata are checked; Windows updater runtime/signing acceptance remains a coverage gap. Resolving advisory version matches is not evidence that six user-facing exploits were reproduced.

## Release-cache boundary

The previous exception established only the checked-in triggers/permissions. GitHub caches can also be restored from the default branch and shared between workflows on the same branch. Manual dispatch can select another branch. Thus read-only `contents` permissions and absence of a PR trigger do not enumerate every cache writer or establish trust in all repository history and operator contexts. [Cache scope](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching), [manual-dispatch branch selection](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).

Removed Rust-cache actions and set an explicit empty `cache` input on setup-node in both Linux and Windows signing builds. Arch/publish had no corresponding restore steps. These builds now perform clean dependency preparation; slower cold builds are the accepted tradeoff. Action pins, tag validation, separate publish permissions and all-target lint remain. No remote cache was deleted or release workflow executed.

Two Rust-cache warnings disappear. **Two setup-node diagnostics remain visible, with a specific reviewed exception:** commit `49933ea…` is v4, whose actual published `dist/setup/index.js` restores only when `core.getInput('cache')` is truthy. It records cache-manager state only inside that branch; `dist/cache-save/index.js` saves only when that state exists. The explicit empty input disables both operations, independent of who could write a cache. The exact bundles were downloaded as text and fingerprinted; they were never executed. The pinned action does not expose the newer `package-manager-cache` input. This exception depends on this SHA and empty input and must be reassessed on any action/configuration upgrade. [Pinned restore source](https://raw.githubusercontent.com/actions/setup-node/49933ea5288caeca8642d1e84afbd3f7d6820020/src/main.ts), [pinned save source](https://raw.githubusercontent.com/actions/setup-node/49933ea5288caeca8642d1e84afbd3f7d6820020/src/cache-save.ts), [action inputs](https://raw.githubusercontent.com/actions/setup-node/49933ea5288caeca8642d1e84afbd3f7d6820020/action.yml).

GitHub now documents `cache-mode: none`, but the installed Actionlint rejects the key and Zizmor does not recognize it. That attempted setting was removed; no syntax error or unsupported action input is left behind to quiet another scanner. The effective no-cache behavior uses the pinned action's supported input. This resolves the original runtime cache paths while retaining two scanner classifications, not a blanket trusted-writer exception or a claim that runner/dependency compromise is impossible.

## Focused code and harness changes

- Rust: playlist URL parsing searches backward for the final nonempty segment, avoiding a full iterator traversal. Added the conventional cache `is_empty` and mock-provider `Default`; simplified redundant borrows/Option handling and fixed documentation indentation. Existing recording identity, queue semantics, scoring and cryptographic block handling are preserved. Complex type aliases/argument groups and the intentionally stateful queue's `next` API are deferred; no broad refactor was justified by a scanner count.
- Three silent inspector-cleanup exceptions now report their type/message while attempting all closes and preserving an existing primary test failure. A controlled injected-close regression verifies those properties.
- Five URL-opening sites use shared QA policies. Local fixture/inspector HTTP requests accept loopback HTTP(S), reject credentials/invalid ports/non-HTTP schemes and validate redirect destinations. The Gaana metadata probe retains its exact HTTPS song origin and now enforces it on redirects too. Tests use local servers and invalid origins; no live-provider acceptance is inferred.
- All retained subprocess warnings were reviewed. Calls use argv lists with no shell-enabled subprocess, operator-selected copied binaries/generated fixtures and fixed desktop utility names. Utilities still resolve through the test operator's PATH; absolute resolution is not claimed. The 88 import/call/partial-path warnings remain visible rather than receiving blanket `nosec` comments.
- Thirteen current Python harness/helper files were formatted with AST equivalence checked before/after formatting. The two Ruff diagnostics in the frozen historical reproduction are retained. Seven GI import-order exceptions are annotated at the exact import because `gi.require_version` must run before loading repositories. No global Ruff ignore was added.
- Bandit's B101 exemption names test scripts, the measurement harness and two explicit historical reproductions only. Other rules remain active in those files. An injected control demonstrates that a production assertion still triggers B101 and a test shell call still triggers B602. Both `.bandit` and its YAML policy participate in Qlty's configuration/cache tracking. [Bandit test-assertion configuration](https://bandit.readthedocs.io/en/latest/plugins/b101_assert_used.html).
- Arch metadata consumed by makepkg is annotated per assignment; the two makepkg-supplied path references are annotated at their use. ShellCheck remains active elsewhere.
- Expanded single-instance execution exposed two obsolete `Configured Music Folders` expectations. Both now use the current `Music Folders` heading. The failed attempts are retained; the complete rerun preserves and passes backend/window identity, route/scroll, queue/Flow, focus-policy, shutdown-drain and crash/relaunch assertions. This was fixture drift, not an independently established application regression.

## Validation and comparison

Final [whole-repository SARIF](2026-10-31-quality-evidence/quality.sarif), [summary](2026-10-31-quality-evidence/quality-summary.json), [rule/location comparison](2026-10-31-quality-evidence/finding-comparison.json), [scanner executions](2026-10-31-quality-evidence/scanner-invocations.json) and [validation summary](2026-10-31-quality-evidence/validation-summary.json) are retained. The independent all-target Clippy gate completes successfully. Every emitted scanner invocation has a successful execution result; unchanged inputs reuse Qlty's cache. No failed Clippy execution is accepted as a pass.

| Scanner | Before | After | Interpretation |
|---|---:|---:|---|
| Bandit | 337 | 88 | Three cleanup-error and five URL patterns addressed; 241 test-assertion results explicitly exempted. Subprocess warnings remain reviewed and visible |
| Ruff | 222 | 2 | 213 statement-style diagnostics cleared by equivalent formatting; seven required GI import locations annotated. Two frozen historical diagnostics retained |
| Clippy | 48 | 34 | Fourteen focused simplification/API/documentation diagnostics cleared; structural refactors deferred |
| Radar Python | 31 | 28 | Cleanup/logging explanations improved; remaining complexity/literal warnings retained. Formatting exposed a repeated protocol-method literal; extraction of a constant was checked by constant-folded AST equivalence |
| OSV | 14 | 8 | Six ZIP affected-version matches removed. GLib duplicate, five maintenance notices and inactive RSA remain individually recorded |
| ShellCheck | 11 | 0 | makepkg metadata/input annotations are specific to known assignments/references |
| Zizmor | 4 | 2 | Rust caches removed; two pinned setup-node diagnostics retain the explicit cache-off proof above |
| **Total** | **667** | **162** | **505 removed or reclassified; no unexplained new findings** |

Final scanner levels are **113 errors, 47 warnings and 2 notes**; these are tool classifications, not confirmed product-bug counts. Quality exit remains **1** for findings. The scan uses `--all --no-fix --jobs 1 --no-progress --no-upgrade-check --print-errors --sarif`; no Qlty automatic fixes are used. The 241 assertion exceptions, seven GI annotations and makepkg annotations must not be described as application defects fixed.

Validation units remain separate:

- Final native embedded build and **259 Rust tests pass**, zero fail, **11 guarded cases skipped**. These opt-ins are not promoted to passes.
- **15 Node test files pass**, zero fail.
- **Six native scripts pass**: tray lifecycle, cached restoration, both-interface continuity, saved-theme startup, autostart, single-instance lifecycle. Two initial stale-heading failures are separately retained before the corrected full run.
- **Three ephemeral WebKit suites pass**: first-use/taste/notice states, responsive/Discovery bounds, and reduced-motion startup (four repetitions here). These use controlled IPC; they are separate from real Rust playback in the native scripts.
- **Four QA support cases**, three Gaana parser cases, seven Arch packaging cases and three quality-gate cases pass. URL tests use a disposable loopback server; no live Gaana/provider pass is inferred.
- The Bandit policy control retains production assertions and test shell-execution diagnostics. Formatting preserves ASTs, and whitespace/report-link/evidence checks complete.

This pass does not rerun the previous memory soak or resolve its allocation-ownership finding. The renderer-growth evidence, live-provider/account gaps, physical media-key limitations and non-Linux runtime gaps from the behavioral report remain open.
