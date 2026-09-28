/**
 * Bridge to the Rust engine (src-tauri/src/commands.rs).
 * Outside Tauri (browser, headless screenshots) `inTauri` is false and the
 * stores fall back to the mock data of Étape 0.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ask, open, save } from '@tauri-apps/plugin-dialog';
import type { KeywordReport } from './export';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// ── Payloads (serde camelCase) ────────────────────────────────────────

/** Error as sent by Rust: `{ code, params }` (AppError wraps CoreError in `core`). */
export interface ApiError {
  code: string;
  params?: Record<string, unknown> & { code?: string; params?: Record<string, unknown> };
}

export interface SiteDto {
  id: string;
  name: string;
  roots: string[];
  docCount: number;
  sizeBytes: number;
  lastIndexed: number | null;
  skipped: Record<string, number>;
  missingRoots: string[];
  status: 'ready' | 'watching' | 'indexing' | 'error' | 'empty';
  error: ApiError | null;
  /** Meaning index ticked (Étape 8). */
  sense?: boolean;
  /** Being computed: passages done / to do. */
  senseProgress?: { done: number; total: number } | null;
  /** Passages already understood. */
  sensePassages?: number | null;
}

export interface SnippetDto {
  line: number;
  text: string;
}

export interface HitDto {
  siteId: string;
  path: string;
  kind: string;
  lang: string;
  sizeBytes: number;
  modified: number;
  /** Absent in results saved before lot 5.8. */
  created?: number;
  /** Detectors (lot 6.2): occurrences per kind of data. */
  detections?: Record<string, number>;
  matchCount: number;
  exactCount: number;
  score: number;
  snippets: SnippetDto[];
  /** Family of a file inside an archive or an e-mail (lot 6.7); absent otherwise. */
  innerKind?: string;
  /** Found by meaning too (lot 8.3). */
  meaning?: MeaningDto;
}

export interface SearchResponseDto {
  hits: HitDto[];
  totalFiles: number;
  exactFiles: number;
  tookMs: number;
  /** Detectors (lot 6.2): per kind, over every file found (not only those returned). */
  detections?: Record<string, { files: number; matches: number }>;
  /** Counts per criterion (lot 6.3), when asked. */
  facets?: FacetsDto;
}

/** Documents found per kind, language, year (`"2026"`) and site id (lot 6.3). */
export interface FacetsDto {
  kinds: Record<string, number>;
  langs: Record<string, number>;
  years: Record<string, number>;
  sites: Record<string, number>;
}

/** How a result answers the meaning of the search (lot 8.3). */
export interface MeaningDto {
  score: number;
  start: number;
  end: number;
  /** Found by meaning only: no word of the search in it. */
  only: boolean;
}

export interface SearchRequestDto {
  query: string;
  fuzzy: boolean;
  /** Also by meaning (lot 8.3). */
  meaning?: boolean;
  /** `Aa` chip. */
  caseSensitive: boolean;
  /** `ab` chip. */
  wholeWord: boolean;
  /** `.*` chip: the whole input is a regular expression. */
  regex: boolean;
  kinds: string[];
  langs: string[];
  minSize?: number;
  maxSize?: number;
  /** Which date the range applies to (lot 5.8). */
  dateField?: 'modified' | 'created' | 'accessed';
  /** Unix seconds, `[dateFrom, dateTo)`. */
  dateFrom?: number;
  dateTo?: number;
  /** Required attributes (`readOnly`, `hidden`, `system`), read on the disk. */
  attributes?: string[];
  /** MD5 or SHA-256 of the file, computed on the disk. */
  hash?: string;
  /** Detectors (lot 6.2): documents holding any of these data. */
  detectors?: string[];
  /** Terms read from a file (lot 7.4): any of them, or all (`termListAll`). */
  termList?: string[];
  termListAll?: boolean;
  /** Also count per kind, language, year and site (lot 6.3). */
  facets?: boolean;
  /** File-name criterion: `*.pdf; facture-*`, `!exclude`, `/regex/`. */
  namePattern?: string;
  /** Look for folders (by name) instead of files. */
  folders?: boolean;
  /** "Search within these results": only these paths. */
  withinPaths?: string[];
  /** Explorer right-click (lot 5.7): only what is inside this folder. */
  inFolder?: string;
  limit?: number;
}

export interface PreviewDto {
  path: string;
  layout: 'code' | 'prose' | 'sheet' | 'slides';
  lines: { n: number; text: string }[];
  matchCount: number;
  truncated: boolean;
}

export interface ProgressDto {
  siteId: string;
  phase: 'scanning' | 'reading' | 'saving';
  total: number;
  done: number;
}

