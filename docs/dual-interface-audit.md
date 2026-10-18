# Kaze dual interface audit

Authoritative scope: 2026-10-15 attached finalization brief. No commit or push.

The 2026-10-16 performance follow-up is documented in [the frontend performance audit](frontend-performance-audit.md), including measured hot paths, fixes, unchanged visual identity, reproducible checks and runtime limits.

Settings/profile refinement (2026-10-16): reproduced a matching complete blank screen by rejecting the delayed `SettingsView` module request after Home mounted; normal controlled Settings data opened successfully. Settings now imports with App, so clicking the gear requires no later screen-module fetch. This removes the reproduced failure path; the original screenshot did not include an error log proving why that particular request failed. The Settings component still mounts only on its route, so no Settings effects/queries start on Home. App's chunk grows about 26 KB / 5.5 KB gzip as the existing Settings code moves into it; no dependency was added.

Both sidebars now pin a profile-photo button immediately below Settings, opening the existing account/listening-statistics route. It uses the current profile's cached image or URL, updates on profile events, and falls back to an initial/user icon on missing or failed artwork. Profile is removed from the Spatial utility menu and the old Brutalist Activity slot to avoid duplicate entries. Brutalist's navigation scrolls independently so the footer remains visible in short windows. `ProfileNavigation` is memoized and receives existing stable state/callbacks; no visual timer, polling or backend change was introduced. Production build, 42 Node tests, 31 shared browser groups, eight preview/layout/cover groups, six new Settings/profile groups and ten native WebKitGTK groups passed. Run `node scripts/test-settings-profile.mjs` for delayed-load fault protection, all Settings sections, photo updates/fallbacks, guest access, footer bounds and playback continuity. The native suite now additionally checks the shortcut against real statistics and exact paused playback.

Navigation refinement (2026-10-16): per the user's request, Now Playing is removed from both sidebars. The bottom player's artwork is a button labeled “Open Now Playing”; click, Enter and Space open the existing view without playback commands. Spatial now has four primary destinations (Home, Discover, Library, Queue), retaining its existing spacing and adjustable rail. This supersedes the five-destination navigation described in the earlier milestone below.

Home cover refinement (2026-10-16): the hero sleeves now show previous, current and next from left to right, using the shared observed previous track, active playback and first actual upcoming queue entry. They no longer select side covers from recent recommendations. All three slots stay mounted and opaque; missing tracks/covers and failed image loads show solid placeholders. Side-cover clicks use the existing Previous/Next commands and the center toggles playback. The memoized upcoming-track metadata is shared with Now Playing/fullscreen and preserves available cover URLs. The production build, 31 shared feature checks and seven preview/cover checks passed, including actual exposed-area pointer clicks, track-order updates, missing-next and broken-image fallbacks. Existing shell/hero/current-cover geometry still matches the lab at four sizes.

Library refinement (2026-10-16): added the missing `playlists-view` root class so existing content-width rules align the playlist header, actions and rows with the Library tabs. Liked Songs is excluded from custom playlist cards in both themes; its dedicated Spatial tab and existing Brutalist navigation remain available. Removed an overly broad route-based `h1` hide rule that also hid collection titles. Liked Songs alone now uses a compact header, smaller tilted cover, normal full-width track sheet and short-window spacing, preserving ordinary collection layouts and all actions. Build, 31 shared checks and eight preview/layout/cover groups passed. The one-liked-track fixture verifies aligned playlist bounds, a visible Liked title and the entire first row above the transport at 1920×1080, 1440×900, 1024×600 and 800×700.

## Preview layout and circular familiarity dial (2026-10-16)

The user's follow-up makes the preview layout authoritative across Spatial: integrate the existing features into it. `DESIGN.md` records that contract. Production now follows the reference Home composition with overlapping real sleeves, its five-destination rail, Library tabs and mixed-size album shelf, editorial playlist rows, tilted collection cover/track sheet and descending queue path. Additional tools use the preview top bar's trailing utility slot. All data/actions still belong to the existing App; no prototype reducer or sample catalogue is imported by these adapters.

The familiarity range is now `FamiliarityDial`, shared by both real presentations. Pointer movement changes only leaf draft state; release invokes the existing steering callback and debounce. Cancel restores the current value. Arrows, Page Up/Down, Home/End and accessible numeric/value text are supported. Dragging neither issues intermediate commands nor restarts playback. Existing Flow start/pause/resume/stop, recommendations/explanations, queueing, Library Moment and week statistics remain available.

`scripts/test-preview-parity.mjs` compares production Home, sleeves, rail and top-bar geometry to the actual lab at four desktop sizes, then tests dial keyboard/pointer/commit/cancel/command continuity and real Library/Queue/menu routes (six groups). The nine fixed-stage groups and 31 shared feature groups also pass, along with the production build and 38 Node regressions. The fixed-stage test now checks actions against the horizontal shelf's clipping boundary as well as the stage/player. Native WebKitGTK verifies the five-icon rail and circular dial updating actual Rust Flow steering, then preserves the existing playback/queue/Flow/MPRIS/session/persistence checks. Review screenshots and logs are regenerated under `/tmp/kaze-preview-parity/`.

New files are `SpatialHome.tsx`, `SpatialUtilities.tsx`, `FamiliarityDial.tsx`, `familiarity-dial.css`, `spatial-preview.css`, the parity script and the design contract. Existing view adapters, Gallery/Queue compositions and browser/native checks were adjusted. No additional Rust production change, dependency, commit or push.

Test maintenance lessons: menu unread counts need a stable accessible name; notification history is frontend event state, so an unused fixture query does not populate it; profile checks must wait for the lazy real content instead of accidentally matching an always-visible sidebar label. These checks now exercise the intended destinations.

