<script lang="ts">
  import Icon from './Icon.svelte';
  import { dismissable } from '../actions';
  import { formatBytes, i18n, LOCALES, t, type MessageKey } from '../i18n/index.svelte';
  import { ui, type Theme } from '../stores/ui.svelte';
  import { api, inTauri, type AppInfoDto, type BackgroundDto, type OcrDto, type ShortcutDto } from '../api';
  import { formatShortcut, SHORTCUT_CHOICES } from '../shortcut';
  import { updates } from '../stores/updates.svelte';
  import { notices } from '../stores/notices.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import { integration } from '../stores/integration.svelte';
  import { sense } from '../stores/sense.svelte';
  import { isolate } from '../text';

  let info = $state<AppInfoDto | null>(null);
  let moving = $state(false);
  // Browser demo: OCR shown as available (screenshots).
  let ocr = $state<OcrDto | null>(inTauri ? null : { enabled: true, available: true, languages: ['fr-FR', 'en-US', 'ar-SA'] });

  /** `fr-FR, ar-SA` → "français, arabe" in the interface language. */
  const ocrLanguages = $derived.by(() => {
    if (!ocr) return '';
    let names: Intl.DisplayNames | null = null;
    try {
      names = new Intl.DisplayNames([i18n.locale], { type: 'language' });
    } catch {
      /* old engine: tags as they are */
    }
    return ocr.languages.map((tag) => names?.of(tag.split('-')[0] ?? tag) ?? tag).join(', ');
  });

  async function toggleOcr(enabled: boolean) {
    if (!inTauri) {
      if (ocr) ocr = { ...ocr, enabled };
      return;
    }
    try {
      ocr = await api.setOcr(enabled);
      if (enabled) notices.push(t('settings.ocrUpdating'));
    } catch (e) {
      notices.error(e);
    }
  }
  // Browser demo: a simulated shortcut, so the section can be seen.
  let shortcut = $state<ShortcutDto | null>(inTauri ? null : { shortcut: SHORTCUT_CHOICES[0], active: true });

  $effect(() => {
    if (ui.settingsOpen && inTauri && !info) {
      api.appInfo().then((i) => (info = i)).catch((e) => notices.error(e));
    }
    if (ui.settingsOpen && inTauri && !ocr) {
      api.getOcr().then((o) => (ocr = o)).catch((e) => notices.error(e));
    }
    if (ui.settingsOpen && inTauri && !sense.status) void sense.load();
    if (ui.settingsOpen && inTauri) void sense.loadPace();
    if (ui.settingsOpen && inTauri && !shortcut) {
      api.getShortcut().then((s) => (shortcut = s)).catch((e) => notices.error(e));
    }
  });

  /** The current one is listed even if it is not a preset (edited settings.json). */
  const shortcutChoices = $derived(
    shortcut && !(SHORTCUT_CHOICES as readonly string[]).includes(shortcut.shortcut)
      ? [shortcut.shortcut, ...SHORTCUT_CHOICES]
      : [...SHORTCUT_CHOICES],
  );

  async function changeShortcut(accelerator: string) {
    if (!inTauri) {
      shortcut = { shortcut: accelerator, active: true };
      return;
    }
    try {
      shortcut = await api.setShortcut(accelerator);
    } catch (e) {
      notices.error(e);
      shortcut = await api.getShortcut().catch(() => shortcut);
    }
  }

  // Windows integration (lot 5.7). The custom command is kept while typing
  // and saved when the field is left.
  let command = $state('');
  $effect(() => {
    command = integration.status?.editorCommand ?? '';
  });
  const firstEditor = $derived(integration.status?.editors[0]?.name ?? null);

  // Background (lot 6.1): read again at each opening (an alert changes the default).
  let background = $state<BackgroundDto | null>(inTauri ? null : { keepRunning: true, automatic: true, startWithWindows: false });
  $effect(() => {
    if (ui.settingsOpen && inTauri) api.getBackground().then((b) => (background = b)).catch((e) => notices.error(e));
  });

  async function setKeepRunning(keep: boolean) {
    if (!inTauri) {
      if (background) background = { ...background, keepRunning: keep, automatic: false };
      return;
    }
    background = await api.setBackground(keep).catch((e) => (notices.error(e), background));
  }

  async function setStartWithWindows(enabled: boolean) {
    if (!inTauri) {
      if (background) background = { ...background, startWithWindows: enabled };
      return;
    }
    try {
      background = await api.setStartWithWindows(enabled);
    } catch (e) {
      notices.error(e);
      background = await api.getBackground().catch(() => background);
    }
  }

  /** Picks a folder and moves catalog + indexes there. */
  async function moveDataDir() {
    const path = await api.pickFolder(t('settings.dataDirPick')).catch(() => null);
    if (!path) return;
    moving = true;
    try {
      info = await api.setDataDir(path);
      notices.push(t('settings.dataDirMoved', { path: info.dataDir }));
      await sitesStore.refresh();
    } catch (e) {
      notices.error(e);
    } finally {
      moving = false;
    }
  }
  import type { Variant } from '../types';

  /** `up`: the panel opens above (rail footer). */
  let { up = false }: { up?: boolean } = $props();

  const layoutName = (v: Variant) => `settings.layouts.${v}.name` as MessageKey;
  const layoutHint = (v: Variant) => `settings.layouts.${v}.hint` as MessageKey;
  const themeName = (th: Theme) => `settings.themes.${th}` as MessageKey;
