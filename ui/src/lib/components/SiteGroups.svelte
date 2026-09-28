<script lang="ts">
  /**
   * Site groups (lot 7.2), at the top of the dig sites panel: "All" and one
   * chip per group (its pencil, its name, its Ctrl digit). A click ticks
   * exactly its sites; a right-click renames it, gives it the ticked sites
   * or removes it; "+ Group" saves the ticked sites under a name.
   */
  import { t } from '../i18n/index.svelte';
  import { groupColor, groups } from '../stores/groups.svelte';
  import { search } from '../stores/search.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import type { SiteGroupDto } from '../api';

  let { compact = false }: { compact?: boolean } = $props();

  /** The name editor: a new group, or renaming one. */
  let editing = $state<{ id: string | null } | null>(null);
  let name = $state('');
  let menuFor = $state<string | null>(null);
  let input: HTMLInputElement | undefined = $state();

  const active = $derived(groups.active);
  const ticked = $derived(search.scope.size);

  function sitesOf(group: SiteGroupDto): string {
    return group.siteIds.map((id) => sitesStore.list.find((s) => s.id === id)?.name).filter(Boolean).join(', ');
  }

  function shortcut(index: number): string {
    return index <= 9 ? t('groups.shortcut', { n: index }) : '';
  }

  function startNew() {
    menuFor = null;
    editing = { id: null };
    name = '';
  }

  function startRename(group: SiteGroupDto) {
    menuFor = null;
    editing = { id: group.id };
    name = group.name;
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const value = name.trim();
    if (!editing || !value) return;
    if (editing.id) await groups.update(editing.id, { name: value });
    else await groups.create(value);
    editing = null;
  }

  function onMenu(event: MouseEvent, group: SiteGroupDto) {
    event.preventDefault();
    editing = null;
    menuFor = menuFor === group.id ? null : group.id;
  }

  $effect(() => {
    if (editing) input?.focus();
  });

  function onWindowKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      editing = null;
      menuFor = null;
    }
  }
</script>

<svelte:window onkeydown={onWindowKey} />

