<script lang="ts">
  import Icon from './Icon.svelte';
  import FilterPanel from './FilterPanel.svelte';
  import LangSwitcher from './LangSwitcher.svelte';
  import SettingsMenu from './SettingsMenu.svelte';
  import SiteGroups from './SiteGroups.svelte';
  import { formatBytes, formatNumber, formatPercent, formatRelative, t } from '../i18n/index.svelte';
  import { inTauri } from '../api';
  import { groupColor, groups } from '../stores/groups.svelte';
  import { formatDuration, sense } from '../stores/sense.svelte';
  import { api, type SenseEstimateDto } from '../api';
  import { saved } from '../stores/saved.svelte';
  import { search } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import { warning } from '../doodles';
  import type { IndexSite, SiteStatus } from '../types';

  let { compact = false, withFilters = false }: { compact?: boolean; withFilters?: boolean } = $props();

  /** Each status has its pencil. */
  const STATUS_COLOR: Record<SiteStatus, string> = {
    ready: 'var(--secondary)',
    watching: 'var(--ok)',
    indexing: 'var(--warn)',
    error: 'var(--danger)',
    empty: 'var(--text-faint)',
  };

  /** Same statuses, in a dark enough ink for small text. */
  const STATUS_INK: Record<SiteStatus, string> = {
    ready: 'var(--secondary-ink)',
    watching: 'var(--ok-ink)',
    indexing: 'var(--warn-ink)',
    error: 'var(--danger-ink)',
    empty: 'var(--text-dim)',
  };

  /** Lot 8.2: the question asked before understanding a site, with its estimate. */
  let askSense = $state<{ id: string; estimate: SenseEstimateDto | null | undefined } | null>(null);

  async function toggleSense(site: IndexSite) {
    if (site.sense) {
      const ok = inTauri ? await api.confirm(t('sense.forgetConfirm', { name: site.name }), t('sense.forget')).catch(() => false) : true;
      if (ok) await sitesStore.setSense(site.id, false);
      return;
    }
    askSense = { id: site.id, estimate: undefined };
    const estimate = await sense.estimate(site.id);
    if (askSense?.id === site.id) askSense = { id: site.id, estimate };
  }

  async function confirmSense() {
    if (!askSense) return;
    const id = askSense.id;
    askSense = null;
    await sitesStore.setSense(id, true);
  }

  function runQuery(query: string) {
    search.query = query;
    search.run();
  }

  function initials(site: IndexSite): string {
    return site.name
      .split(/[\s&–_.-]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0]?.toUpperCase() ?? '')
      .join('');
  }

  /** Roots and size stay out of sight: they live in the tooltip. */
  function details(site: IndexSite): string {
    return [...site.roots, `${t('rail.docs', { count: site.docCount })} · ${formatBytes(site.sizeBytes)}`].join('\n');
  }
</script>

