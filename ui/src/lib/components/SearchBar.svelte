<script lang="ts">
  import Icon from './Icon.svelte';
  import TermList from './TermList.svelte';
  import { formatNumber, t, type MessageKey } from '../i18n/index.svelte';
  import { search, type SearchMode, type SearchOptions } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';
  import { isolate } from '../text';
  import { ALL_DETECTORS, dateLabel, hasDateFilter, shortHash } from '../filters';
  import { gem, magnifier } from '../doodles';
  import type { DocLang, FileKind } from '../types';

  /** Brand letters, each one "colored" with a different pencil. */
  const LETTERS = [...'PROSPECTOR'].map((ch, i) => ({
    ch,
    color: ['var(--green)', 'var(--blue)', 'var(--pink)', 'var(--orange)', 'var(--purple)'][i % 5],
  }));

  const KINDS: FileKind[] = ['pdf', 'word', 'excel', 'powerpoint', 'text', 'code', 'archive', 'email', 'image'];

  /** Power-user options: universal symbols, meaning carried by the label. */
  const OPTIONS: { key: Exclude<keyof SearchOptions, 'fuzzy'>; glyph: string; label: MessageKey }[] = [
    { key: 'caseSensitive', glyph: 'Aa', label: 'search.options.caseSensitive' },
    { key: 'wholeWord', glyph: 'ab', label: 'search.options.wholeWord' },
    { key: 'regex', glyph: '.*', label: 'search.options.regex' },
  ];

  const MODES: { key: SearchMode; label: MessageKey }[] = [
    { key: 'indexed', label: 'search.mode.indexed' },
    { key: 'live', label: 'search.mode.live' },
  ];

  interface Token {
    id: string;
    label: string;
    remove: () => void;
  }

  /** Active filters that are not visible as chips (size, date, language). */
  const tokens = $derived.by<Token[]>(() => {
    const f = search.filters;
    const list: Token[] = [];
    if (f.size !== 'any') list.push({ id: 'size', label: t(`filters.sizes.${f.size}` as MessageKey), remove: () => (search.filters.size = 'any') });
    if (hasDateFilter(f)) list.push({ id: 'date', label: dateLabel(f), remove: () => (search.filters.date = 'any') });
    for (const lang of f.langs) {
      list.push({ id: `l-${lang}`, label: t(`languages.${lang}` as MessageKey), remove: () => search.toggleLang(lang as DocLang) });
    }
    // Every detector = the audit: one chip, not thirteen.
    if (f.detectors.length === ALL_DETECTORS.length) {
      list.push({ id: 'audit', label: t('detectors.audit'), remove: () => (search.filters.detectors = []) });
    } else {
      for (const detector of f.detectors) {
        list.push({ id: `d-${detector}`, label: t(`detectors.names.${detector}` as MessageKey), remove: () => search.toggleDetector(detector) });
      }
    }
    for (const attribute of f.attributes) {
      list.push({ id: `a-${attribute}`, label: t(`filters.attrs.${attribute}` as MessageKey), remove: () => search.toggleAttribute(attribute) });
    }
    if (f.hash.trim()) {
      const copies = search.copies;
      list.push({
        id: 'hash',
        label: copies ? t('filters.copiesToken', { name: copies.name }) : t('filters.hashToken', { hash: shortHash(f.hash) }),
        remove: () => {
          search.filters.hash = '';
          search.copiesOf = null;
        },
      });
    }
    return list;
  });

  function submit(event: SubmitEvent) {
    event.preventDefault();
    search.run();
  }

  // The name row stays open while it holds a criterion (e.g. a saved search).
  let namesOpen = $state(false);
  const showNames = $derived(namesOpen || search.namePattern.trim() !== '' || search.folders);
  let namesInput: HTMLInputElement | undefined = $state();

  function openNames() {
    namesOpen = true;
    requestAnimationFrame(() => namesInput?.focus());
  }

  function clearNames() {
    search.namePattern = '';
    search.folders = false;
    namesOpen = false;
  }