{#if sitesStore.list.length > 1 || groups.list.length > 0}
  <div class="groups" class:compact>
    {#if !compact}
      <div class="g-head">
        <span class="g-title">{t('groups.title')}</span>
        <span class="g-hint">{t('groups.hint')}</span>
      </div>
    {/if}
    <div class="g-chips" role="group" aria-label={t('groups.title')}>
      <button
        type="button"
        class="g-chip sketch"
        class:hatch={active === 'all'}
        class:active={active === 'all'}
        style:--k="var(--ok)"
        style:--h="color-mix(in srgb, var(--ok) 35%, transparent)"
        aria-pressed={active === 'all'}
        title="{t('groups.all')} — {shortcut(0)}"
        onclick={() => groups.apply(null)}
      >
        {#if !compact}<span>{t('groups.all')}</span>{/if}<kbd>0</kbd>
      </button>
      {#each groups.list as group, i (group.id)}
        {@const on = active === group.id}
        <button
          type="button"
          class="g-chip sketch"
          class:hatch={on}
          class:active={on}
          style:--k={groupColor(group)}
          style:--h="color-mix(in srgb, {groupColor(group)} 35%, transparent)"
          aria-pressed={on}
          aria-haspopup="menu"
          title="{group.name}{i < 9 ? ` — ${shortcut(i + 1)}` : ''}&#10;{sitesOf(group)}"
          onclick={() => groups.apply(group)}
          oncontextmenu={(e) => onMenu(e, group)}
        >
          {#if compact}
            <kbd>{i < 9 ? i + 1 : '·'}</kbd>
          {:else}
            <span class="dot" aria-hidden="true"></span><span class="g-name">{group.name}</span>{#if i < 9}<kbd>{i + 1}</kbd>{/if}
          {/if}
        </button>
      {/each}
      {#if !compact}
        <button type="button" class="g-add sketch dashed" onclick={startNew} disabled={ticked === 0} title={t('groups.addHint')}>
          <b aria-hidden="true">+</b><span>{t('groups.add')}</span>
        </button>
      {/if}
    </div>

    {#if menuFor && !compact}
      {@const group = groups.list.find((g) => g.id === menuFor)}
      {#if group}
        <div class="g-menu sketch" role="menu" aria-label={group.name}>
          <button type="button" role="menuitem" onclick={() => startRename(group)}>{t('groups.rename')}</button>
          <button
            type="button"
            role="menuitem"
            disabled={ticked === 0}
            onclick={() => {
              menuFor = null;
              void groups.update(group.id, { ticked: true });
            }}>{t('groups.replace', { count: ticked })}</button
          >
          <button
            type="button"
            role="menuitem"
            class="danger"
            onclick={() => {
              menuFor = null;
              void groups.remove(group.id);
            }}>{t('groups.remove')}</button
          >
        </div>
      {/if}
    {/if}

    {#if editing && !compact}
      <form class="g-editor sketch" onsubmit={submit}>
        <input bind:this={input} bind:value={name} maxlength="40" aria-label={t('groups.name')} placeholder={t('groups.name')} />
        {#if !editing.id}<span class="g-keep">{t('groups.keep', { count: ticked })}</span>{/if}
        <div class="g-row">
          <button type="button" onclick={() => (editing = null)}>{t('groups.cancel')}</button>
          <button type="submit" class="sketch hatch save" disabled={!name.trim()}>{t('groups.save')}</button>
        </div>
      </form>
    {/if}
  </div>
{/if}

<style>
  .groups {
    position: relative;
    z-index: 1;
    margin-block: 2px var(--sp-3);
  }

  .g-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 0 2px 6px;
  }

  .g-title {
    font-size: var(--fs-caption);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    font-weight: 700;
  }

  .g-hint {
    font-size: 12px;
    color: var(--text-faint);
  }

  .g-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }

  .compact .g-chips {
    flex-direction: column;
    align-items: center;
  }

  .g-chip,
  .g-add {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-inline-size: 100%;
    min-block-size: 32px;
    padding: 5px 8px 5px 11px;
    border: 0;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 15px;
    cursor: pointer;
  }

  .compact .g-chip {
    padding: 5px;
  }

  .g-chip > *,
  .g-add > * {
    position: relative;
    z-index: 1;
  }

  .g-chip.active {
    --sw: 2.4px;
    font-weight: 700;
  }

  .g-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dot {
    flex: none;
    inline-size: 9px;
    block-size: 9px;
    border-radius: 50%;
    background: var(--k);
  }

  kbd {
    flex: none;
    min-inline-size: 17px;
    padding: 3px 4px;
    border-radius: 5px;
    background: color-mix(in srgb, var(--k) 22%, transparent);
    color: var(--text);
    font: 700 11px/1 var(--font-mono, monospace);
    text-align: center;
  }

  .g-add {
    padding-inline: 11px;
    color: var(--text-dim);
  }

  .g-add b {
    font-size: 17px;
    line-height: 1;
  }

  .g-add:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .g-menu,
  .g-editor {
    margin-block-start: 10px;
    padding: var(--sp-2);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .g-menu {
    display: grid;
  }

  .g-menu > *,
  .g-editor > * {
    position: relative;
    z-index: 1;
  }

  .g-menu button {
    padding: 6px 10px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    text-align: start;
    cursor: pointer;
  }

  .g-menu button:hover:not(:disabled),
  .g-menu button:focus-visible {
    background: var(--fill-secondary);
  }

  .g-menu button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .g-menu .danger {
    color: var(--danger-ink);
  }

  .g-editor {
    display: grid;
    gap: var(--sp-2);
  }

  .g-editor input {
    padding: 7px 10px;
    border: 1.5px solid var(--line);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 16px;
  }

  .g-keep {
    color: var(--text-dim);
    font-size: var(--fs-caption);
  }

  .g-row {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }

  .g-row button {
    padding: 6px 12px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }

  .g-row .save {
    --k: var(--ok);
    --h: var(--fill-ok);
  }

  .g-row .save:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
