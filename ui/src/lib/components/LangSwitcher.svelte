<script lang="ts">
  import Icon from './Icon.svelte';
  import { dismissable } from '../actions';
  import { i18n, LOCALES, t, type Locale } from '../i18n/index.svelte';

  /** `up`: the menu opens above (rail footer). `compact`: code only, no language name. */
  let { up = false, compact = false }: { up?: boolean; compact?: boolean } = $props();

  let open = $state(false);
  const current = $derived(LOCALES.find((l) => l.code === i18n.locale));

  function choose(code: Locale) {
    i18n.set(code);
    open = false;
  }
</script>

<div class="lang" use:dismissable={() => (open = false)}>
  <button
    type="button"
    class="trigger sketch"
    aria-haspopup="true"
    aria-expanded={open}
    aria-label={t('settings.language')}
    title={t('settings.language')}
    onclick={() => (open = !open)}
  >
    <Icon name="globe" />
    {#if compact}
      <span class="code mono">{i18n.locale.toUpperCase()}</span>
    {:else}
      <span class="current">{current?.native}</span>
    {/if}
  </button>

  {#if open}
    <ul class="menu sketch" class:up aria-label={t('settings.language')}>
      {#each LOCALES as l (l.code)}
        <li>
          <button
            type="button"
            lang={l.code}
            dir={l.dir}
            aria-current={l.code === i18n.locale ? 'true' : undefined}
            onclick={() => choose(l.code)}
          >
            <span class="native">{l.native}</span>
            <span class="mono code">{l.code}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .lang {
    position: relative;
  }

  .trigger {
    --sw: 2px;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    block-size: 38px;
    padding-inline: var(--sp-3);
    background: var(--surface);
    border: 0;
    border-radius: 999px;
    font: 17px var(--font-display);
    text-transform: uppercase;
  }

  .trigger::before {
    border-radius: 999px;
  }

  .trigger:hover,
  .trigger[aria-expanded='true'] {
    --k: var(--secondary);
  }

  .code {
    font-size: var(--fs-caption);
  }

  .menu {
    position: absolute;
    inset-block-start: calc(100% + var(--sp-2));
    inset-inline-start: 0;
    z-index: 20;
    min-inline-size: 190px;
    margin: 0;
    padding: var(--sp-2);
    list-style: none;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .menu.up {
    inset-block: auto calc(100% + var(--sp-2));
  }

  .menu button {
    position: relative;
    display: flex;
    justify-content: space-between;
    align-items: center;
    inline-size: 100%;
    padding: var(--sp-1) var(--sp-3);
    background: transparent;
    border: 0;
    text-align: start;
  }

  .menu button:hover .native {
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 4px;
  }

  .menu button[aria-current='true'] {
    color: var(--secondary-ink);
    font-weight: 700;
  }

  .code {
    color: var(--text-dim);
  }
</style>