One final native run observed an exact-position mismatch when its paused baseline was queried immediately after Pause. The check now waits for observed MPRIS Paused and a non-playing backend snapshot before recording the baseline; the exact equality assertion remains and the final run passes all seven groups. The precise timing mechanism was not instrumented, so this is a test synchronization correction rather than a claimed playback-engine fix.

## Spatial reference restoration (2026-10-15 follow-up)

The first production integration retained the old text sidebar and introductory Discovery hero. It passed functional checks but did not preserve the accepted preview's composition. The user's screenshots confirmed this drift. The production Spatial contract is now:

- A compact icon rail with real destinations, native tooltips/accessibility names, a separately scrollable secondary navigation area, and Settings pinned at the bottom. Its right-edge divider supports pointer dragging, Arrow keys and Home/End within 56–160px. Drag coordinates and live width stay outside App state. Width is session-local; interface style remains the existing backend preference.
- A single top bar with navigation, route and the existing debounced search form. A detached transport retains all real controls. The rail spans the full window height; the content and transport share a grid height budget.
- Discovery has a large tilted artwork pane and a fixed detail/music pane. Hover or focus on a real track, mix or chart updates the cover/caption/details without playback. The large preview's play/pause/loading control uses the same actions and playback truth as the shelf cards. Changing recommendation filters removes stale hover selections.
- Chapter buttons, Page Up/Down and vertical wheel gestures outside horizontal shelves replace content in place. One continuous wheel gesture changes one chapter. Discovery and Now Playing fit the available pane, while search results and opened collections retain normal scrolling. There is no introductory welcome heading in Spatial.
- Shelf filters and arrows share a row in short windows. Card width and virtualization pitch use the same measured size, so the entire card and its actions fit above the transport. Artwork dimensions remain square. The original lab remains isolated from production state.

Implementation: `src/presentation/SpatialNavigation.tsx`, `spatial.css`, `DiscoveryChapters.tsx`, `WindowedStrip.tsx`, plus the real Sidebar/App/DiscoveryView adapters. Playlist cards remain reachable from the icon route and support keyboard opening. No additional Rust change was needed for this correction.

Validation: production build and 38 Node regressions passed; all 31 existing browser groups passed for both presentations, including zero chrome mutations during progress ticks. New `scripts/test-spatial-layout.mjs` passes nine groups covering rail resizing, live preview play/pause, hover/focus and filter selection, chapter/wheel geometry, collection scrolling, and all four chapters plus Now Playing at 1920×1080, 1440×900, 1024×600 and 800×700. It deliberately supplies fresh taste recommendations: the general fixture's empty For You chapter could not expose the short-window filter/action clipping. Screenshots are regenerated into `/tmp/kaze-spatial-restored/`. Native WebKitGTK split-pane/rail checks and the real playback/queue/Flow/session/style-persistence checks passed against an embedded debug build with isolated data. Remote services and other operating systems retain the limits below.

Run the new layout regression with the same Vite server as the general browser check:

```sh
node scripts/test-spatial-layout.mjs
```

It supports the same browser/runtime/server/output environment overrides. Functional parity checks alone do not certify visual fidelity: preserve the supplied reference composition and assert actual content/action bounds with populated chapters.

## Pre-implementation inventory

The functional owner is `src/App.tsx`, using the single Rust event stream, playback-progress publisher, command service and `useNavigationHistory`. The accepted Spatial and matte-black Brutalist labs have separate fixture reducers and cannot serve as production state owners. Legacy `model`/localStorage routing also bypasses complete functional views and must leave the normal app entry.

| Capability | Existing owner / contract | Spatial | Brutalist |
| --- | --- | --- | --- |
| Home; Recent Rotation; Rediscover; For You; Library Moment; Your Week | HomeView + HomeOverview | Shared actions | Shared actions |
| Flow start/continue/pause/stop; steering; Why This Track | App commands; HomeView; TrackExplanation | Shared actions | Shared actions |
| Discovery; world trending; genre/similar recommendations; mixes; regional charts | DiscoveryView and collection adapters | Chapter stage | Editorial sections |
| Global online search; artist search; clear; local matches | App navigation/search; OnlineSearchResultsSection | Shared actions | Shared actions |
| Downloaded library; search; rescan; format; play; feedback; enqueue; playlists | LibraryView | Virtual rows | Virtual rows |
| Albums and artwork loading; album details | AlbumsView; CollectionDetailView | Staggered shelves | Record catalogue |
| Playlists; Liked Songs; smart mixes; details; save; rename; delete; create | PlaylistsView; CollectionDetailView; dialogs | Shared actions | Shared actions |
| Spotify inspect/import; local matches; missing tracks; wishlist | PlaylistsView + App handlers | Shared actions | Shared actions |
| Queue order; play index; enqueue local/online; remove; clear | PlaybackState + existing Rust queue commands | Future path | Numbered sequence |
| Local/online/cached/preview playback; pause/resume; previous/next; seek; volume/mute; shuffle/repeat | PlaybackService; NowPlayingBar; progress leaf | Shared controls | Shared controls |
| Current metadata/art; liked/playlist state; origin; online download; buffering/loading/seek availability | NowPlayingBar | Shared controls | Shared controls |
| Now Playing; full screen; Esc; lyrics and timed seek | FullScreenPlayerView; LyricsView | Spatial sequence | Artwork poster |
| Downloads: provider search/filter; start/cancel/progress; refresh; import; Soulseek | DownloadsView + App; currently missing direct route | Restore access | Restore access |
| Wishlist: add/search/filter/status; direct search; Soulseek | WishlistView + App; currently missing direct route | Restore access | Restore access |
| Notifications; mark read/all; dismiss/all; toasts | NotificationsView + App | Shared actions | Shared actions |
| Profile; authentication; avatar; logout; statistics | StatsView; AuthModal; App | Shared actions | Shared actions |
| Requested Settings surface: folders, temporary-audio storage/cache, outer close behavior and account logout | SettingsView + existing commands | Centered vertical settings | Numbered full-width settings |
| Interface Style selection | New narrowly validated application_settings key | Spatial default | Persisted selection |
| Back/forward; Alt+Left/Right; Space; full screen; collections/search/lyrics history | Existing navigation and keyboard handlers | Preserve | Preserve |
| Onboarding; update banner; playlist/download/auth/name/explanation dialogs; offline/loading/empty/error states | Existing components and handlers | Preserve | Preserve |

