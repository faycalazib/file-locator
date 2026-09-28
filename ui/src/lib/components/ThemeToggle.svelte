<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from '../i18n/index.svelte';
  import { ui } from '../stores/ui.svelte';

  // Two themes, one light and one dark: a click switches to the other one.
  const next = $derived(ui.themes.find((th) => th.id !== ui.theme) ?? { id: ui.theme, chrome: 'light' as const });
  const label = $derived(t(next.chrome === 'dark' ? 'settings.toDark' : 'settings.toLight'));
</script>

<button type="button" class="icon-btn sketch round" aria-label={label} title={label} onclick={() => ui.setTheme(next.id)}>
  <Icon name={next.chrome === 'dark' ? 'moon' : 'sun'} />
</button>

<style>
  .icon-btn {
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

  .icon-btn:hover {
    --k: var(--secondary);
  }
</style>
