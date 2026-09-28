/**
 * Dig sites. In Tauri they come from the engine (sites.json) and are indexed
 * in the background; outside Tauri the Étape 0 mocks are shown.
 */
import { api, inTauri, type ProgressDto, type ShareStatusDto, type SiteDto } from '../api';
import { t } from '../i18n/index.svelte';
import { sites as mockSites } from '../mock/data';
import type { IndexSite } from '../types';
import { groups } from './groups.svelte';
import { notices } from './notices.svelte';
import { search } from './search.svelte';

function fromDto(dto: SiteDto): IndexSite {
  const missing = dto.status === 'error' ? dto.missingRoots[0] : undefined;
  return {
    id: dto.id,
    name: dto.name,
    roots: dto.roots,
    docCount: dto.docCount,
    sizeBytes: dto.sizeBytes,
    status: dto.status,
    lastIndexed: dto.lastIndexed ? new Date(dto.lastIndexed * 1000) : null,
    skipped: Object.values(dto.skipped ?? {}).reduce((a, b) => a + b, 0),
    error: missing ? { code: 'rootUnavailable', path: missing } : undefined,
  };
}

/** Last segment of a Windows or POSIX path: the default site name. */
function folderName(path: string): string {
  const parts = path.replace(/[\\/]+$/, '').split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

/** Browser demo of a shared index read from another PC (`?shared`, screenshots). */
const DEMO_SHARE: ShareStatusDto | null =
  !inTauri && new URLSearchParams(location.search).has('shared') ? { shared: true, holder: 'PC-BUREAU', pc: 'PORTABLE' } : null;

class SitesStore {
  list = $state<IndexSite[]>(inTauri ? [] : mockSites);
  loaded = $state(!inTauri);
  /** Shared index (lot 7.3); null outside Tauri. */
  share = $state<ShareStatusDto | null>(DEMO_SHARE);
  /** Another PC keeps the shared index up to date: this one only reads. */
  readOnly = $derived(this.share?.holder != null);

  async init() {
    if (!inTauri) return;
    await api.onIndexProgress((p) => this.#onProgress(p));
    await api.onIndexFinished((f) => {
      if (f.site) this.#replace(fromDto(f.site));
      if (f.error) notices.error(f.error);
      // New content (update, folder watcher): refresh the current results.
      void search.refresh();
    });
    await api.onShareStatus((s) => (this.share = s));
    // The other PC added, removed or updated sites (or the roles changed).
    await api.onSitesChanged(() => {
      void this.refresh().then(() => search.refresh());
    });
    this.share = await api.getShareStatus().catch(() => null);
    await this.refresh();
    await groups.load();
    // Every site is searched by default.
    for (const site of this.list) search.scope.add(site.id);
  }

  async refresh() {
    try {
      this.list = (await api.listSites()).map(fromDto);
    } catch (e) {
      notices.error(e);
    } finally {
      this.loaded = true;
    }
  }

  #replace(site: IndexSite) {
    this.list = this.list.map((s) => (s.id === site.id ? site : s));
  }

  #patch(id: string, patch: Partial<IndexSite>) {
    this.list = this.list.map((s) => (s.id === id ? { ...s, ...patch } : s));
  }

  #onProgress(p: ProgressDto) {
    this.#patch(p.siteId, {
      status: 'indexing',
      phase: p.phase,
      found: p.phase === 'scanning' ? p.total : undefined,
      progress: p.phase === 'scanning' ? undefined : p.total ? p.done / p.total : 1,
    });
  }

  /** Native folder picker → new site → first indexing. */
  async addFolder() {
    try {
      const path = await api.pickFolder(t('rail.pickFolder'));
      if (!path) return;
      const site = fromDto(await api.addSite(folderName(path), [path]));
      this.list = [...this.list, site];
      // Shared index: a folder of this PC's own disks cannot be opened from the others.
      if (this.share?.shared && !site.roots.every((r) => r.startsWith('\\\\'))) {
        notices.push(t('share.localSite', { name: site.name }));
      }
      search.scope.add(site.id);
      await this.index(site.id);
    } catch (e) {
      notices.error(e);
    }
  }

  async index(id: string) {
    this.#patch(id, { status: 'indexing', phase: 'scanning', found: 0, progress: undefined, error: undefined });
    try {
      await api.indexSite(id, search.filters.excluded);
    } catch (e) {
      notices.error(e);
      await this.refresh();
    }
  }

  async cancel(id: string) {
    await api.cancelIndex(id).catch((e) => notices.error(e));
  }

  async remove(site: IndexSite) {
    const ok = await api.confirm(t('rail.removeConfirm', { name: site.name }), t('rail.remove')).catch(() => false);
    if (!ok) return;
    try {
      await api.removeSite(site.id);
      this.list = this.list.filter((s) => s.id !== site.id);
      search.scope.delete(site.id);
      search.dropSite(site.id);
      groups.forgetSite(site.id);
    } catch (e) {
      notices.error(e);
    }
  }
}

export const sitesStore = new SitesStore();
