/** Remembers the last year a student opened (this browser only), to highlight it on Home. */
const KEY = 'gic-last-year'

export function lastYearSlug(): string | null {
  try {
    return localStorage.getItem(KEY)
  } catch {
    return null
  }
}

export function rememberYear(slug: string) {
  try {
    localStorage.setItem(KEY, slug)
  } catch {
    /* private mode */
  }
}
