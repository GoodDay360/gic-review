import { defineStore } from 'pinia'
import { ref } from 'vue'

export type ToastKind = 'success' | 'error' | 'info' | 'warning'

/** One message for Vuetify's `<v-snackbar-queue>` (see ToastHost.vue). */
export interface Toast {
  text: string
  title?: string
  color: ToastKind
  prependIcon: string
  timeout: number
}

const ICONS: Record<ToastKind, string> = {
  success: 'mdi-check-circle-outline',
  error: 'mdi-alert-octagon-outline',
  warning: 'mdi-alert-outline',
  info: 'mdi-information-outline',
}

const TIMEOUTS: Record<ToastKind, number> = {
  success: 4000,
  info: 5000,
  warning: 6000,
  error: 8000,
}

export const useToastStore = defineStore('toast', () => {
  /** Pending messages; the snackbar queue removes each one as it is shown. */
  const queue = ref<Toast[]>([])

  function push(kind: ToastKind, message: string, title?: string, timeoutMs = TIMEOUTS[kind]) {
    queue.value.push({
      text: message,
      title,
      color: kind,
      prependIcon: ICONS[kind],
      timeout: timeoutMs,
    })
  }

  return {
    queue,
    push,
    success: (m: string, title?: string) => push('success', m, title),
    error: (m: string, title?: string) => push('error', m, title),
    info: (m: string, title?: string) => push('info', m, title),
    warning: (m: string, title?: string) => push('warning', m, title),
  }
})
