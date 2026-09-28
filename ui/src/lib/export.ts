/**
 * Exports of the results (goal.md §3, lot 6.4): CSV that Excel opens as is,
 * a self-contained HTML page, JSON. Columns are chosen by the user; the
 * keyword report adds one column per term of the query (counted by the
 * engine), detected data one column per kind.
 */
import { formatBytes, formatDate, i18n, t, type MessageKey } from './i18n/index.svelte';
import { plain, segments, splitPath } from './text';
import type { SearchHit } from './types';

export type Column =
  | 'name'
  | 'folder'
  | 'path'
  | 'kind'
  | 'size'
  | 'modified'
  | 'created'
  | 'lang'
  | 'site'
  | 'matches'
  | 'excerpt'
  | 'keywords'
  | 'detections';

export const COLUMNS: Column[] = ['name', 'folder', 'path', 'kind', 'size', 'modified', 'created', 'lang', 'site', 'matches', 'excerpt', 'keywords', 'detections'];
export const DEFAULT_COLUMNS: Column[] = ['name', 'folder', 'kind', 'size', 'modified', 'lang', 'matches', 'keywords'];

/** Keyword report of the engine (lot 6.4). */
export interface KeywordReport {
  terms: string[];
  rows: { siteId: string; path: string; counts: number[] }[];
  totals: { files: number; matches: number }[];
}

export interface ExportData {
  hits: SearchHit[];
  columns: Column[];
  keywords: KeywordReport | null;
  /** Detected kinds to show as columns (lot 6.2). */
  detectors: string[];
  siteName: (id: string) => string;
  /** Title and search criteria (HTML header). */
  title: string;
  criteria: { label: string; value: string }[];
}

/** Excel reads a CSV with the list separator of the system's language: `;` where the decimal mark is a comma. */
export function csvSeparator(): ';' | ',' {
  return i18n.locale === 'fr' || i18n.locale === 'es' ? ';' : ',';
}

