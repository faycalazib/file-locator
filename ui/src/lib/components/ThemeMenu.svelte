<script lang="ts">
  import Icon from './Icon.svelte';
  import { dismissable } from '../actions';
  import { t } from '../i18n/index.svelte';
  import { ui, type Theme } from '../stores/ui.svelte';

  /**
   * The themes, each with a small swatch of its colors (rail footer), in two
   * columns: light, then dark. Every theme added to `THEMES`
   * (stores/ui.svelte.ts) shows here by itself.
   */
  let open = $state(false);
  const groups = $derived([
    { chrome: 'light' as const, label: t('settings.themesLight'), themes: ui.themes.filter((th) => th.chrome === 'light') },
    { chrome: 'dark' as const, label: t('settings.themesDark'), themes: ui.themes.filter((th) => th.chrome === 'dark') },
  ]);

  function choose(id: Theme) {
    ui.setTheme(id);
    open = false;
  }
</script>

<div class="themes" use:dismissable={() => (open = false)}>
  <button
    type="button"
    class="trigger sketch round"
    aria-haspopup="true"
    aria-expanded={open}
    aria-label={t('settings.theme')}
    title={t('settings.theme')}
    onclick={() => (open = !open)}
  >
    <Icon name="palette" />
  </button>

  {#if open}
    <div class="menu sketch" role="group" aria-label={t('settings.theme')}>
      {#each groups as group (group.chrome)}
        <section>
          <h3><Icon name={group.chrome === 'dark' ? 'moon' : 'sun'} size={13} /> {group.label}</h3>
          <ul aria-label={group.label}>
            {#each group.themes as theme (theme.id)}
              <li>
                <button type="button" aria-current={theme.id === ui.theme ? 'true' : undefined} onclick={() => choose(theme.id)}>
                  <!-- A swatch of the theme: page, panel, then its two main colors. -->
                  <span class="swatch" aria-hidden="true" style:--s0={theme.swatch[0]} style:--s1={theme.swatch[1]} style:--s2={theme.swatch[2]} style:--s3={theme.swatch[3]}>
                    <i></i><i></i><i></i>
                  </span>
                  <span class="name">{t(`settings.themeNames.${theme.id}`)}</span>
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</div>

<style>
  .themes {
    position: relative;
  }

  .trigger {
    --sw: 2px;
    display: grid;
    place-items: center;
    inline-size: 38px;
    block-size: 38px;
    padding: 0;
    background: var(--surface);
    border: 0;
    border-radius: 50%;
  }

  .trigger:hover,
  .trigger[aria-expanded='true'] {
    --k: var(--secondary);
  }

  /* Opens above the rail footer, towards the page. */
  .menu {
    position: absolute;
    inset-block-end: calc(100% + var(--sp-2));
    inset-inline-start: 0;
    z-index: 20;
    display: grid;
    grid-template-columns: repeat(2, minmax(220px, 1fr));
    gap: var(--sp-2);
    /* More themes will come: the list scrolls rather than leaving the window. */
    max-block-size: 72vh;
    overflow-y: auto;
    margin: 0;
    padding: var(--sp-2);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  .menu ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .menu h3 {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    margin: var(--sp-1) var(--sp-3) var(--sp-1);
    font: 700 var(--fs-caption) / var(--lh-caption) var(--font-body);
    color: var(--text-dim);
    text-transform: var(--display-case, uppercase);
    letter-spacing: 0.04em;
  }

  .menu button {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    inline-size: 100%;
    padding: 6px var(--sp-3);
    background: transparent;
    border: 0;
    border-radius: 12px;
    text-align: start;
    font: var(--fs-body) / var(--lh-body) var(--font-body);
    color: var(--text);
  }

  .menu button:hover {
    background: var(--fill-yellow);
  }

  .menu button[aria-current='true'] {
    color: var(--secondary-ink);
    font-weight: 700;
  }

  .name {
    flex: 1;
  }

  /* Page color with a panel, and two color dots on it. */
  .swatch {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    inline-size: 46px;
    block-size: 30px;
    padding: 0 6px;
    border-radius: 9px;
    background: linear-gradient(135deg, var(--s0) 0 55%, var(--s1) 55%);
    box-shadow: inset 0 0 0 1.5px rgb(0 0 0 / 0.18);
    flex: none;
  }

  .swatch i {
    inline-size: 10px;
    block-size: 10px;
    border-radius: 50%;
    background: var(--s2);
    box-shadow: 0 0 0 1.5px rgb(255 255 255 / 0.7);
  }

  .swatch i:nth-child(2) {
    background: var(--s3);
  }

  .swatch i:nth-child(3) {
    background: var(--s1);
    box-shadow: inset 0 0 0 1.5px rgb(0 0 0 / 0.2);
  }

  .menu button[aria-current='true'] .swatch {
    box-shadow: inset 0 0 0 1.5px rgb(0 0 0 / 0.18), 0 0 0 2.5px var(--secondary);
  }
</style>
