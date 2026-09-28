<script lang="ts">
  /**
   * Lot 8.2: once the meaning module is installed, "Which sites to
   * understand?" — every site with its estimate measured on this PC; the
   * ticked ones are computed in the background.
   */
  import { formatNumber, t } from '../i18n/index.svelte';
  import { formatDuration, sense } from '../stores/sense.svelte';
  import { sitesStore } from '../stores/sites.svelte';
  import type { SenseEstimateDto } from '../api';

  let chosen = $state<Record<string, boolean>>({});
  let estimates = $state<Record<string, SenseEstimateDto | null | undefined>>({});
  let starting = $state(false);

  // Estimates one site after the other (each reads its site's text).
  $effect(() => {
    if (!sense.askSites) return;
    let stop = false;
    void (async () => {
      for (const site of sitesStore.list) {
        if (stop) return;
        if (estimates[site.id] === undefined && site.lastIndexed) {
          const estimate = await sense.estimate(site.id);
          if (stop) return;
          estimates[site.id] = estimate;
          // Ticked for the user when it takes an hour or less; the long ones are a choice.
          if (!(site.id in chosen)) chosen[site.id] = !site.sense && estimate?.seconds != null && estimate.seconds <= 3600;
        }
      }
    })();
    return () => {
      stop = true;
    };
  });

  const anyChosen = $derived(sitesStore.list.some((s) => chosen[s.id] && !s.sense));

  /** Seconds for the ticked sites (null while one is not measured yet). */
  const total = $derived.by(() => {
    let seconds = 0;
    for (const site of sitesStore.list) {
      if (!chosen[site.id] || site.sense) continue;
      const e = estimates[site.id];
      if (!e || e.seconds === null) return null;
      seconds += e.seconds;
    }
    return seconds;
  });

  async function start() {
    starting = true;
    for (const site of sitesStore.list) {
      if (chosen[site.id] && !site.sense) await sitesStore.setSense(site.id, true);
    }
    starting = false;
    sense.askSites = false;
  }
</script>

{#if sense.askSites}
  <div class="shade" role="presentation" onclick={() => (sense.askSites = false)}></div>
  <div class="dialog sketch" role="dialog" aria-modal="true" aria-labelledby="sense-sites-title">
    <h2 id="sense-sites-title">{t('sense.sitesTitle')}</h2>
    <p>{t('sense.sitesIntro')}</p>
    <ul>
      {#each sitesStore.list as site (site.id)}
        {@const e = estimates[site.id]}
        <li>
          <label>
            <input type="checkbox" checked={chosen[site.id] ?? false} onchange={(e) => (chosen[site.id] = e.currentTarget.checked)} disabled={site.sense || !site.lastIndexed} />
            <span class="name">{site.name}<span class="docs">{t('rail.docs', { count: site.docCount })}</span></span>
          </label>
          <span class="time">
            {#if site.sense}{t('sense.already')}{:else if !site.lastIndexed}—{:else if e === undefined}…{:else if e}≈ {formatDuration(e.seconds)}<span class="passages">{t('sense.passages', { count: formatNumber(e.passages) })}</span>{/if}
          </span>
        </li>
      {/each}
    </ul>
    <p class="total">{!anyChosen ? t('sense.noneChosen') : total === null ? t('sense.measuring') : t('sense.total', { time: formatDuration(total) })}</p>
    <p>{t('sense.sitesHint')}</p>
    <div class="row">
      <button type="button" class="sketch" onclick={() => (sense.askSites = false)}>{t('sense.later')}</button>
      <button type="button" class="sketch hatch go" onclick={start} disabled={starting || !anyChosen}>{t('sense.start')}</button>
    </div>
  </div>
{/if}

<style>
  .shade {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgb(0 0 0 / 0.35);
  }

  .dialog {
    --k: var(--secondary);
    position: fixed;
    z-index: 80;
    inset-block-start: 12vh;
    inset-inline-start: 50%;
    translate: -50% 0;
    display: grid;
    gap: 12px;
    inline-size: min(560px, 92vw);
    max-block-size: 76vh;
    overflow: auto;
    padding: 20px 24px;
    background: var(--surface);
    box-shadow: var(--shadow-pop);
  }

  :global([dir='rtl']) .dialog {
    translate: 50% 0;
  }

  .dialog > * {
    position: relative;
    z-index: 1;
    margin: 0;
  }

  h2 {
    font: var(--fs-h2, 22px) / 1.2 var(--font-brand);
    text-transform: uppercase;
  }

  p {
    color: var(--text-dim);
    font-size: 14px;
    line-height: 1.5;
  }

  ul {
    display: grid;
    gap: 6px;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--secondary) 7%, transparent);
  }

  label {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .name {
    display: grid;
  }

  .docs,
  .passages {
    color: var(--text-dim);
    font-size: 13px;
  }

  .time {
    display: grid;
    justify-items: end;
    color: var(--secondary-ink);
    font-weight: 700;
    text-align: end;
  }

  .passages {
    font-weight: 400;
  }

  .total {
    color: var(--text);
    font-size: 15px;
    font-weight: 700;
  }

  .row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  button {
    padding: 6px 12px;
    background: transparent;
    border: 0;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }

  .go {
    --h: var(--fill-secondary);
    font-weight: 700;
  }
</style>
