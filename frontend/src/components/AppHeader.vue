<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import { APP_NAME } from '@/config'
import { useAuthStore } from '@/stores/auth'

import AppIcon from './AppIcon.vue'
import ThemePicker from './ThemePicker.vue'

const auth = useAuthStore()
const router = useRouter()
const loggingOut = ref(false)

async function logout() {
  if (loggingOut.value) return
  loggingOut.value = true
  try {
    await auth.logout()
    router.push('/')
  } finally {
    loggingOut.value = false
  }
}
</script>

<template>
  <header class="sticky top-0 z-20 border-b border-ctp-surface0 bg-ctp-mantle/95 backdrop-blur">
    <div class="mx-auto flex h-14 max-w-6xl items-center justify-between gap-3 px-4">
      <RouterLink to="/" class="flex items-center gap-2 font-bold tracking-tight text-ctp-text">
        <span
          class="flex h-8 w-8 items-center justify-center rounded-lg bg-ctp-accent text-sm text-ctp-on-accent"
        >
          GR
        </span>
        <span class="text-lg">{{ APP_NAME }}</span>
      </RouterLink>
      <nav class="flex items-center gap-1">
        <ThemePicker />
        <template v-if="auth.isStaff">
          <RouterLink to="/admin" class="btn-ghost px-2.5 text-sm">
            <AppIcon name="pencil" :size="16" />
            <span class="hidden sm:inline">Dashboard</span>
          </RouterLink>
          <button
            class="btn-ghost px-2.5 text-sm"
            aria-label="Log out"
            :disabled="loggingOut"
            @click="logout"
          >
            <AppIcon name="logout" :size="16" />
          </button>
        </template>
        <RouterLink v-else to="/login" class="px-2 text-xs text-ctp-subtext0 hover:text-ctp-accent">
          Staff login
        </RouterLink>
      </nav>
    </div>
  </header>
</template>
