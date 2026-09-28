<script lang="ts">
  /** "Prospector x.y.z is available": shown once a newer version is found. */
  import { formatPercent, t } from '../i18n/index.svelte';
  import { updates } from '../stores/updates.svelte';
</script>

{#if !updates.dismissed && (updates.state === 'available' || updates.state === 'downloading')}
  <aside class="update sketch" role="status" aria-live="polite">
    <p>
      <strong>{t('updates.available', { version: updates.version ?? '' })}</strong>
      {#if updates.portable}
        <span class="dim">{t('updates.portableHint')}</span>
      {/if}
      {#if updates.state === 'downloading'}
        <span class="dim">{updates.progress === null ? t('updates.downloading') : t('updates.downloadingPercent', { percent: formatPercent(updates.progress) })}</span>
      {/if}
    </p>
    <div class="actions">
      {#if updates.state === 'available'}
        <button type="button" class="later" onclick={() => (updates.dismissed = true)}>{t('updates.later')}</button>
      {/if}
      <button type="button" class="go sketch hatch" disabled={updates.state === 'downloading'} onclick={() => updates.install()}>
        {updates.portable ? t('updates.download') : t('updates.install')}
      </button>
    </div>
  </aside>
{/if}

<style>
  .update {
    --k: var(--ok);
    position: fixed;
    inset-block-end: var(--sp-5);
    inset-inline-end: var(--sp-6);
    z-index: 55;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    inline-size: min(380px, calc(100vw - 2 * var(--sp-5)));
    padding: var(--sp-3) var(--sp-4);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  p {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
  }

  .dim {
    color: var(--text-dim);
    font-size: var(--fs-caption);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }

  button {
    padding: var(--sp-1) var(--sp-3);
    background: transparent;
    border: 0;
  }

  .go {
    --k: var(--ok);
    --h: var(--fill-ok);
    font-family: var(--font-display);
    text-transform: uppercase;
  }

  .later:hover {
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 4px;
  }
</style>
