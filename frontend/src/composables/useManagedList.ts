import { ref, watch } from 'vue'

import { api } from '@/api/client'
import type { Page } from '@/api/types'
import type { FilterId } from '@/components/admin/CategorySubjectFilter.vue'
import { useInfiniteSearch } from './useInfiniteSearch'

/**
 * A dashboard list (`/admin/quizzes`, `/admin/reviews`) filtered, searched and paged on the
 * server: 20 items per page, more loaded as you scroll. Changing the category / subject
 * filter (or the search, debounced) reloads from the first page.
 */
export function useManagedList<T extends { id: number }>(path: string) {
  const category = ref<FilterId>('')
  const subject = ref<FilterId>('')
  const list = useInfiniteSearch<T>((search, offset, limit) => {
    const query = new URLSearchParams({ offset: String(offset), limit: String(limit) })
    if (search) query.set('search', search)
    if (category.value) query.set('category_id', String(category.value))
    if (subject.value) query.set('subject_id', String(subject.value))
    return api.get<Page<T>>(`${path}?${query}`)
  })
  watch([category, subject], () => list.reset())

  /** Swap one loaded item (after publish / edit) without reloading the list. */
  function replaceItem(item: T) {
    list.items.value = list.items.value.map((x) => (x.id === item.id ? item : x))
  }
  /** Drop one loaded item (after delete); later pages shift by one, matching the server. */
  function removeItem(id: number) {
    list.items.value = list.items.value.filter((x) => x.id !== id)
  }

  return {
    ...list,
    category,
    subject,
    /** A filter or search is active (to pick the right empty state). */
    isFiltered: () => !!(category.value || subject.value || list.search.value.trim()),
    replaceItem,
    removeItem,
  }
}
