/**
 * Subject slugs are only unique inside a category, so every subject link carries both slugs:
 * `/categories/<category>/<subject>/review` (app) and `/categories/<category>/subjects/<subject>` (API).
 */
const enc = encodeURIComponent

export type SubjectPage = 'review' | 'quizzes'

/** App route of a subject page. */
export function subjectPagePath(categorySlug: string, subjectSlug: string, page: SubjectPage) {
  return `/categories/${enc(categorySlug)}/${enc(subjectSlug)}/${page}`
}

/** API path of a subject (append `/reviews` or `/quizzes`). */
export function subjectApiPath(categorySlug: string, subjectSlug: string) {
  return `/categories/${enc(categorySlug)}/subjects/${enc(subjectSlug)}`
}
