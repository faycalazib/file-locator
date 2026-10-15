# Themes

> Updated 2026-10-15. **32 themes**: 9 written by hand (below) and 23 generated from the user's reference images (`scripts/themegen.py`, section "Generated themes"). The hand-written ones: **`crayons`** ("Colored pencils", light, the default), **`sonar`** ("Sonar", dark neon) and **`clay`** ("Modeling clay", light), **`carnet`** ("Notebook", light), **`scrapbook`** ("Scrapbook", light), **`soft`** ("Soft app", light), **`brush`** ("Brush", light), **`electric`** ("Electric", dark) and **`gold`** ("Gold mine", dark). They are chosen with the palette button at the bottom of the rail (`ThemeMenu.svelte`): a menu in two columns, light and dark (`settings.themesLight` / `themesDark`), lists every theme of `THEMES` with a swatch and its name, and scrolls beyond 72 % of the window. Every mockup of `docs/mockups/` is now a theme; more themes can be added (see "Adding a theme").

## Principle

The visual layer is separated from the components:

| File | Contents |
| --- | --- |
| `ui/src/styles/tokens.css` | Structure only: type scale, spacing, motion durations |
| `ui/src/styles/themes/<name>.css` | Colors, fonts, treatments, all under `:root[data-theme="<name>"]` |
| `ui/src/styles/fonts.css` | Local `@import`s of the fonts (Fontsource) |
| `ui/src/lib/components/ThemeDefs.svelte` | SVG filters used by the themes (`#wobble`, `#rough`) |
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
| PROSPECTOR title | `--title-stroke`, `--title-mix`, `--title-mix-pct`, `--title-shadow`; optional `--title-color` gives every letter one color (otherwise each letter has its own) |
| Brand font | `--font-brand` (title and panel headers, never digits) |
| Letter case | `--display-case` (optional): `uppercase` by default (labels, buttons, headers); `none` for a theme written in sentence case. Components write `text-transform: var(--display-case, uppercase)`, never `uppercase` alone |
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
- The list of themes, their chrome and the swatch of the menu (`[page, panel, color 1, color 2]`) live in `THEMES`, in `stores/ui.svelte.ts`. The menu shows them in that order.

## Adding a theme

**The quick way: a generated theme.** Add a spec to `SPECS` in `scripts/themegen.py` (id, names in 4 languages, light/dark, swatch, colors, fonts, material, page background), add its fonts to `FONTS` if they are new (`pnpm add @fontsource/<font>`), then run `python scripts/themegen.py`. It writes `themes/<id>.css`, the font imports (between markers at the end of `fonts.css`), the `import` in `main.ts`, the `THEMES` entry and the names, and reports any color below 4.5:1. Then build and take the screenshots (step 4 below). It can be run again safely.

**By hand**, for a theme with its own treatments (wobble, brush, clay…):

1. Create `themes/<name>.css` that honors the contract, then import it in `main.ts`.
2. Add its fonts to `fonts.css`. Check that each one is legible: render `Il1 O0 C 5S 8B 3 V/Y 2024-11` in it.
3. Add `{ id, chrome, swatch }` to `THEMES` (and the id to the `Theme` type), and the i18n key `settings.themeNames.<name>` in all 4 languages.
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

## Theme `clay` (light)

Based on mockups A and H (`docs/mockups/a-clay-spotlight.html`, `h-clay-arabe.html`, its Arabic version). Added on 2026-10-15.

- **Palette.** Cream `#FBF3E6`, panels `#FFFDF8`, cocoa text `#3D2B2F` (13:1) and `#6E5C60` (6.1:1). Pastels: lilac `#A98BEA` (accent), pink `#F47FA9`, mint `#7FD1A6`, butter `#FFD36B`, sky `#86C3EF`.
- **Fonts.** Fredoka (brand and titles), Nunito (text, digits included), DM Mono (paths); Baloo Bhaijaan 2 in Arabic. Checked on `Il1 O0 C 5S 8B 3 V/Y 2024-11`: `I` and `l` alike in Fredoka and Nunito, as in most sans-serif fonts, the rest distinct.
- **Treatments.** No wobble. Every `.sketch` gets round corners (its background too) and a clay relief: a light rim on top, a darker one below, a lilac shadow under it; the outline stays faint unless a component colors it (`--k`). Flat pastel fills instead of hatching. Soft colored blobs behind the page. Matches as a butter highlighter band, the active one pink.
- **Text.** Sentence case (`--display-case: none`). The PROSPECTOR title has solid letters (`--title-mix-pct: 100%`) with a soft shadow. The 3D magnifier and gem doodles are shown.
- **Screenshots.** `target/tmp/themes/clay-<layout>-<lang>.png` (3 layouts × FR / AR) and `menu-*.png` (the theme menu).

