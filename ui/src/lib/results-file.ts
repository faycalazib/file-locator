/**
 * Saved results (lot 5.6): a `.prospector` file, JSON inside. It keeps the
 * search (words, file name, options, filters, sites) and every result with
 * its passages, so that it reopens exactly as it was, even once the files
 * have changed. The preview still reads the current file.
 */
import type { HitDto } from './api';
import { DISK_FILTERS_OFF, fromHitDto, type SearchSnapshot } from './stores/search.svelte';

export const RESULTS_EXTENSION = 'prospector';
const FORMAT = 'prospector-results';
const VERSION = 1;

interface ResultsFile {
  format: typeof FORMAT;
  version: number;
  savedAt: string;
  search: Pick<
    SearchSnapshot,
    'lastQuery' | 'namePattern' | 'folders' | 'mode' | 'options' | 'scope' | 'sort' | 'durationMs' | 'scanned' | 'within' | 'folder'
  > & { filters: Omit<SearchSnapshot['filters'], 'excluded'> };
  hits: HitDto[];
}

export function toResultsFile(s: SearchSnapshot): string {
  const { excluded: _, ...filters } = s.filters;
  const file: ResultsFile = {
    format: FORMAT,
    version: VERSION,
    savedAt: new Date().toISOString(),
    search: {
      lastQuery: s.lastQuery,
      namePattern: s.namePattern,
      folders: s.folders,
      mode: s.mode,
      options: s.options,
      filters,
      scope: s.scope,
      sort: s.sort,
      durationMs: s.durationMs,
      scanned: s.scanned,
      within: s.within,
      folder: s.folder,
    },
    hits: s.hits.map((h) => ({
      siteId: h.siteId,
      path: h.path,
      kind: h.kind,
      lang: h.lang,
      sizeBytes: h.sizeBytes,
      modified: Math.floor(h.modified.getTime() / 1000),
      created: Math.floor(h.created.getTime() / 1000),
      matchCount: h.matchCount,
      exactCount: h.exactCount,
      score: h.score,
      snippets: h.snippets,
      innerKind: h.innerKind,
      meaning: h.meaning,
    })),
  };
  return JSON.stringify(file, null, 1);
}

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isHit = (v: unknown): v is HitDto =>
  isObject(v) && typeof v.path === 'string' && typeof v.siteId === 'string' && Array.isArray(v.snippets);

/**
 * The search saved in `text`, on top of `base` (the current settings fill
 * whatever an older file lacks). Null when it is not a results file.
 */
export function parseResultsFile(text: string, name: string, base: SearchSnapshot): SearchSnapshot | null {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(data) || data.format !== FORMAT || !Array.isArray(data.hits) || !isObject(data.search)) return null;
  const search = data.search as Partial<ResultsFile['search']>;
  const hits = data.hits
    .filter(isHit)
    .map((dto) => fromHitDto({ ...dto, modified: Number(dto.modified) || 0, created: Number(dto.created) || undefined }));
  return {
    ...base,
    query: typeof search.lastQuery === 'string' ? search.lastQuery : '',
    lastQuery: typeof search.lastQuery === 'string' ? search.lastQuery : '',
    namePattern: typeof search.namePattern === 'string' ? search.namePattern : '',
    folders: search.folders === true,
    mode: search.mode === 'live' ? 'live' : 'indexed',
    options: isObject(search.options) ? { ...base.options, ...search.options } : base.options,
    filters: isObject(search.filters)
      ? { ...base.filters, ...DISK_FILTERS_OFF, ...search.filters, excluded: base.filters.excluded }
      : base.filters,
    scope: Array.isArray(search.scope) ? search.scope.filter((id): id is string => typeof id === 'string') : base.scope,
    sort: search.sort ?? base.sort,
    hits,
    hasRun: true,
    durationMs: typeof search.durationMs === 'number' ? search.durationMs : null,
    scanned: typeof search.scanned === 'number' ? search.scanned : 0,
    selectedId: hits[0]?.id ?? null,
    quickFilter: '',
    within: null,
    folder: typeof search.folder === 'string' ? search.folder : null,
    copiesOf: null,
    openedFrom: name,
    interrupted: false,
    scanId: null,
  };
}