export interface FinishedDto {
  siteId: string;
  site: SiteDto | null;
  error: ApiError | null;
}

export interface ScanFinishedDto {
  scanId: string;
  summary: { filesScanned: number; hits: number; tookMs: number; cancelled: boolean } | null;
  error: ApiError | null;
}

/** What ticking "Meaning" on a site costs (lot 8.2). */
export interface SenseEstimateDto {
  passages: number;
  /** At this PC's measured speed; null without the module. */
  seconds: number | null;
  /** Passages already computed. */
  done: number;
}

/** Progress of a site's meaning index. */
export interface SenseSiteDto {
  siteId: string;
  done: number;
  total: number;
  finished: boolean;
}

/** Meaning-search module (Étape 8). */
export interface SenseStatusDto {
  installed: boolean;
  model: string | null;
  /** Bytes on disk. */
  size: number;
  /** Can be downloaded (the release address is known). */
  downloadable: boolean;
  downloadSize: number;
}

/** Shared index (lot 7.3). */
export interface ShareStatusDto {
  /** The data folder is on a network share. */
  shared: boolean;
  /** Another PC keeps it up to date (its name); null = this one. */
  holder: string | null;
  /** This computer's name. */
  pc: string;
}

/** A site group (lot 7.2); `color` indexes the group pencils. */
export interface SiteGroupDto {
  id: string;
  name: string;
  siteIds: string[];
  color: number;
}

export interface SavedSearchDto {
  id: string;
  label: string;
  query: string;
  created: number;
  /** UI-owned settings (mode, options, filters, sites), stored as is by Rust. */
  settings: unknown;
  /** "Alert me" (lot 6.1): on when present; `fresh` = announced, not looked at yet. */
  alert: { fresh: string[] } | null;
}

/** What a copy of the files found did (lot 6.4). */
export interface CopyReportDto {
  copied: number;
  skipped: string[];
  fromContainers: number;
  target: string;
  cancelled: boolean;
}

/** Duplicates (lot 6.5). */
export interface DupFileDto {
  siteId: string;
  path: string;
  size: number;
  modified: number;
}

export interface DupGroupDto {
  kind: 'exact' | 'similar';
  similarity: number;
  files: DupFileDto[];
  wasted: number;
}

export interface DupReportDto {
  groups: DupGroupDto[];
  filesHashed: number;
  docsCompared: number;
  tookMs: number;
  cancelled: boolean;
}

export interface DupProgressDto {
  dupId: string;
  phase: 'hashing' | 'comparing';
  done: number;
  total: number;
}

/** An alert found new documents (lot 6.1). */
export interface AlertNewsDto {
  savedId: string;
  label: string;
  found: string[];
}

/** Keep running near the clock, start with Windows (lot 6.1). */
export interface BackgroundDto {
  keepRunning: boolean;
  /** Not chosen yet: on as soon as an alert exists. */
  automatic: boolean;
  startWithWindows: boolean;
  /** Portable mode (lot 6.8): start with Windows is not offered. */
  portable?: boolean;
}

export interface OcrDto {
  /** Switched on in Settings. */
  enabled: boolean;
  /** Windows has a recognizer language installed. */
  available: boolean;
  /** BCP 47 tags (`fr-FR`, `ar-SA`…). */
  languages: string[];
}

export interface ShortcutDto {
  /** Tauri accelerator (`CmdOrCtrl+Shift+Space`), `''` = none. */
  shortcut: string;
  /** `false`: another application already uses it. */
  active: boolean;
}

/** Windows integration (lot 5.7): Explorer menu and code editor. */
export interface IntegrationDto {
  explorerMenu: boolean;
  /** Portable mode (lot 6.8): the Explorer menu is not offered. */
  portable?: boolean;
  /** prospector-cli.exe is next to the program (lot 7.1). */
  cliAvailable?: boolean;
  /** Its folder is in the user's PATH. */
  cliInPath?: boolean;
  /** Known code editors installed on this PC. */
  editors: { id: string; name: string }[];
  /** Chosen editor id (`custom` for the command), `''` = the first found. */
  editor: string;
  /** Custom command, with `{file}` and `{line}`. */
  editorCommand: string;
  /** An editor can be used. */
  editorReady: boolean;
}

/** What a launch asks for: the Explorer's folder, or a `.prospector` file. */
export type LaunchRequestDto =
  | { kind: 'folder'; path: string; siteId: string | null }
  | { kind: 'results'; path: string };

