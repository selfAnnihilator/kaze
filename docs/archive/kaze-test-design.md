# Kaze — test branch design

Implemented 2026-09-15 on the existing `test` branch. Earlier uncommitted presentation work was preserved when switching from `prod`; no commits or pushes were made by this task.

## Design and implementation

- `src/kaze-theme.css` supplies the current charcoal/rose palette and responsive presentation, layered after `src/index.css`.
- Self-hosted Cormorant Garamond headings, Inter UI/body, and Caveat handwriting are in `public/fonts/`, with individual SIL Open Font License files. Sources: the corresponding `ofl/cormorantgaramond`, `ofl/inter`, and `ofl/caveat` directories in https://github.com/google/fonts.
- `public/kaze-backdrop.png` is the supplied artwork with baked-in lettering removed. The sticky hero stays at its initial scroll position; opaque discovery content scrolls over it. A ResizeObserver accommodates the search area's size. No scroll listener or animation library is used.
- `public/kaze-icon.png` is the supplied icon recolored to ivory and dusty rose; native PNG sizes are in `src-tauri/icons/`.
- `src/components/ListeningRail.tsx` renders current artwork and the existing local upcoming queue. It is hidden below 1191px; the persistent bottom playback controls remain available. Online playback has no exposed queue here, so no local queue is presented as online upcoming tracks.
- Hero controls delegate to the existing unified playback handlers. Sidebar playlist shortcuts use the existing playlist playback handler. No new backend commands or persistence changes were introduced.
- Screens use actual data and truthful empty states, rather than the sample songs and playlists in the supplied template.

## Verification

Frontend and desktop builds, plus `git diff --check`, are the release checks. Browser verification covered 1440×1000, 1200×800, and 900×600, checking fixed hero coordinates, content overlap, and horizontal overflow. Library, Albums, Playlists, Notifications, and Settings navigation passed. All three fonts loaded while the discovery view was visible. Browser-only sample playback verified `Pause` and `PlayTrack` for the selected queued track. Native audio playback and other OSes were not exercised. The previously recorded browser-only Profile error is outside this design change.

## Generated asset provenance

Built-in image generation/editing tool used, not the CLI. Original provided artwork/logo are preserved separately; current output paths are listed above.

Backdrop prompt:

> Edit this supplied landscape for use as the actual Kaze music app backdrop. Remove ALL text and typographic decoration (KAZE, Japanese letters, English words, divider rules) and seamlessly reconstruct the underlying scenery. Preserve the original composition, seated figure, ruined circular stone arch, vines, mountains, lake, sunset peach pink and muted mauve sky, delicate shooting star, dark charcoal textures and grain. No new elements. Landscape 1536x1024. Return one clean artwork, no text or UI.

Icon prompt:

> Edit this exact app icon for Kaze's dark charcoal and muted dusty rose music player theme. Preserve the exact musical note silhouette, flowing two ribbon flags, ink brush circle, wind marks, rounded square and composition. Change only the color/material finish: replace cyan glow with subtle desaturated dusty rose (#bb918a), bright white with warm ivory (#e8ded5), near-black matte charcoal (#111514) background with faint paper grain. Understated, soft, elegant, no bright glow, no orange, no text. Square icon filling the image, no large external margin.