## Theme `carnet` (light)

Based on mockup B (`docs/mockups/b-carnet.html`). Added on 2026-10-15.

- **Palette.** Cream page `#FFFDF6`, panels `#FFFEF9`, ink `#2F2A3A` (13.9:1) and `#625A70` (5.9:1). Felt pens: pink `#E8558A` (accent), violet `#7B5FD6` (secondary and the query), green `#3FA65F`, blue `#3B82D6`, orange `#F08A24`. Highlighters: yellow `#FFE16B`, pink `#FFB8CF`, lilac `#D6C6FF`, green `#B8EBC4`.
- **Fonts.**
  - Caveat is for the brand only (PROSPECTOR title, panel headers): at chip size it gets too small to read.
  - Mali is for everything else, digits included, a handwriting with distinct `I`, `l` and `1`.
  - Courier Prime is for paths.
  - Baloo Bhaijaan 2 is for Arabic, then Mali for the Latin words of an Arabic page.
  - Kalam, from the mockup, was dropped: its `I`, `l` and `1` are the same stroke, so « 1 263 » read « I 263 ».
- **Treatments.**
  - Outlines are ink strokes with the `#wobble` filter and corners cut by hand.
  - Fills look like highlighter strokes with ragged ends.
  - The page is a notebook: blue rules every 34 px and a double pink margin on the start side (right in Arabic).
  - Doodles are shown, and cards tilt slightly (0.4°).
- **Text.** Sentence case (`--display-case: none`). The title has solid felt-pen letters. The rail is 300 px wide, because Mali is wide.
- **Screenshots.** `target/tmp/themes/carnet-<layout>-<lang>.png`, `menu-carnet-fr.png`.

## Theme `scrapbook` (light)

Based on mockup D (`docs/mockups/d-scrapbook.html`). Added on 2026-10-15.

- **Palette.** Pink-white page `#FFF8FB` with four pastel color clouds in the corners, white panels, black marker `#151319` (18.4:1) and `#5D5763` (6.6:1). Loud markers: pink `#FF2E7E` (accent), violet `#8A4DFF` (secondary and the query), blue `#2F8CFF`, green `#36C46A`, orange `#FF8A1F`, yellow `#FFD60A`.
- **Fonts.**
  - Anton is for the brand, titles, buttons and labels: condensed capitals, `1` with a flag.
  - Patrick Hand is for the text (shared with `crayons`).
  - Courier Prime is for paths.
  - Baloo Bhaijaan 2 is for Arabic, then Patrick Hand for the Latin words.
  - Permanent Marker, from the mockup, was dropped: at chip size its `1` reads as `I` ("Excel I").
- **Treatments.** Thick straight marker frames (3 px), corners cut unevenly (`16px 6px`), flat marker fills, hard black sticker shadows on pop-ups, cards tilted 0.6°, doodles shown.
- **Text.** Capitals kept (the default `--display-case`). The title has solid colored capitals with a hard black shadow. Matches use a yellow marker underline; the active one is a pink sticker with white text (5:1).
- **Screenshots.** `target/tmp/themes/scrapbook-<layout>-<lang>.png`, `menu-scrapbook-fr.png`.

## Theme `soft` (light)

Based on mockup E (`docs/mockups/e-soft-app.html`). Added on 2026-10-15. The calm, "work" theme.

- **Palette.** Cool page `#F6F7FB` with two pale color circles, white panels, ink `#1F2440` (15.3:1) and `#5A6080` (6.2:1). Teal `#16978C` (accent), violet `#7C5CFF` (secondary), green, blue, pink, orange, butter yellow.
- **Fonts.** Baloo 2 for the brand and panel headers; Nunito Sans for everything else, digits included; JetBrains Mono for paths; Baloo Bhaijaan 2 for Arabic, then Nunito Sans for the Latin words.
- **Treatments.** Round corners (backgrounds too), a hairline outline and a soft shadow. A component's color (`--k`) only tints the hairline (40 %): the mockup has almost no outlines. Flat soft fills. Matches as a butter tag; the active one teal with white text (5.2:1).
- **Text.** Sentence case (`--display-case: none`). The title has solid letters with a soft shadow. The 3D magnifier and gem are shown.
- **Screenshots.** `target/tmp/themes/soft-<layout>-<lang>.png`, `menu-soft-ar.png`.

