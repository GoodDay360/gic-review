import { MAX_SAVED_RESULTS } from '@/config'

const DATABASE_NAME = 'gic-review'
const STORE_NAME = 'review-scroll-positions'
const DATABASE_VERSION = 1

interface ReviewScrollPosition {
  reviewId: number
  scrollY: number
  savedAt: number
}

let databasePromise: Promise<IDBDatabase> | undefined

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

function openDatabase(): Promise<IDBDatabase> {
  if (databasePromise) return databasePromise

  const opening = new Promise<IDBDatabase>((resolve, reject) => {
    const request = indexedDB.open(DATABASE_NAME, DATABASE_VERSION)
    request.onupgradeneeded = () => {
      const store = request.result.createObjectStore(STORE_NAME, { keyPath: 'reviewId' })
      store.createIndex('savedAt', 'savedAt')
    }
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error ?? new Error('Could not open review progress'))
  })
  const sharedOpening = opening.catch((error: unknown) => {
    databasePromise = undefined
    throw error
  })
  databasePromise = sharedOpening
  return sharedOpening
}

function transactionComplete(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve()
    transaction.onabort = () =>
      reject(transaction.error ?? new Error('Could not save review progress'))
    transaction.onerror = () =>
      reject(transaction.error ?? new Error('Could not save review progress'))
  })
}

export async function getReviewScrollPosition(reviewId: number): Promise<number> {
  const database = await openDatabase()
  const transaction = database.transaction(STORE_NAME, 'readonly')
  const request = transaction.objectStore(STORE_NAME).get(reviewId)
  const [position] = await Promise.all([
    new Promise<ReviewScrollPosition | undefined>((resolve, reject) => {
      request.onsuccess = () => resolve(request.result as ReviewScrollPosition | undefined)
      request.onerror = () => reject(request.error ?? new Error('Could not read review progress'))
    }),
    transactionComplete(transaction),
  ])
  return position && Number.isFinite(position.scrollY) && position.scrollY >= 0
    ? position.scrollY
    : 0
}

export async function saveReviewScrollPosition(reviewId: number, scrollY: number): Promise<void> {
  const database = await openDatabase()
  const transaction = database.transaction(STORE_NAME, 'readwrite')
  const store = transaction.objectStore(STORE_NAME)
  const saved = transactionComplete(transaction)
  const putPosition = () => {
    store.put({ reviewId, scrollY, savedAt: Date.now() } satisfies ReviewScrollPosition)
  }
  const existingRequest = store.get(reviewId)
  existingRequest.onsuccess = () => {
    if (existingRequest.result) {
      putPosition()
      return
    }

    const countRequest = store.count()
    countRequest.onsuccess = () => {
      if (countRequest.result < MAX_SAVED_RESULTS) {
        putPosition()
        return
      }

      const oldestRequest = store.index('savedAt').openCursor()
      oldestRequest.onsuccess = () => {
        const oldest = oldestRequest.result
        if (!oldest) {
          transaction.abort()
          return
        }
        oldest.delete()
        putPosition()
      }
    }
  }
  existingRequest.onerror = () => transaction.abort()
  await saved
}
