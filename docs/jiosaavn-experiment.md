# JioSaavn experiment

Branch: `codex/jiosaavn-playback-experiment`. No release/version bump, public deployment, installation or production-profile migration is part of this experiment.

Search uses JioSaavn by default. The online source selector retains Alternative sources. Settings → Online music & library details controls availability, automatic gap filling, manual filling, retry and removal. Both new switches default on. Kaze accounts, Home/Flow logic and chart feeds retain their existing roles.

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
