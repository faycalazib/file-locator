<script lang="ts">
  import Icon from './Icon.svelte';
  import Marked from './Marked.svelte';
  import { dismissable } from '../actions';
  import { formatBytes, formatNumber, formatRelative, formatShortDate, t, type MessageKey } from '../i18n/index.svelte';
  import { search } from '../stores/search.svelte';
  import { api, inTauri } from '../api';
  import { notices } from '../stores/notices.svelte';
  import { saved } from '../stores/saved.svelte';
  import { ui } from '../stores/ui.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import { isolate, splitPath } from '../text';
  import { kindIcon } from '../doodles';
  import { tabs } from '../stores/tabs.svelte';
  import { openResults, saveResults } from '../results-io';
  import { keywordReport } from '../exporting';
  import { lazyThumb, thumbInList } from '../thumbs';
  import type { Detector, SearchHit, SortKey, Variant } from '../types';

  let { variant }: { variant: Variant } = $props();

  const SORTS: SortKey[] = ['relevance', 'matches', 'modified', 'name'];
  const sortKey = (s: SortKey) => `results.sort.${s}` as MessageKey;

  let listEl: HTMLElement | undefined = $state();
  let moreOpen = $state(false);
  // `?save` opens the ★ form in the browser demo (screenshots only).
  let saveOpen = $state(!inTauri && new URLSearchParams(location.search).has('save'));
  let saveLabel = $state(inTauri ? '' : search.query);

  /** ★: name the search on screen (prefilled with its current name). */
  function toggleSave() {
    saveOpen = !saveOpen;
    if (saveOpen) saveLabel = saved.current?.label ?? search.lastQuery;
  }

  async function submitSave(event: SubmitEvent) {
    event.preventDefault();
    saveOpen = false;
    await saved.save(saveLabel);
  }

  async function removeSaved() {
    saveOpen = false;
    if (saved.current) await saved.remove(saved.current.id);
  }

  function autofocus(node: HTMLInputElement) {
    node.select();
  }

  /** Strata: hits grouped by folder, in the current sort order. */
  const groups = $derived.by(() => {
    const map = new Map<string, SearchHit[]>();
    for (const hit of search.visible) {
      const { folder } = splitPath(hit.path);
      map.set(folder, [...(map.get(folder) ?? []), hit]);
    }
    return [...map.entries()];
  });

  // Keep the keyboard selection visible.
  $effect(() => {
    const id = search.selectedId;
    if (!id || !listEl) return;
    listEl.querySelector(`[data-id="${id}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  function onKeydown(event: KeyboardEvent) {
    const n = search.visible.length;
    const moves: Record<string, number> = { ArrowDown: 1, ArrowUp: -1, PageDown: 10, PageUp: -10, Home: -n, End: n };
    const delta = moves[event.key];
    if (delta === undefined) return;
    event.preventDefault();
    search.move(delta);
  }

  /** Export / copy window (lot 6.4). */
  function openTransfer(mode: 'export' | 'copy') {
    moreOpen = false;
    ui.transfer = mode;
  }

  /** "Search within these results": the next words only look at them. */
  function searchWithin() {
    search.searchWithin();
    document.getElementById('query')?.focus();
  }

  function saveFile() {
    moreOpen = false;
    void saveResults();
  }

  function openFile() {
    moreOpen = false;
    void openResults();
  }

  /** PDF report: the print dialog ("Save as PDF"), see ReportView.svelte; with the keyword table (lot 6.4). */
  async function exportReport() {
    moreOpen = false;
    ui.reportKeywords = await keywordReport(search.visible).catch(() => null);
    ui.printReport().catch((e) => notices.error(e));
  }
</script>

{#snippet excerpt(hit: SearchHit, clip: boolean)}
  {@const s = hit.snippets[0]}
  {#if s}
    <p class="excerpt" class:code={hit.kind === 'code'} class:clip>
      <span class="txt" dir={hit.kind === 'code' ? 'ltr' : 'auto'}><Marked text={s.text} /></span>
    </p>
  {/if}
{/snippet}

{#snippet count(hit: SearchHit, small: boolean)}
  {@const approx = hit.exactCount === 0 && hit.matchCount > 0}
  <!-- Found by name only (no text asked, or a folder): nothing to count. -->
  {#if hit.matchCount === 0 && hit.snippets.length === 0}
    <span class="num-none" class:small aria-hidden="true"></span>
  {:else}
  <span
    class="num sketch round"
    class:hatch={!approx}
    class:dashed={approx}
    class:small
    aria-label={approx ? `${t('results.matches', { count: hit.matchCount })} — ${t('results.approximate')}` : t('results.matches', { count: hit.matchCount })}
    title={approx ? t('results.approximate') : undefined}
  >
    <span aria-hidden="true">{formatNumber(hit.matchCount)}</span>
  </span>
  {/if}
{/snippet}

{#snippet row(hit: SearchHit)}
  {@const p = splitPath(hit.path)}
  {@const selected = hit.id === search.selectedId}
  <div
    id="hit-{hit.id}"
    data-id={hit.id}
    class="row sketch"
    class:selected
    class:ping={search.fresh.has(hit.id)}
    role="option"
    tabindex="-1"
    aria-selected={selected}
    onclick={() => search.select(hit.id)}
    onkeydown={onKeydown}
  >
    {#if variant === 'ledger'}
      <div class="cells">
        <span class="name-cell">
          <img class="kind" src={kindIcon[hit.kind]} alt="" />
          <span class="name" title={p.name}><span class="ltr-isolate" dir="ltr">{p.name}</span>{#if search.alertNew.has(hit.path)}<span class="new-tag">{t('alerts.newTag')}</span>{/if}</span>
        </span>
        <span class="folder mono" title={p.folder}><span class="ltr-isolate" dir="ltr">{p.folder}</span></span>
        {@render count(hit, true)}
        <span class="cell mono">{formatBytes(hit.sizeBytes)}</span>
        <span class="cell mono">{formatShortDate(hit.modified)}</span>
      </div>
      {#if selected}
        {@render excerpt(hit, true)}
      {/if}
    {:else}
      {#if thumbInList(hit)}
        <img class="kind" src={kindIcon[hit.kind]} alt="" use:lazyThumb={hit.path} />
      {:else}
        <img class="kind" src={kindIcon[hit.kind]} alt="" />
      {/if}
      <div class="main">
        <div class="name" title={p.name}><span class="ltr-isolate" dir="ltr">{p.name}</span>{#if search.alertNew.has(hit.path)}<span class="new-tag">{t('alerts.newTag')}</span>{/if}</div>
        {#if variant === 'journal'}
          <div class="where mono"><span class="ltr-isolate" dir="ltr">{p.folder}</span> · <span class="when">{formatRelative(hit.modified)}</span></div>
        {/if}
        {@render excerpt(hit, variant === 'strata')}
      </div>
      {@render count(hit, false)}
    {/if}
  </div>
{/snippet}

<section class="results v-{variant}" aria-label={t('results.label')}>
  <header class="readout">
    <div class="history">
      <button type="button" class="icon-btn small" aria-label={t('tabs.back')} title={`${t('tabs.back')} (Alt+←)`} disabled={!tabs.canBack} onclick={() => tabs.back()}>
        <Icon name="back" size={15} />
      </button>
      <button type="button" class="icon-btn small" aria-label={t('tabs.forward')} title={`${t('tabs.forward')} (Alt+→)`} disabled={!tabs.canForward} onclick={() => tabs.forward()}>
        <Icon name="chevron" size={15} />
      </button>
    </div>
    {#if search.hasRun}
      <p class="summary">
        {#if search.scanning}<span class="scanning">{t('results.scanning')}</span>{/if}
        {#if search.folders}
          <b>{t('results.folders', { count: search.visible.length })}</b>
        {:else if search.lastQuery === ''}
          <!-- Found by name only: there are files, not matches. -->
          <b>{t('results.files', { count: search.visible.length })}</b>
        {:else}
          <b>{formatNumber(search.totalMatches)} {t('results.matchesIn', { count: search.totalMatches, files: '' }).trim()}</b>
          <span>{t('results.files', { count: search.visible.length })}</span>
        {/if}
        {#if search.mode === 'live' && search.scanned > 0}
          <small>· {t('results.scanned', { count: search.scanned })}</small>
        {/if}
        {#if search.durationMs !== null}
          <small class="mono">· {t('results.duration', { ms: search.durationMs })}</small>
        {/if}
        {#if search.openedFrom}
          <small class="opened">· {t('results.file.openedFrom', { name: isolate(search.openedFrom) })}</small>
        {/if}
      </p>
    {/if}

    {#if search.hasRun && search.filters.detectors.length > 0}
      {@const found = Object.entries(search.detections).toSorted((a, b) => b[1].files - a[1].files)}
      <div class="detections" role="group" aria-label={t('detectors.title')}>
        {#each found as [code, total] (code)}
          <button
            type="button"
            class="detection sketch"
            class:hatch={search.detectorFocus === code}
            aria-pressed={search.detectorFocus === code}
            title={t('detectors.total', { files: t('results.files', { count: total.files }), matches: t('results.matches', { count: total.matches }) })}
            onclick={() => (search.detectorFocus = search.detectorFocus === code ? null : (code as Detector))}
          >
            {t(`detectors.names.${code}` as MessageKey)} <b class="mono">{formatNumber(total.files)}</b>
          </button>
        {:else}
          {#if !search.scanning}<span class="none">{t('detectors.nothing')}</span>{/if}
        {/each}
      </div>
    {/if}

    <div class="tools">
      <span class="sort sketch">
        <select bind:value={search.sort} aria-label={t('results.sort.label')}>
          {#each SORTS as s (s)}
            <option value={s}>{t(sortKey(s))}</option>
          {/each}
        </select>
      </span>
      <div class="more" use:dismissable={() => (saveOpen = false)}>
        <button
          type="button"
          class="icon-btn sketch round star"
          class:on={saved.current !== null}
          aria-haspopup="dialog"
          aria-expanded={saveOpen}
          aria-label={saved.current ? t('saved.saved') : t('search.save')}
          title={saved.current ? `${t('saved.saved')} · ${saved.current.label}` : t('search.save')}
          disabled={!search.hasRun}
          onclick={toggleSave}
        >
          <Icon name="star" size={16} />
        </button>
        {#if saveOpen}
          <form class="menu save-form sketch" onsubmit={submitSave}>
            <label>
              <span class="label">{t('saved.name')}</span>
              <input dir="auto" bind:value={saveLabel} use:autofocus />
            </label>
            <div class="save-actions">
              {#if saved.current}
                <button type="button" onclick={removeSaved}>{t('saved.remove')}</button>
              {/if}
              <button type="submit" class="confirm sketch hatch">{t('saved.confirm')}</button>
            </div>
          </form>
        {/if}
      </div>
      <div class="more" use:dismissable={() => (moreOpen = false)}>
        <button type="button" class="icon-btn sketch round" aria-haspopup="menu" aria-expanded={moreOpen} aria-label={t('results.moreActions')} title={t('results.moreActions')} onclick={() => (moreOpen = !moreOpen)}>
          <Icon name="download" size={16} />
        </button>
        {#if moreOpen}
          <div class="menu sketch" role="menu">
            <button type="button" role="menuitem" onclick={() => openTransfer('export')} disabled={search.visible.length === 0}>{t('export.menu')}</button>
            <button type="button" role="menuitem" onclick={exportReport} disabled={search.visible.length === 0}>{t('results.exportPdf')}</button>
            <button type="button" role="menuitem" onclick={() => openTransfer('copy')} disabled={search.visible.length === 0}>
              <Icon name="copy" size={14} />{t('export.copyMenu')}
            </button>
            <hr />
            <button type="button" role="menuitem" onclick={saveFile} disabled={search.hits.length === 0}>
              <Icon name="save" size={14} />{t('results.file.save')} <kbd dir="ltr">Ctrl+S</kbd>
            </button>
            <button type="button" role="menuitem" onclick={openFile}>
              <Icon name="open" size={14} />{t('results.file.open')} <kbd dir="ltr">Ctrl+O</kbd>
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if search.hasRun && search.hits.length > 0}
    <div class="listbar">
      <label class="quick sketch dashed">
        <Icon name="filter" size={14} />
        <input
          id="quick-filter"
          dir="auto"
          type="search"
          autocomplete="off"
          spellcheck="false"
          aria-label={t('results.quickFilter.label')}
          placeholder={t('results.quickFilter.placeholder')}
          bind:value={search.quickFilter}
        />
        {#if search.quickFilter.trim()}
          <small class="mono shown">{t('results.quickFilter.shown', { shown: formatNumber(search.visible.length), total: formatNumber(search.hits.length) })}</small>
        {/if}
      </label>
      <button type="button" class="within sketch" title={t('results.within.hint')} disabled={search.visible.length === 0} onclick={searchWithin}>
        <Icon name="search" size={14} />{t('results.within.button')}
      </button>
    </div>
  {/if}

  <!-- Announced once, when the search completes. -->
  <div class="sr-only" aria-live="polite">
    {#if search.hasRun && !search.scanning}
      {t('results.done')} — {t('results.files', { count: search.visible.length })}
    {/if}
  </div>

  <div class="body">
    {#if search.scanning}
      <div class="sweep" aria-hidden="true"></div>
    {/if}

    {#if sitesStore.loaded && sitesStore.list.length === 0}
      <p class="state">{t('results.noSites')}</p>
    {:else if !search.hasRun}
      <p class="state">{t('results.idle')}</p>
    {:else if !search.scanning && search.visible.length === 0 && search.hits.length > 0 && search.quickFilter.trim()}
      <!-- The search found files; the instant filter hides them all. -->
      <div class="state">
        <p>{t('results.quickFilter.none', { filter: isolate(search.quickFilter.trim()) })}</p>
        <button type="button" class="clear-filter sketch" onclick={() => (search.quickFilter = '')}>{t('results.quickFilter.clear')}</button>
      </div>
    {:else if !search.scanning && search.visible.length === 0}
      <div class="state">
        <p class="state-title">{t('results.empty')}</p>
        <p>{t('results.emptyHint')}</p>
      </div>
    {:else}
      {#if !search.scanning && search.onlyApproximate}
        <p class="approx-note sketch dashed">{t('results.noExact', { query: search.lastQuery })}</p>
      {/if}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="list"
        role="listbox"
        tabindex="0"
        aria-label={t('results.label')}
        aria-activedescendant={search.selectedId ? `hit-${search.selectedId}` : undefined}
        bind:this={listEl}
        onkeydown={onKeydown}
      >
        {#if variant === 'ledger'}
          <div class="ledger-head" aria-hidden="true">
            <span>{t('results.columns.name')}</span>
            <span>{t('results.columns.folder')}</span>
            <span class="center">{t('results.columns.matches')}</span>
            <span>{t('results.columns.size')}</span>
            <span>{t('results.columns.modified')}</span>
          </div>
        {/if}

        {#if variant === 'strata'}
          {#each groups as [folder, items] (folder)}
            <div class="stratum" role="group" aria-label={folder}>
              <div class="stratum-head">
                <span class="folder-tag sketch hatch mono"><span class="ltr-isolate" dir="ltr">{folder}</span></span>
                <span class="stratum-count">{t('results.files', { count: items.length })}</span>
              </div>
              {#each items as hit (hit.id)}
                {@render row(hit)}
              {/each}
            </div>
          {/each}
        {:else}
          {#each search.visible as hit (hit.id)}
            {@render row(hit)}
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .results {
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-inline-size: 0;
    min-block-size: 0;
  }

  /* ── Readout ───────────────────────────────────────────── */
  .readout {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
    flex: none;
    padding-block: 0 var(--sp-3);
  }

  .summary {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 var(--sp-2);
    margin: 0;
    min-inline-size: 0;
    font: var(--fs-title) / var(--lh-title) var(--font-display);
    text-transform: uppercase;
  }

  .summary b {
    font-weight: 400;
    color: var(--accent-ink);
  }

  .summary small {
    font: 16px var(--font-body);
    text-transform: none;
    color: var(--text-dim);
  }

  .scanning {
    color: var(--secondary-ink);
  }

  .tools {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex: none;
    margin-inline-start: auto;
  }

  .sort {
    --sw: 2px;
    display: block;
  }

  .sort select {
    position: relative;
    block-size: 34px;
    padding-inline: var(--sp-3);
    background: transparent;
    border: 0;
    font: 16px var(--font-body);
    cursor: pointer;
  }

  .icon-btn {
    --sw: 2px;
    display: grid;
    place-items: center;
    inline-size: 36px;
    block-size: 36px;
    padding: 0;
    background: transparent;
    border: 0;
  }

  .icon-btn:hover,
  .icon-btn[aria-expanded='true'] {
    --k: var(--secondary);
  }

  .more {
    position: relative;
  }

  .icon-btn:disabled {
    opacity: 0.4;
  }

  /* ── History ← → ───────────────────────────────────────── */
  .history {
    display: flex;
    flex: none;
    align-self: center;
  }

  .icon-btn.small {
    inline-size: 28px;
    block-size: 28px;
    color: var(--text-dim);
  }

  .icon-btn.small:hover:not(:disabled) {
    color: var(--secondary-ink);
  }

  .opened {
    font-style: italic;
  }

  /* ── List bar: instant filter + search within ──────────── */
  .listbar {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    flex: none;
    padding: 0 6px var(--sp-3);
  }

  .quick {
    --sw: 2px;
    --k: var(--text-faint);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex: 1 1 auto;
    min-inline-size: 0;
    max-inline-size: 420px;
    padding: 2px var(--sp-3);
    border-radius: 18px;
    color: var(--text-dim);
  }

  .quick::before {
    border-radius: 18px;
  }

  .quick:focus-within {
    --k: var(--secondary);
  }

  .quick input {
    position: relative;
    flex: 1 1 auto;
    min-inline-size: 0;
    padding: 0;
    background: transparent;
    border: 0;
    font: 15px/26px var(--font-body);
    color: var(--text);
  }

  .quick input:focus-visible {
    outline: none;
  }

  .quick input::placeholder {
    color: var(--text-faint);
  }

  .shown {
    flex: none;
    font-size: 12px;
    color: var(--accent-ink);
  }

  .within {
    --sw: 2px;
    --k: var(--secondary);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex: none;
    padding: 0 var(--sp-3);
    background: transparent;
    border: 0;
    border-radius: 999px;
    font: 15px/28px var(--font-display);
    text-transform: uppercase;
    color: var(--secondary-ink);
    white-space: nowrap;
  }

  .within::before {
    border-radius: 999px;
  }

  .within:hover:not(:disabled) {
    --h: var(--fill-secondary);
    background-image: repeating-linear-gradient(-35deg, var(--h) 0 2px, transparent 2px 5px);
  }

  .within:disabled {
    opacity: 0.4;
  }

  /* Saved search: the star is filled in with the accent pencil. */
  .star.on {
    --k: var(--accent);
    color: var(--accent-ink);
  }

  .star.on :global(path) {
    fill: currentColor;
  }

  .menu.save-form {
    gap: var(--sp-2);
    min-inline-size: 320px;
    padding: var(--sp-3);
  }

  .save-form label {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
  }

  .save-form .label {
    color: var(--text-dim);
    font-size: 14px;
    line-height: 20px;
  }

  .save-form input {
    padding: var(--sp-1) var(--sp-2);
    color: var(--text);
    background: transparent;
    border: 0;
    border-block-end: 2px solid var(--text-faint);
    font: inherit;
  }

  .save-form input:focus-visible {
    outline: none;
    border-block-end-color: var(--accent);
  }

  .save-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }

  .save-form .confirm {
    --k: var(--ok);
    --h: var(--fill-ok);
    background-color: transparent;
    font-family: var(--font-display);
    text-transform: uppercase;
  }

  .menu {
    position: absolute;
    inset-block-start: calc(100% + var(--sp-2));
    inset-inline-end: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    min-inline-size: 200px;
    padding: var(--sp-2);
    background: var(--surface);
  }

  .menu hr {
    margin: var(--sp-1) var(--sp-2);
    border: 0;
    border-block-start: 2px dashed var(--text-faint);
  }

  .menu kbd {
    margin-inline-start: auto;
    padding-inline-start: var(--sp-4);
    font: 12px var(--font-mono);
    color: var(--text-faint);
  }

  .menu button {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    white-space: nowrap;
    padding: var(--sp-2) var(--sp-3);
    background: transparent;
    border: 0;
    text-align: start;
  }

  .menu button:hover:not(:disabled) {
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 4px;
  }

  /* ── Body ──────────────────────────────────────────────── */
  .body {
    position: relative;
    flex: 1 1 auto;
    min-block-size: 0;
    overflow: hidden;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    block-size: 100%;
    overflow-y: auto;
    padding: 6px 6px var(--sp-6);
  }

  .list:focus-visible {
    outline-offset: -3px;
  }

  .state {
    max-inline-size: 440px;
    margin: var(--sp-8) auto;
    text-align: center;
    color: var(--text-dim);
  }

  .state p {
    margin: 0 0 var(--sp-2);
  }

  .clear-filter {
    --sw: 2px;
    --k: var(--secondary);
    padding: 0 var(--sp-3);
    background: transparent;
    border: 0;
    border-radius: 999px;
    font: 15px/28px var(--font-display);
    text-transform: uppercase;
    color: var(--secondary-ink);
  }

  .clear-filter::before {
    border-radius: 999px;
  }

  .state-title {
    font: var(--fs-title) / var(--lh-title) var(--font-display);
    text-transform: uppercase;
    color: var(--text);
  }

  /* ── Result cards ──────────────────────────────────────── */
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-3) var(--sp-4);
    background: var(--surface);
    cursor: default;
  }

  /* Hand-placed cards: the theme decides the tilt (0 for a straight theme). */
  .row:nth-child(odd) {
    rotate: calc(var(--tilt) * -1);
  }

  .row:nth-child(even) {
    rotate: var(--tilt);
  }

  .row:hover {
    --k: var(--text-dim);
  }

  .row.selected {
    --k: var(--secondary);
    --sw: 3.5px;
  }

  .kind {
    flex: none;
    inline-size: 30px;
    block-size: 30px;
    align-self: flex-start;
    margin-block-start: 2px;
  }

  .main {
    flex: 1 1 auto;
    min-inline-size: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--fs-h2) / var(--lh-h2) var(--font-display);
    letter-spacing: 0.3px;
  }

  .where {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  .when {
    font-family: var(--font-body);
  }

  .excerpt {
    margin: var(--sp-1) 0 0;
  }

  .excerpt.code .txt {
    font-family: var(--font-mono);
    font-size: 14px;
    white-space: pre;
  }

  .excerpt.clip .txt {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .txt {
    unicode-bidi: plaintext;
  }

  /* Found by name only: an empty slot keeps the columns aligned. */
  .num-none {
    flex: none;
    inline-size: 52px;
  }

  .num-none.small {
    inline-size: 34px;
  }

  /* Count: a hatched pink circle, like an assay stamp. */
  .num {
    --k: var(--accent);
    --h: var(--fill-accent);
    flex: none;
    display: grid;
    place-items: center;
    inline-size: 52px;
    block-size: 52px;
    border-radius: 50%;
    font: 28px/1 var(--font-display);
  }

  .num.small {
    --sw: 2px;
    inline-size: 34px;
    block-size: 34px;
    font-size: 18px;
    justify-self: center;
  }

  .approx-note {
    --k: var(--accent);
    --sw: 2px;
    margin: 0 6px var(--sp-3);
    padding: var(--sp-2) var(--sp-4);
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  /* Approximate-only file: an empty dashed circle, no hatching. */
  .num.dashed {
    color: var(--text-dim);
  }

  /* ── Ledger ────────────────────────────────────────────── */
  .v-ledger .row {
    display: block;
    padding-block: var(--sp-2);
  }

  .ledger-head,
  .cells {
    display: grid;
    grid-template-columns: minmax(0, 2.6fr) minmax(0, 1.5fr) 72px 72px 92px;
    gap: var(--sp-4);
    align-items: center;
  }

  .ledger-head > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ledger-head {
    padding-inline: var(--sp-4);
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .center {
    text-align: center;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-inline-size: 0;
  }

  .name-cell .kind {
    inline-size: 22px;
    block-size: 22px;
    align-self: center;
    margin: 0;
  }

  .v-ledger .name {
    font-size: 18px;
  }

  .folder {
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  .cell {
    font-size: var(--fs-caption);
    color: var(--text-dim);
    white-space: nowrap;
  }

  /* ── Strata ────────────────────────────────────────────── */
  .stratum {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .stratum + .stratum {
    margin-block-start: var(--sp-3);
  }

  .stratum-head {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .folder-tag {
    --k: var(--ok);
    --h: var(--fill-ok);
    --sw: 2px;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0 var(--sp-3);
    font-size: var(--fs-caption);
    line-height: 26px;
  }

  .stratum-count {
    flex: none;
    font: 15px var(--font-display);
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .v-strata .row {
    margin-inline-start: var(--sp-5);
  }

  /* ── Motion ────────────────────────────────────────────── */
  /* Digging: a band of pink hatching sweeps across the results. */
  .sweep {
    position: absolute;
    inset-block: 0;
    inset-inline-start: -120px;
    background: var(--sweep-bg);
    z-index: 2;
    inline-size: 120px;
    pointer-events: none;
    mask-image: linear-gradient(to right, transparent, #000 40%, #000 60%, transparent);
    animation: sweep var(--sweep-duration) linear infinite;
  }

  @keyframes sweep {
    from {
      inset-inline-start: -120px;
    }
    to {
      inset-inline-start: 100%;
    }
  }

  /* New find: a quick highlighter flash on the card. */
  .row.ping::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: var(--r-sketch);
    background: var(--ping-bg);
    pointer-events: none;
    animation: ping var(--ping-duration) var(--ease-out) forwards;
  }

  @keyframes ping {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep,
    .row.ping::after {
      display: none;
    }
  }
  /* Announced by an alert (lot 6.1). */
  .new-tag {
    margin-inline-start: var(--sp-2);
    padding: 0 6px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--bg);
    font: 700 11px/16px var(--font-mono);
    text-transform: uppercase;
    vertical-align: middle;
  }

  /* Lot 6.2: what the detectors found, per kind; a click keeps those files. */
  .detections {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1) var(--sp-2);
    flex-basis: 100%;
    order: 3;
  }

  .detection {
    --sw: 1.5px;
    padding: 0 var(--sp-2);
    background: transparent;
    border: 0;
    font-size: var(--fs-caption);
    line-height: 22px;
  }

  .detection b {
    margin-inline-start: 2px;
    color: var(--accent);
  }

  .detections .none {
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  /* Lot 6.6: an image's own thumbnail instead of the doodle. */
  :global(img.kind.thumb) {
    inline-size: 48px;
    block-size: 48px;
    object-fit: cover;
    border-radius: 4px;
    box-shadow: 0 0 0 1.5px var(--text-faint);
  }
</style>