ArtistsView is an unmounted legacy component, with no App artist data/selection path. Artist browsing is available through discovery artist search and collection artist metadata; do not invent a second artist backend path.

## Queue constraints

`queue_track_ids` contains upcoming entries, while `current_queue_index` is absolute. Use absolute offsets only when supplied. Preserve duplicate occurrences and backend order. There is no reorder command or complete past-queue query; do not manufacture one by rebuilding the queue. Previous artwork may reflect an actually observed prior track and must be labeled accordingly. Unknown online entry metadata must have an honest fallback.

## Verification record

The inventory above records the pre-implementation contract. The completed matrix and verification below distinguish shared feature availability, controlled browser exercises and native playback evidence.

## Final architecture and persistence

The real `App` remains mounted once. Its state, API commands, backend subscription, data fetching, navigation history, Flow handlers and dialogs serve both styles. `InterfaceContext` carries only the presentation name; `useInterfacePreference` applies it to the document and provides pending/error state. No style-specific playback reducer or fixture data enters production.

The new `SetInterfaceStyle` command accepts exactly `spatial` or `brutalist`. It writes the existing `application_settings` repository under `interface_style`; `GetSettings` returns that value. No database migration, extra preferences file or browser storage is used. Selection applies optimistically, serializes saves, and rolls back visibly if persistence fails. Spatial is the default because it continues the existing warm, artwork-led direction. Style changes do not key/remount the application or route tree. The existing logout reloads remain unrelated to style selection.

`PlaybackStage`, `Artwork`, `QueueView`, `DiscoveryChapters`, windowed renderers and semantic presentation styles consume real data and callbacks. Presentation geometry changes while the functional tree remains shared. A route scroll hook uses refs to restore outer scroll when navigating; changing Interface Style does not change its route key.

Navigation context restoration (2026-10-17): the mounted `ViewMemoryProvider` retains small leaf-owned snapshots when route contents unmount. Discovery remembers the active chapter, independent shelf offsets, For You filter and preview identity; Brutalist Archive remembers album spine offsets. Both toolbar history controls and Alt+Left/Right use the same existing navigation path. WindowedStrip restores after CSS measurement and updates its visible bucket before paint: restoring against its initial estimated width could clamp a saved For You offset from 212px to 72px. Active views remain windowed and inactive views unmount; scrolling does not update root/chrome state. This is in-memory presentation context, not a second navigation or playback history and not disk persistence. Run `node scripts/test-navigation-memory.mjs` for actual wheel selection, every chapter/filter/shelf round trip, chart collection history, ordinary page scroll and Archive record return, in both themes at 1888×1004, 1440×900 and 1024×600.

## Spatial presentation

A narrow navigation rail, detached player, oversized Flow anchor, tilted real artwork, staggered listening shelves, uneven record shelves and a temporal queue. Stable artwork identities move between the receding previous/next positions and the central current position using transform and opacity. Previously observed artwork is presentation memory, not another playback history authority.

Discovery presents the existing Trending Songs, For You, Mixes For You and Top Charts data/actions as chapters of one stage. Vertical wheel gestures or chapter buttons change chapters; horizontal native scrolling, drag, keyboard and arrow controls browse the shelves. Existing genre/similar-artist filters, chart collections, mixes and search remain available. Chapter names reflect real existing content rather than invented recommendation sources.

## Brutalist presentation

The independent approved preview now controls the production composition: numbered horizontal header, 54px search/history toolbar and a persistent right deck. Now Playing is a record poster; Discovery is a single horizontal sleeve wall; Library uses vertical record spines and numbered playlist rows; Queue is a flat ordered list. Home fits real Flow, its circular familiarity dial and the existing overview into an editorial poster. Full screen also uses the Brutalist poster/deck. No left sidebar, detached bottom player, Spatial cover choreography, terminal or telemetry decoration. See DESIGN.md for responsive and feature-placement contracts.

## Integration file map

