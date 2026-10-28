# JioSaavn experiment

## 2026-10-28 — Library Moment playback and exact album opening

Library Moment now spotlights actual local-library album members, counts those rows instead of using an unset metadata total, and supplies a real first-track ID for playback/artwork. Online placeholders created for metadata/history cannot become an unplayable library spotlight. No stored albums, tracks or files are removed.

Per the user's interaction choice, clicking the cover plays that song directly in the shared Rust player; subsequent clicks pause/resume it. Open record is a separate action in both themes and opens that album without interrupting playback. Album navigation no longer silently depends on the first 200 cached summaries or uses a capped title search. The existing `GetAlbumDetails` query is implemented with exact album-ID membership, disc/track ordering, complete results and current-account feedback. Online rows retain the common source-presentation path. The destination remains the windowed Library track list; missing albums report a visible error. A supplied first-track pointer also lets artwork load without depending on the current/recent recommendations containing that album.

The Brutalist archive number now follows Albums, Tracks, Playlists and Liked instead of always showing the album count. It reflects the displayed section, including an opened album's exact tracks, and excludes smart mixes/the special liked list from regular playlists. The featured song pointer respects existing account-scoped dislike exclusions without restricting manual library browsing.

Verification: 26 Flow and two recommendation-feedback tests pass, including real member counts, ignoring online placeholders, a 105-track exact album, another artist's same-title album, current-account feedback and blocked spotlight pointers. All fifteen frontend test files pass. The focused browser gate passes both themes' cover play/pause/resume, uncached exact album opening, missing-album errors and metadata refresh retention, plus all four archive counts. Navigation restoration and windowing checks pass. Frontend and embedded native builds pass, followed by all fourteen private native continuity groups, including actual Rust cover playback and independent album opening. Tests use disposable data; the live app, profile, music and queue were not modified or restarted. Earlier full Rust coverage completed across runs with an unchanged recommendation-feedback recheck after a transient missing-table error; its cause remains unknown. The final focused suite passes without that error.

## 2026-10-28 — Spatial artwork fits narrow, tall windows

The Discover preview no longer overlaps the right-hand thumbnails/text in narrow native windows. WebKitGTK's implicit artwork grid column made the rotated cover about 543px wide inside a 333px column at 940×1000; Chromium did not reproduce it. An explicit shrinkable grid column contains the existing cover without changing playback, artwork identity or the theme composition.

The native real-App regression failed before repair and passes afterward across all four chapters at 940×1000, 800×1000, 1024×600 and 1440×900. The Spatial browser gate now checks horizontal separation and covers six window sizes, including both narrow/tall cases. Existing frontend regressions and frontend/embedded native builds also pass. Native checks use ephemeral offscreen views and a fixture backend; the live app/profile and music were not changed or restarted.

## 2026-10-28 — For You manual refresh selects fresh relevant songs

Refresh previously forced a source fetch and recomputed deterministic ranking, so the same strongest songs could return indefinitely. It now prefers least recently shown song families within each artist's relevant scoring band, preserving taste-ranked artist slots, score values, source/version identity, dislikes and the two-per-artist cap. Automatic reads preserve that batch until underlying listening/feedback/catalog ranking changes. Selection memory is process-local, account-scoped and bounded; small eligible pools may repeat. Existing cards stay visible with the refresh icon busy while the request runs. Automatic personal refresh no longer inadvertently requests a manual batch. The For You filter also accepts current genre/language explanation strings that it previously discarded. See [recommendation scoring](recommendation-scoring.md).

Verification: the unchanged-list Rust regression and missing-genre/language frontend regression failed before repair and pass afterward. All 226 Rust tests passed (nine optional/manual checks ignored), plus all 56 frontend tests, both layouts' manual/busy/background/stale-scoring/Flow-startup browser checks, frontend and embedded native builds, and thirteen private native continuity groups. Tests verify multiple distinct batches, account isolation, no unrelated/disliked picks, stable automatic reads, unchanged taste/history, retained cards while pending and independent playback/charts. The test executable was rebuilt without changing or restarting the live app/profile, music or queue.

