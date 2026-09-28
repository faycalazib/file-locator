<!--
  Duplicates of the checked sites (lot 6.5): identical files and
  near-identical documents, one card per group. Files can be sent to the
  Recycle Bin (never all the files of a group, always after confirmation).
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import { api, inTauri, type DupGroupDto, type DupReportDto } from '../api';
  import { download } from '../export';
  import { formatBytes, formatDate, formatNumber, formatPercent, t } from '../i18n/index.svelte';
  import { hits as mockHits } from '../mock/data';
  import { notices } from '../stores/notices.svelte';
  import { search } from '../stores/search.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import { ui } from '../stores/ui.svelte';
  import { splitPath } from '../text';

  let exact = $state(true);
  let similar = $state(true);
  let threshold = $state(0.8);
  let running = $state<string | null>(null);
  let progress = $state<{ phase: 'hashing' | 'comparing'; done: number; total: number } | null>(null);
  let report = $state<DupReportDto | null>(inTauri ? null : demoReport());
  let chosen = $state(new Set<string>());

  const groups = $derived(report?.groups ?? []);
  const wasted = $derived(groups.reduce((sum, g) => sum + g.wasted, 0));
  const sites = $derived(sitesStore.list.filter((s) => search.scope.has(s.id)));
  const inner = (path: string) => path.includes(' › ');

  /** Browser demo: groups made of the mock hits. */
  function demoReport(): DupReportDto {
    const file = (i: number, path?: string) => ({ siteId: mockHits[i]!.siteId, path: path ?? mockHits[i]!.path, size: mockHits[i]!.sizeBytes, modified: mockHits[i]!.modified.getTime() / 1000 });
    return {
      groups: [
        { kind: 'exact', similarity: 1, wasted: 2 * mockHits[0]!.sizeBytes, files: [file(0), file(0, 'D:\\Clients\\Dupont SARL\\2024\\copie de contrat-prestation-v3.pdf'), file(0, 'E:\\Sauvegarde\\contrat-prestation-v3.pdf')] },
        { kind: 'similar', similarity: 0.87, wasted: 0, files: [file(1), file(2)] },
      ],
      filesHashed: 412,
      docsCompared: 9321,
      tookMs: 2140,
      cancelled: false,
    };
  }

  async function start() {
    report = null;
    chosen = new Set();
    progress = null;
    if (!inTauri) {
      report = demoReport();
      return;
    }
    const id = crypto.randomUUID();
    running = id;
    let finish!: (r: DupReportDto | null) => void;
    const done = new Promise<DupReportDto | null>((resolve) => (finish = resolve));
    const unlisten = [
      await api.onDupProgress((e) => {
        if (e.dupId === id) progress = { phase: e.phase, done: e.done, total: e.total };
      }),
      await api.onDupFinished((e) => {
        if (e.dupId !== id) return;
        if (e.error) notices.error(e.error);
        finish(e.report);
      }),
    ];
    try {
      await api.findDuplicates(id, sites.map((s) => s.id), { exact, similar, threshold });
      report = await done;
    } catch (e) {
      notices.error(e);
    } finally {
      unlisten.forEach((u) => u());
      running = null;
    }
  }

  function stop() {
    if (running) void api.cancelDuplicates(running).catch(() => {});
  }

  function close() {
    stop();
    ui.duplicates = false;
  }

  /** One file of a group always stays: the last unchecked one cannot be checked. */
  function canChoose(group: DupGroupDto, path: string): boolean {
    if (inner(path)) return false;
    if (chosen.has(path)) return true;
    return group.files.filter((f) => f.path !== path && !chosen.has(f.path)).length > 0;
  }

  function toggle(path: string) {
    const next = new Set(chosen);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    chosen = next;
  }

  async function trash() {
    const paths = [...chosen];
    if (paths.length === 0) return;
    const ok = await api.confirm(t('dupes.trashConfirm', { files: t('results.files', { count: paths.length }) }), t('dupes.trash')).catch(() => false);
    if (!ok) return;
    const failed = inTauri ? await api.trashFiles(paths).catch((e) => (notices.error(e), paths)) : [];
    const moved = new Set(paths.filter((p) => !failed.includes(p)));
    if (report) {
      report = {
        ...report,
        groups: report.groups
          .map((g) => ({ ...g, files: g.files.filter((f) => !moved.has(f.path)), wasted: g.kind === 'exact' ? g.files[0]!.size * (g.files.filter((f) => !moved.has(f.path)).length - 1) : 0 }))
          .filter((g) => g.files.length > 1),
      };
    }
    chosen = new Set(failed);
    notices.push(t('dupes.trashed', { files: t('results.files', { count: moved.size }) }));
    if (failed.length) notices.push(t('dupes.trashFailed', { files: t('results.files', { count: failed.length }) }), 'error');
  }

  function openFile(path: string) {
    if (inTauri) api.openFile(path).catch((e) => notices.error(e));
  }

  function reveal(path: string) {
    if (inTauri) api.revealFile(path).catch((e) => notices.error(e));
  }

  /** Every group as CSV rows (group, kind, similarity, path, size, date). */
  async function exportCsv() {
    const sep = ',';
    const cell = (v: string | number) => (/[",\n]/.test(String(v)) ? `"${String(v).replace(/"/g, '""')}"` : String(v));
    const rows = [[t('dupes.group'), t('dupes.kind'), t('dupes.similarity'), t('export.columns.path'), t('export.columns.size'), t('export.columns.modified')]];
    groups.forEach((g, i) =>
      g.files.forEach((f) => rows.push([String(i + 1), t(g.kind === 'exact' ? 'dupes.exact' : 'dupes.similar'), `${Math.round(g.similarity * 100)}%`, f.path, String(f.size), new Date(f.modified * 1000).toISOString()])),
    );
    const content = '﻿' + rows.map((r) => r.map(cell).join(sep)).join('\r\n');
    if (!inTauri) return download('doublons.csv', content, 'text/csv');
    const path = await api.pickSavePath('doublons.csv', 'csv', 'CSV').catch(() => null);
    if (path) await api.saveTextFile(path, content).then(() => notices.push(t('results.exported', { path }))).catch((e) => notices.error(e));
  }
</script>

{#if ui.duplicates}
  <div class="backdrop" role="presentation" onclick={close}></div>
  <div class="dialog sketch" role="dialog" aria-modal="true" aria-labelledby="dupes-title">
    <header>
      <h2 id="dupes-title">{t('dupes.title')}</h2>
      <button type="button" class="icon-btn" aria-label={t('settings.close')} onclick={close}><Icon name="close" size={16} /></button>
    </header>

    <div class="options">
      <label class="option"><input type="checkbox" bind:checked={exact} disabled={running !== null} /><span>{t('dupes.exactOption')}</span></label>
      <label class="option"><input type="checkbox" bind:checked={similar} disabled={running !== null} /><span>{t('dupes.similarOption')}</span></label>
      <label class="option threshold">
        <span>{t('dupes.threshold')}</span>
        <select bind:value={threshold} disabled={!similar || running !== null}>
          {#each [0.7, 0.8, 0.9, 0.95] as v (v)}<option value={v}>{formatPercent(v)}</option>{/each}
        </select>
      </label>
      <span class="sites">{t('dupes.sites', { sites: sites.map((s) => s.name).join(' · ') || '—' })}</span>
      {#if running}
        <button type="button" class="btn sketch" onclick={stop}><Icon name="stop" size={14} />{t('search.stop')}</button>
      {:else}
        <button type="button" class="btn sketch hatch primary" onclick={start} disabled={sites.length === 0 || (!exact && !similar)}>{t('dupes.start')}</button>
      {/if}
    </div>

    {#if running && progress}
      <p class="status">{t(progress.phase === 'hashing' ? 'dupes.hashing' : 'dupes.comparing', { done: formatNumber(progress.done), total: formatNumber(progress.total) })}</p>
      <div class="progress"><span class="bar" style:inline-size="{progress.total ? (100 * progress.done) / progress.total : 0}%"></span></div>
    {:else if running}
      <p class="status">{t('dupes.starting')}</p>
    {/if}

    {#if report}
      <p class="summary">
        <b>{t('dupes.groups', { count: groups.length })}</b>
        {#if wasted > 0}· {t('dupes.wasted', { size: formatBytes(wasted) })}{/if}
        <small>· {t('dupes.scanned', { hashed: formatNumber(report.filesHashed), compared: formatNumber(report.docsCompared) })}</small>
        {#if report.cancelled}<small class="warn">· {t('dupes.cancelled')}</small>{/if}
      </p>

      <ol class="groups">
        {#each groups as group, gi (gi)}
          <li class="group sketch">
            <p class="group-head">
              <span class="badge" class:similar={group.kind === 'similar'}>
                {group.kind === 'exact' ? t('dupes.exact') : t('dupes.similarAt', { percent: formatPercent(group.similarity) })}
              </span>
              <span>{t('results.files', { count: group.files.length })}</span>
              {#if group.wasted > 0}<span class="dim">· {t('dupes.wasted', { size: formatBytes(group.wasted) })}</span>{/if}
            </p>
            <ul>
              {#each group.files as file (file.path)}
                {@const p = splitPath(file.path)}
                <li class="file">
                  <input
                    type="checkbox"
                    aria-label={t('dupes.choose', { name: p.name })}
                    checked={chosen.has(file.path)}
                    disabled={!canChoose(group, file.path)}
                    title={inner(file.path) ? t('dupes.innerHint') : !canChoose(group, file.path) ? t('dupes.keepOne') : ''}
                    onchange={() => toggle(file.path)}
                  />
                  <span class="name" title={file.path}><span class="ltr-isolate" dir="ltr">{p.name}</span></span>
                  <span class="folder mono" title={p.folder}><span class="ltr-isolate" dir="ltr">{p.folder}</span></span>
                  <span class="mono dim">{formatBytes(file.size)}</span>
                  <span class="dim">{formatDate(new Date(file.modified * 1000))}</span>
                  <button type="button" class="icon-btn small" aria-label={t('preview.open')} title={t('preview.open')} onclick={() => openFile(file.path)}><Icon name="external" size={13} /></button>
                  <button type="button" class="icon-btn small" aria-label={t('preview.reveal')} title={t('preview.reveal')} onclick={() => reveal(file.path)}><Icon name="folder" size={13} /></button>
                </li>
              {/each}
            </ul>
          </li>
        {:else}
          <li class="none">{t('dupes.none')}</li>
        {/each}
      </ol>
    {/if}

    <footer>
      {#if report && groups.length > 0}
        <button type="button" class="btn sketch" onclick={exportCsv}><Icon name="download" size={14} />{t('dupes.export')}</button>
        <button type="button" class="btn sketch danger" onclick={trash} disabled={chosen.size === 0}>
          <Icon name="trash" size={14} />{chosen.size ? t('dupes.trashCount', { count: chosen.size }) : t('dupes.trash')}
        </button>
      {/if}
    </footer>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: color-mix(in srgb, var(--text) 22%, transparent);
  }

  .dialog {
    --k: var(--secondary);
    position: fixed;
    inset: 6vh 8vw;
    z-index: 41;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-5) var(--sp-6);
    background: var(--surface);
  }

  header,
  .options,
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2) var(--sp-4);
  }

  header {
    justify-content: space-between;
  }

  h2 {
    margin: 0;
    font: var(--fs-h2) / var(--lh-h2) var(--font-display);
    text-transform: uppercase;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .sites {
    flex: 1 1 200px;
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  .status,
  .summary {
    margin: 0;
  }

  .summary small,
  .dim {
    color: var(--text-dim);
  }

  .warn {
    color: var(--danger-ink);
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
  }

  .groups {
    flex: 1 1 auto;
    min-block-size: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    margin: 0;
    padding: 2px;
    list-style: none;
  }

  .group {
    --sw: 1.5px;
    padding: var(--sp-3) var(--sp-4);
  }

  .group-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
    margin: 0 0 var(--sp-2);
  }

  .badge {
    padding: 0 8px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--bg);
    font: 700 12px/20px var(--font-mono);
  }

  .badge.similar {
    background: var(--secondary);
  }

  .group ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .file {
    display: grid;
    grid-template-columns: auto minmax(120px, 1fr) minmax(120px, 1.4fr) auto auto auto auto;
    align-items: center;
    gap: var(--sp-3);
    padding-block: 2px;
  }

  .name,
  .folder {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder {
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }

  .none {
    color: var(--text-dim);
  }

  footer {
    justify-content: flex-end;
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

  .btn.danger {
    --k: var(--danger-ink);
    color: var(--danger-ink);
  }
</style>
