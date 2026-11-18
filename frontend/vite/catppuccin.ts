/**
 * Vite plugin that generates the Catppuccin CSS variables from the official
 * `@catppuccin/palette` package (no hand-copied hex values).
 *
 *   <html data-flavor="mocha" data-accent="blue">
 *
 * - `--ctp-<color>` for all 26 colors of the active flavor (rosewater … crust)
 * - `--ctp-accent` → the chosen accent (default mauve)
 * - `--ctp-on-accent` → readable text color on top of the accent
 *
 * Import it once with `import 'virtual:catppuccin.css'`.
 */
import { flavorEntries } from '@catppuccin/palette'
import type { Plugin } from 'vite'

/** WCAG relative luminance / contrast, used to pick readable text on accent buttons. */
function luminance({ r, g, b }: { r: number; g: number; b: number }): number {
  const ch = (v: number) => {
    const c = v / 255
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}
function contrast(a: { r: number; g: number; b: number }, b: { r: number; g: number; b: number }) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number]
  return (hi + 0.05) / (lo + 0.05)
}

const VIRTUAL_ID = 'virtual:catppuccin.css'
// A path-like id ending in .css so Vite's CSS pipeline handles it.
const RESOLVED_ID = '/__catppuccin.css'

export function generateCatppuccinCss(): string {
  const blocks: string[] = []
  const accents = new Set<string>()

  for (const [name, flavor] of flavorEntries) {
    const selector =
      name === 'latte' ? `:root, :root[data-flavor="latte"]` : `:root[data-flavor="${name}"]`
    const vars = flavor.colorEntries.map(([color, { hex, accent }]) => {
      if (accent) accents.add(color)
      return `  --ctp-${color}: ${hex};`
    })
    blocks.push(
      `${selector} {\n${vars.join('\n')}\n  --ctp-on-accent: var(--ctp-base);\n  color-scheme: ${flavor.dark ? 'dark' : 'light'};\n}`,
    )

    // Catppuccin puts `base` on accents; fall back to `text` where that is more readable
    // (e.g. Latte yellow), so buttons keep good contrast in every flavor.
    for (const [color, c] of flavor.colorEntries) {
      if (!c.accent) continue
      const onBase = contrast(c.rgb, flavor.colors.base.rgb)
      const onText = contrast(c.rgb, flavor.colors.text.rgb)
      if (onText > onBase) {
        const sel =
          name === 'latte' ? `:root:not([data-flavor]), :root[data-flavor="latte"]` : selector
        const scoped = sel
          .split(', ')
          .map((s) => `${s}[data-accent="${color}"]`)
          .join(', ')
        blocks.push(`${scoped} {\n  --ctp-on-accent: var(--ctp-text);\n}`)
      }
    }
  }

  blocks.push(`:root {\n  --ctp-accent: var(--ctp-mauve);\n}`)
  for (const accent of accents) {
    blocks.push(`:root[data-accent="${accent}"] {\n  --ctp-accent: var(--ctp-${accent});\n}`)
  }
  return `/* generated from @catppuccin/palette */\n${blocks.join('\n\n')}\n`
}

export default function catppuccin(): Plugin {
  return {
    name: 'gic-catppuccin-css',
    resolveId(id) {
      return id === VIRTUAL_ID ? RESOLVED_ID : undefined
    },
    load(id) {
      return id === RESOLVED_ID ? generateCatppuccinCss() : undefined
    },
  }
}
