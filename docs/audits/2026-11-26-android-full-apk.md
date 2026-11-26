# Full Android APK verification (2026-11-26)

The three Android fixes were verified in a full packaged Kaze app on a Realme
RMX3471 running Android 14. The normal signed APK was then installed over Kaze
with replacement installation, retaining its data. Its existing profile opened
on Home. The installed binary matches the artifact checksum and has no idle
media notification. No release or Git push was published.

## Packaged app results

Tests used `org.intelligentmusicplayer.fullqa`, a separate full-app build with
private library/download folders and one silent 90-second WAV. It contained the
real frontend, Rust player, scanner, Android plugin and updater. No automated
mutations used the normal Kaze profile. QA data and temporary device audio,
server, reverse tunnel and CDP forwards were removed afterward.

| Scenario | Observed result |
| --- | --- |
| First update without overlay grant/history | Native restart prompt shown; server saw no APK request until answered. |
| Decline restart permission | Update downloaded and installed; app closed without reopening. |
| Next update after declining | No repeated prompt; installation succeeded and stayed closed. |
| OK in fresh prompt | Opened Display over other apps Settings. Selected Kaze QA, granted access, returned; installation automatically reopened Kaze QA. |
| Grant already present, answer history absent | Skipped prompt; installation reopened the app. |
| Two injected transient settings errors | Packaged entrypoint made three GetSettings attempts and reached Home without retry UI. Injection ran only through the debug WebView and was removed. |
| Five cold launches | All reached Home without retry UI. |
| Real Rust playback | Silent local WAV produced an active media notification. Home retained background playback. |
| Playing app dismissed from Recents | Both PlaybackService and active media notification disappeared. |
| Shared backend Stop | Both PlaybackService and active media notification disappeared. |

The original intermittent startup error was not naturally reproduced; the
specific upstream cause remains unknown. These checks establish recovery from
transient settings reads, not a guarantee against every possible startup error.
The earlier eight-case native lifecycle fixture and unit/browser checks remain
recorded in the linked domain audits. [Retained full-app results](2026-11-26-android-full-apk-evidence/results.json).

## Delivered artifact

- `target/android-verification/Kaze_2.2.2_fixes_arm64.apk`, 40,983,075 bytes.
- Package `org.intelligentmusicplayer.app`, version 2.2.2 (2002002), arm64-v8a.
- SHA-256: `58b38a924f207c7d353a36eb37292109d604ab5311b15b08e39bbc493e41c8c9`.
- Release certificate SHA-256: `675fdb5a86f1aefcd9f8c70fd248a55c43ce37fb378dd08ee1172ec4cf75d293`, matching the previous installation.
- QA package paths and the localhost test-update URL are absent from the release
  native library. Private signing properties were removed from the build copy.

The build used an isolated snapshot of the current working tree, including the
user's existing uncommitted work. Production sources were unchanged during this
verification. The three fixes remain in commits `1912fb7`, `fd0a91e`, `4125a44`.
The original 40 modified and 20 untracked user files were preserved.

## Build and conventions notes

Production frontend and Android release builds passed. A debug incremental APK
initially retained freed ZIP blocks and reached 456 MB despite 65 MB of live
entries. Removing only the temporary APK output and rerunning
`:app:packageUniversalDebug` (skipping its already-completed Rust build) produced
a compact, signed 65 MB QA APK. The normal release APK was 41 MB without this
workaround. No repository build configuration was changed for packaging.

Before the evidence commit, `npm run audit:conventions` again reported the
existing `settings/useTasteArtistPicks.ts` smell in unrelated user work. The
scoped evidence-only snapshot audit passed with zero source changes. This
checkpoint adds documentation/evidence only: no IPC, schema, style, performance,
or production behavior changes. Previous scoped source checks remain recorded
in the individual fix audits. The pre-existing full Rust scoring snapshot
mismatch is also unchanged. `git diff --check` passed.
