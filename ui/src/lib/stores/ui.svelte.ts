/** UI state: layout, theme (Settings), open panels. */
import { flushSync } from 'svelte';
import { api, inTauri } from '../api';
import type { KeywordReport } from '../export';
import type { Variant } from '../types';

const VARIANTS: readonly Variant[] = ['journal', 'ledger', 'strata'];
const LAYOUT_KEY = 'prospector.layout';

export type Theme =
  | 'crayons'
  | 'sonar'
  | 'clay'
  | 'carnet'
  | 'scrapbook'
  | 'soft'
  | 'brush'
  | 'electric'
  | 'gold'
  | 'leafora'
  | 'potea'
  | 'neural'
  | 'nebula'
  | 'lagoon'
  | 'cosmo'
  | 'greenhouse'
  | 'noor'
  | 'orchard'
  | 'candy'
  | 'cocoon'
  | 'marine'
  | 'daylight'
  | 'cloud'
  | 'lavender'
  | 'frost'
  | 'starry'
  | 'aurora'
  | 'sport'
  | 'fleet'
  | 'command'
  | 'telemetry'
  | 'onyx';

/**
 * Available themes, the native window chrome that goes with each, and the
 * swatch of the theme menu (page, panel, two main colors). A new theme:
 * docs/THEMES.md, "Adding a theme".
 */
export const THEMES: readonly { id: Theme; chrome: 'light' | 'dark'; swatch: readonly [string, string, string, string] }[] = [
  { id: 'crayons', chrome: 'light', swatch: ['#fffefa', '#f5c93a', '#ee5a8e', '#3b9be0'] },
  { id: 'sonar', chrome: 'dark', swatch: ['#120d24', '#1b1436', '#ff4fd8', '#35e6d8'] },
  { id: 'clay', chrome: 'light', swatch: ['#fbf3e6', '#fffdf8', '#a98bea', '#f47fa9'] },
  { id: 'carnet', chrome: 'light', swatch: ['#fffdf6', '#dbe6f3', '#e8558a', '#7b5fd6'] },
  { id: 'scrapbook', chrome: 'light', swatch: ['#fff8fb', '#ffd60a', '#ff2e7e', '#2f8cff'] },
  { id: 'soft', chrome: 'light', swatch: ['#f6f7fb', '#ffffff', '#16978c', '#ffc93c'] },
  { id: 'brush', chrome: 'light', swatch: ['#fffdfb', '#dccdfa', '#f9c6d3', '#c4efd4'] },
  { id: 'electric', chrome: 'dark', swatch: ['#050507', '#16171c', '#3b6cff', '#35d0e6'] },
  { id: 'gold', chrome: 'dark', swatch: ['#0e0b08', '#1a140e', '#f2c14e', '#e08a4b'] },
  { id: 'leafora', chrome: 'dark', swatch: ['#1e2a22', '#2a382e', '#a8cf8e', '#e6d58a'] },
  { id: 'potea', chrome: 'dark', swatch: ['#181a20', '#1f222a', '#12d18e', '#4c9bff'] },
  { id: 'neural', chrome: 'dark', swatch: ['#020402', '#060c06', '#8cff3c', '#3cffd2'] },
  { id: 'nebula', chrome: 'dark', swatch: ['#070a1f', '#11163a', '#8b6bff', '#ff5fcf'] },
  { id: 'lagoon', chrome: 'dark', swatch: ['#0c2c2e', '#13403f', '#3fe0b5', '#5fc8f0'] },
  { id: 'cosmo', chrome: 'dark', swatch: ['#5a2ee6', '#4321c4', '#ffd23f', '#3ad1ff'] },
  { id: 'greenhouse', chrome: 'light', swatch: ['#e7efe6', '#f6faf4', '#4f8a5c', '#d6b54a'] },
  { id: 'noor', chrome: 'light', swatch: ['#f6efe4', '#fffaf2', '#2f5d4b', '#c9a24a'] },
  { id: 'orchard', chrome: 'light', swatch: ['#e2f5e6', '#fde3d6', '#2f8f57', '#f08a3c'] },
  { id: 'candy', chrome: 'light', swatch: ['#f3e9f7', '#fdf8ff', '#8f6be8', '#f070a8'] },
  { id: 'cocoon', chrome: 'light', swatch: ['#f8efe4', '#fffaf3', '#e8735f', '#7fae95'] },
  { id: 'marine', chrome: 'dark', swatch: ['#071226', '#0f1f3d', '#3d8bff', '#5fd0ff'] },
  { id: 'daylight', chrome: 'light', swatch: ['#f7f5fb', '#ffffff', '#6f78cf', '#f5d46a'] },
  { id: 'cloud', chrome: 'light', swatch: ['#cfe7ff', '#fbfdff', '#2c7a52', '#5aa9e6'] },
  { id: 'lavender', chrome: 'light', swatch: ['#efeafc', '#ffffff', '#6c48d8', '#e86fb0'] },
  { id: 'frost', chrome: 'light', swatch: ['#e6ecf8', '#e6ecf8', '#2f6bff', '#7a5cff'] },
  { id: 'starry', chrome: 'dark', swatch: ['#0c1140', '#161c5a', '#ffc34d', '#9a7bff'] },
  { id: 'aurora', chrome: 'dark', swatch: ['#2a3a6a', '#222c4d', '#4a9bff', '#ff6b8a'] },
  { id: 'sport', chrome: 'dark', swatch: ['#1fb5d6', '#22337f', '#19c2ff', '#ff5fa8'] },
  { id: 'fleet', chrome: 'dark', swatch: ['#0b1630', '#10203f', '#2fe07f', '#2f9bff'] },
  { id: 'command', chrome: 'dark', swatch: ['#050b14', '#0a1626', '#19d3ff', '#ffd23f'] },
  { id: 'telemetry', chrome: 'dark', swatch: ['#141716', '#1c201f', '#7fd88f', '#ff7a45'] },
  { id: 'onyx', chrome: 'dark', swatch: ['#0b0b0c', '#18181a', '#f5b83d', '#2cc5b4'] },
];
const THEME_KEY = 'prospector.theme';
const DEFAULT_THEME: Theme = 'crayons';

