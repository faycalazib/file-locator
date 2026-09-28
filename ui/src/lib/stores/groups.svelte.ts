/**
 * Site groups (lot 7.2): named sets of dig sites, kept by the engine
 * (`site-groups.json`, shared with `prospector-cli --group`). Applying one
 * ticks exactly its sites in the current tab; Ctrl+0 = every site, Ctrl+1…9
 * = the groups in creation order.
 */
import { api, inTauri, type SiteGroupDto } from '../api';
import { notices } from './notices.svelte';
import { search } from './search.svelte';
import { sitesStore } from './sites.svelte';

/** Pencil of each group (`color` index given by the engine). */
export const GROUP_COLORS = ['var(--blue)', 'var(--orange)', 'var(--purple)', 'var(--pink)', 'var(--yellow)', 'var(--green)'];

export const groupColor = (group: SiteGroupDto) => GROUP_COLORS[group.color % GROUP_COLORS.length]!;

/** Browser demo (screenshots). */
const DEMO: SiteGroupDto[] = [
  { id: 'g1', name: 'Clients', siteIds: ['clients', 'mail'], color: 0 },
  { id: 'g2', name: 'Code', siteIds: ['code'], color: 1 },
  { id: 'g3', name: 'Archives', siteIds: ['mail', 'archives'], color: 2 },
];

function sameSet(a: Iterable<string>, b: readonly string[]): boolean {
  const left = new Set(a);
  return left.size === new Set(b).size && b.every((id) => left.has(id));
}

class GroupsStore {
  list = $state<SiteGroupDto[]>(inTauri ? [] : DEMO);

  async load() {
    if (!inTauri) return;
    try {
      this.list = await api.listSiteGroups();
    } catch (e) {
      notices.error(e);
    }
  }

  /** The groups a site belongs to. */
  of(siteId: string): SiteGroupDto[] {
    return this.list.filter((g) => g.siteIds.includes(siteId));
  }

  /** `'all'`, a group id, or null: which choice the ticked sites match. */
  get active(): string | null {
    const known = new Set(sitesStore.list.map((s) => s.id));
    const scope = [...search.scope].filter((id) => known.has(id));
    if (known.size > 0 && sameSet(scope, [...known])) return 'all';
    return this.list.find((g) => sameSet(scope, g.siteIds.filter((id) => known.has(id))))?.id ?? null;
  }

  /** Ticks exactly these sites (null = every site). */
  apply(group: SiteGroupDto | null) {
    const ids = group ? group.siteIds : sitesStore.list.map((s) => s.id);
    search.scope.clear();
    for (const id of ids) if (sitesStore.list.some((s) => s.id === id)) search.scope.add(id);
  }

  /** Ctrl+0 … Ctrl+9. False when there is no such group. */
  applyIndex(n: number): boolean {
    if (n === 0) {
      this.apply(null);
      return sitesStore.list.length > 0;
    }
    const group = this.list[n - 1];
    if (group) this.apply(group);
    return group !== undefined;
  }

  /** A new group of the ticked sites. */
  async create(name: string) {
    const siteIds = [...search.scope];
    if (!inTauri) {
      this.list = [...this.list, { id: `g${Date.now()}`, name, siteIds, color: this.list.length % GROUP_COLORS.length }];
      return;
    }
    try {
      this.list = [...this.list, await api.createSiteGroup(name, siteIds)];
    } catch (e) {
      notices.error(e);
    }
  }

  /** Renames a group, or gives it the ticked sites. */
  async update(id: string, change: { name?: string; ticked?: boolean }) {
    const siteIds = change.ticked ? [...search.scope] : undefined;
    try {
      const updated = inTauri
        ? await api.updateSiteGroup(id, change.name, siteIds)
        : { ...this.list.find((g) => g.id === id)!, ...(change.name ? { name: change.name } : {}), ...(siteIds ? { siteIds } : {}) };
      this.list = this.list.map((g) => (g.id === id ? updated : g));
    } catch (e) {
      notices.error(e);
    }
  }

  async remove(id: string) {
    try {
      if (inTauri) await api.removeSiteGroup(id);
      this.list = this.list.filter((g) => g.id !== id);
    } catch (e) {
      notices.error(e);
    }
  }

  /** A site was removed: the engine dropped it from its groups. */
  forgetSite(siteId: string) {
    this.list = this.list.map((g) => ({ ...g, siteIds: g.siteIds.filter((id) => id !== siteId) })).filter((g) => g.siteIds.length > 0);
  }
}

export const groups = new GroupsStore();
