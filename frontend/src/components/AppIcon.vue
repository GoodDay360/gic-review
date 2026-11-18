<script setup lang="ts">
// Small inline icon set (24x24, stroke-based) so we don't need an icon dependency.
const PATHS: Record<string, string> = {
  book: 'M12 6.5C10.5 5 8 4.5 4 4.5v13c4 0 6.5.5 8 2 1.5-1.5 4-2 8-2v-13c-4 0-6.5.5-8 2Zm0 0v13',
  'file-text': 'M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Zm0 0v6h6M8 13h8m-8 4h8',
  search: 'M10.8 18.1a7.3 7.3 0 1 0 0-14.6 7.3 7.3 0 0 0 0 14.6Zm5.3-2 4.4 4.4',
  cards: 'M7 4h11a2 2 0 0 1 2 2v11M4 8h11a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8Z',
  'list-check':
    'M10 6h10M10 12h10M10 18h10M3.5 6l1.5 1.5L7.5 5M3.5 12l1.5 1.5 2.5-2.5M3.5 18l1.5 1.5 2.5-2.5',
  close: 'M6 6l12 12M18 6 6 18',
  'arrow-left': 'M19 12H5m6-6-6 6 6 6',
  'chevron-left': 'm15 6-6 6 6 6',
  'chevron-right': 'm9 6 6 6-6 6',
  'chevron-down': 'm6 9 6 6 6-6',
  check: 'm5 12.5 4.5 4.5L19 7',
  x: 'M6 6l12 12M18 6 6 18',
  refresh: 'M4 12a8 8 0 0 1 14-5.3L20 9M20 4v5h-5M20 12a8 8 0 0 1-14 5.3L4 15m0 5v-5h5',
  upload: 'M12 16V4m0 0-4.5 4.5M12 4l4.5 4.5M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2',
  trash: 'M4 7h16M10 11v6m4-6v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3',
  eye: 'M2.5 12S6 5 12 5s9.5 7 9.5 7-3.5 7-9.5 7-9.5-7-9.5-7Zm9.5 3a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z',
  pencil: 'M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16v4Zm9.5-13.5 4 4',
  plus: 'M12 5v14M5 12h14',
  'arrow-up': 'M12 19V5m-6 6 6-6 6 6',
  'arrow-down': 'M12 5v14m6-6-6 6-6-6',
  logout: 'M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 16l-4-4 4-4M6 12h10',
  'zoom-in': 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm5-2 4 4M11 8v6M8 11h6',
  'zoom-out': 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm5-2 4 4M8 11h6',
  image:
    'M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6Zm0 10 4.5-4.5 3.5 3.5 2.5-2.5L20 18M15.5 9.5h.01',
  users:
    'M16 20v-1.5a3.5 3.5 0 0 0-3.5-3.5h-5A3.5 3.5 0 0 0 4 18.5V20M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Zm10 9v-1.5a3.5 3.5 0 0 0-2.5-3.4M15 4.2a3.5 3.5 0 0 1 0 6.6',
  folder: 'M3.5 7a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2V7Z',
  code: 'm9 8-4 4 4 4m6-8 4 4-4 4M13.5 5l-3 14',
  sparkles:
    'M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8L12 3Zm6 12 .8 2.2L21 18l-2.2.8L18 21l-.8-2.2L15 18l2.2-.8L18 15Z',
  palette:
    'M12 3a9 9 0 0 0 0 18c1 0 1.5-.7 1.5-1.5 0-.5-.2-.8-.4-1.1-.3-.3-.4-.7-.4-1.1 0-.8.7-1.5 1.5-1.5H16a5 5 0 0 0 5-5c0-4.1-4-7.8-9-7.8ZM7.5 12h.01M9.5 8h.01M14.5 8h.01',
  monitor: 'M4 5h16v11H4V5Zm4 15h8m-4-4v4',
  sun: 'M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm0-13v2m0 14v2M5.6 5.6 7 7m10 10 1.4 1.4M3 12h2m14 0h2M5.6 18.4 7 17M17 7l1.4-1.4',
  moon: 'M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5Z',
  import: 'M12 4v11m0 0-4-4m4 4 4-4M5 20h14',
  shuffle: 'M16 4h4v4M20 4l-7 7M4 20l6-6M16 20h4v-4M20 20l-5-5M4 4l5 5',
  info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm0-5v-4m0-4h.01',
  warning:
    'M12 9v4m0 4h.01M10.3 4.3 2.6 18a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 4.3a2 2 0 0 0-3.4 0Z',
  key: 'M15.5 3a5.5 5.5 0 0 0-5.3 7L3 17.2V21h3.8v-2.3h2.3v-2.3h2.3l1.6-1.6A5.5 5.5 0 1 0 15.5 3Zm1.5 4h.01',
  copy: 'M9 9h10v12H9V9Zm-4 6H4V3h11v1',
  calendar:
    'M5 5h14a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1Zm-1 5h16M8 3v4m8-4v4',
  user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-7 8a7 7 0 0 1 14 0',
  grip: 'M9 6h.01M15 6h.01M9 12h.01M15 12h.01M9 18h.01M15 18h.01',
}

withDefaults(defineProps<{ name: string; size?: number | string }>(), { size: 20 })
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    class="shrink-0"
  >
    <path :d="PATHS[name] ?? PATHS.info" />
  </svg>
</template>
