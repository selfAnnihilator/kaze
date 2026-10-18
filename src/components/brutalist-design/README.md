> Production integration: the normal Kaze app now selects Spatial or Brutalist in Settings. This folder remains an isolated sample study at `?lab=`.

# Kaze, brutalist design study

An independent frontend preview at `?lab=brutalist`. The spatial direction at `?lab=spatial` is preserved. The existing Kaze Lab switcher links to both; the brutalist header links back to Spatial, and its toolbar arrow returns to the production frontend with `model=standard`.

Run `npm run dev` and open the local URL with `?lab=brutalist`. The separately running review server uses `http://127.0.0.1:5189/?lab=brutalist`.

## Direction

Swiss industrial print, inspired by record sleeves and printed music catalogues, now on a matte black substrate: off-white type, charcoal structural rules, a single warm red accent and hard edges. Native controls use a dark color scheme. No gradients, gloss, glass, backdrop filters, animated meters, fabricated waveforms or decorative telemetry. Large numerals reflect the actual sample queue position or catalogue counts.

The player occupies a persistent right control column. Now Playing is a record poster with a large track title, actual cover, artist and next-track tickets. Discover is a native horizontal sleeve wall, fitted to the desktop pane. Library is an index of vertical record spines, with track, playlist and liked sections. Collections show the complete sample album selection. Queue exposes earlier/later and remove controls. Short or narrow windows use native scrolling where needed; the narrow layout has a compact current-track link to the deck.

## Scope

This is a frontend-only design exploration. It reuses the existing spatial sample catalogue and local artwork as read-only inputs; it imports none of the spatial components or styles. Both labs are selected in `src/main.tsx` before mounting the production `App`. Neither lab runs its IPC subscriptions or accesses the real database, playback, session, Flow, MPRIS, tray, or single-instance implementation.

The reducer owns a separate sample queue, playing state, likes, shuffle, repeat and Flow sequence. Controls update the prototype only. There is no audible playback or automatic sample clock. Track durations are fixtures. Flow selects a finite sample sequence; it does not run the production recommendation engine. Next/previous are manual, and repeat wraps the sample queue. Selecting a record or playlist replaces the sample selection; playing an individual track selects it in the current queue or appends it.

Seek and volume state are isolated in memoized leaves. Header/navigation and other containers do not receive position updates. No intervals or repeated animation loops run. Search waits 200 ms before filtering. Track lists are windowed with the installed `react-window` 2.x dependency, including collection, search and queue. Native range controls, accessible labels, keyboard library tabs, visible focus and reduced-motion support remain available.

## Controls

- Space outside interactive controls toggles sample playback. `/` focuses search.
- The red deck button, next/previous, shuffle, repeat, like, volume and seeking are interactive.
- Discover has four sample selections. Browse covers with a touchpad, touch, mouse drag, arrow buttons or Left/Right while the wall is focused. Dragging suppresses accidental playback. A cover click plays its sample track; its title opens the album.
- Library tabs support Left/Right and Home/End. Album spines open records; playlist buttons load the entire sample playlist.
- Queue changes apply only to future tracks. Clearing upcoming tracks retains the current track and history within the sample queue.

Artwork attribution remains in `../spatial-design/artwork.json` and its README. Images are existing local display fixtures, not new generated art. No runtime external artwork requests are made.

## Verification

Run `npm run build`. Check the complete navigation and interactions in a browser, including desktop, short and narrow layouts, reduced motion, queue edits, empty search/queue, and the escape links. No native/backend changes are required for this study.
