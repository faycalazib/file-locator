<!--
  Printable report of the search on screen (Save as PDF from the print
  dialog). Mounted by App.svelte only while printing (`ui.printing`).
  Sober on purpose: black on white, whatever the theme; the fonts stay those
  of the theme (Arabic included) and the direction that of the interface.
-->
<script lang="ts">
  import Marked from './Marked.svelte';
  import { formatBytes, formatDate, t, type MessageKey } from '../i18n/index.svelte';
  import { search } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import { splitPath } from '../text';
  import { ALL_DETECTORS, dateLabel, hasDateFilter, maskSensitive, shortHash } from '../filters';

  const hits = $derived(search.visible);
  const generated = new Date();
  const totalMatches = $derived(hits.reduce((sum, h) => sum + h.matchCount, 0));

  const options = $derived(
    (['fuzzy', 'caseSensitive', 'wholeWord', 'regex'] as const)
      .filter((o) => search.options[o])
      .map((o) => t(`search.options.${o}`)),
  );

  const filters = $derived.by(() => {
    const f = search.filters;
    return [
      ...f.kinds.map((k) => t(`filters.kinds.${k}` as MessageKey)),
      ...(f.size !== 'any' ? [t(`filters.sizes.${f.size}` as MessageKey)] : []),
      ...(hasDateFilter(f) ? [dateLabel(f)] : []),
      ...f.attributes.map((a) => t(`filters.attrs.${a}` as MessageKey)),
      ...(f.detectors.length === ALL_DETECTORS.length
        ? [t('detectors.audit')]
        : f.detectors.map((d) => t(`detectors.names.${d}` as MessageKey))),
      ...(f.hash.trim() ? [t('filters.hashToken', { hash: shortHash(f.hash) })] : []),
      ...f.langs.map((l) => t(`languages.${l}` as MessageKey)),
    ];
  });

  const sites = $derived(sitesStore.list.filter((s) => search.scope.has(s.id)).map((s) => s.name));

  // Detectors (lot 6.2): a summary per kind; account and identity numbers masked.
  const detecting = $derived(search.filters.detectors.length > 0);
  const detected = $derived(Object.entries(search.detections).toSorted((a, b) => b[1].files - a[1].files));
  const shown = (text: string) => (detecting ? maskSensitive(text) : text);

  // Keyword report (lot 6.4): totals per term, and each file's counts.
  const keywords = $derived(ui.reportKeywords);
  const countsOf = $derived(new Map((keywords?.rows ?? []).map((r) => [`${r.siteId}:${r.path}`, r.counts])));
</script>

