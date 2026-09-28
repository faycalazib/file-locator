<script lang="ts">
  /** Messages at the bottom of the screen: errors (translated codes) and confirmations. */
  import Icon from './Icon.svelte';
  import { t } from '../i18n/index.svelte';
  import { notices } from '../stores/notices.svelte';
</script>

<div class="notices" role="status" aria-live="polite">
  {#each notices.items as n (n.id)}
    <div class="notice sketch" class:error={n.kind === 'error'}>
      <span>{n.text}</span>
      <button type="button" aria-label={t('errors.dismiss')} title={t('errors.dismiss')} onclick={() => notices.dismiss(n.id)}>
        <Icon name="close" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .notices {
    position: fixed;
    inset-block-end: var(--sp-5);
    inset-inline-start: 50%;
    translate: -50% 0;
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    inline-size: min(560px, calc(100vw - 2 * var(--sp-5)));
    pointer-events: none;
  }

  :global([dir='rtl']) .notices {
    translate: 50% 0;
  }

  .notice {
    --k: var(--secondary);
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding-block: var(--sp-2);
    padding-inline: var(--sp-4) var(--sp-3);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    pointer-events: auto;
  }

  .notice.error {
    --k: var(--danger);
    color: var(--danger-ink);
  }

  .notice span {
    flex: 1 1 auto;
    overflow-wrap: anywhere;
  }

  .notice button {
    position: relative;
    display: grid;
    place-items: center;
    inline-size: 26px;
    block-size: 26px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text-dim);
  }
</style>
