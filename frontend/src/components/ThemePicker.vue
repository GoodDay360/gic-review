<script setup lang="ts">
/** Header dropdown: the 4 Catppuccin flavors + System, and the 14 accent colors. */
import { flavors, type AccentName, type FlavorName } from '@catppuccin/palette'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import { ACCENT_NAMES, FLAVOR_NAMES, useThemeStore, type FlavorChoice } from '@/stores/theme'

import AppIcon from './AppIcon.vue'

const theme = useThemeStore()
const open = ref(false)
const root = ref<HTMLElement | null>(null)

const options = computed(() => [
  { id: 'system' as FlavorChoice, label: 'System', hint: 'Latte / Mocha' },
  ...FLAVOR_NAMES.map((name) => ({
    id: name as FlavorChoice,
    label: flavors[name].name,
    hint: flavors[name].dark ? 'Dark' : 'Light',
  })),
])

// Swatches preview other flavors, so they read colors from the palette package.
function swatch(name: FlavorName) {
  const c = flavors[name].colors
  return [c.base.hex, c.mauve.hex, c.text.hex]
}

function choose(id: FlavorChoice) {
  theme.setFlavor(id)
}

function chooseAccent(name: AccentName) {
  theme.setAccent(name)
}

function onDocClick(e: MouseEvent) {
  if (open.value && root.value && !root.value.contains(e.target as Node)) open.value = false
}
function onKey(e: KeyboardEvent) {
  if (open.value && e.key === 'Escape') open.value = false
}
onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onKey)
})

const currentLabel = computed(() =>
  theme.choice === 'system' ? 'System' : flavors[theme.choice].name,
)
</script>

<template>
  <div ref="root" class="relative">
    <button
      class="btn-ghost px-2.5"
      :aria-expanded="open"
      aria-haspopup="true"
      :aria-label="`Theme: ${currentLabel}`"
      @click="open = !open"
    >
      <AppIcon name="palette" />
      <span class="hidden sm:inline">{{ currentLabel }}</span>
      <AppIcon name="chevron-down" :size="16" />
    </button>

    <div
      v-if="open"
      class="absolute right-0 z-30 mt-2 w-72 rounded-xl border border-ctp-surface0 bg-ctp-mantle p-2 shadow-xl"
    >
      <p class="px-2 pt-1 pb-2 text-xs font-semibold tracking-wide text-ctp-subtext0 uppercase">
        Flavor
      </p>
      <ul role="radiogroup" aria-label="Color flavor" class="space-y-0.5">
        <li v-for="opt in options" :key="opt.id">
          <button
            role="radio"
            :aria-checked="theme.choice === opt.id"
            class="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-left text-sm hover:bg-ctp-surface0"
            :class="theme.choice === opt.id ? 'bg-ctp-surface0' : ''"
            @click="choose(opt.id)"
          >
            <span
              v-if="opt.id === 'system'"
              class="flex h-6 w-10 items-center justify-center rounded-md border border-ctp-surface1 text-ctp-subtext1"
            >
              <AppIcon name="monitor" :size="16" />
            </span>
            <span
              v-else
              class="flex h-6 w-10 overflow-hidden rounded-md border border-ctp-surface1"
            >
              <span
                v-for="(hex, i) in swatch(opt.id as FlavorName)"
                :key="i"
                class="h-full flex-1"
                :style="{ backgroundColor: hex }"
              />
            </span>
            <span class="flex-1">
              <span class="block font-medium text-ctp-text">{{ opt.label }}</span>
              <span class="block text-xs text-ctp-subtext0">{{ opt.hint }}</span>
            </span>
            <AppIcon
              v-if="theme.choice === opt.id"
              name="check"
              :size="16"
              class="text-ctp-accent"
            />
          </button>
        </li>
      </ul>

      <p class="px-2 pt-3 pb-2 text-xs font-semibold tracking-wide text-ctp-subtext0 uppercase">
        Accent
      </p>
      <div role="radiogroup" aria-label="Accent color" class="grid grid-cols-7 gap-1.5 px-2 pb-2">
        <button
          v-for="name in ACCENT_NAMES"
          :key="name"
          role="radio"
          :aria-checked="theme.accent === name"
          :aria-label="name"
          :title="name"
          class="h-7 w-7 rounded-full ring-offset-2 ring-offset-ctp-mantle transition"
          :class="theme.accent === name ? 'ring-2 ring-ctp-text' : 'hover:scale-110'"
          :style="{ backgroundColor: `var(--ctp-${name})` }"
          @click="chooseAccent(name)"
        />
      </div>
    </div>
  </div>
</template>