function isVariant(value: unknown): value is Variant {
  return typeof value === 'string' && (VARIANTS as readonly string[]).includes(value);
}

function isTheme(value: unknown): value is Theme {
  return THEMES.some((t) => t.id === value);
}

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* not persisted */
  }
}

/** URL param (screenshots) → stored choice → default. */
function initial<T>(param: string, key: string, guard: (v: unknown) => v is T, fallback: T): T {
  const fromUrl = new URLSearchParams(location.search).get(param);
  if (guard(fromUrl)) return fromUrl;
  const stored = readStorage(key);
  return guard(stored) ? stored : fallback;
}

/**
 * Aligns the native Tauri window chrome (title bar) with the theme. Outside
 * Tauri (browser, headless screenshots) this does nothing.
 */
async function syncWindowChrome(chrome: 'light' | 'dark') {
  if (!('__TAURI_INTERNALS__' in window)) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setTheme(chrome);
  } catch {
    /* chrome stays as configured in tauri.conf.json */
  }
}

class UiStore {
  readonly variants = VARIANTS;
  readonly themes = THEMES;
  variant = $state<Variant>(initial('v', LAYOUT_KEY, isVariant, 'journal'));
  theme = $state<Theme>(DEFAULT_THEME);
  /** Filters stay folded away until the user asks to refine (progressive disclosure). */
  // Browser demo: `?filters` opens the panel (screenshots only).
  filtersOpen = $state(!inTauri && new URLSearchParams(location.search).has('filters'));
  // `?settings` opens the panel in the browser demo (screenshots only).
  settingsOpen = $state(!inTauri && new URLSearchParams(location.search).has('settings'));
  /** Export or copy window (lot 6.4). `?export` / `?copy` open it in the browser demo. */
  transfer = $state<'export' | 'copy' | null>(
    inTauri ? null : new URLSearchParams(location.search).has('export') ? 'export' : new URLSearchParams(location.search).has('copy') ? 'copy' : null,
  );
  /** Duplicates window (lot 6.5). `?dupes` opens it in the browser demo. */
  duplicates = $state(!inTauri && new URLSearchParams(location.search).has('dupes'));
  /** Keyword table of the PDF report (lot 6.4), fetched just before printing. */
  reportKeywords = $state.raw<KeywordReport | null>(null);
  /** The search report is mounted (printing). `?report` shows it in the browser demo. */
  printing = $state(!inTauri && new URLSearchParams(location.search).has('report'));

  /**
   * Report of the search on screen: the system print dialog, which offers
   * "Save as PDF". The report is rendered first (synchronously), then printed;
   * `afterprint` unmounts it (App.svelte).
   */
  async printReport() {
    flushSync(() => (this.printing = true));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    if (inTauri) await api.printReport();
    else window.print();
  }

  setVariant(variant: Variant) {
    this.variant = variant;
    writeStorage(LAYOUT_KEY, variant);
  }

  /** Called before mount, so the first frame already has the right theme. */
  initTheme() {
    this.#applyTheme(initial('theme', THEME_KEY, isTheme, DEFAULT_THEME));
  }

  setTheme(theme: Theme) {
    this.#applyTheme(theme);
    writeStorage(THEME_KEY, theme);
  }

  #applyTheme(theme: Theme) {
    this.theme = theme;
    document.documentElement.dataset.theme = theme;
    void syncWindowChrome(THEMES.find((t) => t.id === theme)?.chrome ?? 'light');
  }
}

export const ui = new UiStore();
