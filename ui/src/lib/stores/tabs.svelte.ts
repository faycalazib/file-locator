/**
 * Search tabs and their history (lot 5.6). Only the tab on screen lives in
 * the search store; the others are snapshots (`SearchSnapshot`), switched in
 * and out. A live scan keeps running in a tab that is not shown.
 *
 * History: each new search pushes the one it replaces; Alt+← / Alt+→ bring
 * them back as they were (results included, nothing is searched again).
 */
import { inTauri } from '../api';
import { hits as mockHits } from '../mock/data';
import { search, type SearchSnapshot } from './search.svelte';

interface Tab {
  id: string;
  /** Null for the tab on screen (its state is the search store). */
  snap: SearchSnapshot | null;
  back: SearchSnapshot[];
  forward: SearchSnapshot[];
}

const MAX_TABS = 12;
const MAX_HISTORY = 25;

let nextId = 1;
const newTab = (snap: SearchSnapshot | null): Tab => ({ id: `t${nextId++}`, snap, back: [], forward: [] });

class TabsStore {
  // Raw: tabs are replaced, and `version` tells what changed inside them.
  list = $state.raw<Tab[]>([newTab(null)]);
  activeId = $state(this.list[0]!.id);
  /** Bumped when a tab not on screen or a history changes. */
  version = $state(0);

  constructor() {
    search.beforeRun = (previous) => this.#push(previous);
    search.onBackground = () => this.version++;
    if (!inTauri) this.#demo();
  }

  /** Browser demo: `?tabs=soup,وصفة` opens more tabs (screenshots only). */
  #demo() {
    const queries = new URLSearchParams(location.search).get('tabs')?.split(',').filter(Boolean) ?? [];
    const extra = queries.map((q, i) =>
      newTab({ ...search.blank(), query: q, lastQuery: q, hasRun: true, hits: mockHits.slice(0, 3 + ((i * 5) % 7)) }),
    );
    this.list = [...this.list, ...extra];
  }

  get active(): Tab {
    return this.list.find((t) => t.id === this.activeId) ?? this.list[0]!;
  }

  get canBack(): boolean {
    void this.version;
    return this.active.back.length > 0;
  }

  get canForward(): boolean {
    void this.version;
    return this.active.forward.length > 0;
  }

  get canAdd(): boolean {
    return this.list.length < MAX_TABS;
  }

  /** The search of a tab: live for the tab on screen, else its snapshot. */
  view(tab: Tab): {
    query: string;
    names: string;
    hasRun: boolean;
    count: number;
    busy: boolean;
    openedFrom: string | null;
    /** "Find copies": the file's name. */
    copiesOf: string | null;
  } {
    void this.version;
    if (tab.id === this.activeId || !tab.snap) {
      return {
        query: search.lastQuery,
        names: search.namePattern.trim(),
        hasRun: search.hasRun,
        count: search.hits.length,
        busy: search.scanning,
        openedFrom: search.openedFrom,
        copiesOf: search.copies?.name ?? null,
      };
    }
    const s = tab.snap;
    return {
      query: s.lastQuery,
      names: s.namePattern.trim(),
      hasRun: s.hasRun,
      count: s.hits.length,
      busy: s.scanId !== null,
      openedFrom: s.openedFrom,
      copiesOf: s.copiesOf?.name ?? null,
    };
  }

  #push(previous: SearchSnapshot) {
    const tab = this.active;
    tab.back = [...tab.back, previous].slice(-MAX_HISTORY);
    tab.forward = [];
    this.version++;
  }

  select(id: string) {
    if (id === this.activeId) return;
    const target = this.list.find((t) => t.id === id);
    if (!target?.snap) return;
    const current = this.active;
    current.snap = search.switchTo(target.snap);
    target.snap = null;
    this.activeId = id;
    this.version++;
  }

  /** A new tab with the current settings (sites, options), empty. */
  add(snap: SearchSnapshot = search.blank()) {
    if (!this.canAdd) return;
    const tab = newTab(snap);
    const at = this.list.indexOf(this.active) + 1;
    this.list = [...this.list.slice(0, at), tab, ...this.list.slice(at)];
    this.select(tab.id);
  }

  /** The last tab is never closed: it is emptied instead. */
  close(id: string) {
    const tab = this.list.find((t) => t.id === id);
    if (!tab) return;
    if (this.list.length === 1) {
      search.restore(search.blank());
      tab.back = [];
      tab.forward = [];
      this.version++;
      return;
    }
    const index = this.list.indexOf(tab);
    if (id === this.activeId) {
      const neighbour = this.list[index + 1] ?? this.list[index - 1]!;
      this.select(neighbour.id);
    }
    if (tab.snap) search.discard(tab.snap);
    this.list = this.list.filter((t) => t !== tab);
  }

  /** Ctrl+Tab / Ctrl+Shift+Tab. */
  cycle(delta: number) {
    const index = this.list.indexOf(this.active);
    const next = this.list[(index + delta + this.list.length) % this.list.length];
    if (next) this.select(next.id);
  }

  back() {
    const tab = this.active;
    const previous = tab.back.at(-1);
    if (!previous) return;
    tab.back = tab.back.slice(0, -1);
    tab.forward = [...tab.forward, search.capture(false)];
    search.restore(previous);
    this.version++;
  }

  forward() {
    const tab = this.active;
    const next = tab.forward.at(-1);
    if (!next) return;
    tab.forward = tab.forward.slice(0, -1);
    tab.back = [...tab.back, search.capture(false)];
    search.restore(next);
    this.version++;
  }
}

export const tabs = new TabsStore();
