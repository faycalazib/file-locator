<script lang="ts">
  import { t } from '../i18n/index.svelte';
  import { segments } from '../text';
  import type { MarkedText } from '../types';

  let { text }: { text: MarkedText } = $props();
</script>

{#each segments(text) as seg, i (i)}{#if seg.hit}<mark class:fuzzy={seg.fuzzy} title={seg.fuzzy ? t('results.approximate') : undefined}>{seg.text}</mark>{:else}{seg.text}{/if}{/each}

<style>
  /* The theme decides what a match looks like (highlighter hatching in "crayons"). */
  mark {
    background: var(--mark-bg);
    color: var(--mark-text);
    font-weight: 700;
    padding-inline: 3px;
  }

  /* Approximate (typo tolerance): no highlighter, a wavy underline only. */
  mark.fuzzy {
    background: none;
    color: inherit;
    font-weight: 400;
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 3px;
  }
</style>