## 2026-10-28 — Playback Space shortcut and draggable album shelves

After a mouse-selected action, Space now pauses/resumes the current song instead of activating that action again. The existing pointer-focus marker distinguishes mouse actions from Tab/Shift+Tab navigation, which retains ordinary button activation. A capture handler intercepts only the pointer-origin playback shortcut; typing, sliders, modal actions, modified/composing keys and held-key repeat are protected. The earlier outline fix preserved focus but the old global shortcut explicitly skipped buttons, explaining the user's repeated-Next report.

Album-spine metadata now uses bounded 18px title type and separate, clipped single vertical lines for title/artist. Full metadata stays available in the tooltip and accessible label. Shared `useHorizontalDrag` extends Discover's mouse browsing to every horizontal shelf, including Albums. One delegated gesture uses a six-pixel threshold, horizontal intent, capture, immediate scrolling, native image-drag prevention and click suppression after dragging. Release, cancellation, lost capture and blur restore scrolling styles; ordinary clicks, native touch/wheel, arrows and windowed rendering remain. The old Discover-only gesture was removed. Compact Spatial song cards now reserve room for their actual download/feedback controls instead of clipping them below the shelf.

Verification: all fifteen Node test files, both layouts' pointer-Space/held-key/Tab/typing/modal checks, album labels at four window sizes, album/Discover drag and normal-click checks, both layout suites and repeated player-identity checks passed. The Brutalist reference gate now checks archive tabs against the full production pane rather than the original study's obsolete scrollbar gutter. Trusted offscreen WebKitGTK mouse tests passed image dragging, release/click suppression, subsequent normal clicks and blur cancellation. Frontend and embedded native builds passed, followed by all thirteen private native groups, including Next then Space controlling the actual Rust transport without another track transition. The testing executable was rebuilt without restarting or modifying the live user session/profile, queue or music.

## 2026-10-28 — Listening-time scoring

Preference now uses tiered actual listening: quick intentional skips are negative, brief previews are neutral, ordinary songs qualify at 30 seconds, and short songs can qualify at 25% with a ten-second floor. At least 75% heard is stronger; near-complete listening, qualified repeats and explicit likes strengthen preference further. End/seek position alone cannot manufacture a completed listen. Pauses/buffering do not accumulate heard time, and source failures/buffering exits do not imply disliking a song.

Qualified evidence feeds taste, source-linked song performance, Flow/familiarity, discovery and automatic repeat/favorite eligibility. Score-based rankings also exclude unqualified preview seconds while retaining real usage time and original stored counters. Old permissive play counts cannot override available session evidence. A one-time migration expires only automatic mix caches; original history, media, manual collections and cumulative/device totals remain intact. See [ADR 0022](adr/0022-listening-time-preference.md).

Verification: 225 Rust tests across 27 groups passed (nine optional/manual checks ignored), including eleven new listening-policy regressions. All fifteen Node test files, both interfaces' scoring-refresh, buffering and pointer/keyboard focus checks, frontend and embedded native builds, and all twelve private native desktop groups passed. The testing executable was rebuilt without restarting the user's running app or changing its live profile, media or queue.

## 2026-10-28 — One Flow action and complete search results

Home and the Brutalist player's Flow control now expose Start/Pause. Starting an already active paused Flow uses ContinueFlow in the existing session; pausing keeps its generation and queue. The separate Stop Flow action and Home prop chain were removed. Explicitly playing a song outside Flow still stops automatic Flow in the backend.

Search's All view keeps the top match and shows the complete returned song list beside it. The five-song preview, six-extra-results cap and Show all songs link were removed. All and Songs share one windowed row implementation, including exact-source playback, download, playlist and dislike controls. The search panel and list fill the remaining viewport and resize with it; only rows scroll. Compact content widths use a single full-width song list. Other collections keep their existing list sizing.

