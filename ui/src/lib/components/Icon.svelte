<script lang="ts" module>
  /** 16px stroke icons (currentColor). Directional ones get `flip-rtl`. */
  const PATHS = {
    search: 'M11 11l3.5 3.5M7 12.5A5.5 5.5 0 1 0 7 1.5a5.5 5.5 0 0 0 0 11z',
    gear: 'M8 10.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zM8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.4 1.4M11.6 11.6L13 13M3 13l1.4-1.4M11.6 4.4L13 3',
    globe: 'M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14zM1 8h14M8 1c2 2 2.8 4.3 2.8 7S10 13 8 15M8 1C6 3 5.2 5.3 5.2 8S6 13 8 15',
    folder: 'M1.5 3.5h4l1.5 1.5h7.5v8h-13z',
    plus: 'M8 3v10M3 8h10',
    star: 'M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6z',
    clock: 'M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14zM8 4v4l2.5 1.5',
    stop: 'M4 4h8v8H4z',
    filter: 'M1.5 2.5h13l-5 6v5l-3-1.5v-3.5z',
    up: 'M3.5 10L8 5.5l4.5 4.5',
    down: 'M3.5 6L8 10.5 12.5 6',
    chevron: 'M6 3.5L10.5 8 6 12.5',
    back: 'M10 3.5L5.5 8 10 12.5',
    open: 'M1.5 13.5v-10h4l1.5 1.5h6v2M1.5 13.5l2.5-6h11l-2.5 6z',
    save: 'M2.5 2.5h9l2 2v9h-11zM5 2.5v3.5h5V2.5M5 13.5V9.5h6v4',
    external: 'M9 2.5h4.5V7M13.5 2.5L7 9M11.5 9.5v4h-9v-9h4',
    copy: 'M5.5 5.5h8v8h-8zM10.5 5.5v-3h-8v8h3',
    list: 'M6 4h7.5M6 8h7.5M6 12h7.5M2.5 4h.5M2.5 8h.5M2.5 12h.5',
    download: 'M8 2v8.5M4.5 7L8 10.5 11.5 7M2.5 13.5h11',
    close: 'M3.5 3.5l9 9M12.5 3.5l-9 9',
    alert: 'M8 1.5l6.5 12h-13zM8 6v3.5M8 11.5v.5',
    panel: 'M1.5 2.5h13v11h-13zM5.5 2.5v11',
    refresh: 'M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5V6H10',
    trash: 'M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 9h5.6l.7-9',
    code: 'M5.5 4L1.5 8l4 4M10.5 4l4 4-4 4M9 2.5l-2 11',
    twins: 'M2 3.5h5.5v9H2zM8.5 3.5H14v9H8.5zM3.5 6h2.5M3.5 8h2.5M10 6h2.5M10 8h2.5',
    bell: 'M4 11.5V7a4 4 0 0 1 8 0v4.5l1.5 1.5h-11zM6.5 13a1.5 1.5 0 0 0 3 0',
  } as const;

  export type IconName = keyof typeof PATHS;
  const DIRECTIONAL: ReadonlySet<IconName> = new Set<IconName>(['chevron', 'back', 'panel']);
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg
  class:flip-rtl={DIRECTIONAL.has(name)}
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.4"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  <path d={PATHS[name]} />
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
