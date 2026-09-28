/** UI state: layout, theme (Settings), open panels. */
import { flushSync } from 'svelte';
import { api, inTauri } from '../api';
import type { KeywordReport } from '../export';
import type { Variant } from '../types';

const VARIANTS: readonly Variant[] = ['journal', 'ledger', 'strata'];
const LAYOUT_KEY = 'prospector.layout';

export type Theme = 'crayons' | 'sonar';

/** Available themes and the native window chrome that goes with each. */
export const THEMES: readonly { id: Theme; chrome: 'light' | 'dark' }[] = [
  { id: 'crayons', chrome: 'light' },
  { id: 'sonar', chrome: 'dark' },
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