## Theme `brush` (light)

Based on mockup F (`docs/mockups/f-brush-banners.html`). Added on 2026-10-15.

- **Palette.** Warm white `#FFFDFB` (page and panels), ink `#1D1A22` (16.9:1) and `#625B6C` (5.4:1). Pastel paint: pink `#F9C6D3`, lilac `#DCCDFA`, yellow `#FBEAA1`, mint `#C4EFD4`, blue `#C3E3FB`; their strong versions for strokes and small text (`--*-ink`).
- **Fonts.** Caveat Brush for the brand and panel headers (it gets narrow at chip size); Itim for everything else, digits included; Courier Prime for paths; Baloo Bhaijaan 2 for Arabic, then Itim.
- **Treatments.** Every `.sketch` gets a pastel band painted behind its content (`isolation: isolate`, `::before` at `z-index: -1`), 20 % of its component's color (`--k`, lilac by default) with a stronger edge, both frayed by the SVG filter `#rough` (ThemeDefs.svelte). Flat pastel fills. Matches as a pink brush stroke under the word.
- **Text.** Sentence case (`--display-case: none`). Solid painted title letters. Doodles shown, cards tilted 0.5°.
- **To watch.** `#rough` is computed on every band: check that scrolling a long list stays smooth (TESTS-A-FAIRE).
- **Screenshots.** `target/tmp/themes/brush-<layout>-<lang>.png`, `menu-brush-fr.png`.

## Theme `electric` (dark)

After a reference image sent by the user on 2026-10-15 (an electric-car charging app). Added the same day.

- **Palette.** Near-black page `#050507` with a deep blue glow from above, anthracite cards `#16171C`, text `#ECECF1` (15.4:1) and `#A8A8B8` (7.6:1). Electric blue `#3B6CFF` (accent), cyan `#35D0E6` (secondary), vivid icon colors (green, yellow, pink, orange, violet) with light `*-ink` versions for small text.
- **Font.** Inter everywhere (digits included), JetBrains Mono for paths; Baloo Bhaijaan 2 for Arabic, then Inter.
- **Treatments.** Spaced cards: round corners, a hairline tinted by the component's color (45 %) and a glow of that color (`box-shadow: 0 0 24px -10px var(--k)`). Translucent fills. Matches tinted blue, the active one solid blue with white text (4.7:1). No doodles.
- **Title.** Solid white letters (`--title-color: #fff`) with a blue glow. Sentence case.
- **Screenshots.** `target/tmp/themes/electric-<layout>-<lang>.png`.

## Theme `gold` (dark)

"Gold mine": the app's own story (digging, nuggets). Proposed by Claude, chosen by the user on 2026-10-15.

- **Palette.** Dark earth `#0E0B08` lit by two lanterns, rock panels `#1A140E`, cream text `#F3E9D6` (15:1) and `#C4B59C` (8.2:1). Gold `#F2C14E` (accent), copper `#E08A4B` (secondary), gems for the other colors: emerald, sapphire, ruby, amethyst.
- **Fonts.** Cinzel (engraved capitals) for the brand and panel headers only; Outfit for everything else, digits included; JetBrains Mono for paths; Baloo Bhaijaan 2 for Arabic. Rye (western poster) was dropped: too heavy. The rail is 300 px wide (Cinzel is wide).
- **Treatments.** Rock edges tinted by the component's color (50 %), a thin gold rim lit from above, deep shadows, a faint rock grain on the page. Matches as gold nuggets (dark text), the active one copper. The 3D gem doodle is shown.
- **Title.** Solid gold letters (`--title-color: #f2c14e`) with an engraved shadow and a lantern glow.
- **Screenshots.** `target/tmp/themes/gold-<layout>-<lang>.png`, `menu-gold-fr.png` (the menu with 9 themes).

## Generated themes

Made on 2026-10-15 and 16 from 25 reference images sent by the user (near duplicates merged: the two navy-glass apps, AirPods and Moon Play, the two plant shops in green). Each one is a spec in `scripts/themegen.py`; the generator checks the contrast of the text, the dim text, the small colored text (`*-ink`), the matches and the code colors (all at least 4.5:1). Screenshots: `target/tmp/themes/<id>-<layout>-<lang>.png`, checked in the 3 layouts, FR and AR.

Fonts added for them, each checked on `Il1 O0 C 5S 8B 3 V/Y 2024-11`: Lora, Manrope, Playfair Display (brand only: its old-style figures make "1" look like "I"), DM Serif Display, Josefin Sans (brand only), DM Sans, Urbanist, Plus Jakarta Sans, Oxanium, Rajdhani, Barlow, IBM Plex Sans, and Amiri for the Arabic of `noor`.

