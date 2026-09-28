<script lang="ts">
  import { flushSync, onMount, untrack } from 'svelte';
  import ThemeDefs from './lib/components/ThemeDefs.svelte';
  import IndexRail from './lib/components/IndexRail.svelte';
  import SearchBar from './lib/components/SearchBar.svelte';
  import FilterPanel from './lib/components/FilterPanel.svelte';
  import ResultsList from './lib/components/ResultsList.svelte';
  import TabStrip from './lib/components/TabStrip.svelte';
  import { tabs } from './lib/stores/tabs.svelte';
  import { openResults, saveResults } from './lib/results-io';
  import PreviewPanel from './lib/components/PreviewPanel.svelte';
  import Notices from './lib/components/Notices.svelte';
  import { search } from './lib/stores/search.svelte';
  import { saved } from './lib/stores/saved.svelte';
  import { sitesStore } from './lib/stores/sites.svelte';
  import { groups } from './lib/stores/groups.svelte';
  import { ui } from './lib/stores/ui.svelte';
  import ReportView from './lib/components/ReportView.svelte';
  import UpdateBanner from './lib/components/UpdateBanner.svelte';
  import TransferDialog from './lib/components/TransferDialog.svelte';
  import DuplicatesDialog from './lib/components/DuplicatesDialog.svelte';
  import { updates } from './lib/stores/updates.svelte';
  import { api, inTauri } from './lib/api';
  import { focusSearch, initLaunches } from './lib/launch';
  import { integration } from './lib/stores/integration.svelte';
  import { t } from './lib/i18n/index.svelte';
  import { blossom, tulip } from './lib/doodles';

  onMount(() => {
    // Tauri: real dig sites, no search until the user types one.
    // Browser: the Étape 0 demo runs its mock search.
    if (inTauri) {
      // Explorer folder / `.prospector` file: once the sites are known.
      void sitesStore.init().then(() => initLaunches());
      void saved.init();
      void updates.init();
      // Global shortcut: the window comes up ready to type.
      void api.onSummon(focusSearch);
    } else search.run();
    // Ctrl+P prints the report too; it is unmounted once printing is over.
    const beforePrint = () => flushSync(() => (ui.printing = true));
    const afterPrint = () => {
      ui.printing = false;
      ui.reportKeywords = null;
    };
    window.addEventListener('beforeprint', beforePrint);
    window.addEventListener('afterprint', afterPrint);
    if (ui.printing) document.documentElement.classList.add('report-preview');
    return () => {
      search.stop();
      window.removeEventListener('beforeprint', beforePrint);
      window.removeEventListener('afterprint', afterPrint);
    };
  });

  // Explorer menu (lot 5.7): written again with this program and the label
  // in the interface language, at startup and when the language changes.
  $effect(() => {
    const label = t('integration.menuLabel');
    if (inTauri) void untrack(() => integration.sync(label));
  });

  // Tray menu and alert notifications (lot 6.1), in the interface language.
  $effect(() => {
    const texts = { trayOpen: t('background.trayOpen'), trayQuit: t('background.trayQuit'), alertTitle: t('alerts.notificationTitle') };
    if (inTauri) void api.syncShellTexts(texts).catch(() => {});
  });

  // Tauri: filters, options and scope are applied by the engine, so a change
  // re-runs the last search (debounced). Not when the settings come with the
  // search (a tab, the history, a saved search): run()/restore() record them.
  let rerun: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const signature = search.signature();
    if (!inTauri || untrack(() => !search.hasRun || search.ranSignature === signature)) return;
    clearTimeout(rerun);
    rerun = setTimeout(() => {
      if (search.signature() !== search.ranSignature) void search.run();
    }, 250);
  });

  /** Tabs, history and saved results from the keyboard (lot 5.6). */
  function onKeydown(event: KeyboardEvent) {
    const ctrl = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();
    let action: (() => void) | null = null;
    if (event.altKey && !ctrl && event.key === 'ArrowLeft') action = () => tabs.back();
    else if (event.altKey && !ctrl && event.key === 'ArrowRight') action = () => tabs.forward();
    else if (ctrl && event.key === 'Tab') action = () => tabs.cycle(event.shiftKey ? -1 : 1);
    else if (ctrl && !event.altKey && !event.shiftKey && key === 't') action = () => tabs.add();
    else if (ctrl && !event.altKey && !event.shiftKey && key === 'w') action = () => tabs.close(tabs.activeId);
    else if (ctrl && !event.altKey && !event.shiftKey && key === 's') action = () => void saveResults();
    else if (ctrl && !event.altKey && !event.shiftKey && key === 'o') action = () => void openResults();
    // Lot 7.2: Ctrl+0 every site, Ctrl+1…9 a site group (the key, whatever the keyboard layout).
    else if (ctrl && !event.altKey && !event.shiftKey && /^(Digit|Numpad)[0-9]$/.test(event.code)) {
      const n = Number(event.code.slice(-1));
      if (!groups.applyIndex(n)) return;
      event.preventDefault();
      return;
    }
    if (!action) return;
    event.preventDefault();
    action();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<ThemeDefs />

<!-- Masthead across the whole page, then rail | results | preview.
     Grid columns follow the inline direction: Arabic mirrors itself. -->
<div class="app v-{ui.variant}">
  <div class="top"><SearchBar /></div>

  <IndexRail compact={ui.variant === 'strata'} withFilters={ui.variant === 'ledger'} />

  <main class="center">
    <TabStrip />
    <ResultsList variant={ui.variant} />
    {#if ui.variant !== 'ledger' && ui.filtersOpen}
      <FilterPanel placement={ui.variant === 'strata' ? 'drawer' : 'sheet'} />
    {/if}
  </main>

  <PreviewPanel />

  <Notices />
  <UpdateBanner />
  <TransferDialog />
  <DuplicatesDialog />

  <img class="doodle tulip" src={tulip} alt="" aria-hidden="true" />
  <img class="doodle blossom" src={blossom} alt="" aria-hidden="true" />
</div>

{#if ui.printing && search.hasRun}
  <ReportView />
{/if}

<style>
  .app {
    position: relative;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--sp-4) var(--gutter);
    block-size: 100%;
    min-block-size: 0;
    padding: var(--sp-4) var(--sp-8) var(--sp-5);
  }

  .top {
    grid-column: 1 / -1;
  }

  .v-journal {
    grid-template-columns: var(--rail-w) minmax(0, 1fr) minmax(380px, 38%);
  }

  .v-ledger {
    grid-template-columns: calc(var(--rail-w) + 10px) minmax(0, 1.4fr) minmax(360px, 34%);
  }

  .v-strata {
    grid-template-columns: 64px minmax(0, 1fr) minmax(440px, 46%);
  }

  .center {
    position: relative;
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    min-block-size: 0;
  }

  /* Margin doodles: pure decoration, they never catch clicks. */
  .doodle {
    position: absolute;
    z-index: -1;
    filter: var(--wobble);
    pointer-events: none;
    user-select: none;
    opacity: var(--doodle-opacity);
  }

  .tulip {
    inset-block-end: var(--sp-4);
    inset-inline-start: 2px;
    inline-size: 34px;
  }

  .blossom {
    inset-block-end: 2px;
    inset-inline-end: 2px;
    inline-size: 38px;
  }
</style>
