<script lang="ts">
  import Icon from './Icon.svelte';
  import { formatNumber, t } from '../i18n/index.svelte';
  import { tabs } from '../stores/tabs.svelte';

  /** Tab title: the words, else the file name criterion, else the opened file. */
  function title(view: ReturnType<typeof tabs.view>): string {
    return view.query || view.names || view.openedFrom || (view.copiesOf ? t('filters.copiesToken', { name: view.copiesOf }) : '') || t('tabs.untitled');
  }

  function onAuxClick(event: MouseEvent, id: string) {
    // Middle click closes, as in a browser.
    if (event.button === 1) {
      event.preventDefault();
      tabs.close(id);
    }
  }
</script>

<div class="strip" role="tablist" aria-label={t('tabs.label')}>
  {#each tabs.list as tab (tab.id)}
    {@const view = tabs.view(tab)}
    {@const active = tab.id === tabs.activeId}
    <div class="tab sketch" class:active class:hatch={active}>
      <button
        type="button"
        class="pick"
        role="tab"
        aria-selected={active}
        title={title(view)}
        onclick={() => tabs.select(tab.id)}
        onauxclick={(e) => onAuxClick(e, tab.id)}
      >
        {#if view.openedFrom}<Icon name="open" size={13} />{/if}
        <span class="label" dir="auto">{title(view)}</span>
        {#if view.busy}
          <span class="busy" aria-label={t('tabs.running')}></span>
        {:else if view.hasRun}
          <span class="count mono">{formatNumber(view.count)}</span>
        {/if}
      </button>
      <button type="button" class="close" aria-label={t('tabs.close')} title={`${t('tabs.close')} (Ctrl+W)`} onclick={() => tabs.close(tab.id)}>
        <Icon name="close" size={11} />
      </button>
    </div>
  {/each}
  <button
    type="button"
    class="add"
    aria-label={t('tabs.new')}
    title={`${t('tabs.new')} (Ctrl+T)`}
    disabled={!tabs.canAdd}
    onclick={() => tabs.add()}
  >
    <Icon name="plus" size={14} />
  </button>
</div>

<style>
  .strip {
    display: flex;
    align-items: flex-end;
    gap: var(--sp-1);
    flex: none;
    min-inline-size: 0;
    margin-block-end: var(--sp-2);
    padding-inline: 6px;
    overflow-x: auto;
    scrollbar-width: none;
    border-block-end: 2px solid var(--text-faint);
  }

  /* Notebook dividers: the pencil outline stays open at the bottom. */
  .tab {
    --sw: 2px;
    --k: var(--text-faint);
    --h: var(--fill-yellow);
    display: flex;
    align-items: center;
    flex: 0 1 auto;
    min-inline-size: 72px;
    max-inline-size: 220px;
    margin-block-end: -2px;
    color: var(--text-dim);
  }

  .tab::before {
    border-end-start-radius: 0 !important;
    border-end-end-radius: 0 !important;
    border-block-end-width: 0 !important;
  }

  .tab:hover {
    --k: var(--text-dim);
  }

  .tab.active {
    --k: var(--secondary);
    color: var(--text);
    background-color: var(--bg);
  }

  .pick {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    min-inline-size: 0;
    padding: 3px 4px 3px var(--sp-3);
    background: transparent;
    border: 0;
    font: 15px/22px var(--font-display);
    text-transform: uppercase;
    color: inherit;
    cursor: pointer;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    unicode-bidi: plaintext;
  }

  .count {
    flex: none;
    font-size: 12px;
    color: var(--accent-ink);
  }

  /* Live scan still running in this tab: a pencil dot that breathes. */
  .busy {
    flex: none;
    inline-size: 7px;
    block-size: 7px;
    border-radius: 50%;
    background: var(--secondary);
    animation: breathe 1.2s ease-in-out infinite;
  }

  @keyframes breathe {
    50% {
      opacity: 0.25;
    }
  }

  .close,
  .add {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--text-dim);
  }

  .close {
    inline-size: 22px;
    block-size: 22px;
    margin-inline-end: 4px;
    opacity: 0;
  }

  .tab.active .close,
  .tab:hover .close,
  .close:focus-visible {
    opacity: 1;
  }

  .close:hover,
  .add:hover:not(:disabled) {
    color: var(--accent-ink);
  }

  .add {
    inline-size: 28px;
    block-size: 28px;
    margin-block-end: 1px;
  }

  .add:disabled {
    opacity: 0.35;
  }

  @media (prefers-reduced-motion: reduce) {
    .busy {
      animation: none;
    }
  }
</style>
