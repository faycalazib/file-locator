<script lang="ts">
  import Icon from './Icon.svelte';
  import { dismissable } from '../actions';
  import { formatNumber, t, type MessageKey } from '../i18n/index.svelte';
  import { search } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';
  import { ALL_DETECTORS, ATTRIBUTES, DATE_FIELDS, DETECTOR_GROUPS, hashKind } from '../filters';
  import type { Attribute, DateField, DateFilter, Detector, DocLang, SizeFilter } from '../types';

  /**
   * File types are chips in the masthead; this panel holds the rest.
   * sheet = over the results (Log) · rail = in the rail (Ledger) · drawer = side panel (Strata)
   */
  let { placement }: { placement: 'sheet' | 'rail' | 'drawer' } = $props();

  const LANGS: DocLang[] = ['fr', 'es', 'ar', 'en'];
  const SIZES: SizeFilter[] = ['any', 'lt1', '1to10', '10to100', 'gt100'];
  const DATES: DateFilter[] = ['any', 'day', 'week', 'month', 'year', 'custom'];

  const sizeKey = (s: SizeFilter) => `filters.sizes.${s}` as MessageKey;
  const dateKey = (d: DateFilter) => `filters.dates.${d}` as MessageKey;
  const langKey = (l: DocLang) => `languages.${l}` as MessageKey;
  const fieldKey = (d: DateField) => `filters.dateFields.${d}` as MessageKey;
  const attrKey = (a: Attribute) => `filters.attrs.${a}` as MessageKey;
  const detectorKey = (d: Detector) => `detectors.names.${d}` as MessageKey;
  const detectorHint = (d: Detector) => `detectors.hints.${d}` as MessageKey;
  const groupKey = (g: string) => `detectors.groups.${g}` as MessageKey;

  /** Personal-data audit (lot 6.2): every detector, run at once (no words needed). */
  function audit() {
    search.filters.detectors = [...ALL_DETECTORS];
    if (search.signature() === search.ranSignature || !search.hasRun) void search.run();
    close();
  }

  // Lot 5.8: the digest is checked while typing, applied when the field is left.
  let hash = $state('');
  $effect(() => {
    hash = search.filters.hash;
  });
  const hashState = $derived(hashKind(hash));

  function applyHash() {
    if (hashState === 'invalid') return;
    search.filters.hash = hash.trim();
    if (!search.copies) search.copiesOf = null;
  }

  const floating = $derived(placement !== 'rail');

  let newExclusion = $state('');

  function addExclusion(event: SubmitEvent) {
    event.preventDefault();
    const path = newExclusion.trim();
    if (path && !search.filters.excluded.includes(path)) {
      search.filters.excluded = [...search.filters.excluded, path];
    }
    newExclusion = '';
  }

  function close() {
    if (floating) ui.filtersOpen = false;
  }
</script>

