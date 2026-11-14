# Design tokens: one source for shared styling values

Colors, fonts, sizes and spacing were hardcoded across CSS and inline TSX styles. The theme
tokens themselves (`--bg-main`, `--accent`, `--text-main`, …) were defined in five files.
Hardcoded values ignored the active interface: the base text color `#e8d8c9` was written
out 109 times in TSX, so those spots kept the old color in Spatial and Brutalist. Accent
tints such as `rgba(243,112,30,.2)` stayed orange in Brutalist's red theme.

Decision:

| Area | Rule |
| --- | --- |
| Source | CSS custom properties in `src/styles/tokens.css`, imported first. It holds the knobs, scales, layers, palette and every interface theme block, in cascade order. |
| Usage | CSS and inline styles use `var(--token)`; TSX uses the same strings. There is no TypeScript mirror, so there is nothing to drift. `tests/designTokens.test.mjs` fails on a `var(--name)` that is never defined. |
| Knobs | `--font-scale`, `--space-scale`, `--radius-scale` and `--motion-scale` multiply every step, so one edit retunes the whole app. |
| Scales | Step names are their size at scale 1 (`--fs-13`, `--space-12`, `--radius-8`). Values within 1–2px of a step were snapped to it once, during the migration. One-off values use a unit multiple (`calc(188 * var(--space-unit))`), so they still follow the knobs. |
| Colors | Theme colors with alpha use derived tokens (`--accent-a20`, `--text-main-a15`) built with `color-mix` on `:root, .kaze-interface`, so they follow each interface. White and black tints are `--overlay-N` and `--shade-N`. One-off colors are named once in the palette section. |
| Layers | Global z-index layers (10 and up) are named `--z-*`, keeping their previous values and order. Local stacking below 10 stays in components. |
| Enforcement | `npm run audit:conventions` counts hardcoded values per file (`scripts/style-literals.mjs`). New files must have none; recorded legacy files may only shrink. |

Consequences: hardcoded theme colors now follow the active interface, which is an
intended visual change. Snapping moved some sizes and spacings by 1–2px. Component
geometry used in one place (sidebar width, strip item width) stays as local custom
properties on the component.

Alternatives: a TypeScript token object (it can't reach CSS files and duplicates the
source), or Sass variables (compile-time only, so no runtime theme or knob changes).
