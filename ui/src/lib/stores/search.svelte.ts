/**
 * Search state. In Tauri, `run()` queries the Rust engine; outside Tauri a
 * simulated engine searches the Étape 0 mocks. Either way the hits are added
 * one by one so each gets its "new find" flash.
 */
import { SvelteSet } from 'svelte/reactivity';
import { api, inTauri, type FacetsDto, type HitDto, type SearchRequestDto } from '../api';
import { notices } from './notices.svelte';
import { hits as mockHits, recentSearches, searchableText, DEFAULT_QUERY } from '../mock/data';
import { normalize, splitPath } from '../text';
import { ALL_DETECTORS, dateBounds, hasDateFilter } from '../filters';
import type { Attribute, DateField, DateFilter, Detector, DocLang, FileKind, ResultKind, SearchHit, SizeFilter, SortKey } from '../types';

export type SearchMode = 'indexed' | 'live';

export interface SearchOptions {
  caseSensitive: boolean;
  wholeWord: boolean;
  regex: boolean;
  fuzzy: boolean;
}

export interface Filters {
  kinds: FileKind[];
  size: SizeFilter;
  date: DateFilter;
  /** Which date `date` applies to (lot 5.8). */
  dateField: DateField;
  /** Custom period, `YYYY-MM-DD` (`''` = open). */
  dateFrom: string;
  dateTo: string;
  langs: DocLang[];
  /** Required attributes, read on the disk (lot 5.8). */
  attributes: Attribute[];
  /** MD5 or SHA-256 of the file (lot 5.8). */
  hash: string;
  /** Detectors (lot 6.2): any of these data. */
  detectors: Detector[];
  /** Terms read from a file (lot 7.4); kept by value, not by path. */
  termList: TermList | null;
  excluded: string[];
}

/** A term list (lot 7.4): documents with at least one term, or all of them. */
export interface TermList {
  /** The file it was read from (shown in the chip). */
  name: string;
  terms: string[];
  /** Lines left out (comments, repeats, beyond the limit). */
  skipped: number;
  all: boolean;
}

/** Browser demo (screenshots). */
export const DEMO_TERM_LIST: TermList = {
  name: 'fournisseurs.txt',
  terms: ['Dupont SARL', 'García Hermanos', 'Bâti-Sud', 'Menuiserie Lefèvre', '/FR\\d{11}/', 'Transports Martin', 'Atelier 21', 'Électricité Roux', 'BTP Garonne', "Ferronnerie d'art", 'Verrerie Nord', 'Toitures Duval', 'Maçonnerie Blanc', 'Plâtrerie Morel'],
  skipped: 2,
  all: false,
};

/** The filters of lots 5.8 and 6.2, off: what an older saved search did not have. */
export const DISK_FILTERS_OFF = {
  dateField: 'modified',
  dateFrom: '',
  dateTo: '',
  attributes: [],
  hash: '',
  detectors: [],
  termList: null,
} as const satisfies Partial<Filters>;

/** Detectors (lot 6.2): per kind of data, files and occurrences. */
export type DetectionTotals = Record<string, { files: number; matches: number }>;

/** "Find copies" (lot 5.8): the file whose copies are looked for. */
export interface Copies {
  name: string;
  sha256: string;
  size: number;
}

/** "Search within these results" (lot 5.6): the next searches only look at them. */
export interface Within {
  paths: string[];
  /** What found them (shown in the chip). */
  query: string;
}

/**
 * Everything that makes the search on screen: a tab that is not shown, or a
 * step of a tab's history (lot 5.6). The hits are shared, never copied: the
 * store always replaces its arrays, it never changes them.
 */
export interface SearchSnapshot {
  query: string;
  lastQuery: string;
  namePattern: string;
  folders: boolean;
  mode: SearchMode;
  options: SearchOptions;
  filters: Filters;
  scope: string[];
  sort: SortKey;
  hits: SearchHit[];
  hasRun: boolean;
  durationMs: number | null;
  scanned: number;
  selectedId: string | null;
  quickFilter: string;
  within: Within | null;
  /** "Search with Prospector" on a folder (lot 5.7): only what is inside it. */
  folder: string | null;
  /** "Find copies" of this file (lot 5.8). */
  copiesOf: Copies | null;
  /** Results opened from a `.prospector` file (its name). */
  openedFrom: string | null;
  /** Left before the engine answered: searched again when shown. */
  interrupted: boolean;
  /** Live scan still running while the tab is not shown. */
  scanId: string | null;
}

const MB = 1024 * 1024;
const SIZE_RANGES: Record<SizeFilter, [number, number]> = {
  any: [0, Number.POSITIVE_INFINITY],
  lt1: [0, MB],
  '1to10': [MB, 10 * MB],
  '10to100': [10 * MB, 100 * MB],
  gt100: [100 * MB, Number.POSITIVE_INFINITY],
};

// ── Mock query engine ─────────────────────────────────────────────────

interface ParsedQuery {
  include: string[];
  exclude: string[];
  patterns: RegExp[];
}

