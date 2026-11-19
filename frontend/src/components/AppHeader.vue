<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

import { APP_NAME } from '@/config'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

import ThemePicker from './ThemePicker.vue'

const auth = useAuthStore()
const toast = useToastStore()
const router = useRouter()
const loggingOut = ref(false)
const isStaff = computed(() => auth.isStaff)

async function logout() {
  if (loggingOut.value) return
  loggingOut.value = true
  try {
    await auth.logout()
    toast.success('You have been signed out.', 'Signed out')
    router.push('/')
  } catch {
    toast.warning('The server could not be reached, but you were signed out on this device.')
    router.push('/')
  } finally {
    loggingOut.value = false
  }
}
</script>

<template>
  <v-app-bar flat height="60" class="border-b border-ctp-surface0 bg-ctp-mantle/90 backdrop-blur">
    <div class="mx-auto flex w-full max-w-6xl items-center justify-between gap-3 px-4">
      <RouterLink
        to="/"
        class="group flex items-center gap-2 font-bold tracking-tight text-ctp-text"
      >
        <v-avatar
          color="primary"
          rounded="lg"
          size="36"
          class="text-sm font-extrabold transition-transform group-hover:rotate-6"
        >
          GR
        </v-avatar>
        <span class="text-lg whitespace-nowrap">{{ APP_NAME }}</span>
      </RouterLink>
      <nav class="flex items-center gap-1">
        <v-btn
          icon
          variant="text"
          size="small"
          href="https://github.com/GoodDay360/gic-review/blob/main/docs/README.md"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Documentation (opens in a new tab)"
        >
          <v-icon icon="mdi-file-document-outline" />
          <v-tooltip activator="parent">Documentation</v-tooltip>
        </v-btn>
        <ThemePicker />
        <template v-if="isStaff">
          <v-btn
            to="/admin"
            variant="tonal"
            color="primary"
            prepend-icon="mdi-view-dashboard-outline"
            class="hidden sm:inline-flex"
          >
            Dashboard
          </v-btn>
          <v-btn
            to="/admin"
            icon="mdi-view-dashboard-outline"
            variant="text"
            size="small"
            class="sm:hidden"
            aria-label="Dashboard"
          />
          <v-btn
            icon
            variant="text"
            size="small"
            aria-label="Log out"
            :loading="loggingOut"
            @click="logout"
          >
            <v-icon icon="mdi-logout" />
            <v-tooltip activator="parent">Log out</v-tooltip>
          </v-btn>
        </template>
        <v-btn v-else to="/login" variant="text" size="small" prepend-icon="mdi-login">
          Staff login
        </v-btn>
      </nav>
    </div>
  </v-app-bar>
</template>