Verification: all fifteen Node test files and the four focused Home render checks passed. Both layouts passed a 125-result search test covering the last result, bounded DOM rows, full-height All/Songs lists, tall/short/compact windows and outside-song Flow exit. Startup/Trending, direct-download/dislike and the full dual-interface navigation gate passed after aligning fixtures with current controls. Frontend/embedded native builds and all twelve private native groups passed; the extended native Home gate verifies one Start/Pause action and unchanged generation/audio/history identity when continuing. The testing executable was rebuilt without changing the live user profile or restarting the running app.


## 2026-10-28 — Flow startup and Trending loading

Start Flow now resumes a paused current session after Stop Flow without a second Play/Resume click or a new history session. Whole-catalog legacy preparation was removed from the interactive start path; selected online recordings retain exact source binding. Home immediately shows the pending action and no longer waits for its artwork/recommendation refresh to enable playback controls.

Trending loads independent chart playlists together and restores its saved public feed before checking the network. Cold requests show loading, refreshes retain existing cards, and personal recommendation refresh runs separately. Initial account discovery now reissues Trending after discarding a default-account result. Saved responses still filter the current user's dislikes. The duplicate Trending refresh icon was removed. See [ADR 0021](adr/0021-flow-startup-and-trending-latency.md).

Verification: 214 Rust tests across 26 groups passed (nine optional checks ignored), plus the focused single-song resume case and 55 Node tests. Both layouts passed delayed Start/Home/network, cold/warm Trending, independent refresh and retained-card checks. Existing direct-download/dislike, buffering, delayed-scoring, player-identity and recommendation-feedback interface gates passed. Frontend and embedded native builds passed; all twelve private native groups passed, including the actual Home Start button resuming paused/stopped Flow with unchanged audio/history session identity. The rebuilt testing executable is `target/debug/kaze`; the user's running app/profile was not replaced or restarted.



Branch: `codex/jiosaavn-playback-experiment`. No release/version bump, public deployment or installation. The current source-link repair upgrades saved online identities in the testing profile while preserving playlist/history/statistics IDs.

Search and online playback use JioSaavn exclusively during this trial. Alternative sources and legacy download-provider routing are removed from the active backend. Settings → Online music & library details controls availability, automatic gap filling, manual filling, retry and removal. Both new switches default on. Kaze accounts, Home/Flow logic and chart feeds retain their existing roles.

## Source-aware scores and Not interested — 2026-10-27

Recommendation scores now combine the account’s shared song history with JioSaavn artist credits, language and release year. Not interested is a durable exclusion across saved aliases and matching releases, with an Allow again action; generated queues/mixes honor it and manual playlists/direct playback remain available. Explicit preferences survive stale neutral sync counters. See [ADR 0018](adr/0018-source-aware-recommendations-and-feedback.md) for persistence, scope and verification.

## Source consolidation — 2026-10-27

JioSaavn supplies the catalog, charts, playlist imports, downloads and metadata enrichment as well as playback. Retired provider implementations and their launch/settings controls are removed. Saved aliases and original tag columns remain compatible. See [ADR 0017](adr/0017-jiosaavn-catalog-consolidation.md). Earlier feed/fallback descriptions below are dated history.

## Current contract: every displayed online song has a saved source

Buffering repair (2026-10-27): progressive reader stalls/recovery now use `PlaybackBufferingChanged` with the active playback session and track ID. Preparation/obsolete readers do not send current-song state; chunk arrival, reader starvation and terminal download states serialize their flag/notification order. Completion, failure and cancellation clear buffering. Download recovery never emits a fake Resume or position zero. The UI accepts only the active session, preserves position/pause, clears pending selection loading at PlaybackStarted, and discards state-query replies overtaken by newer playback events or queries. Metadata-only next-source resolution remains separate from audio loading. `scripts/test-buffering-ui.mjs` replays these races in both interfaces; progressive-reader and history tests cover EOF, cancelled preparation and paused listening accounting. The opt-in unified JioSaavn native gate also checks completed buffering/pause and rejects fake Resume events.

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


## 2026-10-27 — Catalog cleanup, taste feedback and direct song controls

