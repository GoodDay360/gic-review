/** Remembers the last category a student opened (this browser only), to highlight it on Home. */
const KEY = 'gic-last-category'

export function lastCategorySlug(): string | null {
  try {
    return localStorage.getItem(KEY)
  } catch {
    return null
  }
}

export function rememberCategory(slug: string) {
  try {
    localStorage.setItem(KEY, slug)
  } catch {
    /* private mode */
  }
}
