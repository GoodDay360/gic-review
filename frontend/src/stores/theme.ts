import { flavorEntries, flavors, type AccentName, type FlavorName } from '@catppuccin/palette'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import { applyVuetifyTheme } from '@/plugins/vuetify'

export type FlavorChoice = FlavorName | 'system'

// Keep these keys in sync with the inline script in index.html.
const FLAVOR_KEY = 'gic-theme-flavor'
const ACCENT_KEY = 'gic-theme-accent'

export const FLAVOR_NAMES = flavorEntries.map(([name]) => name)
/** The 14 accent colors, read from the palette package. */
export const ACCENT_NAMES = flavors.mocha.colorEntries
  .filter(([, c]) => c.accent)
  .map(([name]) => name as AccentName)

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function writeStorage(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* private mode: theme just won't persist */
  }
}

export const useThemeStore = defineStore('theme', () => {
  const savedFlavor = readStorage(FLAVOR_KEY)
  const savedAccent = readStorage(ACCENT_KEY)

  const choice = ref<FlavorChoice>(
    savedFlavor && (FLAVOR_NAMES as string[]).includes(savedFlavor)
      ? (savedFlavor as FlavorName)
      : 'system',
  )
  const accent = ref<AccentName>(
    savedAccent && (ACCENT_NAMES as string[]).includes(savedAccent)
      ? (savedAccent as AccentName)
      : 'mauve',
  )
  const systemDark = ref(false)

  /** The flavor actually applied (System → Latte for light OS, Mocha for dark OS). */
  const flavor = computed<FlavorName>(() =>
    choice.value === 'system' ? (systemDark.value ? 'mocha' : 'latte') : choice.value,
  )
  const isDark = computed(() => flavors[flavor.value].dark)

  function apply() {
    const root = document.documentElement
    root.setAttribute('data-flavor', flavor.value)
    root.setAttribute('data-accent', accent.value)
    applyVuetifyTheme(flavor.value, accent.value)
  }

  function setFlavor(next: FlavorChoice) {
    choice.value = next
    writeStorage(FLAVOR_KEY, next)
    apply()
  }

  function setAccent(next: AccentName) {
    accent.value = next
    writeStorage(ACCENT_KEY, next)
    apply()
  }

  let initialized = false
  function init() {
    if (initialized) return
    initialized = true
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    systemDark.value = media.matches
    // Follow OS changes live while "System" is selected.
    media.addEventListener('change', (e) => {
      systemDark.value = e.matches
      apply()
    })
    apply()
  }

  return { choice, accent, flavor, isDark, setFlavor, setAccent, init }
})
