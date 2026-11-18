import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'

import { errorMessage } from '@/api/client'
import type { Page } from '@/api/types'

export const PAGE_SIZE = 20

export function useInfiniteSearch<T>(
  fetchPage: (search: string, offset: number, limit: number) => Promise<Page<T>>,
) {
  const search = ref('')
  const items = shallowRef<T[]>([])
  const loading = ref(false)
  const loadingMore = ref(false)
  const hasMore = ref(true)
  const error = ref<string | null>(null)
  let generation = 0
  let debounce: ReturnType<typeof setTimeout> | undefined

  async function load(reset: boolean) {
    if (!reset && (loading.value || loadingMore.value || !hasMore.value)) return
    const currentGeneration = reset ? ++generation : generation
    const offset = reset ? 0 : items.value.length
    if (reset) {
      items.value = []
      hasMore.value = true
      loading.value = true
    } else {
      loadingMore.value = true
    }
    error.value = null
    try {
      const page = await fetchPage(search.value.trim(), offset, PAGE_SIZE)
      if (currentGeneration !== generation) return
      items.value = reset ? page.items : [...items.value, ...page.items]
      hasMore.value = page.has_more
    } catch (err) {
      if (currentGeneration === generation) error.value = errorMessage(err)
    } finally {
      if (currentGeneration === generation) {
        loading.value = false
        loadingMore.value = false
      }
    }
  }

  function reset() {
    if (debounce) clearTimeout(debounce)
    loadingMore.value = false
    void load(true)
  }

  function retry() {
    void load(items.value.length > 0)
  }

  watch(search, () => {
    if (debounce) clearTimeout(debounce)
    debounce = setTimeout(() => {
      debounce = undefined
      reset()
    }, 250)
  })

  onMounted(reset)
  onBeforeUnmount(() => {
    generation++
    if (debounce) clearTimeout(debounce)
  })

  return {
    search,
    items,
    loading,
    loadingMore,
    hasMore,
    error,
    loadMore: () => load(false),
    reset,
    retry,
  }
}
