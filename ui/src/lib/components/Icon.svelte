<script lang="ts" module>
  /** 16px stroke icons (currentColor). Directional ones get `flip-rtl`. */
  const PATHS = {
    search: 'M11 11l3.5 3.5M7 12.5A5.5 5.5 0 1 0 7 1.5a5.5 5.5 0 0 0 0 11z',
    cog: 'M6.09 3.17L6.6 1.14L9.4 1.14L9.91 3.17L10.07 3.23L11.86 2.16L13.84 4.14L12.77 5.93L12.83 6.09L14.86 6.6L14.86 9.4L12.83 9.91L12.77 10.07L13.84 11.86L11.86 13.84L10.07 12.77L9.91 12.83L9.4 14.86L6.6 14.86L6.09 12.83L5.93 12.77L4.14 13.84L2.16 11.86L3.23 10.07L3.17 9.91L1.14 9.4L1.14 6.6L3.17 6.09L3.23 5.93L2.16 4.14L4.14 2.16L5.93 3.23zM8 10.2a2.2 2.2 0 1 0 0-4.4 2.2 2.2 0 0 0 0 4.4z',
    sun: 'M8 10.8a2.8 2.8 0 1 0 0-5.6 2.8 2.8 0 0 0 0 5.6zM8 1v1.8M8 13.2V15M1 8h1.8M13.2 8H15M3.05 3.05l1.27 1.27M11.68 11.68l1.27 1.27M3.05 12.95l1.27-1.27M11.68 4.32l1.27-1.27',
    moon: 'M13.8 10.2A6 6 0 0 1 5.8 2.2a6 6 0 1 0 8 8z',
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
