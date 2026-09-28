/**
 * Date, attribute and digest filters (lot 5.8), shared by the search
 * request, the filter chips of the masthead and the PDF report.
 */
import { formatShortDate, t, type MessageKey } from './i18n/index.svelte';
import type { Attribute, DateField, DateFilter, Detector } from './types';

const DAY = 24 * 60 * 60 * 1000;

/** Presets: how far back they go. */
export const DATE_WINDOWS: Record<Exclude<DateFilter, 'any' | 'custom'>, number> = {
  day: DAY,
  week: 7 * DAY,
  month: 30 * DAY,
  year: 365 * DAY,
};

export const DATE_FIELDS: DateField[] = ['modified', 'created', 'accessed'];
export const ATTRIBUTES: Attribute[] = ['readOnly', 'hidden', 'system'];

export interface DateCriteria {
  date: DateFilter;
  dateField: DateField;
  /** `YYYY-MM-DD` (date input), `''` = open. */
  dateFrom: string;
  dateTo: string;
}

/** `2024-03-31` → that day's local midnight (ms); null when empty or invalid. */
function dayStart(value: string): number | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return null;
  const date = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return Number.isNaN(date.getTime()) ? null : date.getTime();
}

/** `[from, to)` in ms; null = open. A custom "to" day is included whole. */
export function dateBounds(f: DateCriteria, now = Date.now()): { from: number | null; to: number | null } {
  if (f.date === 'any') return { from: null, to: null };
  if (f.date !== 'custom') return { from: now - DATE_WINDOWS[f.date], to: null };
  const to = dayStart(f.dateTo);
  return { from: dayStart(f.dateFrom), to: to === null ? null : to + DAY };
}

export function hasDateFilter(f: DateCriteria): boolean {
  const { from, to } = dateBounds(f, 0);
  return f.date !== 'any' && (from !== null || to !== null);
}

/** "Créé · 7 derniers jours", "Modifié · 01/01/2024 → 31/03/2024". */
export function dateLabel(f: DateCriteria): string {
  const field = t(`filters.dateFields.${f.dateField}` as MessageKey);
  let range: string;
  if (f.date !== 'custom') {
    range = t(`filters.dates.${f.date}` as MessageKey);
  } else {
    const from = dayStart(f.dateFrom);
    const to = dayStart(f.dateTo);
    const show = (ms: number) => formatShortDate(new Date(ms));
    if (from !== null && to !== null) range = t('filters.dateRange', { from: show(from), to: show(to) });
    else if (from !== null) range = t('filters.dateSince', { from: show(from) });
    else range = t('filters.dateUntil', { to: to === null ? '…' : show(to) });
  }
  return t('filters.dateToken', { field, range });
}

/** What a digest looks like: MD5 (32 hex), SHA-256 (64), invalid, or none. */
export function hashKind(value: string): 'md5' | 'sha256' | 'invalid' | null {
  const hex = value.replace(/\s+/g, '');
  if (hex === '') return null;
  if (!/^[0-9a-f]+$/i.test(hex)) return 'invalid';
  return hex.length === 32 ? 'md5' : hex.length === 64 ? 'sha256' : 'invalid';
}

/** `3fa2…9c01`: a digest shortened for a chip. */
export function shortHash(value: string): string {
  const hex = value.replace(/\s+/g, '').toLowerCase();
  return hex.length > 12 ? `${hex.slice(0, 6)}…${hex.slice(-4)}` : hex;
}

/** Detectors (lot 6.2), by family, in the order of the filter panel. */
export const DETECTOR_GROUPS: { group: 'bank' | 'contact' | 'business' | 'identity'; detectors: Detector[] }[] = [
  { group: 'bank', detectors: ['iban', 'bic', 'rib', 'card'] },
  { group: 'contact', detectors: ['email', 'phone', 'ip'] },
  { group: 'business', detectors: ['vat', 'siren', 'amount'] },
  { group: 'identity', detectors: ['nir', 'dni', 'mrz'] },
];

export const ALL_DETECTORS: Detector[] = DETECTOR_GROUPS.flatMap((g) => g.detectors);

/**
 * The PDF report masks what looks like an account or identity number: in a
 * highlighted passage with at least 8 digits, only the last 4 characters
 * stay (`•••• •••• •••• 4242`). E-mails and short amounts are kept.
 */
export function maskSensitive(marked: string): string {
  return marked.replace(/⟦([^⟧]*)⟧/g, (whole, inner: string) => {
    if ((inner.match(/\p{Nd}/gu) ?? []).length < 8) return whole;
    const chars = [...inner];
    let keep = 4;
    for (let i = chars.length - 1; i >= 0; i--) {
      if (!/[\p{L}\p{Nd}]/u.test(chars[i]!)) continue;
      if (keep > 0) keep--;
      else chars[i] = '•';
    }
    return `⟦${chars.join('')}⟧`;
  });
}
