# JioSaavn experiment

Branch: `codex/jiosaavn-playback-experiment`. No release/version bump, public deployment, installation or production-profile migration is part of this experiment.

Search and online playback use JioSaavn exclusively during this trial. Alternative sources and legacy download-provider routing are removed from the active backend. Settings → Online music & library details controls availability, automatic gap filling, manual filling, retry and removal. Both new switches default on. Kaze accounts, Home/Flow logic and chart feeds retain their existing roles.

## JioSaavn-only listening trial — 2026-10-27

- Online search has a fixed JioSaavn label. Stale requests for Alternative sources and the generic old full-audio resolver fail explicitly.
- Online playback and pre-resolution require an exact JioSaavn song ID or a unique conservative title/artist/duration match. Unmatched catalog items ask the listener to search JioSaavn; they never fall back to YouTube, Audius, Internet Archive, or Soulseek.
- Only `full:v3:jiosaavn:<song-id>` audio cache entries are eligible. Older cache migration and filename-based download substitution are removed. New JioSaavn audio still uses the bounded cache and existing seek behavior.
- Existing local files play from disk even if a stale cloud record labels them online. Original music, artwork, playlists, history and account records are preserved.
- With Kaze stopped, cleared 165 complete audio files and one partial file (749,897,855 bytes / 715.2 MiB) from the real profile's `remote-audio` and `stream_cache` directories. Both directories were verified empty before fresh playback.
- Home, Flow, charts and broader catalog metadata retain their prior data sources pending the user's listening decision. Metadata-only enrichment fallbacks are separate from audio playback. Legacy download UI/actions cannot supply new audio in this build; importing existing local files remains available.
- Validation: 154 affected Rust regressions passed (two existing ignored checks), 13 Node test files, both interface source/settings checks, all eleven native desktop groups, frontend/native builds, and the live unified-player gate passed. The live gate covers fresh progressive streaming, exact-ID URL refresh, pause/resume, seeking, cache replay and history/statistics identity.
- Opened the final build against the existing database and freshly searched/played Mayajaalame (`jiosaavn:YC7AAdVt`, 267 seconds). Provider-resolution logs, advancing playback, a completed 10,901,670-byte file and the player's open audio descriptor confirm the fresh JioSaavn source. Left the song paused at zero with the original volume restored; subjective quality awaits the user.
- Launcher environment detail: Codex supplies `XDG_CACHE_HOME=/home/abhi/.cache/codex-work`, so this launched process writes its new song to `codex-work/music-player/remote-audio`; a normal shell uses `/home/abhi/.cache/music-player/remote-audio`, which remains empty after the purge. The initial final-file check used the normal-shell path and timed out; confirming the active process's actual cache and audio descriptor resolved the check. Always inspect the target process environment/config when checking cache origin.

## First acceptance gate — passed 2026-10-26

Live search, exact song detail, validated artwork, complete decoding with Kaze's production `SymphoniaSource`, and muted native Rodio pause/resume/seeking passed for:

| Track | Language | JioSaavn ID | Advertised / decoded |
| --- | --- | --- | --- |
| Kesariya | Hindi | rjkrTnma | 268 / 268.19 seconds |
| Vaathi Coming | Tamil | IJ3C0q7f | 228 / 228.30 seconds |
| Buttabomma | Telugu | 1UqsPO7u | 198 / 198.81 seconds |
| Believer | English | BeXBcbVK | 204 / 204.38 seconds |

The integrated `PlaybackService` gate also exercises progressive streaming, seek after the complete cache becomes available, pause/resume, exact provider identity and replay with JioSaavn disabled. Pre-resolution and rejected-URL refresh have explicit checks in that gate. These are opt-in live tests, with disposable database/audio files and muted native output.

One repeat encountered an incomplete CDN artwork response (`UnexpectedEof / IncompleteBody`); a subsequent sequential four-language run passed. External API/CDN availability remains a runtime dependency. Previews do not satisfy acceptance.

## Isolated manual trial

Build the frontend and embedded native application:

