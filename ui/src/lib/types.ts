/**
 * Types shared by the UI. They mirror the payloads the Rust backend will emit
 * at Étape 1 (Tauri commands/events). The backend never sends translated
 * text: statuses and errors are codes, translated by the UI (goal.md §5bis-A).
 */

/** `und`: language not detected (or not one of the four). */
export type DocLang = 'en' | 'fr' | 'es' | 'ar' | 'und';

export type FileKind = 'pdf' | 'word' | 'excel' | 'powerpoint' | 'text' | 'code' | 'archive' | 'email' | 'image';
/** What a result can be: a file of one family, or a folder (found by its name). */
export type ResultKind = FileKind | 'folder';

/** `empty`: added but never indexed. `watching` arrives with the file watcher (Étape 3). */
export type SiteStatus = 'ready' | 'indexing' | 'watching' | 'error' | 'empty';

export type IndexPhase = 'scanning' | 'reading' | 'saving';

export type SiteErrorCode = 'rootUnavailable';

export interface IndexSite {
  id: string;
  /** User-chosen name: data, not translated. */
  name: string;
  roots: string[];
  docCount: number;
  sizeBytes: number;
  status: SiteStatus;
  lastIndexed: Date | null;
  /** 0..1 while `status === 'indexing'` (reading phase). */
  progress?: number;
  phase?: IndexPhase;
  /** Files found so far (scanning phase). */
  found?: number;
  /** Files that could not be read at the last indexing. */
  skipped?: number;
  error?: { code: SiteErrorCode; path: string };
}

export interface SavedSearch {
  id: string;
  label: string;
  query: string;
}

/**
 * Text containing matches. At Étape 0 the matches are delimited by ⟦ ⟧ in mock
 * strings; the backend will send character offsets on the original text.
 */
export type MarkedText = string;

export interface Snippet {
  line: number;
  text: MarkedText;
}

export interface SearchHit {
  id: string;
  siteId: string;
  /** Full path. For archives and mailboxes, inner parts are joined with " › ". */
  path: string;
  kind: ResultKind;
  lang: DocLang;
  sizeBytes: number;
  modified: Date;
  /** Creation date (lot 5.8). */
  created: Date;
  /** Detectors (lot 6.2): occurrences per kind of data. */
  detections?: Record<string, number>;
  matchCount: number;
  /** Matches found without typo tolerance (0 = approximate only). */
  exactCount: number;
  score: number;
  snippets: Snippet[];
  /** Family of a file inside an archive or an e-mail (lot 6.7: it can be extracted). */
  innerKind?: ResultKind;
}

export interface PreviewLine {
  n: number;
  text: MarkedText;
}

export interface PreviewDoc {
  hitId: string;
  /** Code: mono with line numbers; sheet (Excel): tables; slides (PowerPoint): cards; otherwise prose. */
  layout: 'code' | 'prose' | 'sheet' | 'slides';
  lines: PreviewLine[];
}

export type Variant = 'journal' | 'ledger' | 'strata';

export type SortKey = 'relevance' | 'matches' | 'modified' | 'name';

export type SizeFilter = 'any' | 'lt1' | '1to10' | '10to100' | 'gt100';

export type DateFilter = 'any' | 'day' | 'week' | 'month' | 'year' | 'custom';

/** Which date the date filter applies to (lot 5.8); `accessed` is read on the disk. */
export type DateField = 'modified' | 'created' | 'accessed';

/** Detectors (lot 6.2): validated personal and business data. */
export type Detector = 'iban' | 'bic' | 'rib' | 'card' | 'email' | 'phone' | 'ip' | 'amount' | 'vat' | 'siren' | 'nir' | 'dni' | 'mrz';

/** File attributes (lot 5.8), read on the disk at search time. */
export type Attribute = 'readOnly' | 'hidden' | 'system';