</script>

<div class="settings" use:dismissable={() => (ui.settingsOpen = false)}>
  <button
    type="button"
    class="icon-btn sketch round"
    aria-haspopup="dialog"
    aria-expanded={ui.settingsOpen}
    aria-label={t('search.settings')}
    title={t('search.settings')}
    onclick={() => (ui.settingsOpen = !ui.settingsOpen)}
  >
    <Icon name="gear" />
  </button>

  {#if ui.settingsOpen}
    <div class="panel sketch" class:up role="dialog" aria-label={t('settings.title')}>
      <header>
        <h2>{t('settings.title')}</h2>
        <button type="button" class="icon-btn" aria-label={t('settings.close')} onclick={() => (ui.settingsOpen = false)}>
          <Icon name="close" />
        </button>
      </header>

      <fieldset>
        <legend>{t('settings.language')}</legend>
        {#each LOCALES as l (l.code)}
          <label class="option">
            <input
              type="radio"
              name="locale"
              value={l.code}
              checked={i18n.locale === l.code}
              onchange={() => i18n.set(l.code)}
            />
            <span lang={l.code} dir={l.dir}>{l.native}</span>
            {#if l.code !== i18n.locale}
              <span class="hint">{t(`languages.${l.code}`)}</span>
            {/if}
          </label>
        {/each}
      </fieldset>

      <fieldset>
        <legend>{t('settings.layout')}</legend>
        {#each ui.variants as v (v)}
          <label class="option layout">
            <input type="radio" name="layout" value={v} checked={ui.variant === v} onchange={() => ui.setVariant(v)} />
            <span class="stack">
              <span>{t(layoutName(v))}</span>
              <span class="hint">{t(layoutHint(v))}</span>
            </span>
          </label>
        {/each}
      </fieldset>

      {#if inTauri}
        {#if info?.portableDir}
          <!-- Lot 6.8: everything stays next to the program. -->
          <fieldset>
            <legend>{t('settings.portable')}</legend>
            <p class="path mono ltr-isolate" dir="ltr" title={info.portableDir}>{info.portableDir}</p>
            <p class="hint block">{t('settings.portableHint')}</p>
          </fieldset>
        {:else}
          <fieldset>
            <legend>{t('settings.dataDir')}</legend>
            <p class="path mono ltr-isolate" dir="ltr" title={info?.dataDir}>{info?.dataDir ?? '…'}</p>
            <p class="hint block">{t('settings.dataDirHint')}</p>
            <button type="button" class="move sketch" onclick={moveDataDir} disabled={moving}>
              {moving ? '…' : t('settings.dataDirChange')}
            </button>
          </fieldset>
        {/if}
      {/if}

      <fieldset>
          <legend>{t('settings.shortcut')}</legend>
          {#each shortcutChoices as accelerator (accelerator)}
            <label class="option">
              <input
                type="radio"
                name="shortcut"
                value={accelerator}
                checked={shortcut?.shortcut === accelerator}
                onchange={() => changeShortcut(accelerator)}
              />
              {#if accelerator}
                <kbd class="ltr-isolate" dir="ltr">{formatShortcut(accelerator)}</kbd>
              {:else}
                <span>{formatShortcut(accelerator)}</span>
              {/if}
            </label>
          {/each}
          {#if shortcut && shortcut.shortcut && !shortcut.active}
            <p class="hint block warn">{t('settings.shortcutTaken')}</p>
          {:else}
            <p class="hint block">{t('settings.shortcutHint')}</p>
          {/if}
        </fieldset>

      {#if integration.status}
        {@const status = integration.status}
        <fieldset>
          <legend>{t('integration.title')}</legend>
          {#if !status.portable}
            <label class="option">
              <input type="checkbox" checked={status.explorerMenu} onchange={(e) => integration.setExplorerMenu(e.currentTarget.checked, t('integration.menuLabel'))} />
              <span>{t('integration.menuOn')}</span>
            </label>
            <p class="hint block">{t('integration.menuHint')}</p>
          {/if}
          {#if status.cliAvailable}
            <label class="option">
              <input type="checkbox" checked={status.cliInPath} onchange={(e) => integration.setCliPath(e.currentTarget.checked)} />
              <span>{t('integration.cliOn')}</span>
            </label>
            <p class="hint block">{t('integration.cliHint')}</p>
          {/if}

          <p class="sub">{t('integration.editor')}</p>
          {#if firstEditor}
            <label class="option">
              <input type="radio" name="editor" checked={status.editor === ''} onchange={() => integration.setEditor('', command)} />
              <span>{t('integration.editorAuto', { name: isolate(firstEditor) })}</span>
            </label>
            {#each status.editors.slice(1) as editor (editor.id)}
              <label class="option">
                <input type="radio" name="editor" checked={status.editor === editor.id} onchange={() => integration.setEditor(editor.id, command)} />
                <span>{editor.name}</span>
              </label>
            {/each}
          {/if}
          <label class="option">
            <input type="radio" name="editor" checked={status.editor === 'custom' || (!firstEditor && status.editor === '')} onchange={() => integration.setEditor('custom', command)} />
            <span>{t('integration.editorCustom')}</span>
          </label>
          {#if status.editor === 'custom' || !firstEditor}
            <input
              class="command mono sketch"
              type="text"
              dir="ltr"
              spellcheck="false"
              aria-label={t('integration.editorCommand')}
              placeholder={'"C:\…\editor.exe" {file}:{line}'}
              bind:value={command}
              onchange={() => integration.setEditor('custom', command)}
            />
            <p class="hint block">{t('integration.editorCommandHint')}</p>
          {/if}
          {#if !firstEditor && !status.editorReady}
            <p class="hint block warn">{t('integration.editorNone')}</p>
          {/if}
        </fieldset>
      {/if}

      {#if background}
        <fieldset>
          <legend>{t('background.title')}</legend>
          <label class="option">
            <input type="checkbox" checked={background.keepRunning} onchange={(e) => setKeepRunning(e.currentTarget.checked)} />
            <span>{t('background.keep')}</span>
          </label>
          <p class="hint block">{background.automatic ? t('background.keepAuto') : t('background.keepHint')}</p>
          {#if !background.portable}
            <label class="option">
              <input type="checkbox" checked={background.startWithWindows} onchange={(e) => setStartWithWindows(e.currentTarget.checked)} />
              <span>{t('background.startup')}</span>
            </label>
            <p class="hint block">{t('background.startupHint')}</p>
          {/if}
        </fieldset>
      {/if}

      {#if ocr}
        <fieldset>
          <legend>{t('settings.ocr')}</legend>
          <label class="option">
            <input type="checkbox" checked={ocr.enabled} disabled={!ocr.available} onchange={(e) => toggleOcr(e.currentTarget.checked)} />
            <span>{t('settings.ocrOn')}</span>
          </label>
          {#if ocr.available}
            <p class="hint block">{t('settings.ocrHint', { languages: ocrLanguages })}</p>
          {:else}
            <p class="hint block warn">{t('settings.ocrUnavailable')}</p>
          {/if}
        </fieldset>
      {/if}

      <!-- Étape 8: meaning search, an optional local module. -->
      {#if sense.status}
        {@const status = sense.status}
        <fieldset class="sense">
          <legend>{t('sense.title')}</legend>
          <p class="hint block intro">{t('sense.intro')}</p>
          {#if sense.progress}
            <div class="bar" role="progressbar" aria-valuemin={0} aria-valuemax={sense.progress.total} aria-valuenow={sense.progress.done}>
              <span style:inline-size="{sense.progress.total ? Math.min(100, (100 * sense.progress.done) / sense.progress.total) : 0}%"></span>
            </div>
            <p class="hint block">{t('sense.downloading', { done: formatBytes(sense.progress.done), total: formatBytes(sense.progress.total) })}</p>
            <div class="row"><button type="button" class="move sketch" onclick={() => sense.cancel()}>{t('sense.cancel')}</button></div>
          {:else if status.installed}
            <p class="ok">✓ {t('sense.installed', { size: formatBytes(status.size) })}</p>
            <p class="hint block">{t('sense.next')}</p>
            <!-- Lot 8.2: how much of the PC the computation may use. -->
            <label class="option"><input type="radio" name="sense-pace" checked={sense.pace === 'normal'} onchange={() => sense.setPace('normal')} /><span>{t('sense.paceNormal')}</span></label>
            <label class="option"><input type="radio" name="sense-pace" checked={sense.pace === 'economy'} onchange={() => sense.setPace('economy')} /><span>{t('sense.paceEconomy')}</span></label>
            <div class="row"><button type="button" class="move sketch danger" onclick={() => sense.remove()}>{t('sense.remove')}</button></div>
          {:else}
            <div class="row">
              <button type="button" class="move sketch" disabled={!status.downloadable || sense.busy} onclick={() => sense.download()}>
                {t('sense.download', { size: formatBytes(status.downloadSize) })}
              </button>
              <button type="button" class="move sketch" disabled={sense.busy} onclick={() => sense.installFromFile()}>{t('sense.fromFile')}</button>
            </div>
            {#if !status.downloadable}<p class="hint block">{t('sense.notYet')}</p>{/if}
          {/if}
          <p class="hint block">{t('sense.privacy')}</p>
        </fieldset>
      {/if}

      {#if inTauri && updates.enabled}
        <fieldset>
          <legend>{t('updates.title')}</legend>
          <p class="hint block">{t('updates.current', { version: info?.version ?? '…' })}</p>
          <label class="option">
            <input type="checkbox" checked={updates.auto} onchange={(e) => updates.setAuto(e.currentTarget.checked)} />
            <span>{t('updates.auto')}</span>
          </label>
          <button type="button" class="move sketch" onclick={() => updates.check(true)} disabled={updates.state === 'checking' || updates.state === 'downloading'}>
            {updates.state === 'checking' ? t('updates.checking') : t('updates.check')}
          </button>
        </fieldset>
      {/if}

      <fieldset>
        <legend>{t('settings.theme')}</legend>
        {#each ui.themes as th (th.id)}
          <label class="option">
            <input type="radio" name="theme" value={th.id} checked={ui.theme === th.id} onchange={() => ui.setTheme(th.id)} />
            <span>{t(themeName(th.id))}</span>
          </label>
        {/each}
      </fieldset>
    </div>
  {/if}
</div>

<style>
  .settings {
    position: relative;
  }

  .icon-btn {
    --sw: 2px;
    display: grid;
    place-items: center;
    inline-size: 38px;
    block-size: 38px;
    padding: 0;
    background: var(--surface);
    border: 0;
  }

  .icon-btn:hover,
  .icon-btn[aria-expanded='true'] {
    --k: var(--secondary);
  }

  .panel {
    position: absolute;
    inset-block-start: calc(100% + var(--sp-2));
    inset-inline-start: 0;
    z-index: 20;
    inline-size: 320px;
    padding: var(--sp-3) var(--sp-5) var(--sp-4);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .panel.up {
    inset-block: auto calc(100% + var(--sp-2));
  }

  /* Many sections: the panel scrolls rather than leaving the window. */
  .panel {
    max-block-size: calc(100vh - 96px);
    overflow-y: auto;
  }

  kbd {
    font: 14px/20px var(--font-mono);
  }

  .hint.warn {
    color: var(--danger-ink);
  }

  header {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-block-end: var(--sp-1);
  }

  header .icon-btn {
    inline-size: 30px;
    block-size: 30px;
    background: transparent;
  }

  header .icon-btn::before {
    display: none;
  }

  h2 {
    margin: 0;
    font: var(--fs-title) / var(--lh-title) var(--font-brand);
    text-transform: uppercase;
  }

  fieldset {
    position: relative;
    margin: 0;
    padding: var(--sp-1) 0 var(--sp-2);
    border: 0;
    border-block-start: 2px dashed var(--text-faint);
  }

  legend {
    padding: var(--sp-2) 0 0;
    font: 17px/22px var(--font-display);
    text-transform: uppercase;
    color: var(--accent-ink);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: 2px 0;
    cursor: pointer;
  }

  .option input {
    accent-color: var(--secondary);
    margin: 0;
  }

  .option.layout {
    align-items: flex-start;
  }

  .option.layout input {
    margin-block-start: 6px;
  }

  .stack {
    display: flex;
    flex-direction: column;
  }

  .stack .hint {
    margin-inline-start: 0;
  }

  .path {
    margin: var(--sp-1) 0;
    overflow-wrap: anywhere;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
  }

  .hint.block {
    display: block;
    margin: 0 0 var(--sp-2);
  }

  /* A second heading inside a section (Windows integration → editor). */
  .sub {
    margin: var(--sp-2) 0 var(--sp-1);
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .command {
    --sw: 1.5px;
    display: block;
    inline-size: 100%;
    margin: var(--sp-1) 0 var(--sp-2);
    padding: var(--sp-1) var(--sp-2);
    background: var(--surface);
    border: 0;
    font-size: var(--fs-caption);
    color: var(--text);
  }

  .sense .intro {
    color: var(--text);
  }

  .sense .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    margin-block: 6px;
  }

  .sense .bar {
    position: relative;
    overflow: hidden;
    block-size: 12px;
    margin-block: 6px;
    border-radius: 6px;
    background: var(--fill-secondary);
  }

  .sense .bar > span {
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    background: var(--secondary);
  }

  .sense .ok {
    margin: 0 0 4px;
    color: var(--ok-ink);
    font-size: var(--fs-caption);
    font-weight: 700;
  }

  .move.danger {
    --k: var(--danger);
  }

  .move:disabled {
    opacity: 0.5;
  }

  .move {
    --sw: 2px;
    padding: 2px var(--sp-3);
    background: transparent;
    border: 0;
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
  }

  .hint {
    margin-inline-start: auto;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }
</style>
