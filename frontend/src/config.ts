export const APP_NAME = import.meta.env.VITE_APP_NAME || 'GIC Review'
export const API_BASE_URL = (
  import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080/api'
).replace(/\/+$/, '')

const configuredMaxSavedResults = Number(import.meta.env.VITE_MAX_SAVED_RESULTS)
export const MAX_SAVED_RESULTS =
  Number.isSafeInteger(configuredMaxSavedResults) && configuredMaxSavedResults > 0
    ? configuredMaxSavedResults
    : 1_000_000