export interface AppInfoDto {
  version: string;
  dataDir: string;
  defaultDataDir: string;
  /** Automatic updates are configured in this build. */
  updatesEnabled: boolean;
  /** Portable mode (lot 6.8): the data folder next to the program. */
  portableDir?: string | null;
}

/** Innermost error code (AppError `core` wraps the engine's CoreError). */
export function errorCode(e: unknown): { code: string; params: Record<string, unknown> } {
  const err = e as ApiError | undefined;
  if (err && typeof err === 'object' && typeof err.code === 'string') {
    if (err.code === 'core' && err.params && typeof err.params.code === 'string') {
      return { code: err.params.code, params: (err.params.params as Record<string, unknown>) ?? {} };
    }
    return { code: err.code, params: (err.params as Record<string, unknown>) ?? {} };
  }
  return { code: 'unknown', params: {} };
}

// ── Commands ──────────────────────────────────────────────────────────

export const api = {
  appInfo: () => invoke<AppInfoDto>('app_info'),
  listSites: () => invoke<SiteDto[]>('list_sites'),
  addSite: (name: string, roots: string[]) => invoke<SiteDto>('add_site', { name, roots }),
  removeSite: (id: string) => invoke<void>('remove_site', { id }),
  indexSite: (id: string, excluded: string[]) => invoke<void>('index_site', { id, excluded }),
  cancelIndex: (id: string) => invoke<void>('cancel_index', { id }),
  search: (siteIds: string[], request: SearchRequestDto) => invoke<SearchResponseDto>('search', { siteIds, request }),
  /** `passage`: the passage found by meaning, marked when no word is (lot 8.3). */
  preview: (siteId: string, path: string, request: SearchRequestDto, passage?: [number, number] | null) =>
    invoke<PreviewDto>('preview', { siteId, path, request, passage: passage ?? null }),
  liveScan: (scanId: string, siteIds: string[], excluded: string[], request: SearchRequestDto) =>
    invoke<void>('live_scan', { scan: { scanId, siteIds, excluded, request } }),
  cancelScan: (scanId: string) => invoke<void>('cancel_scan', { scanId }),
  onScanHit: (cb: (e: { scanId: string; hit: HitDto }) => void): Promise<UnlistenFn> =>
    listen<{ scanId: string; hit: HitDto }>('scan://hit', (e) => cb(e.payload)),
  onScanProgress: (cb: (e: { scanId: string; scanned: number; total: number }) => void): Promise<UnlistenFn> =>
    listen<{ scanId: string; scanned: number; total: number }>('scan://progress', (e) => cb(e.payload)),
  onScanFinished: (cb: (e: ScanFinishedDto) => void): Promise<UnlistenFn> =>
    listen<ScanFinishedDto>('scan://finished', (e) => cb(e.payload)),
  listSavedSearches: () => invoke<SavedSearchDto[]>('list_saved_searches'),
  /** Meaning-search module (Étape 8). */
  senseStatus: () => invoke<SenseStatusDto>('sense_status'),
  installSenseModule: (path: string) => invoke<SenseStatusDto>('install_sense_module', { path }),
  downloadSenseModule: () => invoke<SenseStatusDto>('download_sense_module'),
  cancelSenseDownload: () => invoke<void>('cancel_sense_download'),
  removeSenseModule: () => invoke<SenseStatusDto>('remove_sense_module'),
  onSenseProgress: (cb: (p: { done: number; total: number }) => void): Promise<UnlistenFn> =>
    listen<{ done: number; total: number }>('sense://progress', (e) => cb(e.payload)),
  /** Meaning index of the sites (lot 8.2). */
  senseEstimate: (id: string) => invoke<SenseEstimateDto>('sense_estimate', { id }),
  setSiteSense: (id: string, on: boolean) => invoke<SiteDto>('set_site_sense', { id, on }),
  getSensePace: () => invoke<'normal' | 'economy'>('get_sense_pace'),
  setSensePace: (pace: 'normal' | 'economy') => invoke<'normal' | 'economy'>('set_sense_pace', { pace }),
  onSenseSite: (cb: (p: SenseSiteDto) => void): Promise<UnlistenFn> => listen<SenseSiteDto>('sense://site', (e) => cb(e.payload)),
  /** The terms of a list file (lot 7.4). */
  readTermList: (path: string) => invoke<{ terms: string[]; skipped: number }>('read_term_list', { path }),
  /** Shared index (lot 7.3): who keeps it up to date. */
  getShareStatus: () => invoke<ShareStatusDto>('get_share_status'),
  onShareStatus: (cb: (s: ShareStatusDto) => void): Promise<UnlistenFn> => listen<ShareStatusDto>('share://status', (e) => cb(e.payload)),
  /** The other PC changed the shared catalog (or the role changed). */
  onSitesChanged: (cb: () => void): Promise<UnlistenFn> => listen('sites://changed', () => cb()),
  /** Site groups (lot 7.2). */
  listSiteGroups: () => invoke<SiteGroupDto[]>('list_site_groups'),
  createSiteGroup: (name: string, siteIds: string[]) => invoke<SiteGroupDto>('create_site_group', { name, siteIds }),
  updateSiteGroup: (id: string, name?: string, siteIds?: string[]) =>
    invoke<SiteGroupDto>('update_site_group', { id, name: name ?? null, siteIds: siteIds ?? null }),
  removeSiteGroup: (id: string) => invoke<void>('remove_site_group', { id }),
  saveSearch: (label: string, query: string, settings: unknown) =>
    invoke<SavedSearchDto>('save_search', { label, query, settings }),
  removeSavedSearch: (id: string) => invoke<void>('remove_saved_search', { id }),
  /** On with the engine request built from the saved settings; off with null. */
  setAlert: (id: string, request: SearchRequestDto | null, siteIds: string[]) =>
    invoke<SavedSearchDto>('set_alert', { id, request, siteIds }),
  markAlertRead: (id: string) => invoke<SavedSearchDto>('mark_alert_read', { id }),
  onAlertNews: (cb: (news: AlertNewsDto[]) => void): Promise<UnlistenFn> =>
    listen<AlertNewsDto[]>('alerts://news', (e) => cb(e.payload)),
  /** Tray menu and notification texts in the interface language. */
  syncShellTexts: (texts: { trayOpen: string; trayQuit: string; alertTitle: string }) =>
    invoke<void>('sync_shell_texts', { texts }),
  getBackground: () => invoke<BackgroundDto>('get_background'),
  setBackground: (keepRunning: boolean) => invoke<BackgroundDto>('set_background', { keepRunning }),
  setStartWithWindows: (enabled: boolean) => invoke<BackgroundDto>('set_start_with_windows', { enabled }),
  getOcr: () => invoke<OcrDto>('get_ocr'),
  setOcr: (enabled: boolean) => invoke<OcrDto>('set_ocr', { enabled }),
  getShortcut: () => invoke<ShortcutDto>('get_shortcut'),
  setShortcut: (shortcut: string) => invoke<ShortcutDto>('set_shortcut', { shortcut }),
  /** The global shortcut brought the window up. */
  onSummon: (cb: () => void): Promise<UnlistenFn> => listen('app://summon', () => cb()),
  /** Print dialog of the window (WebView2 / WKWebView / WebKitGTK). */
  printReport: () => invoke<void>('print_report'),
  setDataDir: (path: string) => invoke<AppInfoDto>('set_data_dir', { path }),
  /** Portable mode: the page of the new version, in the browser (lot 6.8). */
  openReleasesPage: () => invoke<void>('open_releases_page'),
  /** Opens a file with its application; a file inside an archive or an e-mail is extracted first (lot 6.7). */
  openFile: (path: string) => invoke<void>('open_file', { path }),
  /** "Extract to…" (lot 6.7): writes a file inside an archive or an e-mail to `target`. */
  extractTo: (path: string, target: string) => invoke<void>('extract_to', { path, target }),
  revealFile: (path: string) => invoke<void>('reveal_file', { path }),
  saveTextFile: (path: string, contents: string) => invoke<void>('save_text_file', { path, contents }),
  readTextFile: (path: string) => invoke<string>('read_text_file', { path }),
  getIntegration: () => invoke<IntegrationDto>('get_integration'),
  /** Writes the Explorer menu again (if on) with this label. */
  syncExplorerMenu: (label: string) => invoke<IntegrationDto>('sync_explorer_menu', { label }),
  setExplorerMenu: (enabled: boolean, label: string) => invoke<IntegrationDto>('set_explorer_menu', { enabled, label }),
  setEditor: (editor: string, command: string) => invoke<IntegrationDto>('set_editor', { editor, command }),
  /** Adds or removes the command line's folder in the user's PATH (lot 7.1). */
  setCliPath: (enabled: boolean) => invoke<IntegrationDto>('set_cli_path', { enabled }),
  openInEditor: (path: string, line: number) => invoke<void>('open_in_editor', { path, line }),
  /** SHA-256 and size of a file on disk ("Find copies"). */
  fileDigest: (path: string) => invoke<{ sha256: string; size: number }>('file_digest', { path }),
  /** Occurrences of each term of the query in each document (lot 6.4). */
  keywordReport: (docs: { siteId: string; path: string }[], request: SearchRequestDto) =>
    invoke<KeywordReport>('keyword_report', { docs, request }),
  /** Copies the files behind `paths` to a folder or a ZIP (lot 6.4); events follow. */
  copyFiles: (copyId: string, paths: string[], target: string, zip: boolean, keepTree: boolean, extract: boolean) =>
    invoke<void>('copy_files', { copyId, paths, target, zip, keepTree, extract }),
  cancelCopy: (copyId: string) => invoke<void>('cancel_copy', { copyId }),
  /** Duplicates in these sites (lot 6.5); events follow. */
  findDuplicates: (dupId: string, siteIds: string[], options: { exact: boolean; similar: boolean; threshold: number }) =>
    invoke<void>('find_duplicates', { dupId, siteIds, options }),
  cancelDuplicates: (dupId: string) => invoke<void>('cancel_duplicates', { dupId }),
  onDupProgress: (cb: (e: DupProgressDto) => void): Promise<UnlistenFn> => listen<DupProgressDto>('dup://progress', (e) => cb(e.payload)),
  onDupFinished: (cb: (e: { dupId: string; report: DupReportDto | null; error: ApiError | null }) => void): Promise<UnlistenFn> =>
    listen<{ dupId: string; report: DupReportDto | null; error: ApiError | null }>('dup://finished', (e) => cb(e.payload)),
  /** A thumbnail of an image file as a `data:` URL (lot 6.6). */
  thumbnail: (path: string, size: number) => invoke<string>('thumbnail', { path, size }),
  /** Boxes (fractions of the image) of the words matching the search (lot 6.6). */
  imageMatches: (path: string, request: SearchRequestDto) =>
    invoke<{ x: number; y: number; w: number; h: number; fuzzy: boolean }[]>('image_matches', { path, request }),
  /** To the Recycle Bin; returns the paths that could not be moved. */
  trashFiles: (paths: string[]) => invoke<string[]>('trash_files', { paths }),
  onCopyProgress: (cb: (e: { copyId: string; done: number; total: number }) => void): Promise<UnlistenFn> =>
    listen<{ copyId: string; done: number; total: number }>('copy://progress', (e) => cb(e.payload)),
  onCopyFinished: (cb: (e: { copyId: string; report: CopyReportDto | null; error: ApiError | null }) => void): Promise<UnlistenFn> =>
    listen<{ copyId: string; report: CopyReportDto | null; error: ApiError | null }>('copy://finished', (e) => cb(e.payload)),
  /** Folders / results files this launch (or a later one) asked to open. */
  takeLaunchRequests: () => invoke<LaunchRequestDto[]>('take_launch_requests'),
  /** A second launch handed over its arguments: take them. */
  onLaunch: (cb: () => void): Promise<UnlistenFn> => listen('app://launch', () => cb()),

  onIndexProgress: (cb: (p: ProgressDto) => void): Promise<UnlistenFn> =>
    listen<ProgressDto>('index://progress', (e) => cb(e.payload)),
  onIndexFinished: (cb: (f: FinishedDto) => void): Promise<UnlistenFn> =>
    listen<FinishedDto>('index://finished', (e) => cb(e.payload)),

  /** Native folder picker; null when cancelled. */
  async pickFolder(title: string): Promise<string | null> {
    const picked = await open({ directory: true, multiple: false, title });
    return typeof picked === 'string' ? picked : null;
  },

  /** Native "Open" dialog for one file; null when cancelled. */
  async pickOpenPath(extension: string, filterName: string, title: string): Promise<string | null> {
    const picked = await open({ multiple: false, directory: false, title, filters: [{ name: filterName, extensions: [extension] }] });
    return typeof picked === 'string' ? picked : null;
  },

  /** Native "Save as" dialog; null when cancelled. */
  async pickSavePath(defaultPath: string, extension: string, filterName: string): Promise<string | null> {
    return save({ defaultPath, filters: [{ name: filterName, extensions: [extension] }] });
  },

  /** Native "Open" dialog for a term list (lot 7.4); null when cancelled. */
  async pickTermFile(title: string, filterName: string): Promise<string | null> {
    const picked = await open({ multiple: false, directory: false, title, filters: [{ name: filterName, extensions: ['txt', 'csv', 'tsv', 'lst'] }] });
    return typeof picked === 'string' ? picked : null;
  },

  /** Native "Save as" dialog for any kind of file; null when cancelled. */
  async pickSaveAs(defaultPath: string): Promise<string | null> {
    return save({ defaultPath });
  },

  /** Native yes/no confirmation. */
  confirm: (message: string, title: string) => ask(message, { title, kind: 'warning' }),
};