{#snippet siteMeta(site: IndexSite)}
  {#if site.status === 'indexing'}
    {#if site.phase === 'reading' && site.progress !== undefined}
      <span class="bar sketch" aria-hidden="true"><span class="hatch" style:inline-size="{site.progress * 100}%"></span></span>
      <span class="meta">{t('rail.indexingProgress', { percent: formatPercent(site.progress) })}</span>
    {:else if site.phase === 'saving'}
      <span class="meta">{t('rail.saving')}</span>
    {:else}
      <span class="meta">{t('rail.scanning', { count: site.found ?? 0 })}</span>
    {/if}
  {:else if site.error}
    <span class="meta error">
      <img src={warning} alt="" />
      {t(`rail.errors.${site.error.code}`, { path: site.error.path })}
    </span>
  {:else if site.lastIndexed}
    <span class="meta">{t('rail.docs', { count: site.docCount })} · {formatRelative(site.lastIndexed)}</span>
    {#if site.skipped}
      <span class="meta faint">{t('rail.skipped', { count: site.skipped })}</span>
    {/if}
  {:else}
    <span class="meta">{t('rail.status.empty')}</span>
  {/if}
{/snippet}

<aside class="rail" class:compact aria-label={t('rail.sites')}>
  <div class="scroll">
    <section class="panel sketch" style:--k="var(--ok)">
      {#if !compact}
        <header>
          <h2>{t('rail.sites')}</h2>
          <span class="head-actions">
            <button type="button" class="add sketch round" aria-label={t('dupes.open')} title={t('dupes.open')} onclick={() => (ui.duplicates = true)} disabled={sitesStore.list.length === 0}>
              <Icon name="twins" size={14} />
            </button>
            <!-- Shared index (lot 7.3): sites are added on the PC that keeps it. -->
            {#if !sitesStore.readOnly}
              <button type="button" class="add sketch round" aria-label={t('rail.addSite')} title={t('rail.addSite')} onclick={() => sitesStore.addFolder()}>
                <Icon name="plus" size={14} />
              </button>
            {/if}
          </span>
        </header>
      {/if}

      {#if sitesStore.share?.shared || sitesStore.readOnly}
        {@const share = sitesStore.share!}
        <!-- Lot 7.3: who keeps the shared index up to date. -->
        <p class="shared" class:reader={sitesStore.readOnly} title={sitesStore.readOnly ? t('share.readerHint', { pc: share.holder ?? '' }) : t('share.holderHint')}>
          {#if compact}⇄{:else}{sitesStore.readOnly ? t('share.byOther', { pc: share.holder ?? '' }) : t('share.byThisPc')}{/if}
        </p>
      {/if}

      <SiteGroups {compact} />

      {#if sitesStore.loaded && sitesStore.list.length === 0 && !sitesStore.readOnly}
        {#if compact}
          <button type="button" class="add sketch round solo" aria-label={t('rail.addSite')} title={t('rail.addSite')} onclick={() => sitesStore.addFolder()}>
            <Icon name="plus" size={16} />
          </button>
        {:else}
          <div class="empty">
            <p class="empty-title">{t('rail.emptyTitle')}</p>
            <p>{t('rail.emptyHint')}</p>
            <button type="button" class="cta sketch hatch" onclick={() => sitesStore.addFolder()}>+ {t('rail.addSite')}</button>
          </div>
        {/if}
      {/if}

      <ul class="sites">
        {#each sitesStore.list as site (site.id)}
          {@const included = search.scope.has(site.id)}
          <li class="site-row" style:--sc={STATUS_COLOR[site.status]} style:--si={STATUS_INK[site.status]}>
            <button
              type="button"
              class="site"
              class:chip={compact}
              class:sketch={compact}
              class:round={compact}
              class:hatch={compact && included}
              aria-pressed={included}
              aria-label={t('rail.includeInSearch', { name: site.name })}
              title={`${site.name} — ${t(`rail.status.${site.status}`)}\n${details(site)}`}
              onclick={() => search.toggleScope(site.id)}
            >
              {#if compact}
                <span>{initials(site)}</span>
              {:else}
                <span class="lead">
                  <span class="check sketch" aria-hidden="true">{included ? '✓' : ''}</span>
                  <!-- Lot 7.2: the pencils of its groups. -->
                  {#if groups.of(site.id).length > 0}
                    <span class="gtags" aria-hidden="true">
                      {#each groups.of(site.id) as group (group.id)}<i style:--gk={groupColor(group)}></i>{/each}
                    </span>
                  {/if}
                </span>
                <span class="site-body">
                  <span class="site-line">
                    <span class="site-name">{site.name}</span>
                    {#if search.facets && search.facets.sites[site.id] !== undefined}
                      <span class="found mono" title={t('facets.siteFound', { files: t('results.files', { count: search.facets.sites[site.id] ?? 0 }) })}>{formatNumber(search.facets.sites[site.id] ?? 0)}</span>
                    {/if}
                    <span class="status">{t(`rail.status.${site.status}`)}</span>
                  </span>
                  {@render siteMeta(site)}
                  <!-- Lot 8.2: the meaning index of the site. -->
                  {#if site.sense}
                    {#if site.senseProgress && site.senseProgress.total > 0}
                      <span class="sense-line">
                        {t('sense.computing', { percent: formatPercent(site.senseProgress.done / site.senseProgress.total) })}{#if site.senseRemaining} · {t('sense.left', { time: formatDuration(site.senseRemaining) })}{/if}
                      </span>
                      <span class="sense-bar" aria-hidden="true"><span style:inline-size="{Math.min(100, (100 * site.senseProgress.done) / site.senseProgress.total)}%"></span></span>
                    {:else}
                      <span class="sense-line done">{t('sense.upToDate', { count: formatNumber(site.sensePassages ?? 0) })}</span>
                    {/if}
                  {/if}
                </span>
              {/if}
            </button>

            {#if inTauri && !compact && !sitesStore.readOnly}
              <span class="actions">
                {#if site.status === 'indexing'}
                  <button type="button" aria-label={t('rail.cancel')} title={t('rail.cancel')} onclick={() => sitesStore.cancel(site.id)}>
                    <Icon name="stop" size={13} />
                  </button>
                {:else}
                  <button type="button" aria-label={t('rail.reindex')} title={t('rail.reindex')} onclick={() => sitesStore.index(site.id)}>
                    <Icon name="refresh" size={13} />
                  </button>
                  {#if sense.status?.installed}
                    <button
                      type="button"
                      class="sense-toggle"
                      class:on={site.sense}
                      aria-pressed={site.sense ?? false}
                      aria-label={site.sense ? t('sense.forget') : t('sense.understand')}
                      title={site.sense ? t('sense.forget') : t('sense.understand')}
                      onclick={() => toggleSense(site)}>≈</button
                    >
                  {/if}
                  <button type="button" aria-label={t('rail.remove')} title={t('rail.remove')} onclick={() => sitesStore.remove(site)}>
                    <Icon name="trash" size={13} />
                  </button>
                {/if}
              </span>
            {/if}
            {#if askSense?.id === site.id}
              {@const e = askSense.estimate}
              <div class="sense-ask sketch" role="dialog" aria-label={t('sense.ask', { name: site.name })}>
                <p class="q">{t('sense.ask', { name: site.name })}</p>
                {#if e === undefined}
                  <p class="n">{t('sense.estimating')}</p>
                {:else if e}
                  <p class="n">{t('rail.docs', { count: site.docCount })} → ≈ {t('sense.passages', { count: formatNumber(e.passages) })}</p>
                  <p>{e.seconds !== null ? t('sense.askTime', { time: formatDuration(e.seconds) }) : t('sense.askNoTime')}</p>
                {/if}
                <div class="row">
                  <button type="button" class="sketch" onclick={() => (askSense = null)}>{t('sense.cancel')}</button>
                  <button type="button" class="sketch hatch go" onclick={confirmSense} disabled={e === undefined}>{t('sense.understandShort')}</button>
                </div>
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    </section>

    {#if withFilters}
      <FilterPanel placement="rail" />
    {:else if !compact}
      {#if search.recents.length > 0}
        <section class="panel sketch" style:--k="var(--orange)">
          <header><h2>{t('rail.recent')}</h2></header>
          <ul class="queries">
            {#each search.recents.slice(0, 3) as query (query)}
              <li>
                <button type="button" class="query" onclick={() => runQuery(query)} title={query}>
                  <span aria-hidden="true">✎</span>
                  <span class="ltr-isolate" dir="auto">{query}</span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if saved.list.length > 0}
        <section class="panel sketch" style:--k="var(--purple)">
          <header><h2>{t('rail.saved')}</h2></header>
          <ul class="queries">
            {#each saved.list as entry (entry.id)}
              <li class="saved-row">
                <button type="button" class="query" class:current={saved.current?.id === entry.id} onclick={() => saved.apply(entry)} title={entry.query}>
                  <span aria-hidden="true">★</span>
                  <span dir="auto">{entry.label}</span>
                </button>
                {#if entry.alert && entry.alert.fresh.length > 0}
                  <span class="news" title={t('alerts.news', { count: entry.alert.fresh.length })}>{entry.alert.fresh.length}</span>
                {/if}
                <button
                  type="button"
                  class="bell"
                  class:on={entry.alert !== null}
                  aria-pressed={entry.alert !== null}
                  aria-label={t(entry.alert ? 'alerts.stop' : 'alerts.start', { label: entry.label })}
                  title={t(entry.alert ? 'alerts.stop' : 'alerts.start', { label: entry.label })}
                  onclick={() => saved.toggleAlert(entry)}
                >
                  <Icon name="bell" size={13} />
                </button>
                <button
                  type="button"
                  class="forget"
                  aria-label={t('saved.removeNamed', { label: entry.label })}
                  title={t('saved.removeNamed', { label: entry.label })}
                  onclick={() => saved.remove(entry.id)}
                >
                  <Icon name="close" size={12} />
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
  </div>

  <footer class="foot">
    <LangSwitcher up compact={compact} />
    <SettingsMenu up />
  </footer>
</aside>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-block-size: 0;
  }

  .scroll {
    flex: 1 1 auto;
    min-block-size: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    overflow-y: auto;
    padding: 6px;
  }

  .panel {
    padding: var(--sp-2) var(--sp-4) var(--sp-3);
    background: var(--surface);
  }

  .compact .panel {
    padding-inline: var(--sp-1);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-block-end: var(--sp-2);
  }

  h2 {
    margin: 0;
    font: var(--fs-h2) / var(--lh-h2) var(--font-brand);
    text-transform: uppercase;
  }

  .add {
    display: grid;
    place-items: center;
    inline-size: 28px;
    block-size: 28px;
    padding: 0;
    background: transparent;
    border: 0;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* ── Sites ─────────────────────────────────────────────── */
  .site {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    inline-size: 100%;
    padding: var(--sp-1);
    background: transparent;
    border: 0;
    text-align: start;
  }

  .check {
    --sw: 2px;
    display: grid;
    place-items: center;
    flex: none;
    inline-size: 20px;
    block-size: 20px;
    margin-block-start: 2px;
    font: 18px/1 var(--font-display);
    color: var(--ok-ink);
  }

  .check::before {
    border-radius: 4px;
  }

  .shared {
    margin: 0 2px var(--sp-2);
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--fill-secondary);
    color: var(--text);
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
  }

  .shared.reader {
    background: var(--fill-warn);
  }

  .sense-line {
    display: block;
    margin-block-start: 3px;
    color: var(--secondary-ink);
    font-size: 12px;
    font-weight: 700;
  }

  .sense-line.done {
    color: var(--text-dim);
    font-weight: 400;
  }

  .sense-bar {
    position: relative;
    display: block;
    overflow: hidden;
    block-size: 6px;
    margin-block-start: 3px;
    border-radius: 3px;
    background: var(--fill-secondary);
  }

  .sense-bar > span {
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    background: var(--secondary);
  }

  .actions .sense-toggle {
    font-weight: 700;
  }

  .actions .sense-toggle.on {
    background: var(--fill-secondary);
    color: var(--secondary-ink);
  }

  .sense-ask {
    --k: var(--secondary);
    position: relative;
    z-index: 5;
    display: grid;
    gap: 6px;
    margin: 6px 0 4px;
    padding: 12px 14px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .sense-ask > * {
    position: relative;
    z-index: 1;
    margin: 0;
  }

  .sense-ask p {
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.45;
  }

  .sense-ask .q {
    color: var(--text);
    font-size: 15px;
    font-weight: 700;
  }

  .sense-ask .n {
    color: var(--secondary-ink);
    font-weight: 700;
  }

  .sense-ask .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .sense-ask button {
    padding: 5px 12px;
    background: transparent;
    border: 0;
    color: var(--text);
    font: inherit;
    font-size: 14px;
  }

  .sense-ask .go {
    --h: var(--fill-secondary);
    font-weight: 700;
  }

  .lead {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    flex: none;
  }

  .gtags {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 3px;
    max-inline-size: 22px;
  }

  .gtags i {
    inline-size: 8px;
    block-size: 8px;
    border-radius: 50%;
    background: var(--gk);
  }

  .site-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 auto;
    min-inline-size: 0;
  }

  .site-line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-2);
  }

  .site-name {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: calc(var(--fs-body) + 1px) / 24px var(--font-display);
  }

  .site[aria-pressed='false'] .site-name {
    color: var(--text-dim);
  }

  .status {
    flex: none;
    font: var(--fs-caption) / var(--lh-caption) var(--font-display);
    text-transform: uppercase;
    color: var(--si);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  .meta img {
    inline-size: 16px;
    block-size: 16px;
  }

  .meta.error {
    align-items: flex-start;
    white-space: normal;
    color: var(--danger-ink);
  }

  .bar {
    --sw: 2px;
    --k: var(--warn);
    display: block;
    block-size: 12px;
    margin-block: var(--sp-1) 2px;
  }

  .bar span {
    --h: var(--fill-warn);
    display: block;
    block-size: 100%;
    border-radius: 6px;
  }

  /* ── Compact (Strata) ──────────────────────────────────── */
  .site.chip {
    --k: var(--sc);
    --h: var(--fill-yellow);
    display: grid;
    place-items: center;
    inline-size: 44px;
    block-size: 44px;
    margin-inline: auto;
    padding: 0;
    font: 16px/1 var(--font-display);
  }

  .site.chip[aria-pressed='false'] {
    color: var(--text-faint);
  }

  /* ── Recent / saved ────────────────────────────────────── */
  .query {
    display: flex;
    align-items: baseline;
    gap: var(--sp-2);
    inline-size: 100%;
    padding: 2px var(--sp-1);
    background: transparent;
    border: 0;
    text-align: start;
  }

  .query span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .saved-row {
    position: relative;
    display: flex;
    align-items: center;
  }

  .query.current span:last-child {
    font-weight: 700;
  }

  .forget {
    display: grid;
    flex: none;
    place-items: center;
    inline-size: 20px;
    block-size: 20px;
    padding: 0;
    color: var(--text-dim);
    background: transparent;
    border: 0;
    opacity: 0;
    transition: opacity 120ms;
  }

  .saved-row:hover .forget,
  .saved-row:focus-within .forget,
  .saved-row:hover .bell,
  .saved-row:focus-within .bell,
  .bell.on {
    opacity: 1;
  }

  /* Alert (lot 6.1): the bell stays visible while on; the count of news. */
  .bell {
    display: grid;
    flex: none;
    place-items: center;
    inline-size: 20px;
    block-size: 20px;
    padding: 0;
    color: var(--text-dim);
    background: transparent;
    border: 0;
    opacity: 0;
    transition: opacity 120ms;
  }

  .bell.on {
    color: var(--accent);
  }

  .news {
    flex: none;
    min-inline-size: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--bg);
    font: 700 11px/18px var(--font-mono);
    text-align: center;
  }

  .query:hover span:last-child {
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 4px;
  }

  /* ── Footer: language + settings ───────────────────────── */
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-1);
    flex: none;
    padding-inline: 6px;
  }

  .compact .foot {
    flex-direction: column;
  }

  /* ── Site actions (shown on hover / keyboard focus) ───── */
  .site-row {
    position: relative;
  }

  .actions {
    position: absolute;
    inset-block-start: 2px;
    inset-inline-end: 0;
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface);
    opacity: 0;
    transition: opacity 120ms;
  }

  .site-row:hover .actions,
  .site-row:focus-within .actions {
    opacity: 1;
  }

  .actions button {
    display: grid;
    place-items: center;
    inline-size: 24px;
    block-size: 24px;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--text-dim);
  }

  .actions button:hover {
    color: var(--accent-ink);
  }

  .meta.faint {
    color: var(--text-faint);
  }

  /* ── Empty state ───────────────────────────────────────── */
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
    padding-block: var(--sp-1) var(--sp-2);
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  .empty p {
    margin: 0;
  }

  .empty-title {
    font: var(--fs-body) / var(--lh-body) var(--font-display);
    text-transform: uppercase;
    color: var(--text);
  }

  .cta {
    --k: var(--ok);
    --h: var(--fill-ok);
    padding: var(--sp-1) var(--sp-3);
    background-color: transparent;
    border: 0;
    font: 16px/24px var(--font-display);
    text-transform: uppercase;
  }

  .add.solo {
    margin-inline: auto;
    inline-size: 40px;
    block-size: 40px;
  }

  /* Lot 6.3: files found in this site (also when it is not checked). */
  .found {
    flex: none;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--fill-accent, transparent);
    color: var(--accent);
    font-size: 11px;
    line-height: 16px;
  }

  .head-actions {
    display: flex;
    gap: var(--sp-2);
  }
</style>
