/**
 * Saved searches (★). In Tauri they live in the data folder (Rust,
 * `saved-searches.json`); outside Tauri in localStorage, seeded with the
 * Étape 0 mocks. A saved search restores the query and everything that
 * shapes the results: mode, options, filters (not the exclusions, which are
 * global) and the sites searched.
 */
import { api, inTauri, type SavedSearchDto, type SearchRequestDto } from '../api';
import { t } from '../i18n/index.svelte';
import { savedSearches as mockSaved } from '../mock/data';
import type { Attribute, DateField, DateFilter, Detector, DocLang, FileKind, SizeFilter } from '../types';
import { notices } from './notices.svelte';
import { buildRequest, DISK_FILTERS_OFF, search, type SearchMode, type SearchOptions } from './search.svelte';
import { sitesStore } from './sites.svelte';

export interface SavedSettings {
  mode: SearchMode;
  options: SearchOptions;
  filters: {
    kinds: FileKind[];
    size: SizeFilter;
    date: DateFilter;
    langs: DocLang[];
    /** Lot 5.8; absent when off, so searches saved before still match. */
    dateField?: DateField;
    dateFrom?: string;
    dateTo?: string;
    attributes?: Attribute[];
    hash?: string;
    /** Lot 6.2; absent when off. */
    detectors?: Detector[];
  };
  sites: string[];
  /** File-name criterion (lot 5.1); absent in searches saved before. */
  names?: { pattern: string; folders: boolean };
  /** Explorer right-click (lot 5.7); absent when the search is not limited. */
  folder?: string;
}

export interface SavedEntry {
  id: string;
  label: string;
  query: string;
  /** `null` for the Étape 0 mocks: only the query is restored. */
  settings: SavedSettings | null;
  /** "Alert me" (lot 6.1): on when present; `fresh` = announced, not looked at yet. */
  alert: { fresh: string[] } | null;
}

const STORAGE_KEY = 'prospector.saved';

/** JSON with sorted keys: Rust sends the settings back with its own key order. */
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>).toSorted(([a], [b]) => a.localeCompare(b));
    return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

/** The settings of the search on screen. */
function snapshot(): SavedSettings {
  const { options, filters } = search;
  return {
    mode: search.mode,
    options: {
      caseSensitive: options.caseSensitive,
      wholeWord: options.wholeWord,
      regex: options.regex,
      fuzzy: options.fuzzy,
    },
    filters: {
      kinds: filters.kinds.toSorted(),
      size: filters.size,
      date: filters.date,
      langs: filters.langs.toSorted(),
      ...(filters.dateField !== 'modified' ? { dateField: filters.dateField } : {}),
      ...(filters.date === 'custom' ? { dateFrom: filters.dateFrom, dateTo: filters.dateTo } : {}),
      ...(filters.attributes.length > 0 ? { attributes: filters.attributes.toSorted() } : {}),
      ...(filters.hash.trim() ? { hash: filters.hash.trim() } : {}),
      ...(filters.detectors.length > 0 ? { detectors: filters.detectors.toSorted() } : {}),
    },
    sites: [...search.scope].toSorted(),
    names: { pattern: search.namePattern.trim(), folders: search.folders },
    // Only when set: searches saved before still count as the same.
    ...(search.folder ? { folder: search.folder } : {}),
  };
}

function fromDto(dto: SavedSearchDto): SavedEntry {
  return {
    id: dto.id,
    label: dto.label,
    query: dto.query,
    settings: (dto.settings as SavedSettings | null) ?? null,
    alert: dto.alert ?? null,
  };
}

function loadLocal(): SavedEntry[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return JSON.parse(stored) as SavedEntry[];
  } catch {
    /* unreadable: start from the mocks */
  }
  // Browser demo: the first one has an alert with news (screenshots).
  return mockSaved.map((s, i) => ({ ...s, settings: null, alert: i === 0 ? { fresh: ['a', 'b', 'c'] } : null }));
}

class SavedStore {
  list = $state<SavedEntry[]>(inTauri ? [] : loadLocal());

  /** The search on screen is already saved (★ filled). */
  current = $derived.by(() => {
    if (!search.hasRun) return null;
    const settings = canonical(snapshot());
    return this.list.find((s) => s.query === search.lastQuery && s.settings && canonical(s.settings) === settings) ?? null;
  });

