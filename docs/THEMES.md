# Themes

> Updated 2026-09-24. Available themes: **`crayons`** ("Colored pencils", light, the default) and **`sonar`** ("Sonar", dark neon). They are chosen in Settings → Theme.

## Principle

The visual layer is separated from the components:

| File | Contents |
| --- | --- |
| `ui/src/styles/tokens.css` | Structure only: type scale, spacing, motion durations |
| `ui/src/styles/themes/<name>.css` | Colors, fonts, treatments, all under `:root[data-theme="<name>"]` |
| `ui/src/styles/fonts.css` | Local `@import`s of the fonts (Fontsource) |
| `ui/src/lib/components/ThemeDefs.svelte` | SVG filters used by the themes (`#wobble`) |
| `ui/index.html` | `<html data-theme="crayons">` (default theme) |

Components never contain a raw color or font. They use:

- the **semantic variables** listed below;
- the **treatment classes** `.sketch` (outline) and `.hatch` (fill);
- a few local modulation variables: `--k` (stroke color), `--h` (hatching color), `--sw` (stroke width).

## Contract for a theme

Every theme must define:

| Group | Variables |
| --- | --- |
| Surfaces | `--bg`, `--surface`, `--line` |
| Text | `--text`, `--text-dim` (≥ 4.5:1), `--text-faint` (decoration only), `--query-ink` |
| Colors | `--accent`, `--secondary`, `--ok`, `--warn`, `--danger`, `--purple`, `--yellow` |
| Readable colors (small text, ≥ 4.5:1) | `--accent-ink`, `--secondary-ink`, `--ok-ink`, `--warn-ink`, `--danger-ink` |
| Matches | `--mark-bg`, `--mark-text`, `--mark-active-bg`, `--mark-active-text` (a background may be a gradient) |
| Fonts | `--font-display`, `--font-body`, `--font-mono`, plus their Arabic version under `:root:lang(ar)` |
| Strokes | `--stroke`, `--r-sketch`, `--wobble`, `--focus-ring` |
| Fills (`--h` values) | `--fill-accent`, `--fill-accent-strong`, `--fill-secondary`, `--fill-ok`, `--fill-warn`, `--fill-yellow` |
| Effects | `--shadow-pop`, `--tilt` (card tilt), `--doodle-opacity`, `--sweep-bg`, `--ping-bg` |
| PROSPECTOR title | `--title-stroke`, `--title-mix`, `--title-mix-pct`, `--title-shadow` |
| Brand font | `--font-brand` (title and panel headers, never digits) |
| Layout | `--rail-w` (optional, 280 px by default) |
| Treatments | `.sketch::before` and `.hatch`. A "flat" theme can turn them into a plain border and a solid fill |

**Trap: `--wobble` must stay a valid filter.** It is combined with others (`filter: var(--title-shadow) var(--wobble)`), and `none` is invalid inside a list. A theme without the effect uses `opacity(1)`.

**Scoping.** Every treatment of a theme (`.sketch::before`, `.hatch`, `body` background) is prefixed with `[data-theme='<name>']`, so two themes never overlap.

## How the switch works

- `ui.initTheme()` is called in `main.ts` **before mount**, so the first frame already uses the right theme (no flash).
  Theme source, in order: `?theme=` in the URL (screenshots), then `localStorage["prospector.theme"]`, then `crayons`.
- `ui.setTheme(id)`:
  - sets `<html data-theme>`;
  - stores the choice;
  - aligns the native Tauri window chrome through `getCurrentWindow().setTheme('light' | 'dark')`. This needs the `core:window:allow-set-theme` permission in `src-tauri/capabilities/default.json`, and does nothing outside Tauri.
- The list of themes and their chrome lives in `THEMES`, in `stores/ui.svelte.ts`.

## Adding a theme

1. Create `themes/<name>.css` that honors the contract, then import it in `main.ts`.
2. Add its fonts to `fonts.css`. Check that each one is legible: render `Il1 O0 C 5S 8B 3 V/Y 2024-11` in it.
3. Add `{ id, chrome }` to `THEMES` and the i18n key `settings.themes.<name>` in all 4 languages.
4. Take a screenshot of the 3 layouts, in FR and AR, with headless Chrome (`?theme=<name>&lang=ar&v=ledger`) before delivering.

## Theme `crayons` (light, default)

The full spec is in goal.md §5. Mockup: `docs/mockups/g-crayons.html`. Screenshots: `docs/screenshots/crayons-*.png`.

## Theme `sonar` (dark)

Based on mockup C (`docs/mockups/c-sonar-quest.html`). Screenshots: `docs/screenshots/sonar-*.png`.

- **Palette.** Night `#120D24`, panels `#1B1436`, text `#ECEBFF` (14.6:1) and `#B3ACD9` (7.9:1). Neon: cyan `#35E6D8`, pink `#FF4FD8`, yellow `#FFD23F`, green `#69F58C`, orange `#FF8A3D`, violet `#9B7BFF`.
- **Fonts.**
  - Tiny5 (pixel) is for the brand only: title and panel headers.
  - Chakra Petch is for everything else, digits included.
  - JetBrains Mono is for paths.
  - Pixelify Sans, from the mockup, was dropped: its "C" looked like "O" and its "3" like "8" (BUG-013).
- **Treatments.** Outlines are straight neon strokes with a halo (no wobble). Fills are flat and translucent instead of hatched. The page has a dot grid and a violet glow. No doodles (`--doodle-opacity: 0`), no card tilt.
- **Motion.** A cyan sweep while digging, and a yellow flash on each new find.
- **Layout.** The rail is 300 px wide, because Chakra Petch is wider than Patrick Hand.