- `src/App.tsx`: production ownership, interface provider, complete routes/actions, real queue event projection, navigation callbacks and settings wiring. Legacy early-return design routing is removed. Progress snapshots seed the existing leaf emitter; external shuffle state maps from the actual backend DTO. Existing-current-track selections pause/resume rather than rebuilding playback. Playback errors expose feedback and refresh authoritative metadata.
- `src/main.tsx`: normal entry always mounts the real App. Original accepted studies remain explicitly available at `?lab=spatial` and `?lab=brutalist`; their readmes/links explain their sample scope. Their reducers remain isolated.
- `src/presentation/`: presentation context/preference, scroll restoration, real artwork, shared player stage/queue adapter, chapter composition, bounded list/table/gallery/strip renderers, debounced filters and both systems' token/layout styles.
- `Sidebar`, `NowPlayingBar`, `GlobalTopSearchBar`: all navigation destinations, queue/Now Playing access, accessible transport/search controls, leaf progress and debounced input.
- `HomeView`, `DiscoveryView`, `AlbumsView`, `LibraryView`, `CollectionDetailView`, `FullScreenPlayerView`: semantic compositions around existing real features. Album selection retains its original filtered-library destination; playlist/mix/chart collections keep full details/actions.
- `PlaylistsView`, `DownloadsView`, `WishlistView`, `NotificationsView`, `OnlineSearchResultsSection`, `AddToPlaylistModal`, `DownloadOptionsModal`: preserved capabilities with viewport-bounded rendering. Wishlist has native labeled forms, status controls, search, direct-download and Soulseek access.
- `SettingsView`, `types.ts`: Interface Style persistence and the simplified requested settings surface, with dedicated folder/storage sections and account logout.
- Rust production changes are limited to `core/command.rs` and `core/processor.rs` for saving/reading the interface setting. `lifecycle_tests.rs` adds focused persistence/failure/continuity regressions. PlaybackService, Flow, queue semantics, recommendation scoring, MPRIS, tray, single-instance, history/statistics and database schema are unchanged.
- `scripts/test-dual-interface.mjs`, `scripts/test-dual-interface-native.py`, `tests/fixtures/dualInterfaceBackend.mjs`, `tests/interfacePreference.test.mjs`, `tests/interfacePresentation.test.mjs`: reproducible checks. The old untracked lab test now checks the actual three preserved model names rather than stale four-model expectations.

## Completed capability matrix

YES means the real functional capability is available in each presentation. Browser tests exercise representative real component/action paths with controlled IPC. Native verification certifies the local playback/style-switch path. External services are qualified separately below.

| Feature | Spatial | Brutalist |
| --- | --- | --- |
| Home: Flow, Recent Rotation, Rediscover, For You, Library Moment, Your Week | YES | YES |
| Start/continue/pause/stop Flow, steering, Why This Track | YES | YES |
| Discovery, world charts, genre/similar recommendations, mixes, artist searches | YES | YES |
| Online search, categories, local matching, clear and empty results | YES | YES |
| Local/downloaded library, search, rescan, metadata, feedback | YES | YES |
| Album shelves and existing album-to-library detail browsing | YES | YES |
| Playlists, Liked Songs, smart mixes, full collection details | YES | YES |
| Create/save/rename/delete playlists; Spotify inspect/import; missing-track actions | YES | YES |
| Current queue order/index, enqueue local/online, remove, clear | YES | YES |
| Now Playing and full screen; lyrics; origin and explanations | YES | YES |
| Play/pause/resume, previous/next, seek, volume/mute, shuffle/repeat | YES | YES |
| Like/unlike, playlist management, supported online downloads/context actions | YES | YES |
| Loading, buffering, seek availability, offline/empty/error feedback | YES | YES |
| Download search/providers/filter/start/cancel/progress/import/Soulseek | YES | YES |
| Wishlist add/search/filter/status/direct search/Soulseek | YES | YES |
| Notifications, read/dismiss/all actions, toasts | YES | YES |
| Profile/authentication/avatar/logout and statistics | YES | YES |
| Simplified Settings, dedicated folders/storage, close behavior and account logout | YES | YES |
| Interface Style, immediate changes, persistence and failed-save rollback | YES | YES |
| Back/forward, Alt+Left/Right, Space, F11/Esc and route scroll | YES | YES |
| Onboarding, updater and all existing functional dialogs | YES | YES |

## Shared versus style-specific responsibilities

Commands, playback/queue truth, user/library/download/recommendation data, likes, Flow decisions, settings storage, navigation and functional forms/dialogs are shared. Tokens, layout geometry, shelf rhythm, artwork positioning, Discovery chapter/page interaction, navigation treatment and type hierarchy are presentation-specific. Conventional tools such as Settings stay recognizable in each system.

No existing feature was intentionally omitted for aesthetics. The queue exposes the backend's actual actions. General queue reordering and a complete past queue are not existing backend capabilities, so no fabricated reorder/rebuilt queue was introduced. Unknown queued online metadata has an explicit fallback until metadata is available. Album detail navigation retains the original library filtering behavior rather than adding an unsupported backend album query.

## Performance and desktop behavior

Playback position remains in `playbackProgress` and isolated progress/lyrics leaves. No audio element, frontend audio engine, animation clock, mouse-coordinate root state, repeated blur surface or additional style-specific data fetch is introduced. Search/filter state is debounced at 200 ms (Flow steering retains its existing 250 ms command debounce). Containers remain memoized; sidebar navigation callbacks are stable. Async subscription cleanup handles StrictMode disposal before listener registration resolves.

Potentially long libraries, collections, galleries, playlist pickers, search-song lists, downloads, notifications and queue lists mount a bounded viewport. Native horizontal sleeves retain scrolling while mounting only nearby items. Gallery row height follows available artwork width through ResizeObserver; it does not measure continuously. Only transform/opacity animate. The old Discovery backdrop observer and sticky hero are removed from the production composition. Reduced motion disables animation and smooth scrolling.

Verified desktop sizes: 1920×1080, 1440×900, 1024×600 and 800×700. Rails, poster text and Flow composition compress; short windows prioritize the Discovery stage. Footer transport remains accessible, while long pages and virtual lists scroll normally. This work does not introduce a mobile-first layout.

## Verification commands and evidence

Run frontend checks with:

```sh
npm run build
node --test --test-isolation=none --test-reporter=tap tests/*.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --offline --test lifecycle_tests
```

Browser regression:

```sh
npm run dev -- --host 127.0.0.1 --port 5194 --strictPort
node scripts/test-dual-interface.mjs
node scripts/test-hover-stability.mjs
```

