<script setup lang="ts">
import { useToastStore } from '@/stores/toast'

import AppIcon from './AppIcon.vue'

const toast = useToastStore()

const styles = {
  success: { icon: 'check', cls: 'border-ctp-green text-ctp-green' },
  error: { icon: 'warning', cls: 'border-ctp-red text-ctp-red' },
  warning: { icon: 'warning', cls: 'border-ctp-yellow text-ctp-yellow' },
  info: { icon: 'info', cls: 'border-ctp-blue text-ctp-blue' },
} as const
</script>

<template>
  <div
    class="pointer-events-none fixed inset-x-0 bottom-0 z-50 flex flex-col items-center gap-2 p-4 sm:items-end"
    aria-live="polite"
    role="status"
  >
    <TransitionGroup
      enter-active-class="transition duration-200"
      enter-from-class="translate-y-2 opacity-0"
      leave-active-class="transition duration-150"
      leave-to-class="opacity-0"
    >
      <div
        v-for="t in toast.toasts"
        :key="t.id"
        class="pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-xl border-l-4 bg-ctp-mantle p-4 shadow-lg"
        :class="styles[t.kind].cls"
      >
        <AppIcon :name="styles[t.kind].icon" />
        <p class="flex-1 text-sm text-ctp-text">{{ t.message }}</p>
        <button
          class="text-ctp-overlay1 hover:text-ctp-text"
          aria-label="Dismiss"
          @click="toast.dismiss(t.id)"
        >
          <AppIcon name="close" :size="16" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
