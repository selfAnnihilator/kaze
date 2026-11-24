# QA and optimization run — PC and Android (2026-11-24)

Branch `ai/android-foundation`. PC tested on the worktree release build in the isolated
native harness (`scripts/profile-frontend-native.py`). Android tested on the RMX3471
(Android 14) over adb/CDP.

## Gates

| Check | Result |
|---|---|
| `npm run audit:conventions` | No failures |
| `npm test` | 158 pass |
| `cargo test` (all) | 429 pass |
| `cargo clippy --all-targets` and `--target aarch64-linux-android` | Clean (one Android warning fixed) |
| `scripts/check-quality.sh` | Exit 0; trufflehog: 0 secrets. The remaining issues predate this branch (dependency advisories in `Cargo.lock`, zizmor cache findings on the existing jobs). |

## Defects found and fixed

1. **Cloud sync used up the Cloudflare D1 free write quota (100k rows/day).**
   - **Cause:** every signed-in device ran a full sync every 2 minutes, and each one re-uploaded every song, stat, device stat, daily stat and playlist row. About 2,700 rows went up per sync from a real profile, and the Worker upserts each one; D1 also counts index writes.
   - **Fix:** `src-tauri/src/cloud/sync_manager/delta.rs` remembers a fingerprint of each row this device uploaded (ignoring `updated_at`) and sends only new or changed rows. Tombstones always go. A sync with nothing new sends no request at all. A failed upload isn't recorded, so its rows are retried.
   - **Tests:** `delta_tests.rs` (3); the existing cloud sync suites still pass.
   - **Limits:** the ledger is held in memory, so the first sync after launch is still a full upload. Old installs (installed PC 2.1.1, CI APKs before this) keep full uploads until updated.
2. **Restored queue showed "Queued track" for online songs.** Playback snapshots carried only IDs, and a restore never emits `QueueUpdated`.
   - **Fix:** `PlaybackStateDto.upcoming_titles` and `upcoming_artists` (optional), used by `upcomingFromPlayback`.
   - **Tests:** `interfacePresentation.test.mjs` and `flow_online_tests.rs`. Verified on the phone after a restart.
3. **Updater listeners piled up.** After a failed install, every retry added another `update-progress`/`update-failed` pair, so failure toasts repeated. Fixed: there is now only ever one pair (`appUpdate.ts`).
4. **The phone UI was bundled into the PC's main chunk.** `MobileApp` is now lazy. The PC `App` chunk went from 457 kB to 323 kB JS, and its CSS from 253 kB to 178 kB.
5. **Small hook fixes:**
   - `useSideSwipe` re-attached its touch listeners on every render; it now attaches once.
   - `useFitText` observed its own element, so its own font changes re-triggered it; it now observes the parent.

## Measurements

**PC (native harness, release, isolated profile):**
- Passes all scenarios.
- React commits are equal to or lower than the 2026-11 baseline everywhere (navigation cycles 245 vs 276–282).
- Private memory is 30–45 MB below that baseline.
- Renderer CPU during playback is 3.8–4.2%, against 1.8–2.0% in the old baseline. The installed 2.1.1 measures the same today (3.6–3.8%), so the difference comes from the environment, not this branch.

**Android (RMX3471):**

| Metric | Result |
|---|---|
| Cold start (`am start -W`, debug) | 1.1 s |
| PSS at idle (debug) | ~219 MB |
| PSS during playback (release Rust) | 152 MB |
| Paused CPU, settled | 0–1.6% |
| Playback CPU, debug build | 55% (AudioTrack thread ~40%) |
| Playback CPU, release Rust | 12–22% of 800% (AudioTrack 4–7%) |
| React commits during playback | ~3.4/s, from the progress leaf only |

- **Paused state:** the audio device suspends 60 s after a pause (`playback/output.rs`), confirmed on the device.
- **Playback CPU:** the debug build's cost is unoptimized Rust decoding, not an app bug.

## Not done / open

- Screen-off playback CPU on Android wasn't measured: wireless adb drops when the screen turns off, and the on-device logger was killed with it.
- A Worker-side guard (upserts that skip identical rows) would also protect old clients. It needs a Worker deploy, so the user decides.

## Worker guard (prepared, not deployed)

`worker/src/index.ts`: every sync upsert has `ON CONFLICT ... DO UPDATE ... WHERE <something
changes>`, so a row that would stay the same isn't rewritten and isn't billed. The
`updated_at` the client sends is its sync time, so it never decides alone. Stats rows only
change when a counter grows (they already merge with `MAX`), likes when they flip, and songs
when a field changes or gets filled in. This protects old clients too, as soon as it is
deployed. Test: `tests/statisticsWorker.test.mjs` checks that resending unchanged data
writes 0 rows and that one changed play writes 4.