</script>

<header class="masthead">
  <img class="doodle d-start" src={magnifier} alt="" aria-hidden="true" />
  <img class="doodle d-end" src={gem} alt="" aria-hidden="true" />

  <div class="brand">
    <span class="ribbon sketch hatch">{t('app.ribbon')}</span>
    <h1 class="title" aria-label={t('app.name')}>
      {#each LETTERS as l, i (i)}<span style:--c={l.color} aria-hidden="true">{l.ch}</span>{/each}
    </h1>
  </div>

  <form class="query-row" role="search" onsubmit={submit}>
    <label class="box sketch dashed">
      <span class="lead">{t('search.lead')}</span>
      <input
        id="query"
        class="query"
        dir="auto"
        type="search"
        autocomplete="off"
        spellcheck="false"
        aria-label={t('search.label')}
        placeholder={search.within
          ? t('results.within.placeholder', { files: t('results.files', { count: search.within.paths.length }) })
          : search.folder
            ? t('results.folder.placeholder')
            : t('search.placeholder')}
        bind:value={search.query}
      />
    </label>
    {#if search.scanning}
      <button type="button" class="go stop sketch" onclick={() => search.stop()}>
        <Icon name="stop" size={16} />
        {t('search.stop')}
      </button>
    {:else}
      <button type="submit" class="go sketch hatch">{t('search.submit')}</button>
    {/if}
  </form>

  {#if showNames}
    <form class="names-row" onsubmit={submit}>
      <label class="names sketch dashed">
        <span class="names-lead">{t('search.names.lead')}</span>
        <input
          bind:this={namesInput}
          class="names-input mono"
          dir="ltr"
          type="text"
          autocomplete="off"
          spellcheck="false"
          aria-label={t('search.names.label')}
          title={t('search.names.label')}
          placeholder={t('search.names.placeholder')}
          bind:value={search.namePattern}
        />
      </label>
      <button
        type="button"
        class="chip sketch"
        class:hatch={search.folders}
        aria-pressed={search.folders}
        title={t('search.names.foldersHint')}
        onclick={() => (search.folders = !search.folders)}
      >
        {search.folders ? '✓ ' : ''}{t('search.names.folders')}
      </button>
      <button type="button" class="names-clear" aria-label={t('search.names.clear')} title={t('search.names.clear')} onclick={clearNames}>
        <Icon name="close" size={14} />
      </button>
    </form>
  {/if}

  <div class="chips">
    <button
      type="button"
      class="chip sketch"
      class:hatch={search.options.fuzzy}
      aria-pressed={search.options.fuzzy}
      onclick={() => (search.options.fuzzy = !search.options.fuzzy)}
    >
      {search.options.fuzzy ? '✓ ' : ''}{t('search.options.fuzzy')}
    </button>
    {#each OPTIONS as opt (opt.key)}
      <button
        type="button"
        class="chip sketch mono tiny"
        class:hatch={search.options[opt.key]}
        class:underline={opt.key === 'wholeWord'}
        aria-pressed={search.options[opt.key]}
        aria-label={t(opt.label)}
        title={t(opt.label)}
        onclick={() => (search.options[opt.key] = !search.options[opt.key])}
      >
        {opt.glyph}
      </button>
    {/each}

    <span class="sep" aria-hidden="true"></span>

    {#each KINDS as kind (kind)}
      <button
        type="button"
        class="chip sketch"
        class:hatch={search.filters.kinds.includes(kind)}
        aria-pressed={search.filters.kinds.includes(kind)}
        onclick={() => search.toggleKind(kind)}
      >
        {t(`filters.kinds.${kind}` as MessageKey)}
        {#if search.facets}<span class="count mono" class:zero={!search.facets.kinds[kind]}>{formatNumber(search.facets.kinds[kind] ?? 0)}</span>{/if}
      </button>
    {/each}

    {#if !showNames}
      <button type="button" class="chip sketch refine" onclick={openNames}>+ {t('search.names.toggle')}</button>
    {/if}

    <TermList />

    {#if ui.variant !== 'ledger'}
      <button
        type="button"
        class="chip sketch refine"
        aria-expanded={ui.filtersOpen}
        aria-controls="filter-panel"
        onclick={() => (ui.filtersOpen = !ui.filtersOpen)}
      >
        + {t('filters.refine')}
      </button>
    {/if}

    {#if search.within}
      <span class="chip sketch hatch token within" title={t('results.within.hint')}>
        <Icon name="search" size={13} />{t('results.within.chip', { query: isolate(search.within.query), files: t('results.files', { count: search.within.paths.length }) })}
        <button type="button" aria-label={t('results.within.clear')} title={t('results.within.clear')} onclick={() => search.clearWithin()}>
          <Icon name="close" size={12} />
        </button>
      </span>
    {/if}

    {#if search.folder}
      <span class="chip sketch hatch token within" title="{t('results.folder.hint')}&#10;{search.folder}">
        <Icon name="folder" size={13} /><span class="folder-path">{t('results.folder.chip', { path: isolate(search.folder) })}</span>
        <button type="button" aria-label={t('results.folder.clear')} title={t('results.folder.clear')} onclick={() => search.clearFolder()}>
          <Icon name="close" size={12} />
        </button>
      </span>
    {/if}

    {#each tokens as token (token.id)}
      <span class="chip sketch hatch token">
        {token.label}
        <button type="button" aria-label={t('filters.removeToken', { name: token.label })} onclick={token.remove}>
          <Icon name="close" size={12} />
        </button>
      </span>
    {/each}

    <span class="sep" aria-hidden="true"></span>

    <div class="modes" role="radiogroup" aria-label={t('search.mode.label')}>
      {#each MODES as m (m.key)}
        <button
          type="button"
          role="radio"
          class="chip sketch"
          class:hatch={search.mode === m.key}
          aria-checked={search.mode === m.key}
          onclick={() => (search.mode = m.key)}
        >
          {t(m.label)}
        </button>
      {/each}
    </div>
  </div>
</header>

<style>
  .masthead {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-3);
    padding-block: var(--sp-1) var(--sp-2);
  }

  .doodle {
    position: absolute;
    inset-block-start: 0;
    inline-size: 92px;
    filter: var(--wobble);
    pointer-events: none;
    user-select: none;
    opacity: var(--doodle-opacity);
  }

  .d-start {
    inset-inline-start: var(--sp-4);
    rotate: -18deg;
  }

  .d-end {
    inset-inline-end: var(--sp-4);
    inline-size: 96px;
  }

  /* ── Brand: ribbon + hatched letters ─────────────────────── */
  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .ribbon {
    --k: var(--green);
    --h: var(--fill-ok);
    padding: 1px var(--sp-8);
    font: 20px/26px var(--font-brand);
    letter-spacing: 1px;
    text-transform: uppercase;
  }

  .title {
    margin: 0;
    font: var(--fs-display) / var(--lh-display) var(--font-brand);
    letter-spacing: 3px;
    direction: ltr;
  }

  .title span {
    color: transparent;
    -webkit-text-stroke-color: var(--line);
    -webkit-text-stroke-width: var(--title-stroke);
    background-image: repeating-linear-gradient(
      -30deg,
      var(--c) 0 3px,
      color-mix(in srgb, var(--c) var(--title-mix-pct), var(--title-mix)) 3px 5px
    );
    background-clip: text;
    -webkit-background-clip: text;
    filter: var(--title-shadow) var(--wobble);
    padding-inline: 1px;
  }

  /* ── Query: the dashed "name tag" box ────────────────────── */
  .query-row {
    display: flex;
    align-items: center;
    gap: var(--sp-5);
    inline-size: min(980px, 100%);
  }

  .box {
    --k: var(--purple);
    --sw: 3px;
    flex: 1 1 auto;
    min-inline-size: 0;
    display: flex;
    align-items: baseline;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-6);
    border-radius: 26px;
    cursor: text;
  }

  .box::before {
    border-radius: 26px;
  }

  .lead {
    flex: none;
    font: 22px/30px var(--font-display);
    text-transform: uppercase;
  }

  .query {
    position: relative;
    flex: 1 1 auto;
    min-inline-size: 0;
    padding: 0 var(--sp-2) 2px;
    background: transparent;
    border: 0;
    border-block-end: 2px solid var(--line);
    font: 24px/32px var(--font-body);
    color: var(--query-ink);
  }

  .query:focus-visible {
    outline: none;
    border-block-end-color: var(--secondary);
  }

  .query::placeholder {
    color: var(--text-faint);
  }

  .query::-webkit-search-cancel-button {
    display: none;
  }

  .go {
    --k: var(--pink);
    --h: var(--fill-accent);
    --sw: 3px;
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-6);
    background-color: transparent;
    border: 0;
    font: 28px/36px var(--font-display);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .go:hover {
    --h: var(--fill-accent-strong);
  }

  .go.stop {
    --k: var(--danger);
  }

  /* ── File-name row (lot 5.1) ─────────────────────────────── */
  .names-row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    inline-size: min(980px, 100%);
    padding-inline: var(--sp-6);
  }

  .names {
    --k: var(--blue);
    --sw: 2px;
    flex: 1 1 auto;
    min-inline-size: 0;
    display: flex;
    align-items: baseline;
    gap: var(--sp-3);
    padding: 2px var(--sp-5);
    border-radius: 20px;
    cursor: text;
  }

  .names::before {
    border-radius: 20px;
  }

  .names-lead {
    flex: none;
    font: 17px/26px var(--font-display);
    text-transform: uppercase;
  }

  .names-input {
    flex: 1 1 auto;
    min-inline-size: 0;
    padding: 0 var(--sp-1);
    background: transparent;
    border: 0;
    border-block-end: 2px dashed var(--text-faint);
    font-size: 17px;
    line-height: 26px;
    color: var(--query-ink);
  }

  .names-input:focus-visible {
    outline: none;
    border-block-end: 2px solid var(--secondary);
  }

  .names-input::placeholder {
    color: var(--text-faint);
  }

  .names-clear {
    display: grid;
    place-items: center;
    inline-size: 26px;
    block-size: 26px;
    padding: 0;
    color: var(--text-dim);
    background: transparent;
    border: 0;
  }

  /* ── Chips ───────────────────────────────────────────────── */
  .chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: var(--sp-2);
  }

  .chip {
    --h: var(--fill-yellow);
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

  .chip:hover {
    --k: var(--secondary);
  }

  .chip.tiny {
    min-inline-size: 32px;
    justify-content: center;
    font-size: 13px;
    text-transform: none;
  }

  .chip.underline {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .refine {
    --k: var(--secondary);
    color: var(--secondary-ink);
  }

  .refine[aria-expanded='true'] {
    --h: var(--fill-secondary);
    background-image: repeating-linear-gradient(-35deg, var(--h) 0 2px, transparent 2px 5px);
  }

  .within {
    --k: var(--secondary);
    --h: var(--fill-secondary);
    max-inline-size: 420px;
  }

  /* A long path is cut; the whole of it is in the tooltip. */
  .folder-path {
    /* A path keeps its case (chips are upper case). */
    text-transform: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .token button {
    display: grid;
    place-items: center;
    padding: 0;
    background: none;
    border: 0;
  }

  .sep {
    inline-size: 2px;
    block-size: 20px;
    margin-inline: var(--sp-1);
    background: var(--text-faint);
    border-radius: 2px;
    rotate: 8deg;
  }

  .modes {
    display: flex;
    gap: var(--sp-1);
  }

  .chip:disabled {
    opacity: 0.45;
  }

  /* Lot 6.3: documents found per kind (without the kind filter itself). */
  .count {
    margin-inline-start: 4px;
    font-size: 11px;
    color: var(--accent);
  }

  .count.zero {
    color: var(--text-faint);
  }
</style>