<section id="filter-panel" class="filters sketch {placement}" aria-label={t('filters.title')} use:dismissable={close}>
  <header>
    <h2 class="tag sketch hatch">{t('filters.title')}</h2>
    <div class="header-actions">
      {#if search.activeFilterCount > 0}
        <button type="button" class="link" onclick={() => search.resetFilters()}>{t('filters.reset')}</button>
      {/if}
      {#if floating}
        <button type="button" class="icon-btn" aria-label={t('settings.close')} onclick={close}>
          <Icon name="close" size={16} />
        </button>
      {/if}
    </div>
  </header>

  <div class="groups">
    <div class="group">
      <label class="label" for="f-size-{placement}">{t('filters.size')}</label>
      <span class="select sketch">
        <select id="f-size-{placement}" bind:value={search.filters.size}>
          {#each SIZES as s (s)}
            <option value={s}>{t(sizeKey(s))}</option>
          {/each}
        </select>
      </span>
    </div>

    <div class="group">
      <label class="label" for="f-date-{placement}">{t('filters.date')}</label>
      <div class="date-row">
        <span class="select sketch">
          <select aria-label={t('filters.dateField')} bind:value={search.filters.dateField}>
            {#each DATE_FIELDS as d (d)}
              <option value={d}>{t(fieldKey(d))}</option>
            {/each}
          </select>
        </span>
        <span class="select sketch">
          <select id="f-date-{placement}" bind:value={search.filters.date}>
            {#each DATES as d (d)}
              <option value={d}>{t(dateKey(d))}</option>
            {/each}
          </select>
        </span>
      </div>
      {#if search.filters.date === 'custom'}
        <div class="date-row">
          <label class="day">
            <span>{t('filters.dateFrom')}</span>
            <input type="date" class="sketch" bind:value={search.filters.dateFrom} max={search.filters.dateTo || undefined} />
          </label>
          <label class="day">
            <span>{t('filters.dateTo')}</span>
            <input type="date" class="sketch" bind:value={search.filters.dateTo} min={search.filters.dateFrom || undefined} />
          </label>
        </div>
      {/if}
      {#if search.facets && Object.keys(search.facets.years).length > 0}
        <div class="years" role="group" aria-label={t('facets.years')}>
          <span class="family-name">{t('facets.years')}</span>
          <div class="chips">
            {#each Object.entries(search.facets.years).toSorted((a, b) => b[0].localeCompare(a[0])) as [year, n] (year)}
              <button type="button" class="chip sketch" class:hatch={search.isYear(year)} aria-pressed={search.isYear(year)} onclick={() => search.toggleYear(year)}>
                {year} <span class="count mono">{formatNumber(n)}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
      {#if search.filters.dateField === 'accessed' && search.filters.date !== 'any'}
        <p class="note">{t('filters.accessedHint')}</p>
      {/if}
    </div>

    <div class="group">
      <span class="label">{t('filters.language')}</span>
      <div class="chips">
        {#each LANGS as lang (lang)}
          <button
            type="button"
            class="chip sketch"
            class:hatch={search.filters.langs.includes(lang)}
            aria-pressed={search.filters.langs.includes(lang)}
            onclick={() => search.toggleLang(lang)}
          >
            {t(langKey(lang))}
            {#if search.facets}<span class="count mono" class:zero={!search.facets.langs[lang]}>{formatNumber(search.facets.langs[lang] ?? 0)}</span>{/if}
          </button>
        {/each}
      </div>
    </div>

    <div class="group">
      <span class="label">{t('filters.attributes')}</span>
      <div class="chips">
        {#each ATTRIBUTES as attribute (attribute)}
          <button
            type="button"
            class="chip sketch"
            class:hatch={search.filters.attributes.includes(attribute)}
            aria-pressed={search.filters.attributes.includes(attribute)}
            onclick={() => search.toggleAttribute(attribute)}
          >
            {t(attrKey(attribute))}
          </button>
        {/each}
      </div>
      {#if search.filters.attributes.includes('hidden') && search.mode === 'indexed'}
        <p class="note warn">{t('filters.hiddenIndexed')}</p>
      {:else if search.filters.attributes.length > 0}
        <p class="note">{t('filters.attributesHint')}</p>
      {/if}
    </div>

    <div class="group">
      <label class="label" for="f-hash-{placement}">{t('filters.hash')}</label>
      <span class="field sketch" class:invalid={hashState === 'invalid'}>
      <input
        id="f-hash-{placement}"
        class="hash mono"
        dir="ltr"
        spellcheck="false"
        autocomplete="off"
        placeholder={t('filters.hashPlaceholder')}
        aria-invalid={hashState === 'invalid'}
        bind:value={hash}
        onchange={applyHash}
      />
      </span>
      {#if hashState === 'invalid'}
        <p class="note warn">{t('filters.hashInvalid')}</p>
      {:else if hashState}
        <p class="note">{t(hashState === 'md5' ? 'filters.hashMd5' : 'filters.hashSha256')}</p>
      {/if}
    </div>

    <div class="group wide detectors">
      <div class="detectors-head">
        <span class="label">{t('detectors.title')}</span>
        <button type="button" class="audit sketch hatch" onclick={audit}>{t('detectors.audit')}</button>
      </div>
      <div class="detector-groups">
        {#each DETECTOR_GROUPS as family (family.group)}
          <div class="family">
            <span class="family-name">{t(groupKey(family.group))}</span>
            <div class="chips">
              {#each family.detectors as detector (detector)}
                <button
                  type="button"
                  class="chip sketch"
                  class:hatch={search.filters.detectors.includes(detector)}
                  aria-pressed={search.filters.detectors.includes(detector)}
                  title={t(detectorHint(detector))}
                  onclick={() => search.toggleDetector(detector)}
                >
                  {t(detectorKey(detector))}
                </button>
              {/each}
            </div>
          </div>
        {/each}
      </div>
      <p class="note">{t('detectors.hint')}</p>
    </div>

    <div class="group wide">
      <span class="label">{t('filters.excluded')}</span>
      <ul class="excluded">
        {#each search.filters.excluded as path (path)}
          <li class="sketch">
            <span class="mono ltr-isolate path" dir="ltr">{path}</span>
            <button
              type="button"
              class="remove"
              aria-label={t('filters.removeExcluded', { path })}
              title={t('filters.removeExcluded', { path })}
              onclick={() => (search.filters.excluded = search.filters.excluded.filter((p) => p !== path))}
            >
              <Icon name="close" size={12} />
            </button>
          </li>
        {/each}
        <li class="add sketch dashed">
          <form onsubmit={addExclusion}>
            <input class="mono" dir="ltr" aria-label={t('filters.addExcluded')} placeholder={t('filters.addExcluded')} bind:value={newExclusion} />
            <button type="submit" class="remove" aria-label={t('filters.addExcluded')}><Icon name="plus" size={12} /></button>
          </form>
        </li>
      </ul>
    </div>
  </div>
</section>

<style>
  .filters {
    --k: var(--secondary);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding: var(--sp-6) var(--sp-5) var(--sp-5);
    background: var(--surface);
  }

  .sheet {
    position: absolute;
    inset-block-start: var(--sp-2);
    inset-inline: var(--sp-2);
    z-index: 10;
    box-shadow: var(--shadow-pop);
  }

  .sheet .groups {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .sheet .wide {
    grid-column: 1 / -1;
  }

  .drawer {
    position: absolute;
    inset-block: var(--sp-2);
    inset-inline-end: var(--sp-2);
    z-index: 10;
    inline-size: min(340px, calc(100% - var(--sp-4)));
    overflow-y: auto;
    box-shadow: var(--shadow-pop);
  }

  .rail {
    --k: var(--secondary);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .tag {
    --k: var(--secondary);
    --h: var(--fill-secondary);
    margin: 0;
    padding: 0 var(--sp-4);
    font: var(--fs-h2) / var(--lh-h2) var(--font-brand);
    text-transform: uppercase;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4) var(--sp-5);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    min-inline-size: 0;
  }

  .label {
    font: 17px/22px var(--font-display);
    text-transform: uppercase;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }

  .chip {
    --h: var(--fill-yellow);
    padding: 1px var(--sp-3);
    background-color: transparent;
    border: 0;
    border-radius: 999px;
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
  }

  .chip::before {
    border-radius: 999px;
  }

  .select {
    --sw: 2px;
    display: block;
  }

  select {
    position: relative;
    inline-size: 100%;
    block-size: 36px;
    padding-inline: var(--sp-3);
    background: transparent;
    border: 0;
    font: 16px var(--font-body);
    cursor: pointer;
  }

  .excluded {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .excluded li {
    --sw: 2px;
    --k: var(--text-faint);
    display: flex;
    align-items: center;
    gap: 2px;
    padding-inline: var(--sp-3) var(--sp-1);
  }

  .path {
    position: relative;
    font-size: var(--fs-caption);
    line-height: 32px;
    color: var(--text-dim);
  }

  .remove,
  .icon-btn {
    position: relative;
    display: grid;
    place-items: center;
    inline-size: 26px;
    block-size: 26px;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--text-dim);
  }

  .remove:hover,
  .icon-btn:hover {
    color: var(--danger-ink);
  }

  .add form {
    position: relative;
    display: flex;
    align-items: center;
  }

  .add input {
    inline-size: 180px;
    block-size: 32px;
    padding: 0;
    background: transparent;
    border: 0;
    font-size: var(--fs-caption);
  }

  .add input::placeholder {
    font-family: var(--font-body);
    color: var(--text-faint);
  }

  .link {
    padding: 0;
    background: none;
    border: 0;
    color: var(--secondary-ink);
    font: 16px var(--font-display);
    text-transform: uppercase;
    text-decoration: underline wavy 1.5px;
    text-underline-offset: 4px;
  }
  /* Lot 5.8: date field + period, custom days, digest. */
  .date-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
  }

  .date-row + .date-row {
    margin-block-start: var(--sp-2);
  }

  .day {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  .day input,
  .hash {
    padding: 2px var(--sp-2);
    background: var(--surface);
    border: 0;
    font-size: var(--fs-caption);
    color: var(--text);
    color-scheme: inherit;
  }

  .hash {
    display: block;
    inline-size: 100%;
    background: transparent;
  }

  .field {
    --sw: 1.5px;
    display: block;
  }

  .field.invalid {
    --k: var(--danger-ink);
  }

  .note {
    margin: var(--sp-1) 0 0;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  .note.warn {
    color: var(--danger-ink);
  }

  /* Lot 6.2: detectors by family, and the audit button. */
  .detectors-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
  }

  .audit {
    --sw: 2px;
    padding: 2px var(--sp-3);
    background: transparent;
    border: 0;
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
  }

  .detector-groups {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: var(--sp-2) var(--sp-4);
    margin-block-start: var(--sp-2);
  }

  .family-name {
    display: block;
    margin-block-end: var(--sp-1);
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  /* Lot 6.3: counts on the language chips, and the years. */
  .count {
    margin-inline-start: 4px;
    font-size: 11px;
    color: var(--accent);
  }

  .count.zero {
    color: var(--text-faint);
  }

  .years {
    margin-block-start: var(--sp-2);
  }
</style>