const OPERATORS = new Set(['and', 'or', 'not']);

function parseQuery(query: string): ParsedQuery {
  const parsed: ParsedQuery = { include: [], exclude: [], patterns: [] };
  const tokens = query.match(/\/[^/]+\/|"[^"]*"|«[^»]*»|\S+/g) ?? [];
  let negate = false;
  for (const raw of tokens) {
    if (raw.startsWith('/') && raw.endsWith('/') && raw.length > 2) {
      try {
        parsed.patterns.push(new RegExp(raw.slice(1, -1), 'iu'));
      } catch {
        /* invalid regex: ignored by the mock */
      }
      continue;
    }
    const lower = raw.toLowerCase();
    if (OPERATORS.has(lower)) {
      negate = lower === 'not';
      continue;
    }
    const term = normalize(raw.replace(/^["«]|["»]$/g, '').trim());
    if (!term) continue;
    (negate ? parsed.exclude : parsed.include).push(term);
    negate = false;
  }
  return parsed;
}

function levenshtein(a: string, b: string): number {
  if (Math.abs(a.length - b.length) > 1) return 2;
  const prev = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    let diag = prev[0]!;
    prev[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const tmp = prev[j]!;
      prev[j] = Math.min(prev[j]! + 1, prev[j - 1]! + 1, diag + (a[i - 1] === b[j - 1] ? 0 : 1));
      diag = tmp;
    }
  }
  return prev[b.length]!;
}

function matches(hit: SearchHit, q: ParsedQuery, fuzzy: boolean): boolean {
  const raw = searchableText(hit);
  const text = normalize(raw);
  if (q.exclude.some((term) => text.includes(term))) return false;
  if (q.patterns.some((re) => re.test(raw))) return true;
  if (q.include.length === 0) return false;
  const words = fuzzy ? text.match(/[\p{L}\p{N}_]+/gu) ?? [] : [];
  return q.include.some(
    (term) => text.includes(term) || (term.length >= 5 && words.some((w) => levenshtein(term, w) <= 1)),
  );
}

/** Browser demo only: a criterion given in the address (screenshots). */
function demoParam(name: string): string | null {
  return new URLSearchParams(location.search).get(name);
}

/** Browser demo only: the engine's file-name rules (see core/src/names.rs). */
function mockNameMatcher(input: string): ((name: string) => boolean) | null {
  const toRegex = (item: string) => {
    if (item.length > 2 && item.startsWith('/') && item.endsWith('/')) return new RegExp(item.slice(1, -1), 'iu');
    const source = normalize(item)
      .split('')
      .map((c) => (c === '*' ? '.*' : c === '?' ? '.' : c.replace(/[.+^${}()|[\]\\]/g, '\\$&')))
      .join('');
    return new RegExp(/[*?]/.test(item) ? `^${source}$` : source, 'iu');
  };
  const items = input.split(/[;,]/).map((s) => s.trim()).filter(Boolean);
  if (items.length === 0) return null;
  const include = items.filter((i) => !i.startsWith('!')).map(toRegex);
  const exclude = items.filter((i) => i.startsWith('!')).map((i) => toRegex(i.slice(1).trim()));
  return (name) => {
    const folded = normalize(name);
    return (include.length === 0 || include.some((r) => r.test(folded))) && !exclude.some((r) => r.test(folded));
  };
}

/** Browser demo only: the folders of the mock hits, as folder results. */
function mockFolders(keep: ((name: string) => boolean) | null): SearchHit[] {
  if (!keep) return [];
  const seen = new Map<string, SearchHit>();
  for (const hit of mockHits) {
    // Every folder along the path (D:\Clients, D:\Clients\Dupont SARL…).
    const parts = splitPath(hit.path).folder.split(/[\\/]/).filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const folder = parts.slice(0, i + 1).join('\\');
      if (!keep(parts[i]!) || seen.has(folder)) continue;
      seen.set(folder, { ...hit, id: `folder:${folder}`, path: folder, kind: 'folder', matchCount: 0, exactCount: 0, snippets: [], sizeBytes: 0 });
    }
  }
  return [...seen.values()];
}

/** Browser demo only: the lot 5.8 filters given in the address. */
function demoDiskFilters(): Partial<Filters> {
  const out: Partial<Filters> = {};
  const field = demoParam('field');
  if (field === 'created' || field === 'accessed') out.dateField = field;
  const from = demoParam('from');
  const to = demoParam('to');
  if (from || to) Object.assign(out, { date: 'custom', dateFrom: from ?? '', dateTo: to ?? '' });
  const attrs = demoParam('attrs')?.split(',').filter((a): a is Attribute => a === 'readOnly' || a === 'hidden' || a === 'system');
  if (attrs?.length) out.attributes = attrs;
  const hash = demoParam('hash');
  if (hash) out.hash = hash;
  if (demoParam('audit') !== null) out.detectors = [...ALL_DETECTORS];
  return out;
}

