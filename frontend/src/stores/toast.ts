import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ToastKind = 'success' | 'error' | 'info' | 'warning'

export interface Toast {
  id: number
  kind: ToastKind
  message: string
}

let nextId = 1

export const useToastStore = defineStore('toast', () => {
  const toasts = ref<Toast[]>([])

  function dismiss(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id)
  }

  function push(kind: ToastKind, message: string, timeoutMs = 5000) {
    const id = nextId++
    toasts.value.push({ id, kind, message })
    if (timeoutMs > 0) setTimeout(() => dismiss(id), timeoutMs)
    return id
  }

  return {
    toasts,
    dismiss,
    push,
    success: (m: string) => push('success', m),
    error: (m: string) => push('error', m, 8000),
    info: (m: string) => push('info', m),
    warning: (m: string) => push('warning', m),
  }
})