```bash
npm run build
cargo build --locked --bin kaze --features tauri/custom-protocol
```

On Linux, use a separate profile and private session bus to avoid reusing the installed single-instance process:

```bash
KAZE_EXPERIMENT_DIR=$(mktemp -d -t kaze-jiosaavn.XXXXXX)
XDG_DATA_HOME="$KAZE_EXPERIMENT_DIR/data" \
XDG_CACHE_HOME="$KAZE_EXPERIMENT_DIR/cache" \
XDG_CONFIG_HOME="$KAZE_EXPERIMENT_DIR/config" \
dbus-run-session -- /home/abhi/Projects/music_player/target/debug/kaze
```

Search in multiple languages, enqueue a local file with online tracks, seek after caching, and try both interface styles. For enrichment, scan a test folder containing accurately tagged songs with missing artwork/year/album/track numbers. Conservative matching intentionally leaves ambiguous or incomplete files unchanged. Rescanning after adding embedded tags should preserve those tags; removal restores scanned values without editing the files.

## Repeatable checks

The implementation passes 151 affected Rust regressions, both live gates, the Node test files, the new two-interface controls checks, the Settings layout checks and eleven native regression groups. Two existing Rust tests remain intentionally ignored (manual/live-only checks). The frontend production build and native production-feature build pass.

Use disposable XDG data/cache/config directories for Rust suites. Network/mock-server/native tests may need host socket access.

```bash
node --test tests/*.test.mjs
cargo test --locked --lib --test jiosaavn_tests --test provider_tests --test library_tests \
  --test source_resolution_tests --test progressive_stream_tests --test cache_tests \
  --test playback_tests --test playback_restore_tests --test history_tests \
  --test cloud_sync_tests --test flow_online_tests --test flow_tests --test stats_and_account_tests
cargo test --locked --test jiosaavn_live_tests -- --ignored --nocapture --test-threads=1
```

With a local Vite server on port 5194 and the existing Playwright setup:

```bash
node scripts/test-jiosaavn-ui.mjs
node scripts/test-settings-layout.mjs
```

The new UI checks exercise source selection, availability, fill/retry/removal in Spatial and Brutalist. Settings layout checks cover 1888, 1440, 1024, 800 and 390 pixel widths. The native regression uses disposable music, account data and a private bus:

```bash
KAZE_PRIVATE_DUAL_TEST=1 dbus-run-session -- \
  python3 scripts/test-dual-interface-native.py /home/abhi/Projects/music_player/target/debug/kaze
```

Its eleven groups verify actual playback, queue/position/Flow preservation, MPRIS, hidden-window mutation suspension, paused session/history identity, profile/fullscreen behavior and a cold theme restart. Do not run native suites concurrently because they share compositor focus.

## Limits

- This is an unofficial, replaceable provider. Regional availability and future response changes need continuing compatibility checks.
- Enrichment requires conservative evidence. It does not fingerprint audio, infer absent track numbers or strip version labels to force matches. No-match attempts wait seven days; temporary failures wait one hour; manual retry clears those delays.
- URLs/details/search results have finite timeouts and bounded caches. Cached audio uses the existing configured size/age/active-track protection. No permanent JioSaavn download workflow is introduced.
- Seeking during an incomplete progressive stream retains the existing player restriction; full cached songs seek normally.
- Cloud identity is verified by local serialization/ingestion round-trips; no real account or remote production cloud was modified. Windows/macOS runtime and packaged-release acceptance remain unverified.
- Enrichment correctness and cached artwork across restart are covered by controlled provider fixtures. Automatic match coverage against a real personal library has not been measured.
- The broader pre-existing `test-dual-interface.mjs` browser script passed eighteen groups, then stopped at a missing Brutalist “Smart Mixes” navigation target. The new JioSaavn checks, Settings checks and native eleven-group suite pass separately. This experiment does not claim a complete browser-suite or release sign-off.

Architecture and source attribution: [ADR 0015](adr/0015-jiosaavn-and-reversible-enrichment.md).