/** What an engine request is made of (the search on screen, or a saved one). */
export interface RequestInput {
  query: string;
  options: SearchOptions;
  filters: Omit<Filters, 'excluded'>;
  namePattern: string;
  folders: boolean;
  withinPaths?: string[];
  folder: string | null;
  copies: Copies | null;
}

/** The engine request, from the UI's settings (also used by the alerts, lot 6.1). */
export function buildRequest(s: RequestInput): SearchRequestDto {
  const f = s.filters;
  // Copies: exactly the size of the file (the engine hashes only those).
  const [minSize, maxSize] = s.copies ? [s.copies.size, s.copies.size + 1] : SIZE_RANGES[f.size];
  const { from, to } = dateBounds(f);
  const seconds = (ms: number | null) => (ms === null ? undefined : Math.floor(ms / 1000));
  return {
    query: s.query,
    fuzzy: s.options.fuzzy,
    caseSensitive: s.options.caseSensitive,
    wholeWord: s.options.wholeWord,
    regex: s.options.regex,
    kinds: f.kinds,
    langs: f.langs,
    minSize: minSize > 0 ? minSize : undefined,
    maxSize: Number.isFinite(maxSize) ? maxSize : undefined,
    dateField: f.dateField,
    dateFrom: seconds(from),
    dateTo: seconds(to),
    attributes: f.attributes,
    hash: f.hash.trim() || undefined,
    detectors: f.detectors.length > 0 ? f.detectors : undefined,
    termList: f.termList?.terms,
    termListAll: f.termList?.all || undefined,
    namePattern: s.namePattern.trim() || undefined,
    folders: s.folders || undefined,
    withinPaths: s.withinPaths,
    inFolder: s.folder ?? undefined,
    limit: 200,
  };
}

// ── Store ─────────────────────────────────────────────────────────────

class SearchStore {
  // Browser demo: `?q=`, `?names=`, `?folders` set the criteria (screenshots only).
  query = $state(inTauri ? '' : (demoParam('q') ?? DEFAULT_QUERY));
  /** Query of the last run (the preview highlights this one, not the one being typed). */
  lastQuery = $state('');
  recents = $state<string[]>(loadRecents());
  mode = $state<SearchMode>('indexed');
  // Whole word is on by default: "alger" must not find "algérie" (BUG-020).
  options = $state<SearchOptions>({ caseSensitive: false, wholeWord: true, regex: false, fuzzy: true });
  filters = $state<Filters>({
    kinds: [],
    size: 'any',
    date: 'any',
    ...DISK_FILTERS_OFF,
    attributes: [],
    detectors: [],
    // Browser demo: `?field=created&from=…&to=…&attrs=readOnly&hash=…` (screenshots only).
    ...(inTauri ? {} : demoDiskFilters()),
    ...(!inTauri && demoParam('terms') !== null ? { termList: DEMO_TERM_LIST } : {}),
    langs: [],
    excluded: ['D:\\Clients\\_corbeille', '**\\node_modules'],
  });
  sort = $state<SortKey>('relevance');
  /** File-name criterion (lot 5.1), next to the text query. */
  namePattern = $state(inTauri ? '' : (demoParam('names') ?? ''));
  /** Look for folders by name instead of files. */
  folders = $state(!inTauri && demoParam('folders') !== null);

  /** Sites included in the query (multi-index search). */
  scope = new SvelteSet<string>(inTauri ? [] : ['clients', 'mail', 'code', 'archives']);

  // Raw: hits are only ever replaced, and tabs / history share them.
  hits = $state.raw<SearchHit[]>([]);
  /** Instant filter of the list (name or folder), applied while typing. */
  // Browser demo: `?filter=`, `?within` (screenshots only).
  quickFilter = $state(inTauri ? '' : (demoParam('filter') ?? ''));
  within = $state<Within | null>(
    !inTauri && demoParam('within') !== null ? { paths: mockHits.slice(0, 5).map((h) => h.path), query: DEFAULT_QUERY } : null,
  );
  /** Explorer right-click (lot 5.7): the search is limited to this folder. */
  // Browser demo: `?folder=` (screenshots only).
  folder = $state<string | null>(inTauri ? null : demoParam('folder'));
  /** "Find copies" (lot 5.8): shown in the tab title and the chips. */
  copiesOf = $state<Copies | null>(null);
  /** Results opened from a file: shown whatever the sites selected. */
  openedFrom = $state<string | null>(null);
  /** Settings of the search on screen: a change re-runs it (App.svelte). */
  ranSignature = $state('');
  /** Called with the search a new one replaces (tab history). */
  beforeRun: ((previous: SearchSnapshot) => void) | null = null;
  /** A tab that is not shown got hits from its live scan. */
  onBackground: (() => void) | null = null;
  /** Opened from an alert (lot 6.1): the documents it announced, marked "new". */
  // Browser demo: `?news` marks two hits (screenshots only).
  alertNew = $state.raw<ReadonlySet<string>>(
    new Set(!inTauri && demoParam('news') !== null ? mockHits.slice(0, 2).map((h) => h.path) : []),
  );
  /** Detectors (lot 6.2): totals of the index over every file found; null = count the hits. */
  engineDetections = $state.raw<DetectionTotals | null>(null);
  /** Counts per criterion from the index (lot 6.3); null = count the hits. */
  engineFacets = $state.raw<FacetsDto | null>(null);
  /** Detectors: only the files holding this kind of data (a click on the banner). */
  detectorFocus = $state<Detector | null>(null);
  /** Hits added during the current scan: they get the amber ping. */
  fresh = new SvelteSet<string>();
  scanning = $state(false);
  scanned = $state(0);
  durationMs = $state<number | null>(null);
  hasRun = $state(false);
  selectedId = $state<string | null>(null);

