<script lang="ts">
  /**
   * Term list (lot 7.4), in the row of criteria under the search box:
   * "+ Term list" opens a .txt / .csv file; once loaded, a chip (file name,
   * number of terms, ×) opens a panel: "At least one" / "All", a preview of
   * the terms, keyword report, replace, remove.
   */
  import Icon from './Icon.svelte';
  import { formatNumber, t } from '../i18n/index.svelte';
  import { api, inTauri } from '../api';
  import { notices } from '../stores/notices.svelte';
  import { search, DEMO_TERM_LIST, type TermList } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';

  /** Terms shown in the panel; the others are counted. */
  const PREVIEW = 12;

  // Browser demo: `?terms=open` shows the panel (screenshots).
  let open = $state(!inTauri && new URLSearchParams(location.search).get('terms') === 'open');
  let loading = $state(false);
  let root: HTMLElement | undefined = $state();

  const list = $derived(search.filters.termList);

  /** The panel opens towards the side where there is room (physical sides: RTL too). */
  let alignRight = $state(false);

  function align() {
    if (!root) return;
    const rect = root.getBoundingClientRect();
    alignRight = rect.left + Math.min(470, window.innerWidth * 0.9) > window.innerWidth - 16;
  }

  function toggle() {
    if (!open) align();
    open = !open;
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function load() {
    open = false;
    if (!inTauri) {
      search.filters.termList = { ...DEMO_TERM_LIST };
      return;
    }
    const path = await api.pickTermFile(t('terms.pick'), t('terms.fileKind')).catch(() => null);
    if (!path) return;
    loading = true;
    try {
      const read = await api.readTermList(path);
      if (read.terms.length === 0) {
        notices.push(t('terms.empty', { name: fileName(path) }), 'error');
        return;
      }
      const all = search.filters.termList?.all ?? false;
      search.filters.termList = { name: fileName(path), terms: read.terms, skipped: read.skipped, all } satisfies TermList;
    } catch (e) {
      notices.error(e);
    } finally {
      loading = false;
    }
  }

  function setAll(all: boolean) {
    if (list) search.filters.termList = { ...list, all };
  }

  function remove() {
    open = false;
    search.filters.termList = null;
  }

  function report() {
    open = false;
    ui.transfer = 'export';
  }

  // Opened by the demo URL: aligned once in place.
  $effect(() => {
    if (open && root) {
      align();
      // The chips move once the fonts are there.
      void document.fonts?.ready.then(align);
    }
  });

  /** A click outside or Escape closes the panel. */
  function onWindowClick(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }

  function onWindowKey(event: KeyboardEvent) {
    if (event.key === 'Escape') open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey} />

<span class="term-list" bind:this={root}>
  {#if list}
    <span class="chip sketch hatch token list-chip" title={t('terms.hint')}>
      <button type="button" class="open" aria-expanded={open} aria-controls="term-panel" onclick={toggle}>
        <Icon name="list" size={13} />
        <span class="name">{t('terms.chip')}&nbsp;<span class="ltr-isolate" dir="ltr">{list.name}</span></span>
        <span class="count">· {t('terms.count', { count: list.terms.length })}</span>
      </button>
      <button type="button" aria-label={t('terms.remove')} title={t('terms.remove')} onclick={remove}>
        <Icon name="close" size={12} />
      </button>
    </span>
  {:else}
    <button type="button" class="chip sketch refine" onclick={load} disabled={loading}>+ {t('terms.add')}</button>
  {/if}

  {#if open && list}
    <div id="term-panel" class="panel sketch" class:right={alignRight} role="dialog" aria-label={t('terms.title')}>
      <div class="top">
        <h3>{t('terms.title')}</h3>
        <div class="modes" role="radiogroup" aria-label={t('terms.mode')}>
          <button type="button" role="radio" class="sketch" class:hatch={!list.all} aria-checked={!list.all} onclick={() => setAll(false)}>{t('terms.any')}</button>
          <button type="button" role="radio" class="sketch" class:hatch={list.all} aria-checked={list.all} onclick={() => setAll(true)}>{t('terms.all')}</button>
        </div>
      </div>
      <span class="file">
        <span class="ltr-isolate" dir="ltr">{list.name}</span> · {t('terms.count', { count: list.terms.length })}{#if list.skipped > 0} · {t('terms.skipped', { count: list.skipped })}{/if}
      </span>
      <p class="hint">{list.all ? t('terms.allHint') : t('terms.anyHint')}</p>
      <div class="terms">
        {#each list.terms.slice(0, PREVIEW) as term, i (i)}
          <span class:rx={term.startsWith('/') && term.endsWith('/') && term.length > 2} dir="auto">{term}</span>
        {/each}
        {#if list.terms.length > PREVIEW}
          <span class="more">{t('terms.more', { count: formatNumber(list.terms.length - PREVIEW) })}</span>
        {/if}
      </div>
      <div class="actions">
        <button type="button" class="sketch" onclick={report}>{t('terms.report')}</button>
        <span>
          <button type="button" class="sketch" onclick={load}>{t('terms.replace')}</button>
          <button type="button" class="sketch danger" onclick={remove}>{t('terms.remove')}</button>
        </span>
      </div>
    </div>
  {/if}
</span>

<style>
  .term-list {
    position: relative;
    display: inline-flex;
  }

  /* The chip itself: same look as the other chips of the row. */
  .chip {
    --h: var(--fill-secondary);
    --k: var(--secondary);
    display: inline-flex;
    align-items: center;
    gap: var(--sp-1);
    padding: 0 10px;
    background-color: transparent;
    border: 0;
    border-radius: 999px;
    font: 15px/24px var(--font-display);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .chip::before {
    border-radius: 999px;
  }

  .refine {
    color: var(--secondary-ink);
  }

  .list-chip {
    max-inline-size: 440px;
  }

  .list-chip button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    background: none;
    border: 0;
    color: inherit;
    font: inherit;
    text-transform: inherit;
  }

  .list-chip .open {
    min-inline-size: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .count {
    opacity: 0.85;
  }

  .panel {
    --k: var(--secondary);
    position: absolute;
    z-index: 40;
    inset-block-start: calc(100% + 10px);
    left: 0;
    display: grid;
    gap: 12px;
    inline-size: min(470px, 90vw);
    padding: 16px 18px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    text-align: start;
  }

  .panel.right {
    right: 0;
    left: auto;
  }

  .panel > * {
    position: relative;
    z-index: 1;
  }

  .top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }

  h3 {
    margin: 0;
    font: var(--fs-h2, 20px) / 1.1 var(--font-brand);
    text-transform: uppercase;
  }

  .modes {
    display: inline-flex;
    gap: 6px;
  }

  .modes button,
  .actions button {
    padding: 5px 12px;
    background: transparent;
    border: 0;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }

  .modes button {
    --h: var(--fill-secondary);
  }

  .file,
  .hint {
    margin: 0;
    color: var(--text-dim);
    font-size: var(--fs-caption);
  }

  .terms {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-block-size: 150px;
    overflow: hidden;
  }

  .terms span {
    padding: 3px 9px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--secondary) 16%, transparent);
    font-size: 14px;
  }

  .terms .rx {
    font-family: var(--font-mono, monospace);
    font-size: 13px;
  }

  .terms .more {
    background: transparent;
    color: var(--text-dim);
  }

  .actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  .actions .danger {
    --k: var(--danger);
    color: var(--danger-ink);
  }
</style>
