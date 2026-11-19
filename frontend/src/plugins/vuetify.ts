/**
 * Vuetify setup: one theme per Catppuccin flavor (built from `@catppuccin/palette`, no
 * hand-copied hex values), Material Design Icons, and app-wide component defaults.
 *
 * - Theme names are the flavor names (`latte`, `frappe`, `macchiato`, `mocha`).
 * - `primary` is the chosen Catppuccin accent; `setVuetifyAccent()` updates it at runtime.
 * - Every palette color is also available as `ctp-<name>` (e.g. `color="ctp-teal"`).
 * - The font comes from `--v-font-body` (see styles/main.css), i.e. the Plus Jakarta Sans that
 *   vite-plugin-webfont-dl self-hosts.
 */
import '@mdi/font/css/materialdesignicons.css'
import 'vuetify/styles'

import { flavorEntries, flavors, type AccentName, type FlavorName } from '@catppuccin/palette'
import { createVuetify, type ThemeDefinition } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'

type Rgb = { r: number; g: number; b: number }

/** WCAG relative luminance / contrast, used to pick readable text on colored surfaces. */
function luminance({ r, g, b }: Rgb): number {
  const ch = (v: number) => {
    const c = v / 255
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}
function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number]
  return (hi + 0.05) / (lo + 0.05)
}

/** Catppuccin puts `base` on colored surfaces; use `text` where that reads better. */
function onColor(flavor: FlavorName, color: keyof (typeof flavors)['mocha']['colors']): string {
  const f = flavors[flavor].colors
  const c = f[color].rgb
  return contrast(c, f.text.rgb) > contrast(c, f.base.rgb) ? f.text.hex : f.base.hex
}

function themeFor(name: FlavorName, accent: AccentName): ThemeDefinition {
  const { colors: c, dark } = flavors[name]
  const ctp = Object.fromEntries(
    flavors[name].colorEntries.map(([color, { hex }]) => [`ctp-${color}`, hex]),
  )
  return {
    dark,
    colors: {
      ...ctp,
      background: c.base.hex,
      'on-background': c.text.hex,
      surface: c.mantle.hex,
      'on-surface': c.text.hex,
      'surface-bright': c.base.hex,
      'surface-light': c.surface0.hex,
      'surface-variant': c.text.hex,
      'on-surface-variant': c.base.hex,
      primary: c[accent].hex,
      'on-primary': onColor(name, accent),
      secondary: c.lavender.hex,
      'on-secondary': onColor(name, 'lavender'),
      error: c.red.hex,
      'on-error': onColor(name, 'red'),
      warning: c.yellow.hex,
      'on-warning': onColor(name, 'yellow'),
      info: c.blue.hex,
      'on-info': onColor(name, 'blue'),
      success: c.green.hex,
      'on-success': onColor(name, 'green'),
    },
    variables: {
      'border-color': c.text.hex,
      'border-opacity': dark ? 0.14 : 0.16,
      'hover-opacity': 0.06,
    },
  }
}

const field = {
  variant: 'outlined',
  density: 'comfortable',
  color: 'primary',
  hideDetails: 'auto',
  rounded: 'lg',
} as const

/**
 * Start Vuetify on the flavor/accent the inline script in index.html already put on <html>
 * (saved choice or OS preference), so the first render matches and nothing flashes.
 */
function initialTheme(): { flavor: FlavorName; accent: AccentName } {
  const root = document.documentElement
  const flavorAttr = root.getAttribute('data-flavor')
  const accentAttr = root.getAttribute('data-accent')
  const flavor = flavorEntries.some(([n]) => n === flavorAttr)
    ? (flavorAttr as FlavorName)
    : 'latte'
  const isAccent = flavors.mocha.colorEntries.some(([n, c]) => c.accent && n === accentAttr)
  return { flavor, accent: isAccent ? (accentAttr as AccentName) : 'mauve' }
}

const initial = initialTheme()

export const vuetify = createVuetify({
  icons: { defaultSet: 'mdi', aliases, sets: { mdi } },
  theme: {
    defaultTheme: initial.flavor,
    themes: Object.fromEntries(
      flavorEntries.map(([name]) => [name, themeFor(name, initial.accent)]),
    ),
  },
  defaults: {
    VBtn: { rounded: 'lg', class: 'font-weight-semibold', style: 'letter-spacing: normal' },
    VCard: { rounded: 'xl', variant: 'flat', border: true },
    VTextField: field,
    VTextarea: { ...field, autoGrow: true },
    VSelect: field,
    VAutocomplete: field,
    VFileInput: { ...field, prependIcon: '', prependInnerIcon: 'mdi-paperclip' },
    VCheckbox: { color: 'primary', hideDetails: 'auto', density: 'comfortable' },
    VSwitch: { color: 'primary', hideDetails: 'auto', inset: true, density: 'comfortable' },
    VRadioGroup: { color: 'primary', hideDetails: 'auto' },
    VAlert: { variant: 'tonal', rounded: 'lg', density: 'comfortable' },
    VChip: { rounded: 'lg' },
    VDialog: { scrollable: true },
    VTooltip: { location: 'bottom' },
    VProgressCircular: { color: 'primary', indeterminate: true },
  },
})

/** Apply a Catppuccin flavor + accent to Vuetify (called by the theme store). */
export function applyVuetifyTheme(flavor: FlavorName, accent: AccentName) {
  const theme = vuetify.theme
  for (const [name] of flavorEntries) {
    const colors = theme.themes.value[name]?.colors
    if (!colors) continue
    colors.primary = flavors[name].colors[accent].hex
    colors['on-primary'] = onColor(name, accent)
  }
  if (theme.global.name.value !== flavor) theme.change(flavor)
}