<article class="report">
  <header>
    <p class="brand">PROSPECTOR</p>
    <h1>{t('report.title')}</h1>
    <p class="generated">{t('report.generated', { date: formatDate(generated) })}</p>
  </header>

  <dl class="criteria">
    <dt>{t('report.query')}</dt>
    <dd class="query"><span dir="auto">{search.lastQuery}</span></dd>
    {#if search.namePattern.trim() || search.folders}
      <dt>{t('report.names')}</dt>
      <dd class="mono">
        <span class="ltr-isolate" dir="ltr">{search.namePattern.trim() || '—'}</span>{#if search.folders}<span class="sep">·</span>{t('search.names.folders')}{/if}
      </dd>
    {/if}
    {#if search.filters.termList}
      {@const list = search.filters.termList}
      <dt>{t('terms.title')}</dt>
      <dd><span class="ltr-isolate" dir="ltr">{list.name}</span> · {t('terms.count', { count: list.terms.length })} · {list.all ? t('terms.all') : t('terms.any')}</dd>
    {/if}
    <dt>{t('search.mode.label')}</dt>
    <dd>{t(`search.mode.${search.mode}`)}</dd>
    <dt>{t('report.options')}</dt>
    <dd>{options.length ? options.join(' · ') : t('report.none')}</dd>
    <dt>{t('filters.title')}</dt>
    <dd>{filters.length ? filters.join(' · ') : t('report.none')}</dd>
    {#if search.filters.excluded.length}
      <dt>{t('filters.excluded')}</dt>
      <dd class="mono">
        {#each search.filters.excluded as path, i (path)}{#if i > 0}<span class="sep">·</span>{/if}<span class="ltr-isolate" dir="ltr">{path}</span>{/each}
      </dd>
    {/if}
    <dt>{t('report.sites')}</dt>
    <dd>
      {#each sites as name, i (i)}{#if i > 0}<span class="sep">·</span>{/if}<bdi>{name}</bdi>{:else}{t('report.none')}{/each}
    </dd>
    <dt>{t('results.label')}</dt>
    <dd>{t('results.files', { count: hits.length })} · {t('results.matches', { count: totalMatches })}</dd>
  </dl>

  {#if keywords && keywords.terms.length > 0}
    <section class="audit">
      <h2>{t('export.byKeyword')}</h2>
      <table>
        <thead>
          <tr><th>{t('export.term')}</th><th class="num">{t('detectors.files')}</th><th class="num">{t('detectors.occurrences')}</th></tr>
        </thead>
        <tbody>
          {#each keywords.terms as term, i (term)}
            <tr><td dir="auto">{term}</td><td class="num">{keywords.totals[i]?.files ?? 0}</td><td class="num">{keywords.totals[i]?.matches ?? 0}</td></tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/if}

  {#if detecting}
    <section class="audit">
      <h2>{t('detectors.reportTitle')}</h2>
      <table>
        <thead>
          <tr><th>{t('detectors.kind')}</th><th class="num">{t('detectors.files')}</th><th class="num">{t('detectors.occurrences')}</th></tr>
        </thead>
        <tbody>
          {#each detected as [code, total] (code)}
            <tr><td>{t(`detectors.names.${code}` as MessageKey)}</td><td class="num">{total.files}</td><td class="num">{total.matches}</td></tr>
          {:else}
            <tr><td colspan="3">{t('detectors.nothing')}</td></tr>
          {/each}
        </tbody>
      </table>
      <p class="masked">{t('detectors.masked')}</p>
    </section>
  {/if}

  <ol class="hits">
    {#each hits as hit (hit.id)}
      {@const p = splitPath(hit.path)}
      <li>
        <h2><span class="ltr-isolate" dir="ltr">{p.name}</span></h2>
        <p class="path mono"><span class="ltr-isolate" dir="ltr">{hit.path}</span></p>
        <p class="meta">
          {t(`filters.kinds.${hit.kind}` as MessageKey)} · <span class="mono">{formatBytes(hit.sizeBytes)}</span> · {formatDate(hit.modified)} ·
          {t(`languages.${hit.lang}` as MessageKey)} · <strong>{t('results.matches', { count: hit.matchCount })}</strong>
        </p>
        {#if keywords && countsOf.has(hit.id)}
          <p class="meta">
            {#each keywords.terms as term, i (term)}{#if i > 0}<span class="sep">·</span>{/if}<bdi>{term}</bdi> <strong>{countsOf.get(hit.id)?.[i] ?? 0}</strong>{/each}
          </p>
        {/if}
        {#each hit.snippets as snippet, i (i)}
          <p class="snippet" dir="auto"><Marked text={shown(snippet.text)} /></p>
        {/each}
      </li>
    {/each}
  </ol>

  <footer>{t('report.legend')}</footer>
</article>

<style>
  .report {
    --ink: #1b1b1b;
    --dim: #5a5a5a;
    color: var(--ink);
    background: #fff;
    font-size: 12pt;
    line-height: 1.45;
    padding: 0 2mm;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }

  header {
    padding-block-end: 4mm;
    border-block-end: 2px solid var(--ink);
  }

  .brand {
    margin: 0;
    font: 11pt/1.2 var(--font-display);
    letter-spacing: 0.08em;
    color: var(--dim);
  }

  h1 {
    margin: 1mm 0 0;
    font: 22pt/1.2 var(--font-display);
  }

  .generated {
    margin: 1mm 0 0;
    color: var(--dim);
    font-size: 10pt;
  }

  .criteria {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 1.5mm 6mm;
    margin: 5mm 0;
    font-size: 11pt;
  }

  dt {
    color: var(--dim);
  }

  dd {
    margin: 0;
  }

  .query {
    font-weight: 700;
  }

  .sep {
    margin-inline: 1.5mm;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 9.5pt;
  }

  .hits {
    margin: 0;
    padding-inline-start: 7mm;
  }

  li {
    padding-block: 3mm;
    border-block-start: 1px solid #c8c8c8;
    break-inside: avoid;
  }

  h2 {
    margin: 0;
    font-size: 13pt;
  }

  .path {
    margin: 0.5mm 0 0;
    color: var(--dim);
    overflow-wrap: anywhere;
  }

  .meta {
    margin: 0.5mm 0 1.5mm;
    color: var(--dim);
    font-size: 10pt;
  }

  .snippet {
    margin: 1mm 0 0;
    padding-inline-start: 3mm;
    border-inline-start: 2px solid #c8c8c8;
    unicode-bidi: plaintext;
  }

  /* Matches: yellow highlighter, approximate ones underlined (as in the app). */
  .report :global(mark) {
    background: #ffe86b;
    color: var(--ink);
  }

  .report :global(mark.fuzzy) {
    background: none;
  }

  footer {
    margin-block-start: 5mm;
    padding-block-start: 2mm;
    border-block-start: 2px solid var(--ink);
    color: var(--dim);
    font-size: 9.5pt;
  }

  /* Lot 6.2: personal-data summary. */
  .audit {
    margin-block: 4mm;
    break-inside: avoid;
  }

  .audit h2 {
    margin: 0 0 2mm;
    font-size: 13pt;
  }

  .audit table {
    border-collapse: collapse;
    min-inline-size: 60%;
  }

  .audit th,
  .audit td {
    padding: 1mm 3mm;
    border-block-end: 1px solid #ccc;
    text-align: start;
  }

  .audit .num {
    text-align: end;
    font-variant-numeric: tabular-nums;
  }

  .masked {
    margin: 2mm 0 0;
    font-size: 10pt;
    color: var(--dim);
  }
</style>