| Id | Name (FR) | Light / dark | Reference | Fonts (brand / text) | Material |
| --- | --- | --- | --- | --- | --- |
| `leafora` | Feuillage | dark | Leafora (plant shop, dark sage glass) | Lora / Manrope | radius 22px, outline 35 % of the panel color |
| `potea` | Pousse | dark | Potea (plant shop, charcoal and vivid green) | Urbanist / Urbanist | radius 16px, outline 30 % of the panel color |
| `neural` | Interface neurale | dark | Neural Profit Interface (lime HUD) | Oxanium / Oxanium | radius 4px, one fixed outline color, glow, capitals |
| `nebula` | Nébuleuse | dark | AI assistant and Futuristic Glass UI (indigo glass, violet and pink glow) | Outfit / Outfit | radius 22px, outline 55 % of the panel color, glow |
| `lagoon` | Lagon | dark | Plant app (deep teal and emerald) | Manrope / Manrope | radius 26px, outline 40 % of the panel color |
| `cosmo` | Cosmo | dark | Plixie (space education game, vivid purple and yellow) | Fredoka / Nunito | radius 24px, outline 30 % of the panel color |
| `greenhouse` | Serre | light | Plant care app (frosted sage glass, light) | Plus Jakarta Sans / Plus Jakarta Sans | radius 24px, outline 25 % of the panel color |
| `noor` | Noor | light | Noor (Islamic companion app: cream, forest green, gold, serif) | Playfair Display / Lora | radius 22px, outline 30 % of the panel color, Arabic in Amiri |
| `orchard` | Verger | light | Fruit nutrition app (pastel mint to peach) | DM Sans / DM Sans | radius 22px, outline 20 % of the panel color |
| `candy` | Bonbon | light | Liquid glass pastel widgets (glossy candy buttons) | Fredoka / Nunito | radius 28px, outline 35 % of the panel color |
| `cocoon` | Cocon | light | Music dashboard (cozy cream, sage, peach) | Baloo 2 / Nunito | radius 22px, outline 18 % of the panel color |
| `marine` | Marine | dark | AutoHub (navy glass, blue bokeh) | Plus Jakarta Sans / Plus Jakarta Sans | radius 18px, outline 40 % of the panel color, glow |
| `daylight` | Lumière du jour | light | Daylight (wellness: off-white, periwinkle, butter, navy) | Josefin Sans / DM Sans | radius 20px, outline 25 % of the panel color |
| `cloud` | Nuage | light | Recenter (wellness: sky, clouds, mint, serif) | DM Serif Display / Nunito | radius 24px, outline 20 % of the panel color |
| `lavender` | Lavande | light | AirPods shop and Moon Play (lilac and deep violet) | Plus Jakarta Sans / Plus Jakarta Sans | radius 22px, outline 25 % of the panel color |
| `frost` | Givre | light | Login screen (neumorphism, soft blue) | Manrope / Manrope | radius 18px, outline 10 % of the panel color, neumorphic shadows |
| `starry` | Nuit étoilée | dark | Sleep app (indigo night, stars, yellow) | Lora / Manrope | radius 22px, outline 35 % of the panel color, glow |
| `aurora` | Aurore | dark | Glazzed dashboard (blurred color gradient, dark panels) | Nunito Sans / Nunito Sans | radius 4px, outline 30 % of the panel color |
| `sport` | Élan | dark | Running app (cyan to indigo gradient, thin type) | Barlow / Barlow | radius 6px, outline 30 % of the panel color, capitals |
| `fleet` | Tableau de bord | dark | Fleet analytics dashboard (navy, panels glowing in their color) | Inter / Inter | radius 14px, outline 100 % of the panel color, glow, panel tinted |
| `command` | Centre de commande | dark | Online income command center (navy-black, cyan neon, uppercase) | Rajdhani / Rajdhani | radius 8px, one fixed outline color, glow, capitals |
| `telemetry` | Télémétrie | dark | Airframe system status (drone telemetry: dark terrain, translucent cards, green nominal, orange warn) | IBM Plex Sans / IBM Plex Sans | radius 10px, outline 25 % of the panel color, capitals |
| `onyx` | Onyx | dark | Payro (finance app: pure black, dark cards, colored categories, amber and teal) | DM Sans / DM Sans | radius 18px, outline 22 % of the panel color |