The browser script supports `KAZE_TEST_URL`, `KAZE_PLAYWRIGHT_MODULE`, `KAZE_CHROMIUM` and `KAZE_TEST_OUTPUT`. Default output is `/tmp/kaze-dual-interface/`. Its test-only backend module is never imported by production. It checks both interfaces, command routing, same-node switching/no duplicate library fetch, progress isolation, full screen/lyrics/buffering/seek, settings rollback/reload, empty/error states and signature-view layouts at four desktop sizes.

Native continuity verification uses a real embedded debug build, private D-Bus, temporary data/cache and silent WAVs through the normal audio clock:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --offline --bin kaze --features tauri/custom-protocol
KAZE_PRIVATE_DUAL_TEST=1 dbus-run-session -- python3 scripts/test-dual-interface-native.py /absolute/path/to/kaze
```

The test creates only its own processes and data. It checks actual advancing Rust playback, exact paused position, queue/index, current track, Flow generation, MPRIS state, live journal/history identity, route/DOM continuity and cold relaunch persistence. A first null-ALSA-device trial invalidated wall-clock bounds because null output consumed audio too fast; the normal-clock silent-WAV run passed. Do not use a non-real-time null sink to certify elapsed playback timing.

Final results (2026-10-15): frontend build PASS; 38 Node tests PASS; 8 Rust lifecycle/settings tests PASS; 31 browser check groups PASS with zero page errors; native continuity/cold-relaunch checks PASS; whitespace checks PASS. The original isolated Spatial lab still emits the existing mixed static/dynamic Library import bundler warning; it does not block the production build.

The native checks passed all three live style flips, exact paused-position preservation, unchanged journal/history session identity and persisted Brutalist after cold relaunch. No production playback/Flow/MPRIS/tray/single-instance code was changed. Nothing was committed or pushed.

## Remaining verification limits

The controlled browser suite verifies UI/action contracts, not live provider availability or production account mutations. Live online streaming/download providers, Spotify/cloud/auth/avatar services and updater installation require their external services/credentials and were not certified in this run. Native playback tests use silent local audio; they do not certify speaker quality or every remote stream. Windows/macOS, native Xorg and the full cross-compositor desktop lifecycle matrix were not rerun for this frontend change. No existing native lifecycle implementation was modified.


## Brutalist preview restoration (2026-10-17)

The user reported that production Brutalist inherited the old sidebar/bottom player and a broken oversized Flow panel instead of its separate accepted study. Replaced that structural drift with independent production adapters using only the existing App state and callbacks. No study fixture reducer, extra playback state machine, new dependency or production Rust change was introduced. Spatial retains its own composition and the same mounted App/main element survives theme switching.

`BrutalistNavigation.tsx`, `BrutalistDeck.tsx`, `BrutalistPoster.tsx`, `BrutalistHome.tsx`, `BrutalistAlbums.tsx` and `brutalist.css` implement the reference regions. The shared Discovery adapter mounts only the active bin and remembers each shelf. Albums stay windowed horizontally; playlists use a separate numbered index layout. Existing account/photo, Settings, search, filters, downloads, likes, playlists, Flow, explanations, queue, lyrics and fullscreen actions remain wired to production commands.

Removed obsolete Brutalist selectors from `interfaces.css`. Confirmed integration traps were inherited Flow top padding, the old two-column Discovery grid, square aspect/height constraints collapsing sleeve labels, and intrinsic wide-strip sizes expanding the short-window grid. Explicit pane columns/min-width and native scrolling bound those layouts. Compact ticket artwork matches the study's short-window dimensions. Additional deck actions share its options row; the account footer remains pinned while short-window controls scroll.

Validation uses `scripts/test-brutalist-parity.mjs` to compare the untouched live study and production at 1920×1080, 1440×900, 1024×600 and 800×700, plus 390px narrow behavior. Screenshots/geometry are written to `/tmp/kaze-brutalist-restored/`. Verification passes: production build; 42 frontend regressions; 31 shared browser groups; eight Spatial parity groups; nine Spatial interaction/layout groups; six Settings/profile groups; eight direct Brutalist comparison/interaction groups. The native suite covers eleven groups including the independent full screen, actual playback, queue/Flow/MPRIS/history continuity, hide/restore, profile and SQLite persistence. Native checks use silent WAVs, private D-Bus and disposable XDG data; they do not replace the installed app or live profile. The earlier unresolved native memory-growth observation remains open; this layout work does not claim to resolve it. No commit or push.


### Discovery wheel refinement (2026-10-17)

Brutalist now handles vertical wheel gestures as in-place bin navigation, including gestures over record sleeves. One gesture advances or returns one bin, clamped at the first/last bin; the heading and outer page stay fixed. Horizontal and Shift-wheel browsing remain native. In short windows only the shelf region can overflow, keeping the heading in place. The shared handler retains Spatial's existing shelf exception and input/select behavior.

Build, nine Brutalist comparison/interaction groups and nine Spatial interaction/layout groups pass. Coverage includes real vertical wheel events at four desktop sizes, burst gating, both bounds, fixed geometry and native sideways browsing. When comparing against an open reference tab, bring the production tab to the foreground before testing native wheel defaults; synthetic handler events alone do not establish native scrolling behavior.

### Archive Liked tab (2026-10-17)

Brutalist Liked Songs now renders beneath the Archive's persistent heading and tabs, with Liked selected, a compact section title, the existing toolbar and virtualized tracks. The standalone cover/Back hero is omitted only for this presentation. The same account-scoped collection, loader, history and track callbacks remain authoritative; no extra liked-track state or backend route was introduced. Ordinary collections and Spatial retain their existing layouts. Desktop Library scrolling is confined to the content panel, preserving the Archive heading and tabs.

Build, 42 frontend tests and the shared interface browser suite pass. Focused browser checks at 1888×1006, 1440×900, 1024×600 and 800×700 confirm fixed Archive geometry, selected-tab state, back/forward navigation, no playback commands during tab changes, debounced search, unlike/empty behavior and all 1,036 liked tracks reachable with bounded rows. Theme switching retains Spatial's collection presentation and one backend subscription. Native playback and external services were not rerun for this presentation refinement.

### Home adjacent covers (2026-10-17)

Brutalist Home now places smaller previous/next artwork to the left/right of its current poster, with perspective rotation and hard shadows. `HomeView` forwards the existing observed previous track, next queued track and transport callbacks to `BrutalistHome`; no new playback/history state was added. The center toggles playback. Missing neighbors and failed artwork use opaque placeholders, and unavailable neighbor controls are disabled. Spatial and Now Playing keep their own composition.

Focused browser checks pass at 1888×1006, 1440×900, 1024×600, 800×700 and 390×844: actual previous/current/next identities and commands, smaller side-cover geometry, exposed pointer targets, depth, opaque placeholders, keyboard controls, no horizontal page overflow and one backend subscription. The production build and Spatial preview checks pass. The Brutalist comparison suite also includes a permanent adjacent-cover transport/placeholder check.

### Home spotlight and visible artwork fallbacks (2026-10-17)

Library Moment retains its left metadata/right cover layout with stronger album typography, available track/year metadata, a framed clickable sleeve and the existing Open record action. Your Week presents the same three aggregate totals in a ruled typographic layout. Existing recent artwork can supply the featured cover when album/artist match; no fabricated activity data or additional root state was introduced.

`Artwork` marks missing/failed images explicitly. Brutalist renders those states as a contrasting paper sleeve with a dark record glyph, frame and red center, preserving opacity even inside disabled artwork buttons. Discovery cards and collection row thumbnails now use the shared component so broken URLs also reach this fallback. Spatial's own palette and composition remain intact. Focused browser checks at five sizes verified album/discovery navigation, bounded layouts, unchanged totals and missing/broken fallback behavior; the tested sleeve contrast is 9.5:1 against its surrounding frame and 10.9:1 for its icon. Build, frontend tests and Spatial preview checks pass; shared interface checks include the weekly totals and featured fallback.

### Fixed album shelf and footer (2026-10-17)

At 1888×1004 the Albums panel had 505px available but its fixed strip, padding and footer required 518px. Wheel input therefore moved the whole shelf/footer up by 13px. The panel now uses a bounded flex layout with a nonshrinking footer and an adaptable horizontal strip. Normal spines retain their 350px height when space permits; artwork and metadata fit shorter shelves and expose full album/artist tooltips. Very short windows contain any necessary vertical overflow inside the shelf, with compact padding/controls and the Archive heading/tabs/footer fixed.

Build and all ten Brutalist preview/interaction groups pass. The comparison script now tests real vertical wheel input against shelf/footer positions and visible arrows, followed by horizontal browsing and reaching the final one of 130 albums while keeping fewer than thirty spines mounted. A focused five-size check includes the reported 1888×1004 geometry; the original 13px movement is now zero.

### Hover stability, bounded titles and chart accent (2026-10-17)

A repeated pointer probe at a Discovery card's lower edge reproduced six enter/leave cycles in 24 samples: the hovered card was translating its own hit area away from the pointer. Cards and buttons now keep stable bounds; artwork inside them supplies the remaining transform feedback. Spatial retains its static sleeve rotations, Brutalist retains its static perspective, and nested album spine images have explicit transitions. Legacy direct mouse-handler lift/scale mutations were removed from artist/search cards, collection playback and lyrics sync controls.

The long-title fixture previously increased the deck identity area by about 98px, shifting transport and shrinking Up Next to 130px even though its heading and three rows need 171px. Title/artist lines now truncate, identity sizing stays consistent, and Up Next and Flow cannot shrink into each other. The controls still scroll independently of the pinned Settings/Profile footer. Memoized `OverflowTitle` uses local resize/text measurements and a CSS transform to reveal clipped names on hover or keyboard focus, including Now Playing/fullscreen; reduced motion retains the static accessible title. No progress subscriptions or pointer state were added to App.

The generic Brutalist gradient fallback was overriding the old chart bars with the page's charcoal wash. Graph bars now use an explicit class and flat theme accent, with Brutalist red for days with listening time and neutral zero-day marks. The Archive Liked heading remains transparent and compact; generic cover-banner rules target `.collection-hero` rather than an arbitrary first child.

`scripts/test-hover-stability.mjs` exercises fixed deck geometry at five sizes, exposed pointer edges in both themes, title end visibility/keyboard/reduced motion/fullscreen and chart colours. Build, 42 frontend tests and 31 shared browser checks pass. The original Spatial preview comparison also passes. Evidence: `/tmp/kaze-stable-title.png` and `/tmp/kaze-red-graph.png`; fixtures contain no live user data.

### Separate Home lists and F shortcut (2026-10-17)

Recent Rotation, Rediscover and For You now have separate ruled panels, substantial numbered heading bands, descriptive subtitles and actual record counts. Their bounded scroll areas show four rows on desktop and three in narrow windows, with empty sections still clearly identified. The old height limit targeted a class not supplied by the renderer; the actual `react-window` list also supplies an inline `max-height: 100%`. The new scoped role selector overrides that inline limit, and the existing virtualization continues to reach the last of 1,036 test tracks with fewer than twenty mounted rows.

F is a case-insensitive alias for the existing fullscreen controller in both themes. It handles repeat/composition/prevented-event guards, Ctrl/Alt/Meta combinations, editable controls and other dialogs; the fullscreen player's dialog remains eligible for F to exit. Escape and F11 remain available, and button tooltips expose the new shortcut. No playback or queue command is dispatched by this binding.

Build and 42 frontend tests pass. Focused browser checks cover five responsive sizes, distinct list bounds and empty state, a long list's final item, actual browser fullscreen entry/exit in both themes, uppercase and held F, search/contenteditable typing, modifier guards, a real explanation dialog and unchanged playback/Flow/subscription state. The shared browser suite also exercises F alongside the existing fullscreen controls. Native OS fullscreen was not rerun for this frontend shortcut alias.

## Simplified Settings and account action (2026-10-17)

The user requested a smaller top-to-bottom settings surface. The main page now has Interface Style, Window Close Behavior, Music Folders, System & Storage and Account. Folder/storage rows replace the old horizontal tabs and open dedicated sections with keyboard-focus restoration and a Back action. Session view memory retains the selected section on route return. Folder add/remove/rescan still call the existing commands; more than 50 folders use the shared windowed list. System & Storage shows queried temporary-audio usage and the existing protected cache clear action. Technical-only engine/provider/Soulseek/cloud panels, internal file locations, endpoint/security explanations and onboarding test controls are removed from this surface. Backend configuration and automatic services are unchanged.

Log Out moved from Profile to Settings and uses the existing Logout command. A failed logout now stays on Settings with retry feedback instead of unconditionally reloading; successful logout retains the existing reload. Profile avatar/name editing, listening statistics and guest sign-in remain available. Rejected rescan requests release the busy state. Settings is memoized, still eagerly imported, and does not subscribe to progress ticks. Obsolete Settings-only cloud status state/queries and unused manual settings callbacks were removed from App.

Spatial has its own centered warm reading column, soft outlined choices, orange folder icons and compact account row. After the user found the initial oversized controls confusing, both use compact theme choices with radio indicators and an explicit Selected label, no decorative theme thumbnails/captions or tiny sequence numbers, and grouped folder/storage rows with Manage actions. Brutalist independently uses a left-aligned ruled column, compact uppercase section headings, hard edges and red accents. The original previews and player/chrome compositions remain intact. Controls have no moving hover hit areas.

Validation: `npm run build`, 42 Node regressions (including adapted actual-component close-behavior rejection/serialization/refresh tests), `node scripts/test-settings-layout.mjs` and `node scripts/test-settings-profile.mjs`. The layout script uses disposable IPC fixtures for both themes at 1888×1004, 1440×900, 1024×600, 800×700 and 390×844, checks outer/detail controls, section return, 80-folder windowing, real command routing and failure recovery, theme switches without playback change, Settings-only logout/retry and guest sign-in. The profile script still blocks late Settings module requests and checks current photo updates/fallbacks, account/statistics entry and playback equality. Screenshots regenerate under `/tmp/kaze-settings-simple/`. Native account/filesystem/cache writes and cross-platform runtime checks were not rerun for this frontend change.

## Logout scope choice (2026-10-17)

Settings Log Out now opens a native modal with This device, All devices and Cancel. The choices call the existing Logout or LogoutAll command through one App callback and reload only after success. Opening, Cancel and Escape send no account command. Focus starts on Cancel, Tab/Shift+Tab stay in the dialog, and dismissal returns focus to the trigger. Pending requests disable both scopes and dismissal and use a ref guard against duplicate calls. Failures stay in the dialog with retry feedback. Both themes style the modal independently; the backdrop is flat and no playback or root pointer/progress state was added.

A confirmed backend correctness issue was fixed with this request: LogoutAll discarded errors from the cloud client and cleared the local session, falsely succeeding when the server rejected global revocation. It now requires an active session/token and awaits successful server confirmation before local credential/session/playback cleanup. This-device Logout retains its existing local/offline behavior. No new authentication endpoint or queue authority was introduced.

Validation: production build, 42 Node tests, the 18 Settings browser groups (now exercising both scope commands, three dialog window sizes, keyboard/cancel/error/pending/retry behavior and successful reload), and two final dialog visual/keyboard checks pass. The existing Rust `test_session_management_lifecycle_and_hybrid_expiry` passes with a rejecting local auth-worker response: session metadata and credentials remain on failure, then successful LogoutAll clears the session while retaining stored user data. Run it under private D-Bus and disposable XDG data with `cargo test --offline --test stats_and_account_tests test_session_management_lifecycle_and_hybrid_expiry -- --test-threads=1`. No live device sessions were revoked; native GUI and other platforms were not retested. Modal screenshots: `/tmp/kaze-logout-brutalist.png` and `/tmp/kaze-logout-spatial.png`.


## Queue frame-stall workaround (2026-10-18)

The user confirmed that the page-scroll mitigation still froze. It is reverted: Spatial Queue keeps the existing `WindowedList`, artwork composition and row actions. `PageWindowedList.tsx` and its structural test were removed. Nested scroll ownership was a hypothesis, not the cause established by this investigation.

The stuck normal-profile window provided a stronger signal: the inspector reported Library and a focused Library button while a compositor screenshot still showed Queue. Timers and JavaScript continued; `requestAnimationFrame` delivered zero frames for one second even after explicitly focusing the visible native window. Hit tests were correct, controls were enabled and no overlay intercepted input. Resizing/fullscreen produced only two frames before stalling again. Kernel ptrace restrictions prevented attaching a native debugger; no security settings were changed.

This host runs WebKitGTK 2.54.1 on Niri/Wayland with NVIDIA 615.78.08 loaded, and the renderer loads NVIDIA EGL/GBM/Wayland libraries on its hybrid graphics setup. A controlled [same-binary A/B experiment](performance/queue-frames-comparison.json) reproduced zero Home frames immediately with the default renderer. Changing only `WEBKIT_DISABLE_DMABUF_RENDERER=1` kept frames advancing through all six Queue/Home round trips. This establishes a working rendering-path workaround on this host; the exact upstream driver/engine defect is not traced.

`src-tauri/src/linux_rendering.rs` now configures that setting before GTK/Tauri, backend threads and activation-token exec. Apply only when a Wayland session and loaded NVIDIA module are detected, skip an explicitly selected X11 backend, and preserve any existing value of the WebKit variable. Other operating systems do not compile this module. The change does not disable application-wide GPU acceleration or alter playback state. Three environment-policy tests cover Wayland/hybrid detection, other sessions and explicit overrides.

Verification: production frontend and embedded native builds, 42 Node tests and all three policy tests pass. The [final native frame regression](performance/queue-frames-native.json) passes six round trips with the original Queue layout and no externally supplied workaround variable: all twelve focused frame probes advance, real Rust artwork requests run, queue rows remain bounded, inner list offset reaches 400px, and no JavaScript errors occur. The harness uses native PageDown for outer scrolling and explicitly sets the inner scroll offset because WebKit's PageDown targets the outer page despite list focus. It does not certify native mouse-wheel targeting. Playback events are simulated; the normal-profile window is separately reopened for the user's real music/Flow check. The user then confirmed that the reopened normal-profile window stays responsive after starting music/Flow, opening and scrolling What’s next, then clicking Home. The earlier native private-memory growth remains unresolved.

Repeat the opt-in native test on Niri, with Python GI and wtype installed:

```sh
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --offline --features tauri/custom-protocol --bin kaze
KAZE_PRIVATE_QUEUE_TEST=1 dbus-run-session -- python3 scripts/test-queue-frames-native.py /absolute/path/to/embedded/kaze --artwork-database /explicit/local/library.db --output /tmp/kaze-queue-frames.json
```

The test reads at most 21 local-track metadata rows from the explicitly supplied database read-only, copies them into disposable XDG state and runs on a private D-Bus session. It does not read account tables or send live playback/account commands. Embedded tests must use `tauri://localhost`: a directly launched development binary can stay at about:blank, and an HTTP frontend changes native capability checks. Test actual frame delivery while the native window is focused; successful DOM navigation, low CPU and responsive inspector calls do not rule out a stale displayed frame.

