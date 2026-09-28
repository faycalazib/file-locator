/**
 * Minimal i18n runtime (goal.md §5bis-A) — no dependency.
 * - Dotted keys, type-checked against `en.json` (reference locale).
 * - ICU plurals through `Intl.PluralRules` (Arabic uses the 6 categories).
 * - `{name}` placeholders; numbers are formatted in the active locale.
 * - `<html lang dir>` follows the active locale (RTL for Arabic).
 */
import en from './locales/en.json';
import fr from './locales/fr.json';
import es from './locales/es.json';
import ar from './locales/ar.json';

export type Locale = 'en' | 'fr' | 'es' | 'ar';

type PluralForms = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };
type MessageTree = { [key: string]: string | PluralForms | MessageTree };

/** Every dotted key of the reference catalog, e.g. `results.matches`. */
type KeyOf<T, P extends string = ''> = {
  [K in keyof T & string]: T[K] extends string
    ? `${P}${K}`
    : T[K] extends { other: string }
      ? `${P}${K}`
      : KeyOf<T[K], `${P}${K}.`>;
}[keyof T & string];

export type MessageKey = KeyOf<typeof en>;
export type Params = Record<string, string | number>;

export const LOCALES: readonly { code: Locale; native: string; dir: 'ltr' | 'rtl' }[] = [
  { code: 'en', native: 'English', dir: 'ltr' },
  { code: 'fr', native: 'Français', dir: 'ltr' },
  { code: 'es', native: 'Español', dir: 'ltr' },
  { code: 'ar', native: 'العربية', dir: 'rtl' },
];

const catalogs: Record<Locale, MessageTree> = { en, fr, es, ar };

/**
 * BCP 47 tags handed to `Intl`. Arabic keeps Latin digits (`nu-latn`) so that
 * counts, sizes and dates read the same as the file paths shown next to them.
 */
const INTL_TAG: Record<Locale, string> = {
  en: 'en',
  fr: 'fr',
  es: 'es',
  ar: 'ar-u-nu-latn',
};

const STORAGE_KEY = 'prospector.locale';
const FALLBACK: Locale = 'en';

function isLocale(value: unknown): value is Locale {
  return typeof value === 'string' && value in catalogs;
}

/** Stored choice → OS/browser languages → English. */
function detectLocale(): Locale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    /* storage unavailable: fall through to detection */
  }
  const candidates = typeof navigator === 'undefined' ? [] : navigator.languages ?? [navigator.language];
  for (const tag of candidates) {
    const base = tag.toLowerCase().split('-')[0];
    if (isLocale(base)) return base;
  }
  return FALLBACK;
}

const state = $state<{ locale: Locale }>({ locale: FALLBACK });

function applyToDocument(locale: Locale) {
  const meta = LOCALES.find((l) => l.code === locale);
  document.documentElement.lang = locale;
  document.documentElement.dir = meta?.dir ?? 'ltr';
}

function lookup(tree: MessageTree, key: string): string | PluralForms | undefined {
  let node: string | PluralForms | MessageTree | undefined = tree;
  for (const part of key.split('.')) {
    if (node === undefined || typeof node === 'string') return undefined;
    node = (node as MessageTree)[part];
  }
  if (node === undefined || typeof node === 'string') return node;
  return 'other' in node && typeof node.other === 'string' ? (node as PluralForms) : undefined;
}

const pluralRules = new Map<Locale, Intl.PluralRules>();
const numberFormats = new Map<Locale, Intl.NumberFormat>();

function plural(locale: Locale, count: number): Intl.LDMLPluralRule {
  let rules = pluralRules.get(locale);
  if (!rules) {
    rules = new Intl.PluralRules(INTL_TAG[locale]);
    pluralRules.set(locale, rules);
  }
  return rules.select(count);
}

export function formatNumber(value: number): string {
  const locale = state.locale;
  let fmt = numberFormats.get(locale);
  if (!fmt) {
    fmt = new Intl.NumberFormat(INTL_TAG[locale]);
    numberFormats.set(locale, fmt);
  }
  return fmt.format(value);
}

function interpolate(template: string, params: Params | undefined): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = params[name];
    if (value === undefined) return match;
    return typeof value === 'number' ? formatNumber(value) : value;
  });
}

/**
 * Translates `key` in the active locale. Reading `state.locale` makes every
 * template that calls `t()` re-render when the language changes.
 */
export function t(key: MessageKey, params?: Params): string {
  const locale = state.locale;
  const message = lookup(catalogs[locale], key) ?? lookup(catalogs[FALLBACK], key);
  if (message === undefined) {
    if (import.meta.env.DEV) console.warn(`[i18n] missing key "${key}"`);
    return key;
  }
  if (typeof message === 'string') return interpolate(message, params);
  const count = typeof params?.count === 'number' ? params.count : 0;
  const form = message[plural(locale, count)] ?? message.other;
  return interpolate(form, params);
}

export const i18n = {
  get locale(): Locale {
    return state.locale;
  },
  get dir(): 'ltr' | 'rtl' {
    return LOCALES.find((l) => l.code === state.locale)?.dir ?? 'ltr';
  },
  get intlTag(): string {
    return INTL_TAG[state.locale];
  },
  /** Reads `?lang=` (screenshots), then the stored/detected locale. */
  init() {
    const fromUrl = new URLSearchParams(location.search).get('lang');
    state.locale = isLocale(fromUrl) ? fromUrl : detectLocale();
    applyToDocument(state.locale);
  },
  set(locale: Locale) {
    state.locale = locale;
    applyToDocument(locale);
    try {
      localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      /* preference simply not persisted */
    }
  },
};

// ── Formatters (sizes, dates) ──────────────────────────────────────────

const BYTE_UNITS = ['byte', 'kilobyte', 'megabyte', 'gigabyte', 'terabyte'] as const;

export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < BYTE_UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return new Intl.NumberFormat(INTL_TAG[state.locale], {
    style: 'unit',
    unit: BYTE_UNITS[unit],
    unitDisplay: 'short',
    maximumFractionDigits: value < 10 && unit > 0 ? 1 : 0,
  }).format(value);
}

export function formatDate(date: Date): string {
  return new Intl.DateTimeFormat(INTL_TAG[state.locale], {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(date);
}

export function formatShortDate(date: Date): string {
  return new Intl.DateTimeFormat(INTL_TAG[state.locale], { dateStyle: 'short' }).format(date);
}

const RELATIVE_STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['second', 60],
  ['minute', 60],
  ['hour', 24],
  ['day', 30],
  ['month', 12],
  ['year', Number.POSITIVE_INFINITY],
];

export function formatRelative(date: Date, now: Date = new Date()): string {
  const fmt = new Intl.RelativeTimeFormat(INTL_TAG[state.locale], { numeric: 'auto' });
  let delta = (date.getTime() - now.getTime()) / 1000;
  for (const [unit, size] of RELATIVE_STEPS) {
    if (Math.abs(delta) < size) return fmt.format(Math.round(delta), unit);
    delta /= size;
  }
  return fmt.format(Math.round(delta), 'year');
}

export function formatPercent(ratio: number): string {
  return new Intl.NumberFormat(INTL_TAG[state.locale], { style: 'percent' }).format(ratio);
}