/** `2026-09-27 14:32`: a date Excel recognizes, in local time. */
function excelDate(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

const header = (c: Column) => t(`export.columns.${c}` as MessageKey);

/** The columns expanded: one per keyword, one per detected kind. */
interface Col {
  label: string;
  value: (h: SearchHit, html: boolean) => string | number;
  /** Keyword columns: the index of their term. */
  term?: number;
}

function layout(data: ExportData): Col[] {
  const counts = new Map((data.keywords?.rows ?? []).map((r) => [`${r.siteId}:${r.path}`, r.counts]));
  const out: Col[] = [];
  for (const column of data.columns) {
    switch (column) {
      case 'name':
        out.push({ label: header(column), value: (h) => splitPath(h.path).name });
        break;
      case 'folder':
        out.push({ label: header(column), value: (h) => splitPath(h.path).folder });
        break;
      case 'path':
        out.push({ label: header(column), value: (h) => h.path });
        break;
      case 'kind':
        out.push({ label: header(column), value: (h) => t(`filters.kinds.${h.kind}` as MessageKey) });
        break;
      case 'size':
        out.push({ label: header(column), value: (h, html) => (html ? formatBytes(h.sizeBytes) : h.sizeBytes) });
        break;
      case 'modified':
        out.push({ label: header(column), value: (h, html) => (html ? formatDate(h.modified) : excelDate(h.modified)) });
        break;
      case 'created':
        out.push({ label: header(column), value: (h, html) => (html ? formatDate(h.created) : excelDate(h.created)) });
        break;
      case 'lang':
        out.push({ label: header(column), value: (h) => t(`languages.${h.lang}` as MessageKey) });
        break;
      case 'site':
        out.push({ label: header(column), value: (h) => data.siteName(h.siteId) });
        break;
      case 'matches':
        out.push({ label: header(column), value: (h) => h.matchCount });
        break;
      case 'excerpt':
        out.push({ label: header(column), value: (h) => (h.snippets[0] ? plain(h.snippets[0].text) : '') });
        break;
      case 'keywords':
        (data.keywords?.terms ?? []).forEach((term, i) => {
          out.push({ label: term, value: (h) => counts.get(h.id)?.[i] ?? 0, term: i });
        });
        break;
      case 'detections':
        for (const code of data.detectors) {
          out.push({ label: t(`detectors.names.${code}` as MessageKey), value: (h) => h.detections?.[code] ?? 0 });
        }
        break;
    }
  }
  return out;
}

function csvCell(value: string | number, sep: string): string {
  const s = String(value);
  return s.includes(sep) || /["\n\r]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** CSV for Excel: UTF-8 with BOM (accents and Arabic), the list separator of the language. */
export function toExcelCsv(data: ExportData): string {
  const sep = csvSeparator();
  const cols = layout(data);
  const rows: (string | number)[][] = [cols.map((c) => c.label), ...data.hits.map((h) => cols.map((c) => c.value(h, false)))];
  // Keyword report: a totals row (files that contain each term).
  const totals = data.keywords?.totals ?? [];
  if (cols.some((c) => c.term !== undefined)) {
    rows.push(cols.map((c, i) => (c.term !== undefined ? (totals[c.term]?.files ?? 0) : i === 0 ? t('export.totalFiles') : '')));
  }
  return '﻿' + rows.map((r) => r.map((v) => csvCell(v, sep)).join(sep)).join('\r\n');
}

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** A passage with its matches as `<mark>`. */
function markedHtml(text: string): string {
  return segments(text)
    .map((s) => (s.hit ? `<mark${s.fuzzy ? ' class="fuzzy"' : ''}>${escape(s.text)}</mark>` : escape(s.text)))
    .join('');
}

/** A self-contained page: criteria, keyword totals, the table, the passages. */
export function toHtml(data: ExportData): string {
  const cols = layout(data);
  const dir = i18n.dir;
  const numeric = (v: string | number) => typeof v === 'number';
  const keywordTable =
    data.columns.includes('keywords') && data.keywords && data.keywords.terms.length > 0
      ? `<h2>${escape(t('export.byKeyword'))}</h2><table class="totals"><thead><tr><th>${escape(t('export.term'))}</th><th>${escape(t('detectors.files'))}</th><th>${escape(t('detectors.occurrences'))}</th></tr></thead><tbody>${data.keywords.terms
          .map((term, i) => `<tr><td dir="auto">${escape(term)}</td><td class="num">${data.keywords!.totals[i]!.files}</td><td class="num">${data.keywords!.totals[i]!.matches}</td></tr>`)
          .join('')}</tbody></table>`
      : '';
  const rows = data.hits
    .map((h) => {
      const cells = cols.map((c) => {
        const v = c.value(h, true);
        return `<td${numeric(v) ? ' class="num"' : ' dir="auto"'}>${escape(String(v))}</td>`;
      });
      const passages = h.snippets.map((s) => `<p dir="auto">${markedHtml(s.text)}</p>`).join('');
      return `<tr>${cells.join('')}</tr>${passages ? `<tr class="passages"><td colspan="${cols.length}">${passages}</td></tr>` : ''}`;
    })
    .join('\n');
  return `<!doctype html>
<html lang="${i18n.locale}" dir="${dir}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escape(data.title)}</title>
<style>
  body { font: 14px/1.5 system-ui, "Segoe UI", Tahoma, sans-serif; color: #1b1b1b; background: #fff; margin: 24px; }
  h1 { font-size: 22px; margin: 0 0 4px; } h2 { font-size: 16px; margin: 24px 0 8px; }
  .meta { color: #5a5a5a; margin: 0 0 16px; }
  dl { display: grid; grid-template-columns: max-content 1fr; gap: 4px 16px; margin: 0 0 16px; } dt { color: #5a5a5a; } dd { margin: 0; }
  table { border-collapse: collapse; width: 100%; } th, td { padding: 4px 8px; border-bottom: 1px solid #ddd; text-align: start; vertical-align: top; }
  th { background: #f4f4f4; position: sticky; top: 0; } .num { text-align: end; font-variant-numeric: tabular-nums; }
  table.totals { width: auto; min-width: 40%; }
  tr.passages td { border-bottom: 2px solid #ccc; color: #333; font-size: 13px; } tr.passages p { margin: 2px 0; }
  mark { background: #ffe58a; padding: 0 2px; } mark.fuzzy { background: none; text-decoration: underline wavy #c0392b; }
</style>
</head>
<body>
<h1>${escape(data.title)}</h1>
<p class="meta">${escape(t('report.generated', { date: formatDate(new Date()) }))} · Prospector</p>
<dl>${data.criteria.map((c) => `<dt>${escape(c.label)}</dt><dd dir="auto">${escape(c.value)}</dd>`).join('')}</dl>
${keywordTable}
<h2>${escape(t('results.files', { count: data.hits.length }))}</h2>
<table>
<thead><tr>${cols.map((c) => `<th>${escape(c.label)}</th>`).join('')}</tr></thead>
<tbody>
${rows}
</tbody>
</table>
</body>
</html>
`;
}

export function toJson(query: string, data: ExportData): string {
  const counts = new Map((data.keywords?.rows ?? []).map((r) => [`${r.siteId}:${r.path}`, r.counts]));
  return JSON.stringify(
    {
      query,
      exportedAt: new Date().toISOString(),
      terms: data.keywords?.terms,
      results: data.hits.map((h) => ({
        path: h.path,
        site: data.siteName(h.siteId),
        kind: h.kind,
        lang: h.lang,
        sizeBytes: h.sizeBytes,
        modified: h.modified.toISOString(),
        created: h.created.toISOString(),
        matchCount: h.matchCount,
        keywordCounts: counts.get(h.id),
        detections: h.detections,
        snippets: h.snippets.map((s) => ({ line: s.line, text: plain(s.text) })),
      })),
    },
    null,
    2,
  );
}

export function download(filename: string, content: string, mime: string) {
  const url = URL.createObjectURL(new Blob([content], { type: mime }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