## Saved theme before the first app frame (2026-10-18)

The stored Brutalist choice was correct, but App mounted with null settings and the preference hook started as Spatial. Its asynchronous settings effect then replaced an already visible Spatial shell. The regression is reproduced by holding the `GetSettings` response while observing the real app's first shell commit.

Production entry now loads the App chunk and existing SQLite settings concurrently, showing a neutral startup surface until both are ready. It applies the document theme before mounting App, passes the same settings snapshot into App, and initializes the preference hook from that snapshot. The old duplicate initial settings fetch is removed; explicit Settings refresh remains available. A failed load offers Try again without mounting a guessed theme. Spatial remains the default only when no preference is stored. The two standalone labs bypass production startup queries. There is no browser theme cache, new backend command, preference migration or playback subscription change.

Verification: `npm run build`, 43 Node regressions, six focused browser startup cases and two embedded WebKitGTK cases pass. `node scripts/test-interface-startup.mjs` checks delayed settings, both first-commit themes, absent preference, failure/retry, Strict Mode request deduplication and lab isolation. `KAZE_PRIVATE_THEME_TEST=1 dbus-run-session -- python3 scripts/test-interface-startup-native.py /absolute/embedded/kaze` checks cold launches against real disposable SQLite settings and instruments a full document reload with delayed actual IPC responses to observe the first shell commit. The broader shared browser run passed startup, switching/playback continuity, all Spatial groups and the first Brutalist groups, then stopped at an obsolete lookup for the user-removed Brutalist “Library and more” menu. That navigation test limitation is separate from the verified startup fix; no navigation surface was changed here.

