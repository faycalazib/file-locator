<!--
  Export the results / copy the files found (lot 6.4). One window, two modes
  (`ui.transfer`): the scope (files shown / everything found), then either
  the format and the columns, or the destination (folder / ZIP) with the
  copy's progress and its summary.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { api, inTauri, type CopyReportDto } from '../api';
  import { COLUMNS, DEFAULT_COLUMNS, csvSeparator, type Column } from '../export';
  import { ALL_LIMIT, canFetchAll, cancelCopy, hasTerms, runExport, startCopy, type Format, type Scope } from '../exporting';
  import { formatNumber, t, type MessageKey } from '../i18n/index.svelte';
  import { notices } from '../stores/notices.svelte';
  import { search } from '../stores/search.svelte';
  import { ui } from '../stores/ui.svelte';

  const COLUMNS_KEY = 'prospector.exportColumns';

  function loadColumns(): Column[] {
    try {
      const stored = JSON.parse(localStorage.getItem(COLUMNS_KEY) ?? 'null');
      if (Array.isArray(stored)) return stored.filter((c): c is Column => (COLUMNS as string[]).includes(c));
    } catch {
      /* default columns */
    }
    return [...DEFAULT_COLUMNS];
  }

  let format = $state<Format>('excel');
  let scope = $state<Scope>(canFetchAll() ? 'all' : 'visible');
  let columns = $state<Column[]>(loadColumns());
  let zip = $state(false);
  let keepTree = $state(true);
  let extract = $state(true);
  let busy = $state(false);
  let progress = $state<{ done: number; total: number } | null>(null);
  let copyId = $state<string | null>(null);
  let report = $state<CopyReportDto | null>(null);

  const mode = $derived(ui.transfer);
  const hasDetections = $derived(Object.keys(search.detections).length > 0);
  /** A column that cannot apply to this search. */
  const unavailable = (c: Column) => (c === 'keywords' && !hasTerms()) || (c === 'detections' && !hasDetections);
  const columnKey = (c: Column) => `export.columns.${c}` as MessageKey;

  function toggle(c: Column) {
    columns = columns.includes(c) ? columns.filter((x) => x !== c) : COLUMNS.filter((x) => x === c || columns.includes(x));
    try {
      localStorage.setItem(COLUMNS_KEY, JSON.stringify(columns));
    } catch {
      /* not remembered */
    }
  }

  function close() {
    if (copyId) return;
    ui.transfer = null;
    report = null;
    progress = null;
  }

  async function exportNow() {
    busy = true;
    try {
      await runExport(format, scope, columns.filter((c) => !unavailable(c)));
      ui.transfer = null;
    } catch (e) {
      notices.error(e);
    } finally {
      busy = false;
    }
  }

  async function copyNow() {
    busy = true;
    report = null;
    try {
      const run = await startCopy(scope, zip, keepTree, extract, (done, total) => (progress = { done, total }));
      if (!run) return;
      copyId = run.id;
      report = await run.done;
    } catch (e) {
      notices.error(e);
    } finally {
      busy = false;
      copyId = null;
    }
  }

  function reveal() {
    if (report && inTauri) api.revealFile(report.target).catch((e) => notices.error(e));
  }
</script>

