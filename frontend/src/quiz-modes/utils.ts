import type { Question } from '@/api/types'

/** Fisher–Yates shuffle (returns a new array). */
export function shuffle<T>(items: readonly T[]): T[] {
  const out = items.slice()
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[out[i], out[j]] = [out[j]!, out[i]!]
  }
  return out
}

/** All image URLs a question can show (front, back, explanation, options). */
export function questionImageUrls(q: Question | undefined): string[] {
  if (!q) return []
  return [
    q.image_url,
    q.answer_image_url,
    q.explanation_image_url,
    ...q.options.map((o) => o.image_url),
  ].filter((u): u is string => !!u)
}

const preloaded = new Set<string>()

/** Warm the browser cache so the next question's images appear without delay. */
export function preloadQuestionImages(q: Question | undefined) {
  for (const url of questionImageUrls(q)) {
    if (preloaded.has(url)) continue
    preloaded.add(url)
    const img = new Image()
    img.decoding = 'async'
    img.src = url
  }
}

/** Short label for a question in result lists. */
export function questionLabel(q: Question): string {
  return q.prompt?.trim() || q.image_alt?.trim() || 'Image question'
}

/** What the back of a flashcard shows when there is no explicit answer. */
export function correctOptions(q: Question) {
  return q.options.filter((o) => o.is_correct)
}