  async init() {
    if (!inTauri) return;
    // New documents announced by an alert: the badge grows.
    await api.onAlertNews((news) => {
      this.list = this.list.map((entry) => {
        const item = news.find((n) => n.savedId === entry.id);
        return item && entry.alert ? { ...entry, alert: { fresh: [...entry.alert.fresh, ...item.found] } } : entry;
      });
    });
    try {
      this.list = (await api.listSavedSearches()).map(fromDto);
    } catch (e) {
      notices.error(e);
    }
  }

  /** Saves the search on screen (renames it if it is already saved). */
  async save(label: string) {
    const query = search.lastQuery;
    const settings = snapshot();
    const name = label.trim() || query;
    try {
      const entry = inTauri
        ? fromDto(await api.saveSearch(name, query, settings))
        : { id: this.current?.id ?? `s${Date.now().toString(36)}`, label: name, query, settings, alert: this.current?.alert ?? null };
      this.list = this.list.some((s) => s.id === entry.id)
        ? this.list.map((s) => (s.id === entry.id ? entry : s))
        : [...this.list, entry];
      this.#persistLocal();
      notices.push(t('saved.done', { label: entry.label }));
    } catch (e) {
      notices.error(e);
    }
  }

  async remove(id: string) {
    try {
      if (inTauri) await api.removeSavedSearch(id);
      this.list = this.list.filter((s) => s.id !== id);
      this.#persistLocal();
    } catch (e) {
      notices.error(e);
    }
  }

  /**
   * "Alert me" on or off (lot 6.1). On: the engine request is built from the
   * saved settings, without relative dates (a file that just changed is
   * recent anyway); what it finds now is remembered, never announced.
   */
  async toggleAlert(entry: SavedEntry) {
    const on = entry.alert === null;
    if (!inTauri) {
      this.#replace({ ...entry, alert: on ? { fresh: [] } : null });
      return;
    }
    try {
      const known = new Set(sitesStore.list.map((s) => s.id));
      const settings = entry.settings;
      const sites = (settings?.sites ?? [...search.scope]).filter((id) => known.has(id));
      let request: SearchRequestDto | null = null;
      if (on) {
        const filters = { ...search.filters, ...DISK_FILTERS_OFF, attributes: [], ...settings?.filters };
        request = buildRequest({
          query: entry.query,
          options: settings?.options ?? search.options,
          filters: { ...filters, date: filters.date === 'custom' ? 'custom' : 'any' },
          namePattern: settings?.names?.pattern ?? '',
          folders: settings?.names?.folders ?? false,
          folder: settings?.folder ?? null,
          copies: null,
        });
      }
      this.#replace(fromDto(await api.setAlert(entry.id, request, sites)));
      notices.push(t(on ? 'alerts.on' : 'alerts.off', { label: entry.label }));
    } catch (e) {
      notices.error(e);
    }
  }

  #replace(entry: SavedEntry) {
    this.list = this.list.map((s) => (s.id === entry.id ? entry : s));
    this.#persistLocal();
  }

  /** Restores a saved search and runs it. */
  apply(entry: SavedEntry) {
    search.query = entry.query;
    const settings = entry.settings;
    if (settings) {
      search.mode = settings.mode;
      search.options = { ...settings.options };
      search.filters = { ...search.filters, ...DISK_FILTERS_OFF, attributes: [], ...settings.filters };
      search.copiesOf = null;
      search.namePattern = settings.names?.pattern ?? '';
      search.folders = settings.names?.folders ?? false;
      search.folder = settings.folder ?? null;
      // Sites removed since are skipped; if none is left, the scope stays.
      const known = new Set(sitesStore.list.map((s) => s.id));
      const sites = settings.sites.filter((id) => known.has(id));
      if (sites.length > 0) {
        search.scope.clear();
        for (const id of sites) search.scope.add(id);
      }
    }
    // run() records the new settings: App.svelte does not run it a second time.
    void search.run();
    // Opened from an alert: its news are marked, and the badge goes.
    const fresh = entry.alert?.fresh ?? [];
    if (fresh.length > 0) {
      search.alertNew = new Set(fresh);
      this.#replace({ ...entry, alert: { fresh: [] } });
      if (inTauri) void api.markAlertRead(entry.id).catch((e) => notices.error(e));
    }
  }

  #persistLocal() {
    if (inTauri) return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.list));
    } catch {
      /* not persisted */
    }
  }
}

export const saved = new SavedStore();
