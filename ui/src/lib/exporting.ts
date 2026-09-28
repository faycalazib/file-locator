/**
 * Export and copy of the results (lot 6.4): gathers the files (those shown,
 * or everything the index found), the keyword report, then writes the file
 * or starts the copy. The formats themselves are in export.ts.
 */
import { api, inTauri, type CopyReportDto } from './api';
import { download, toExcelCsv, toHtml, toJson, type Column, type ExportData, type KeywordReport } from './export';
import { t, type MessageKey } from './i18n/index.svelte';
import { notices } from './stores/notices.svelte';
import { fromHitDto, search } from './stores/search.svelte';
import { sitesStore } from './stores/sites.svelte';
import { plain } from './text';
import type { SearchHit } from './types';

export type Scope = 'visible' | 'all';
export type Format = 'excel' | 'html' | 'json';

/** Most files an export or a copy of "everything found" takes. */
export const ALL_LIMIT = 10_000;

/** "Everything found" asks the index again; a live scan or opened results only have their hits. */
export function canFetchAll(): boolean {
  return inTauri && search.hasRun && search.mode === 'indexed' && search.openedFrom === null;
}

export async function gatherHits(scope: Scope): Promise<SearchHit[]> {
  if (scope === 'visible' || !canFetchAll()) return search.visible;
  const response = await api.search([...search.scope], search.exportRequest(ALL_LIMIT));
  return response.hits.map(fromHitDto);
}

/** The query has terms to count (a name or detectors alone have none). */
export function hasTerms(): boolean {
  return search.lastQuery.trim() !== '' || search.filters.termList != null;
}

export async function keywordReport(hits: SearchHit[]): Promise<KeywordReport | null> {
  if (!hasTerms() || hits.length === 0) return null;
  if (!inTauri) return demoKeywordReport(hits);
  return api.keywordReport(
    hits.map((h) => ({ siteId: h.siteId, path: h.path })),
    search.exportRequest(ALL_LIMIT),
  );
}

/** Browser demo: the words of the query counted in the passages. */
function demoKeywordReport(hits: SearchHit[]): KeywordReport {
  const terms = [...new Set(search.lastQuery.split(/\s+/).filter((w) => w && !['OR', 'AND', 'NOT', '|'].includes(w)))];
  const rows = hits.map((h) => {
    const text = h.snippets.map((s) => plain(s.text)).join(' ').toLowerCase();
    return { siteId: h.siteId, path: h.path, counts: terms.map((term) => text.split(term.toLowerCase()).length - 1) };
  });
  const totals = terms.map((_, i) => ({
    files: rows.filter((r) => r.counts[i]! > 0).length,
    matches: rows.reduce((sum, r) => sum + r.counts[i]!, 0),
  }));
  return { terms, rows, totals };
}

function siteName(id: string): string {
  return sitesStore.list.find((s) => s.id === id)?.name ?? (id === '@folder' ? (search.folder ?? '') : id);
}

function criteria(): { label: string; value: string }[] {
  const out = [{ label: t('report.query'), value: search.lastQuery || '—' }];
  if (search.namePattern.trim()) out.push({ label: t('report.names'), value: search.namePattern.trim() });
  const list = search.filters.termList;
  if (list) out.push({ label: t('terms.title'), value: `${list.name} · ${t('terms.count', { count: list.terms.length })} · ${list.all ? t('terms.all') : t('terms.any')}` });
  if (search.folder) out.push({ label: t('export.columns.folder'), value: search.folder });
  out.push({ label: t('search.mode.label'), value: t(`search.mode.${search.mode}` as MessageKey) });
  const sites = sitesStore.list.filter((s) => search.scope.has(s.id)).map((s) => s.name);
  out.push({ label: t('report.sites'), value: sites.join(' · ') || t('report.none') });
  return out;
}

/** `contrat-resiliation`: a file name from the search. */
function baseName(): string {
  const words = (search.lastQuery || search.namePattern || 'prospector')
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
  return words || 'prospector';
}

/** Tauri: native "Save as" + write by Rust. Browser: download. */
async function save(name: string, ext: string, content: string, mime: string) {
  if (!inTauri) {
    download(name, content, mime);
    return;
  }
  const path = await api.pickSavePath(name, ext, ext.toUpperCase());
  if (!path) return;
  await api.saveTextFile(path, content);
  notices.push(t('results.exported', { path }));
}

export async function runExport(format: Format, scope: Scope, columns: Column[]) {
  const hits = await gatherHits(scope);
  const keywords = columns.includes('keywords') || format === 'json' ? await keywordReport(hits) : null;
  const data: ExportData = {
    hits,
    columns,
    keywords,
    detectors: Object.keys(search.detections),
    siteName,
    title: t('export.title', { query: search.lastQuery || search.namePattern || '…' }),
    criteria: criteria(),
  };
  const base = baseName();
  if (format === 'excel') await save(`${base}.csv`, 'csv', toExcelCsv(data), 'text/csv;charset=utf-8');
  else if (format === 'html') await save(`${base}.html`, 'html', toHtml(data), 'text/html;charset=utf-8');
  else await save(`${base}.json`, 'json', toJson(search.lastQuery, data), 'application/json');
}

export interface CopyRun {
  id: string;
  done: Promise<CopyReportDto | null>;
}

/**
 * Copies the files found to a folder or a ZIP chosen in a native dialog.
 * `null`: cancelled before starting. Progress goes to `onProgress`.
 */
export async function startCopy(
  scope: Scope,
  zip: boolean,
  keepTree: boolean,
  extract: boolean,
  onProgress: (done: number, total: number) => void,
): Promise<CopyRun | null> {
  const hits = await gatherHits(scope);
  const paths = hits.map((h) => h.path);
  const id = crypto.randomUUID();
  if (!inTauri) return { id, done: demoCopy(paths, onProgress) };
  const target = zip ? await api.pickSavePath(`${baseName()}.zip`, 'zip', 'ZIP') : await api.pickFolder(t('export.pickFolder'));
  if (!target) return null;
  // Both listeners are in place before the copy starts: a small one may end at once.
  let finish!: (report: CopyReportDto | null) => void;
  const done = new Promise<CopyReportDto | null>((resolve) => (finish = resolve));
  const unlisten = [
    await api.onCopyProgress((e) => {
      if (e.copyId === id) onProgress(e.done, e.total);
    }),
    await api.onCopyFinished((e) => {
      if (e.copyId !== id) return;
      unlisten.forEach((u) => u());
      if (e.error) notices.error(e.error);
      finish(e.report);
    }),
  ];
  await api.copyFiles(id, paths, target, zip, keepTree, extract);
  return { id, done };
}

export function cancelCopy(id: string) {
  if (inTauri) void api.cancelCopy(id).catch(() => {});
}

/** Browser demo: a copy that only pretends. */
function demoCopy(paths: string[], onProgress: (done: number, total: number) => void): Promise<CopyReportDto> {
  const files = [...new Set(paths.map((p) => p.split(' › ')[0]!))];
  return new Promise((resolve) => {
    let done = 0;
    const timer = setInterval(() => {
      done = Math.min(files.length, done + 1);
      onProgress(done, files.length);
      if (done === files.length) {
        clearInterval(timer);
        resolve({ copied: files.length - 1, skipped: [files[0]!], fromContainers: paths.length - files.length, target: 'D:\\Export\\contrat', cancelled: false });
      }
    }, 120);
  });
}