  #timers: ReturnType<typeof setTimeout>[] = [];
  /** Current live scan (hits of an older scan are ignored). */
  #scanId: string | null = null;
  /** Live hits waiting to be shown (flushed every 80 ms, not one render per hit). */
  #pending: SearchHit[] = [];
  #flushTimer: ReturnType<typeof setTimeout> | undefined;
  /** Every hit of the index answer being streamed in. */
  #streaming: SearchHit[] | null = null;
  /** Live scans of tabs that are not shown, by scan id. */
  #background = new Map<string, SearchSnapshot>();

  constructor() {
    if (inTauri) this.#listenLiveScan();
  }

  #listenLiveScan() {
    void api.onScanHit(({ scanId, hit }) => {
      const hidden = this.#background.get(scanId);
      if (hidden) {
        hidden.hits = [...hidden.hits, fromHitDto(hit)];
        this.onBackground?.();
        return;
      }
      if (scanId !== this.#scanId) return;
      this.#pending.push(fromHitDto(hit));
      this.#flushTimer ??= setTimeout(() => this.#flush(), 80);
    });
    void api.onScanProgress(({ scanId, scanned }) => {
      const hidden = this.#background.get(scanId);
      if (hidden) hidden.scanned = scanned;
      if (scanId === this.#scanId) this.scanned = scanned;
    });
    void api.onScanFinished(({ scanId, summary, error }) => {
      const hidden = this.#background.get(scanId);
      if (hidden) {
        this.#background.delete(scanId);
        hidden.scanId = null;
        hidden.durationMs = summary?.tookMs ?? null;
        if (summary) hidden.scanned = summary.filesScanned;
        if (error) notices.error(error);
        this.onBackground?.();
        return;
      }
      if (scanId !== this.#scanId) return;
      this.#flush();
      this.scanning = false;
      this.durationMs = summary?.tookMs ?? null;
      if (summary) this.scanned = summary.filesScanned;
      if (error) notices.error(error);
      this.#timers.push(setTimeout(() => this.fresh.clear(), 1000));
    });
  }

  #flush() {
    clearTimeout(this.#flushTimer);
    this.#flushTimer = undefined;
    if (this.#pending.length === 0) return;
    const batch = this.#pending;
    this.#pending = [];
    this.hits = [...this.hits, ...batch];
    for (const hit of batch) this.fresh.add(hit.id);
    this.selectedId ??= batch[0]?.id ?? null;
  }
  /** Ignores the answer of a search that was replaced by a newer one. */
  #generation = 0;

  visible = $derived.by(() => {
    const f = this.filters;
    const [minSize, maxSize] = SIZE_RANGES[f.size];
    // The last access is not in the results: the engine alone applies it.
    const { from, to } = f.dateField === 'accessed' ? { from: null, to: null } : dateBounds(f);
    const dateOf = (h: SearchHit) => (f.dateField === 'created' ? h.created : h.modified).getTime();
    const needle = normalize(this.quickFilter.trim());
    const list = this.hits.filter(
      (h) =>
        (this.detectorFocus === null || (h.detections?.[this.detectorFocus] ?? 0) > 0) &&
        // A folder outside every site is scanned on its own (site `@folder`).
        (this.openedFrom !== null || this.folder !== null || this.scope.has(h.siteId)) &&
        (needle === '' || normalize(h.path).includes(needle)) &&
        // Folders have no family and no language: those filters do not apply.
        (f.kinds.length === 0 || h.kind === 'folder' || f.kinds.includes(h.kind)) &&
        (f.langs.length === 0 || h.kind === 'folder' || f.langs.includes(h.lang)) &&
        h.sizeBytes >= minSize &&
        h.sizeBytes < maxSize &&
        (from === null || dateOf(h) >= from) &&
        (to === null || dateOf(h) < to),
    );
    const by: Record<SortKey, (a: SearchHit, b: SearchHit) => number> = {
      relevance: (a, b) => b.score - a.score,
      matches: (a, b) => b.matchCount - a.matchCount,
      modified: (a, b) => b.modified.getTime() - a.modified.getTime(),
      name: (a, b) => splitPath(a.path).name.localeCompare(splitPath(b.path).name),
    };
    return list.toSorted(by[this.sort]);
  });

  selected = $derived(this.visible.find((h) => h.id === this.selectedId) ?? null);
  totalMatches = $derived(this.visible.reduce((sum, h) => sum + h.matchCount, 0));
  /** Every visible file matched only through typo tolerance. */
  // Results found by name only (no text asked) are neither exact nor approximate.
  onlyApproximate = $derived(
    this.visible.some((h) => h.matchCount > 0) && this.visible.every((h) => h.exactCount === 0),
  );
  activeFilterCount = $derived(
    (this.filters.kinds.length > 0 ? 1 : 0) +
      (this.filters.size !== 'any' ? 1 : 0) +
      (hasDateFilter(this.filters) ? 1 : 0) +
      (this.filters.langs.length > 0 ? 1 : 0) +
      (this.filters.attributes.length > 0 ? 1 : 0) +
      (this.filters.hash.trim() ? 1 : 0) +
      (this.filters.detectors.length > 0 ? 1 : 0),
  );

  /** `history: false` for a search shown again (a tab left mid-search). */
  run({ history = true }: { history?: boolean } = {}) {
    // The same search again (Enter pressed twice) is not a new history step.
    const key = this.#runKey();
    if (history && this.hasRun && key !== this.#lastRunKey) this.beforeRun?.(this.capture(false));
    this.#lastRunKey = key;
    this.openedFrom = null;
    this.ranSignature = this.signature();
    return inTauri ? this.#runEngine() : this.#runMock();
  }

  #lastRunKey = '';

  #runKey(): string {
    return JSON.stringify([this.query.trim(), this.namePattern.trim(), this.mode, this.within?.paths.length ?? 0, this.signature()]);
  }

  /** What re-runs the search when it changes (App.svelte). */
  signature(): string {
    return JSON.stringify([this.filters, this.options, [...this.scope].toSorted(), this.folders, this.folder]);
  }

  /**
   * The search on screen. `keepScan`: a running live scan goes on in the
   * background (a tab being left); otherwise it counts as stopped.
   */
  capture(keepScan: boolean): SearchSnapshot {
    this.#flush();
    const liveScan = this.scanning && this.mode === 'live' && inTauri ? this.#scanId : null;
    const f = this.filters;
    return {
      query: this.query,
      lastQuery: this.lastQuery,
      namePattern: this.namePattern,
      folders: this.folders,
      mode: this.mode,
      options: { ...this.options },
      filters: { ...f, kinds: [...f.kinds], langs: [...f.langs], attributes: [...f.attributes], detectors: [...f.detectors], excluded: [...f.excluded] },
      scope: [...this.scope],
      sort: this.sort,
      hits: this.scanning && this.#streaming ? this.#streaming : this.hits,
      hasRun: this.hasRun,
      durationMs: this.durationMs,
      scanned: this.scanned,
      selectedId: this.selectedId,
      quickFilter: this.quickFilter,
      within: this.within,
      folder: this.folder,
      copiesOf: this.copiesOf,
      openedFrom: this.openedFrom,
      interrupted: this.scanning && !liveScan && !this.#streaming,
      scanId: keepScan ? liveScan : null,
    };
  }

  /** An empty search with the current settings (a new tab). */
  blank(): SearchSnapshot {
    const current = this.capture(false);
    return {
      ...current,
      // A digest belongs to one search (its copies), not to the next tab.
      filters: { ...current.filters, hash: '' },
      query: '',
      lastQuery: '',
      namePattern: '',
      folders: false,
      hits: [],
      hasRun: false,
      durationMs: null,
      scanned: 0,
      selectedId: null,
      quickFilter: '',
      within: null,
      folder: null,
      copiesOf: null,
      openedFrom: null,
      interrupted: false,
    };
  }

  /** Leaves the search on screen (returned) and shows `next` instead. */
  switchTo(next: SearchSnapshot): SearchSnapshot {
    const left = this.capture(true);
    if (left.scanId) {
      this.#background.set(left.scanId, left);
      this.#scanId = null;
    }
    this.stop();
    this.restore(next);
    return left;
  }

  /** Shows a search again, as it was left (no new search, except if interrupted). */
  restore(s: SearchSnapshot) {
    this.stop();
    this.query = s.query;
    this.lastQuery = s.lastQuery;
    this.namePattern = s.namePattern;
    this.folders = s.folders;
    this.mode = s.mode;
    this.options = { ...s.options };
    // The excluded folders are global (Settings), not part of a search.
    this.filters = { ...s.filters, excluded: this.filters.excluded };
    this.scope.clear();
    for (const id of s.scope) this.scope.add(id);
    this.sort = s.sort;
    this.hits = s.hits;
    this.fresh.clear();
    this.hasRun = s.hasRun;
    this.durationMs = s.durationMs;
    this.scanned = s.scanned;
    this.selectedId = s.selectedId;
    this.quickFilter = s.quickFilter;
    this.within = s.within;
    this.folder = s.folder;
    this.copiesOf = s.copiesOf;
    this.openedFrom = s.openedFrom;
    this.ranSignature = this.signature();
    this.#lastRunKey = '';
    // Its live scan went on in the background: it comes back on screen.
    const running = s.scanId !== null && this.#background.delete(s.scanId);
    this.#scanId = running ? s.scanId : null;
    this.scanning = running;
    if (s.interrupted) void this.run({ history: false });
  }

  /** A tab is closed: its background scan stops. */
  discard(s: SearchSnapshot) {
    if (s.scanId && this.#background.delete(s.scanId)) void api.cancelScan(s.scanId).catch(() => {});
  }

  /** "Search within these results": the next searches only look at them. */
  searchWithin() {
    const paths = [...new Set(this.visible.map((h) => h.path))];
    if (paths.length === 0) return;
    this.within = { paths, query: this.lastQuery || this.namePattern.trim() };
    this.quickFilter = '';
    this.query = '';
  }

  /** Back to searching everywhere (the search runs again if there is one). */
  clearWithin() {
    this.within = null;
    if (this.hasRun && this.hasCriteria) void this.run();
  }

  /** Back to the sites (App.svelte re-runs the search: the settings changed). */
  clearFolder() {
    this.folder = null;
  }

  /** Something to look for: text, a file name, or a digest. */
  get hasCriteria(): boolean {
    return (
      this.query.trim().length > 0 ||
      this.namePattern.trim().length > 0 ||
      this.filters.hash.trim().length > 0 ||
      this.filters.detectors.length > 0 ||
      // `undefined` in a search saved before lot 7.4.
      this.filters.termList != null
    );
  }

  /** The copies being looked for, while their digest is still the filter. */
  get copies(): Copies | null {
    return this.copiesOf && this.filters.hash.trim().toLowerCase() === this.copiesOf.sha256 ? this.copiesOf : null;
  }

  /**
   * What the preview highlights: the last query run with the current options.
   * No filters (and no `Date.now()`): the request must stay identical between
   * two renders, otherwise the preview would reload on every keystroke.
   */
  previewRequest(): SearchRequestDto {
    return {
      query: this.lastQuery,
      fuzzy: this.options.fuzzy,
      caseSensitive: this.options.caseSensitive,
      wholeWord: this.options.wholeWord,
      regex: this.options.regex,
      kinds: [],
      langs: [],
      // Detected data are highlighted in the preview too.
      detectors: this.filters.detectors.length > 0 ? this.filters.detectors : undefined,
    };
  }

  /**
   * Counts per criterion (lot 6.3): from the index (every file found, each
   * criterion without its own filter); in live scan, for opened results and
   * the browser demo, counted on the hits received.
   */
  get facets(): FacetsDto | null {
    if (!this.hasRun || this.folders) return null;
    if (this.engineFacets && this.openedFrom === null) return this.engineFacets;
    const out: FacetsDto = { kinds: {}, langs: {}, years: {}, sites: {} };
    const bump = (map: Record<string, number>, key: string) => (map[key] = (map[key] ?? 0) + 1);
    for (const hit of this.hits) {
      bump(out.kinds, hit.kind);
      bump(out.langs, hit.lang);
      bump(out.years, String((this.filters.dateField === 'created' ? hit.created : hit.modified).getFullYear()));
      bump(out.sites, hit.siteId);
    }
    return out;
  }

  /** The date filter is exactly this calendar year. */
  isYear(year: string): boolean {
    const f = this.filters;
    return f.date === 'custom' && f.dateFrom === `${year}-01-01` && f.dateTo === `${year}-12-31`;
  }

  /** A click on a year: only that year (on the date the years are counted on); again: any date. */
  toggleYear(year: string) {
    if (this.isYear(year)) {
      this.filters = { ...this.filters, date: 'any', dateFrom: '', dateTo: '' };
      return;
    }
    const dateField = this.filters.dateField === 'accessed' ? 'modified' : this.filters.dateField;
    this.filters = { ...this.filters, date: 'custom', dateField, dateFrom: `${year}-01-01`, dateTo: `${year}-12-31` };
  }

  /** The engine request of the search on screen, for an export of everything found (lot 6.4). */
  exportRequest(limit: number): SearchRequestDto {
    return { ...this.#request(), query: this.lastQuery, limit };
  }

  /** Detectors (lot 6.2): per kind, from the index when it counted, else from the hits. */
  get detections(): DetectionTotals {
    if (this.engineDetections && this.openedFrom === null) return this.engineDetections;
    const totals: DetectionTotals = {};
    for (const hit of this.hits) {
      for (const [code, count] of Object.entries(hit.detections ?? {})) {
        const total = (totals[code] ??= { files: 0, matches: 0 });
        total.files += 1;
        total.matches += count;
      }
    }
    return totals;
  }

  /** Request for the engine, from the current options and filters. */
  #request(): SearchRequestDto {
    return buildRequest({
      query: this.query,
      options: this.options,
      filters: this.filters,
      namePattern: this.namePattern,
      folders: this.folders,
      withinPaths: this.within?.paths,
      folder: this.folder,
      copies: this.copies,
    });
  }

  async #runEngine() {
    this.stop();
    // A new search: the alert's "new" marks go (saved.apply sets them after).
    this.alertNew = new Set();
    this.engineDetections = null;
    this.engineFacets = null;
    this.detectorFocus = null;
    const generation = ++this.#generation;
    const query = this.query.trim();
    this.hits = [];
    this.fresh.clear();
    this.durationMs = null;
    this.selectedId = null;
    this.hasRun = this.hasCriteria;
    if (!this.hasRun) return;
    this.lastQuery = query;
    if (query) this.#remember(query);
    this.scanning = true;
    this.scanned = 0;
    if (this.mode === 'live') {
      // Live scan: hits arrive through `scan://hit` events.
      this.#scanId = crypto.randomUUID();
      try {
        await api.liveScan(this.#scanId, [...this.scope], this.filters.excluded, { ...this.#request(), limit: 1000 });
      } catch (e) {
        this.scanning = false;
        notices.error(e);
      }
      return;
    }
    try {
      const response = await api.search([...this.scope], { ...this.#request(), facets: true });
      if (generation !== this.#generation) return;
      this.durationMs = response.tookMs;
      this.engineDetections = response.detections ?? null;
      this.engineFacets = response.facets ?? null;
      this.#stream(response.hits.map(fromHitDto));
    } catch (e) {
      if (generation !== this.#generation) return;
      this.scanning = false;
      notices.error(e);
    }
  }

  /**
   * The index changed (folder watcher, update finished): the last search runs
   * again quietly. New hits flash; the selected file stays selected if it is
   * still found. Not in live-scan mode (it reads the disk each time anyway).
   */
  async refresh() {
    if (!inTauri || this.mode === 'live' || !this.hasRun || this.scanning || this.openedFrom) return;
    const generation = ++this.#generation;
    try {
      const response = await api.search([...this.scope], { ...this.#request(), query: this.lastQuery, facets: true });
      if (generation !== this.#generation) return;
      this.engineFacets = response.facets ?? null;
      this.engineDetections = response.detections ?? null;
      const found = response.hits.map(fromHitDto);
      const known = new Set(this.hits.map((h) => h.id));
      this.hits = found;
      this.durationMs = response.tookMs;
      for (const hit of found) if (!known.has(hit.id)) this.fresh.add(hit.id);
      if (!found.some((h) => h.id === this.selectedId)) this.selectedId = found[0]?.id ?? null;
      this.#timers.push(setTimeout(() => this.fresh.clear(), 1000));
    } catch {
      /* not shown: the next search the user runs reports the error */
    }
  }

  /** Adds hits one by one (each gets the flash), then the rest at once. */
  #stream(found: SearchHit[]) {
    this.#streaming = found;
    const STEP_MS = 28;
    const ANIMATED = 30;
    found.slice(0, ANIMATED).forEach((hit, i) => {
      this.#timers.push(
        setTimeout(() => {
          this.hits = [...this.hits, hit];
          this.fresh.add(hit.id);
          this.selectedId ??= hit.id;
        }, STEP_MS * i),
      );
    });
    this.#timers.push(
      setTimeout(() => {
        this.hits = found;
        this.#streaming = null;
        this.selectedId ??= found[0]?.id ?? null;
        this.scanning = false;
        this.#timers.push(setTimeout(() => this.fresh.clear(), 1000));
      }, STEP_MS * Math.min(found.length, ANIMATED) + 60),
    );
  }

  #remember(query: string) {
    this.recents = [query, ...this.recents.filter((q) => q !== query)].slice(0, 6);
    try {
      localStorage.setItem(RECENTS_KEY, JSON.stringify(this.recents));
    } catch {
      /* not persisted */
    }
  }

  /** A site was removed: its hits disappear. */
  dropSite(siteId: string) {
    this.hits = this.hits.filter((h) => h.siteId !== siteId);
  }

  #runMock() {
    this.stop();
    this.lastQuery = this.query;
    const q = parseQuery(this.query);
    const byName = mockNameMatcher(this.namePattern);
    const textless = this.query.trim().length === 0;
    const within = this.within ? new Set(this.within.paths) : null;
    const found = (this.folders ? mockFolders(byName ?? mockNameMatcher(this.query)) : mockHits)
      .filter((h) => this.folders || ((textless || matches(h, q, this.options.fuzzy)) && (!byName || byName(splitPath(h.path).name))))
      .filter((h) => !within || within.has(h.path))
      .filter((h) => !this.folder || normalize(h.path).startsWith(normalize(this.folder)))
      // Demo detectors: the hits with detected data of those kinds.
      .filter((h) => this.filters.detectors.length === 0 || this.filters.detectors.some((d) => (h.detections?.[d] ?? 0) > 0))
      // Demo copies: the same size stands for the same digest.
      .filter((h) => !this.copies || h.sizeBytes === this.copies.size)
      .map((h) => (textless && !this.folders ? { ...h, matchCount: 0, exactCount: 0, snippets: [] } : h))
      .toSorted((a, b) => b.score - a.score);
    const live = this.mode === 'live';
    const step = live ? 170 : 55;
    const started = performance.now();

    this.hits = [];
    this.fresh.clear();
    this.scanned = 0;
    this.durationMs = null;
    this.hasRun = this.hasCriteria;
    this.selectedId = null;
    if (!this.hasRun) return;
    this.scanning = true;
    this.#streaming = found;

    found.forEach((hit, i) => {
      this.#timers.push(
        setTimeout(() => {
          this.hits = [...this.hits, hit];
          this.fresh.add(hit.id);
          // `?sel=h4` preselects a demo hit (screenshots of the preview).
          const wanted = new URLSearchParams(location.search).get('sel');
          if (wanted === hit.id) this.selectedId = hit.id;
          this.selectedId ??= hit.id;
          if (live) this.scanned += 180 + ((i * 97) % 240);
        }, step * (i + 1)),
      );
    });

    const end = step * (found.length + 1) + (live ? 400 : 120);
    this.#timers.push(
      setTimeout(() => {
        this.scanning = false;
        this.#streaming = null;
        if (live) this.scanned += 612;
        // Indexed mode reports the engine time, not the streaming animation.
        this.durationMs = live ? Math.round(performance.now() - started) : 38;
        this.#timers.push(setTimeout(() => this.fresh.clear(), 1000));
      }, end),
    );
  }

  stop() {
    if (this.#scanId && this.scanning) void api.cancelScan(this.#scanId).catch(() => {});
    this.#timers.forEach(clearTimeout);
    this.#timers = [];
    // An index answer being streamed in: shown whole at once.
    if (this.#streaming) {
      this.hits = this.#streaming;
      this.#streaming = null;
    }
    if (this.scanning) {
      this.scanning = false;
      this.fresh.clear();
    }
  }

  select(id: string) {
    this.selectedId = id;
  }

  /** Keyboard navigation in the results list. */
  move(delta: number) {
    const list = this.visible;
    if (list.length === 0) return;
    const index = list.findIndex((h) => h.id === this.selectedId);
    const next = Math.min(list.length - 1, Math.max(0, index + delta));
    this.selectedId = list[next]!.id;
  }

  toggleKind(kind: FileKind) {
    const kinds = this.filters.kinds;
    this.filters.kinds = kinds.includes(kind) ? kinds.filter((k) => k !== kind) : [...kinds, kind];
  }

  toggleLang(lang: DocLang) {
    const langs = this.filters.langs;
    this.filters.langs = langs.includes(lang) ? langs.filter((l) => l !== lang) : [...langs, lang];
  }

  resetFilters() {
    this.filters = { ...this.filters, kinds: [], size: 'any', date: 'any', langs: [], ...DISK_FILTERS_OFF, attributes: [], detectors: [], termList: null };
    this.copiesOf = null;
  }

  toggleDetector(detector: Detector) {
    const list = this.filters.detectors;
    this.filters.detectors = list.includes(detector) ? list.filter((d) => d !== detector) : [...list, detector];
  }

  toggleAttribute(attribute: Attribute) {
    const list = this.filters.attributes;
    this.filters.attributes = list.includes(attribute) ? list.filter((a) => a !== attribute) : [...list, attribute];
  }

  toggleScope(siteId: string) {
    if (this.scope.has(siteId)) this.scope.delete(siteId);
    else this.scope.add(siteId);
  }
}

const RECENTS_KEY = 'prospector.recents';

function loadRecents(): string[] {
  if (!inTauri) return recentSearches;
  try {
    const stored = JSON.parse(localStorage.getItem(RECENTS_KEY) ?? '[]');
    return Array.isArray(stored) ? stored.filter((q): q is string => typeof q === 'string') : [];
  } catch {
    return [];
  }
}

const KINDS: readonly ResultKind[] = ['pdf', 'word', 'excel', 'powerpoint', 'text', 'code', 'archive', 'email', 'image', 'folder'];
const LANGS: readonly DocLang[] = ['en', 'fr', 'es', 'ar', 'und'];

export function fromHitDto(dto: HitDto): SearchHit {
  return {
    id: `${dto.siteId}:${dto.path}`,
    siteId: dto.siteId,
    path: dto.path,
    kind: (KINDS as readonly string[]).includes(dto.kind) ? (dto.kind as ResultKind) : 'text',
    lang: (LANGS as readonly string[]).includes(dto.lang) ? (dto.lang as DocLang) : 'und',
    sizeBytes: dto.sizeBytes,
    modified: new Date(dto.modified * 1000),
    // Saved results from before lot 5.8 have no creation date.
    created: new Date((dto.created ?? dto.modified) * 1000),
    detections: dto.detections,
    matchCount: dto.matchCount,
    exactCount: dto.exactCount,
    score: dto.score,
    snippets: dto.snippets,
    innerKind: dto.innerKind && (KINDS as readonly string[]).includes(dto.innerKind) ? (dto.innerKind as ResultKind) : undefined,
  };
}

export const search = new SearchStore();
