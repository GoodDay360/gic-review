<script setup lang="ts">
/** Header menu: the 4 Catppuccin flavors + System, and the 14 accent colors. */
import { flavors, type FlavorName } from '@catppuccin/palette'
import { computed } from 'vue'

import { ACCENT_NAMES, FLAVOR_NAMES, useThemeStore, type FlavorChoice } from '@/stores/theme'

const theme = useThemeStore()

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
  return [c.base.hex, c[theme.accent].hex, c.text.hex]
}

const currentLabel = computed(() =>
  theme.choice === 'system' ? 'System' : flavors[theme.choice].name,
)
</script>

<template>
  <v-menu location="bottom end" :close-on-content-click="false" offset="8">
    <template #activator="{ props }">
      <v-btn
        v-bind="props"
        variant="text"
        prepend-icon="mdi-palette-outline"
        append-icon="mdi-chevron-down"
        :aria-label="`Theme: ${currentLabel}`"
        class="px-2"
      >
        <span class="hidden sm:inline">{{ currentLabel }}</span>
      </v-btn>
    </template>

    <v-card width="300" class="p-2 shadow-xl" elevation="8">
      <p class="px-2 pt-1 pb-2 text-xs font-semibold tracking-wide text-ctp-subtext0 uppercase">
        Flavor
      </p>
      <v-list
        density="compact"
        nav
        class="bg-transparent p-0"
        role="radiogroup"
        aria-label="Color flavor"
      >
        <v-list-item
          v-for="opt in options"
          :key="opt.id"
          role="radio"
          :aria-checked="theme.choice === opt.id"
          :active="theme.choice === opt.id"
          color="primary"
          rounded="lg"
          @click="theme.setFlavor(opt.id)"
        >
          <template #prepend>
            <span
              v-if="opt.id === 'system'"
              class="me-3 flex h-6 w-10 items-center justify-center rounded-md border border-ctp-surface1 text-ctp-subtext1"
            >
              <v-icon icon="mdi-monitor" size="16" />
            </span>
            <span
              v-else
              class="me-3 flex h-6 w-10 overflow-hidden rounded-md border border-ctp-surface1"
            >
              <span
                v-for="(hex, i) in swatch(opt.id as FlavorName)"
                :key="i"
                class="h-full flex-1"
                :style="{ backgroundColor: hex }"
              />
            </span>
          </template>
          <v-list-item-title class="font-medium">{{ opt.label }}</v-list-item-title>
          <v-list-item-subtitle>{{ opt.hint }}</v-list-item-subtitle>
          <template v-if="theme.choice === opt.id" #append>
            <v-icon icon="mdi-check" size="18" />
          </template>
        </v-list-item>
      </v-list>

      <p class="px-2 pt-3 pb-2 text-xs font-semibold tracking-wide text-ctp-subtext0 uppercase">
        Accent
      </p>
      <div role="radiogroup" aria-label="Accent color" class="grid grid-cols-7 gap-1.5 px-2 pb-2">
        <button
          v-for="name in ACCENT_NAMES"
          :key="name"
          v-ripple
          type="button"
          role="radio"
          :aria-checked="theme.accent === name"
          :aria-label="name"
          :title="name"
          class="flex h-7 w-7 items-center justify-center rounded-full ring-offset-2 ring-offset-ctp-mantle transition"
          :class="theme.accent === name ? 'ring-2 ring-ctp-text' : 'hover:scale-110'"
          :style="{ backgroundColor: `var(--ctp-${name})`, color: 'var(--ctp-base)' }"
          @click="theme.setAccent(name)"
        >
          <v-icon v-if="theme.accent === name" icon="mdi-check" size="14" />
        </button>
      </div>
    </v-card>
  </v-menu>
</template>
