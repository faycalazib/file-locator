/**
 * Rich preview (Étape 3): the engine sends Excel and PowerPoint text as lines
 * (`— Sheet —` / `— 3 —` titles, cells separated by tabs, see
 * core/src/extract/office.rs). These helpers rebuild sheets (tables) and
 * slides (cards) from those lines, keeping the matches and their
 * document-wide index (`occ`) for the ↑ ↓ navigation.
 */
import type { SyntaxToken } from './syntax';

export interface Seg {
  text: string;
  hit: boolean;
  fuzzy: boolean;
  /** Index of the match in the document, -1 for plain text. */
  occ: number;
  parts: SyntaxToken[];
}

export interface SegLine {
  n: number;
  segs: Seg[];
}

/** A line of a sheet or slide, or `gap`: lines skipped by a long preview. */
export type Row = { kind: 'row'; n: number; cells: Seg[][] } | { kind: 'gap'; n: number };

export interface Section {
  /** `null` for text before the first title. */
  title: Seg[] | null;
  n: number;
  rows: Row[];
  /** Widest row (tables). */
  columns: number;
}

const TITLE = /^— (.+) —$/;

const plain = (segs: Seg[]) => segs.map((s) => s.text).join('');

/** Removes `head` characters at the start and `tail` at the end. */
function trim(segs: Seg[], head: number, tail: number): Seg[] {
  const total = plain(segs).length;
  const out: Seg[] = [];
  let pos = 0;
  for (const seg of segs) {
    const from = Math.max(head - pos, 0);
    const to = Math.min(total - tail - pos, seg.text.length);
    if (to > from) out.push({ ...seg, text: seg.text.slice(from, to) });
    pos += seg.text.length;
  }
  return out;
}

/** Splits a line on `separator`; a match cut in two keeps its index. */
function split(segs: Seg[], separator: string): Seg[][] {
  const cells: Seg[][] = [[]];
  for (const seg of segs) {
    seg.text.split(separator).forEach((piece, i) => {
      if (i > 0) cells.push([]);
      if (piece) cells[cells.length - 1]!.push({ ...seg, text: piece });
    });
  }
  return cells;
}

/** Groups lines under their `— … —` titles; `cells` splits rows on tabs. */
function sections(lines: SegLine[], cells: boolean): Section[] {
  const out: Section[] = [];
  let previous: number | null = null;
  for (const line of lines) {
    const text = plain(line.segs);
    const gap = previous !== null && line.n > previous + 1;
    previous = line.n;
    if (TITLE.test(text.trim())) {
      out.push({ title: trim(line.segs, 2, 2), n: line.n, rows: [], columns: 1 });
      continue;
    }
    if (!text.trim()) continue;
    let current = out[out.length - 1];
    if (!current) {
      current = { title: null, n: line.n, rows: [], columns: 1 };
      out.push(current);
    }
    if (gap && current.rows.length > 0) current.rows.push({ kind: 'gap', n: line.n - 1 });
    const row = cells ? split(line.segs, '\t') : [line.segs];
    current.columns = Math.max(current.columns, row.length);
    current.rows.push({ kind: 'row', n: line.n, cells: row });
  }
  return out;
}

/** Excel: one table per sheet. */
export const sheets = (lines: SegLine[]) => sections(lines, true);

/** PowerPoint: one card per slide (first line = title of the slide). */
export const slides = (lines: SegLine[]) => sections(lines, false);

/** Long text cell: it wraps; shorter ones stay on one line (references, dates). */
export function isLong(cell: Seg[]): boolean {
  return plain(cell).length > 32;
}

/** Amounts, dates, percentages: aligned to the end of the cell. */
export function isNumeric(cell: Seg[]): boolean {
  return /^[-+]?[\d\s.,:/]+\s?(%|€|\$|£)?$/.test(plain(cell).trim());
}
