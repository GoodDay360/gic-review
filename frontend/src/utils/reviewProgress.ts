const STORAGE_KEY = 'gic-review-scroll-position'

interface ReviewScrollMessage {
  type: 'gic-review-scroll'
  scrollY: number
  final?: boolean
}

export function isReviewScrollMessage(value: unknown): value is ReviewScrollMessage {
  if (!value || typeof value !== 'object') return false
  const message = value as Partial<ReviewScrollMessage>
  return (
    message.type === 'gic-review-scroll' &&
    typeof message.scrollY === 'number' &&
    Number.isFinite(message.scrollY) &&
    message.scrollY >= 0
  )
}

export function reviewScrollKey(reviewId: number): string {
  return `${STORAGE_KEY}:${reviewId}`
}

export function getReviewScrollPosition(reviewId: number): number {
  const saved = localStorage.getItem(reviewScrollKey(reviewId))
  const scrollY = saved === null ? 0 : Number(saved)
  return Number.isFinite(scrollY) && scrollY >= 0 ? scrollY : 0
}

export function saveReviewScrollPosition(reviewId: number, scrollY: number): void {
  localStorage.setItem(reviewScrollKey(reviewId), String(scrollY))
}