Cleanup was committed first as `6f105cb`, removing the retired online providers/adapters and keeping the unified Rust playback, cache/history and local-library paths. Source-aware scoring and durable account-scoped feedback follow ADR 0018. The Not interested icon is reversible, hydrates its saved state across views/accounts, excludes related recordings from generated suggestions, and affects taste/scoring. Search/manual playback remain available.

Download now uses the displayed recording's saved identity and saves native AAC as M4A directly; the options modal is removed. Recording identity persists in download tasks, including saved aliases. Each memoized button displays its own percentage and transform-based fill, then the downloaded icon. Stale task snapshots cannot rewind byte progress or undo terminal completion. UTF-8 filename limits cover multilingual titles. Discovery has refresh for all four chapters, real chart covers, source-bound alias artwork and original-release preference during recommendation selection. Normal UI labels no longer display provider brands. See ADR 0019.

Live investigation confirmed two causes of India-only charts: the service reads language from its `L` cookie rather than the query alone, and most rows use `listid`/`listname` instead of `id`/`title`. The adapter handles both shapes and explicitly requests English/international programming. Actual returned lists include International : India Superhits Top 50, Most Searched Songs - English, Most Streamed Love Songs - English and English decade lists. Their names, identities and images are preserved. A local-server regression asserts the cookie, both response shapes, international editorial lists, artwork normalization and forced cache refresh.

Verification: all 208 Rust tests across 26 groups passed (eight optional/manual tests ignored), plus the live catalog/playlist/direct-download gate. That gate downloads the exact source behind the displayed chart song without another title search, checks its stored source ID and decodes the saved M4A through Kaze. Subsequent focused recommendation/Flow/feedback checks also verify missing online artwork does not borrow another recording's enrichment cover. All fifteen Node test files pass. Both interfaces pass controlled UI checks for persisted toggle on/off, exact direct download, independent same-title releases, percentage/fill/completion, all refresh chapters and chart covers, plus existing source/settings, feedback, buffering and delayed-scoring checks. Frontend/embedded native builds pass. The native eleven-group gate verifies real playback, queue/Flow/MPRIS continuity, ten hide/restore cycles, paused history/session identity, profile/fullscreen and cold style persistence.

An earlier native run failed its Playing assertion during hide/restore, then the unchanged assertion passed on rerun; cause remains unconfirmed and failure diagnostics were added. A browser buffering fixture had one startup timeout during a combined run and passed in isolation; no product fix or confirmed cause is claimed for that timeout. An automatic approval-review usage limit temporarily prevented the native rerun; approved checks resumed after its stated reset time. Tests use disposable data and do not edit the user's real profile, purge caches, install or publish a build.

## 2026-10-27 — Sia compilation artwork and artist-edition selection

The reported Breathe Me entry resolves to the Night Train compilation, whose real album metadata credits Various Artists. Its train cover is the correct cover for that saved edition. The service's Sia single and Colour The Small One versions have separate IDs and artwork. The live artist-edition gate selected `SJYw8KAn` (Sia / Breathe Me), decoded its 500×500 artwork and downloaded/decoded its exact M4A.

ADR 0020 changes default edition ordering and adds bounded, cached artist-credit verification for recent online songs. Verified artist editions outrank compilations in generated picks, with listening/familiarity retained. Saved sources/playlists and existing queued selections are not retargeted; re-search/play or a fresh generated selection chooses an artist edition. Complete fetched album identity/year/language are now preserved in source snapshots, so later release checks do not lose that evidence. No cover-only substitution, filename guessing, user file/tag rewrite, cache purge or live-profile repair is performed.

Verification for this follow-up: 211 Rust checks across 26 groups passed, with nine optional/manual checks ignored; the new live Sia artist-edition/cover/exact-download gate passed separately. All fifteen Node files, both interfaces' song controls/scoring/player-identity browser checks, frontend/embedded native builds and all eleven private native desktop groups passed. The old compilation and new artist edition retain separate immutable source snapshots, artwork and audio identities. Tests leave the real user profile, queued recording and media files unchanged.