{#if mode}
  <div class="backdrop" role="presentation" onclick={close}></div>
  <div class="dialog sketch" role="dialog" aria-modal="true" aria-labelledby="transfer-title">
    <!-- The frame stays put; only its content scrolls (the outline is drawn on the frame). -->
    <div class="body">
    <header>
      <h2 id="transfer-title">{t(mode === 'export' ? 'export.dialogTitle' : 'export.copyTitle')}</h2>
      <button type="button" class="icon-btn" aria-label={t('settings.close')} onclick={close} disabled={copyId !== null}>
        <Icon name="close" size={16} />
      </button>
    </header>

    <fieldset>
      <legend>{t('export.scope')}</legend>
      <label class="option">
        <input type="radio" name="scope" value="visible" bind:group={scope} />
        <span>{t('export.scopeVisible', { files: t('results.files', { count: search.visible.length }) })}</span>
      </label>
      <label class="option" class:off={!canFetchAll()}>
        <input type="radio" name="scope" value="all" bind:group={scope} disabled={!canFetchAll()} />
        <span>{t('export.scopeAll', { max: formatNumber(ALL_LIMIT) })}</span>
      </label>
      {#if !canFetchAll()}<p class="hint">{t('export.scopeAllHint')}</p>{/if}
    </fieldset>

    {#if mode === 'export'}
      <fieldset>
        <legend>{t('export.format')}</legend>
        <label class="option">
          <input type="radio" name="format" value="excel" bind:group={format} />
          <span>{t('export.formats.excel')} <small class="dim">({t('export.separator', { sep: csvSeparator() })})</small></span>
        </label>
        <label class="option"><input type="radio" name="format" value="html" bind:group={format} /><span>{t('export.formats.html')}</span></label>
        <label class="option"><input type="radio" name="format" value="json" bind:group={format} /><span>{t('export.formats.json')}</span></label>
      </fieldset>

      {#if format !== 'json'}
        <fieldset>
          <legend>{t('export.columnsTitle')}</legend>
          <div class="columns">
            {#each COLUMNS as c (c)}
              <label class="option" class:off={unavailable(c)} title={unavailable(c) ? t(c === 'keywords' ? 'export.noTerms' : 'export.noDetections') : ''}>
                <input type="checkbox" checked={columns.includes(c) && !unavailable(c)} disabled={unavailable(c)} onchange={() => toggle(c)} />
                <span>{t(columnKey(c))}</span>
              </label>
            {/each}
          </div>
          {#if columns.includes('keywords') && hasTerms()}<p class="hint">{t('export.keywordsHint')}</p>{/if}
        </fieldset>
      {/if}
    {:else}
      <fieldset>
        <legend>{t('export.destination')}</legend>
        <label class="option"><input type="radio" name="dest" value={false} bind:group={zip} disabled={busy} /><span>{t('export.toFolder')}</span></label>
        <label class="option"><input type="radio" name="dest" value={true} bind:group={zip} disabled={busy} /><span>{t('export.toZip')}</span></label>
        <label class="option"><input type="checkbox" bind:checked={keepTree} disabled={busy} /><span>{t('export.keepTree')}</span></label>
        <label class="option"><input type="checkbox" bind:checked={extract} disabled={busy} /><span>{t('export.extractInner')}</span></label>
        <p class="hint">{t('export.copyHint')} {t('export.extractInnerHint')}</p>
      </fieldset>

      {#if progress && (busy || report)}
        <div class="progress" role="progressbar" aria-valuemin={0} aria-valuemax={progress.total} aria-valuenow={progress.done}>
          <span class="bar" style:inline-size="{progress.total ? (100 * progress.done) / progress.total : 100}%"></span>
        </div>
        <p class="hint mono">{formatNumber(progress.done)} / {formatNumber(progress.total)}</p>
      {/if}

      {#if report}
        <div class="summary" role="status">
          <p><b>{t(report.cancelled ? 'export.copyCancelled' : 'export.copied', { files: t('results.files', { count: report.copied }) })}</b></p>
          {#if report.fromContainers > 0}<p class="hint">{t('export.fromContainers', { count: report.fromContainers })}</p>{/if}
          {#if report.skipped.length > 0}
            <details>
              <summary>{t('export.skipped', { files: t('results.files', { count: report.skipped.length }) })}</summary>
              <ul class="mono">
                {#each report.skipped.slice(0, 50) as path (path)}<li dir="ltr">{path}</li>{/each}
              </ul>
            </details>
          {/if}
        </div>
      {/if}
    {/if}

    <footer>
      {#if mode === 'copy' && copyId}
        <button type="button" class="btn sketch" onclick={() => copyId && cancelCopy(copyId)}><Icon name="stop" size={14} />{t('search.stop')}</button>
      {:else if mode === 'copy' && report}
        <button type="button" class="btn sketch" onclick={reveal}><Icon name="folder" size={14} />{t('preview.reveal')}</button>
        <button type="button" class="btn sketch hatch primary" onclick={close}>{t('settings.close')}</button>
      {:else}
        <button type="button" class="btn sketch" onclick={close}>{t('export.cancel')}</button>
        <button type="button" class="btn sketch hatch primary" disabled={busy || search.visible.length === 0} onclick={mode === 'export' ? exportNow : copyNow}>
          {busy ? '…' : t(mode === 'export' ? 'export.run' : 'export.copyRun')}
        </button>
      {/if}
    </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: color-mix(in srgb, var(--text) 18%, transparent);
  }

  .dialog {
    --k: var(--secondary);
    position: fixed;
    inset-block-start: 12vh;
    inset-inline-start: 50%;
    z-index: 41;
    display: flex;
    flex-direction: column;
    inline-size: min(560px, calc(100vw - 32px));
    max-block-size: 80vh;
    background: var(--surface);
    translate: -50% 0;
  }

  .body {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    min-block-size: 0;
    overflow: auto;
    padding: var(--sp-5) var(--sp-6);
  }

  :global([dir='rtl']) .dialog {
    translate: 50% 0;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font: var(--fs-h2) / var(--lh-h2) var(--font-display);
    text-transform: uppercase;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    margin-block-end: var(--sp-1);
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding-block: 2px;
  }

  .option.off {
    color: var(--text-faint);
  }

  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0 var(--sp-3);
  }

  .hint {
    margin: var(--sp-1) 0 0;
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  .dim {
    color: var(--text-dim);
  }

  .progress {
    block-size: 10px;
    border: 1.5px solid var(--text);
    border-radius: 6px;
    overflow: hidden;
  }

  .bar {
    display: block;
    block-size: 100%;
    background: var(--accent);
    transition: inline-size 150ms;
  }

  .summary p {
    margin: 0;
  }

  .summary ul {
    max-block-size: 140px;
    overflow: auto;
    margin: var(--sp-1) 0 0;
    padding-inline-start: var(--sp-4);
    font-size: var(--fs-caption);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }

  .btn {
    --sw: 2px;
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    padding: 4px var(--sp-4);
    background: transparent;
    border: 0;
    font: 16px/22px var(--font-display);
    text-transform: uppercase;
  }

  @media (prefers-reduced-motion: reduce) {
    .bar {
      transition: none;
    }
  }
</style>
