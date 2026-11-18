import { ref, type Ref } from 'vue'

import { errorMessage } from '@/api/client'
import { useToastStore } from '@/stores/toast'

/** Small helper for "load data, show skeleton, toast on error". */
export function useAsync<T>(loader: () => Promise<T>, initial: T) {
  const data = ref(initial) as Ref<T>
  const loading = ref(false)
  const error = ref<string | null>(null)
  const toast = useToastStore()

  async function run() {
    loading.value = true
    error.value = null
    try {
      data.value = await loader()
    } catch (err) {
      error.value = errorMessage(err)
      toast.error(error.value)
    } finally {
      loading.value = false
    }
  }

  return { data, loading, error, run }
}