## Branded startup animation (2026-10-18)

The user clarified that the name should fade softly from the start of the word to the end, like flowing into view on air, then requested theme-appropriate versions with similar motion in Brutalist too. `StartupScreen.tsx` reuses the 256px native icon. Both versions use overlapping 1.2-second word fades, 140ms spacing, a slight lateral/upward drift and two-second introduction. Spatial keeps circular motion, warm charcoal and bold spaced lettering. Brutalist matches its own matte surface, square frame with extending red rules, heavier compact lettering and indeterminate red loading sweep. There are no typing steps or cursor. All animation uses transform/opacity, with one mount-only completion timer. Reduced motion shows the complete theme-specific still composition and skips the minimum introduction time. A single loading announcement replaces per-letter announcements.

`main.tsx` initially shows only a neutral static icon while the existing `GetSettings` query and App module load concurrently. The saved preference chooses the intro as soon as it is known; animation overlaps remaining App loading. App receives the same settings snapshot when both the intro and module are ready. There is no guessed theme, duplicate settings query, browser preference or backend change. Errors retain the existing retry surface and cancel any late settings-to-intro continuation, preventing a delayed response from reviving a failed startup. The timer and CSS animation unmount on success or error.

Build and all 43 Node regressions pass. The startup browser script covers twelve scenarios plus four animation checks: distinct compositions/motion, bold wording, fixed geometry, stable accessible text, image loading, each intro duration, both reduced-motion fallbacks, delayed settings, delayed App code, late-response cancellation, missing preference, retry and lab isolation. Screenshots are regenerated under `/tmp/kaze-startup-animation/`. The native script checks the static pre-preference phase, actual saved-theme intro, focused loading-indicator/frame progress, icon decoding, each word reveal and first saved-theme shell. These tests use disposable state and do not modify the installed app or live account.
