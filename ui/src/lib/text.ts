import type { MarkedText } from './types';

export interface Segment {
  text: string;
  hit: boolean;
  /** Found through typo tolerance only (⟪ ⟫ from the engine). */
  fuzzy: boolean;
}

/** ⟦exact⟧ or ⟪approximate⟫ (see core/src/highlight.rs). */
const MARK = /⟦([^⟧]*)⟧|⟪([^⟫]*)⟫/g;

/** Splits a marked string into plain / match segments. */
export function segments(marked: MarkedText): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const m of marked.matchAll(MARK)) {
    const start = m.index ?? 0;
    if (start > last) out.push({ text: marked.slice(last, start), hit: false, fuzzy: false });
    const fuzzy = m[2] !== undefined;
    out.push({ text: (fuzzy ? m[2] : m[1]) ?? '', hit: true, fuzzy });
    last = start + m[0].length;
  }
  if (last < marked.length) out.push({ text: marked.slice(last), hit: false, fuzzy: false });
  return out;
}

export function plain(marked: MarkedText): string {
  return marked.replace(MARK, (_, exact, fuzzy) => exact ?? fuzzy ?? '');
}

/** Arabic diacritics (tashkeel) and tatweel. */
const ARABIC_MARKS = /[ً-ٰٟـ]/g;

/**
 * Preview of the §5bis-B "generic" analyzer: lowercase, accents and Arabic
 * vowel marks removed, alef/ya/ta marbuta variants unified. The real
 * implementation lives in the Rust core (Étape 1); this one only drives mocks.
 */
export function normalize(text: string): string {
  return text
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(ARABIC_MARKS, '')
    .replace(/[إأآ]/g, 'ا')
    .replace(/ى/g, 'ي')
    .replace(/ة/g, 'ه')
    .normalize('NFC');
}

/** Splits a path into folder + file name (supports `\`, `/` and ` › `). */
export function splitPath(path: string): { folder: string; name: string } {
  const inner = path.lastIndexOf(' › ');
  const sep = Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/'));
  const cut = inner > sep ? inner + 3 : sep + 1;
  return { folder: path.slice(0, cut), name: path.slice(cut) };
}

/**
 * Bidi isolation (first-strong isolate … pop): a query or a file name in
 * another script does not reorder the sentence around it ("« عقد » · 5 files").
 */
export function isolate(text: string): string {
  return `\u2068${text}\u2069`;
}
