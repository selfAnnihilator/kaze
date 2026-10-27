# JioSaavn experiment

Branch: `codex/jiosaavn-playback-experiment`. No release/version bump, public deployment or installation. The current source-link repair upgrades saved online identities in the testing profile while preserving playlist/history/statistics IDs.

Search and online playback use JioSaavn exclusively during this trial. Alternative sources and legacy download-provider routing are removed from the active backend. Settings → Online music & library details controls availability, automatic gap filling, manual filling, retry and removal. Both new switches default on. Kaze accounts, Home/Flow logic and chart feeds retain their existing roles.

## Current contract: every displayed online song has a saved source

The source-link repair supersedes the search-redirect workaround below. `online_track_sources` stores the exact JioSaavn ID and provider metadata for each saved Kaze identity. Before Home, discovery, charts, playlists, Flow or a restored queue exposes an online row, it is linked to a provider version and its displayed metadata is replaced with that version's metadata. Missing sources are omitted from playable lists, preserving their original saved rows.

For old catalog entries, import prefers the matching album, then provider order among same-title/artist releases. A 210-second placeholder is not treated as measured audio evidence. This choice happens once, before display; subsequent playback reads the saved provider ID and refreshes media only for that ID. Local-file enrichment keeps its separate strict matching rules. The old “No unique JioSaavn match” playback path and manual search redirects are removed.

Source preparation uses four concurrent searches, a twenty-second batch deadline and persistent snapshots. Metadata events are batched. Restored queues retain entry IDs, an available selected entry, repeat and shuffle ordering while repairing source labels/durations. No original music files or tags are changed. A private database recovery backup was taken before applying the upgrade to the user's profile.

Verification includes 157 affected Rust regressions, the focused final source/restore checks, Node tests and both interface browser checks. Live muted native playback passed for saved aliases of Babydoll, Dance With Me, Missing Piece, Bloody Mary and Bad Romance; every playback source and displayed album/title/duration agreed. A two-release fixture proves an album version chosen before display remains selected after restart/provider outage and plays its own cached audio without another title search. Browser checks also prove selecting another same-name release dispatches that release's exact ID.

Real-profile verification exposed a legacy unique constraint on `(provider, provider_id)`: several saved IDs can point to a JioSaavn song that is already registered. The dedicated link table now supports that many-to-one relation without rewriting the original provider pair, and search reuses existing non-prefixed JioSaavn identities. Regression coverage includes an already-present canonical row. The original queue bookmark was restored after the probe; 70 source-ready entries remain playable from its 99 entries, while unlinked records remain saved. The actual saved Babydoll entry (`itunes:6794168373`) played JioSaavn `efAHHwFG`, a 7,437,911-byte complete audio file. Queue selection and Resume preserved the 70-entry queue; volume was restored and the app left paused near the beginning. Provider duration is 178 seconds; the decoder reports approximately 178.85 seconds, an expected subsecond container difference.

See [ADR 0016](adr/0016-source-linked-online-catalog.md) for the current contract. Older dated sections retain the history of the experiment and the superseded workaround.

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

## Search availability and saved catalog aliases — 2026-10-27

User-facing JioSaavn search now shares the playback source validator: it omits restricted entries, entries without ordinary full-song media or duration, invalid media URLs, non-song records and duplicate IDs. It validates fields already returned by search, without downloading audio or adding requests per result. Metadata-only enrichment keeps a separate catalog lookup so local tags/artwork can still be filled independently of streaming availability.

Submitting a search immediately opens its own results view. Loading and no-results states show no fallback discovery shelves. Manual selections of older online catalog entries, including the deck's stopped/paused Play action, open JioSaavn search to choose an exact version; playing local files and exact JioSaavn identities retain their existing paths. Saved records, playlist identities and metadata matching thresholds are preserved. Native/automatic queue resolution of old aliases still uses the conservative backend matcher; this is not a bulk catalog migration.

The reported Dance With Me / Bruno Mars error was a saved `itunes:1866733083` entry with a 210-second duration, album The Romantic. The provider currently returns two exact Bruno Mars versions (`TgDMvmA8`, `6uRb-VKP`), each advertised at 219 seconds. Both actual media responses were 8,725,921 bytes and decoded completely to 219.31 seconds in Kaze's Rust decoder. Thus this example exists on JioSaavn; its old identity/duration caused the matcher failure. Do not relax recording matching or remove a valid provider result to mask that failure. [JioSaavn album-version listing](https://www.jiosaavn.com/song/dance-with-me/Rh05U1lmfGM).

Verification: new unavailable-entry fixture failed before the filter (all nine catalog entries were accepted) and passed after it; loading-state browser reproduction also failed before the UI fix. All 67 focused Rust checks passed sequentially (one existing ignored check), 13 Node files, both interface search/legacy-entry/settings checks, both full-song live decodes, and frontend/native builds passed. One concurrent run of an existing MPRIS database fixture reported `no such table: tracks`; the focused sequential rerun passed without changing MPRIS code. The intermittent fixture cause remains unestablished.

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
