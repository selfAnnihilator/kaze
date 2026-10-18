> Production integration: the normal Kaze app now selects Spatial or Brutalist in Settings. This folder remains an isolated sample study at `?lab=`.

# Kaze spatial design lab

One frontend-only direction on `design/full-ui-exploration`. No production UI was removed. The older experiments are retained.

## Open and leave

Run `npm run dev` and open `http://localhost:5173/?lab=spatial`, or use the **Spatial design** link in the existing Kaze Lab switcher. A separately started review server may use another port, such as 5189.

Use the **Current Kaze** back-arrow icon in the left rail to return to `?model=standard`. The `design` query parameter is removed. Opening the app without `design=spatial` continues to use the existing frontend.

The entry is selected before mounting `App`. Production IPC subscriptions and effects are never mounted by the lab. Both entries are lazy-loaded.

## Scope and sample controls

This is an interactive visual prototype with a finite sample catalogue: 12 real album covers and 36 sample tracks. Durations and playlists are fixtures. There is no audio, backend command, database access, MPRIS integration, or modification of production playback/Flow state.

Play/pause, track selection, next/previous, liking, queue ordering, playlist/album selection and Flow state are local preview interactions. Seeking and volume stay in isolated leaves. Seek position changes manually and resets on track selection; it does not run an artificial playback clock. Shuffle rearranges the upcoming sample selection; repeat cycles through its visible states and repeat-all permits wrapping the sample queue. The preview does not simulate automatic track completion.

## Design system

A listener is browsing at a dim desktop between work and an evening listening session. A warm charcoal field keeps cover colour in front; restrained orange identifies active controls. Typography uses the existing self-hosted Inter asset, with a large tightly spaced display treatment and readable sans-serif music metadata.

All new styles live under `.spatial-kaze`. Semantic OKLCH variables define background, raised surfaces, text, muted text, borders and accent. Artwork has nearly square edges, ordinary controls use a 6px radius, and the transport's primary buttons are circular. There are no decorative glass surfaces or blurred backdrops.

## Spatial grammar

| Surface     | Composition                                                                                                                                                                                                                                                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Home / Flow | A large asymmetric anchor, overlapping real sleeves, staggered recent rotation, and an offset rediscovery section.                                                                                                                                                                                                                                                                                         |
| Now Playing | A viewport-fitted stage never scrolls vertically. A central current sleeve and smaller past/future sleeves move between roles with stable keys and CSS transforms.                                                                                                                                                                                                                                         |
| Discover    | Two fixed panes replace content in place. Page Up/Down and vertical wheel gestures select a chapter; the small left-facing vertical Scroll down control advances through chapters. The large cover has a song/artist caption instead of a thumbnail catalogue. Hovering or focusing right-hand covers updates the artwork, caption and details without playback. Horizontal music browsing remains native. |
| Library     | A repeating mixed-size shelf composition, including a large featured sleeve. Tracks and liked music use a virtual list; playlists use larger editorial rows.                                                                                                                                                                                                                                               |
| Collection  | Oversized artwork is offset beside large album typography. The track sheet cuts across the sleeve's lower area. Album/playlist play loads the complete sample selection.                                                                                                                                                                                                                                   |
| Queue       | A present sleeve leads into a descending future path, followed by a virtual ordered list. Native drag-and-drop and explicit earlier/later buttons preserve ordering.                                                                                                                                                                                                                                       |
| Transport   | A detached surface groups artwork/metadata, compact playback controls, a separate seek line, volume and queue access.                                                                                                                                                                                                                                                                                      |

## Performance and access

- No intervals, scroll listeners, automatic sample clocks, synthetic audio, or high-frequency shell state.
- Drag coordinates, velocity and animation-frame IDs stay in refs. Only a short momentum coast updates native scroll position, and new pointer/keyboard/arrow input takes over immediately.
- Discover and Now Playing fit the available pane without vertical scrolling. Discover consumes one chapter change per vertical wheel gesture outside horizontal strips; there is no scroll loop or translated vertical content. Horizontal strips remain native.
- Track and queue rows use the existing `react-window` 2.x dependency. Artwork fixtures and music strips are bounded.
- Navigation, transport, artwork, cards and main view components are memoized. Seek and volume changes do not rerender the shell.
- Only transform and opacity animate. Reduced motion disables transitions and momentum. Search waits 200ms before filtering.
- The icon-only rail uses tooltips and accessible names. Drag its right edge to resize between 56 and 160px; the focused divider also supports arrow keys and Home/End. Width is saved in a preview-only preference. Pointer moves write the grid width directly, outside shell React state.
- Controls have names and focus treatment. Queue ordering works without dragging. The narrow layout retains transport and navigation.
- Local artwork files reserve square image dimensions and load lazily outside the main artwork. There are no runtime requests to external artwork providers.

## Artwork sources

`artwork.json` records the Apple Music catalogue page for each local cover. The images depict the named albums and remain the property of their respective rights holders; they are sample display assets, not newly generated artwork or a new Kaze asset identity. The source records include [Migration](https://music.apple.com/us/album/migration/1169245868), [Modal Soul](https://music.apple.com/us/album/modal-soul/1078914477), [Con Todo El Mundo](https://music.apple.com/us/album/con-todo-el-mundo/1299241642), and [Currents](https://music.apple.com/us/album/currents/1440838039).

## Check

Run `npm run build`. Review Home, Discover, Library, a collection, Now Playing and Queue at a practical desktop window size. Use the separate sample mode label to distinguish this exploration from a player connected to real playback.
