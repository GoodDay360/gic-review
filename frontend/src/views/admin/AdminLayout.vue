<script setup lang="ts">
import { computed } from 'vue'

import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()

// Everyone gets content, categories (their own), subjects (in their categories) and their API key;
// admins also manage accounts and everyone's API keys.
const links = computed(() => [
  { to: '/admin/quizzes', label: 'Quizzes', icon: 'mdi-format-list-checks' },
  { to: '/admin/import', label: 'Bulk import', icon: 'mdi-import' },
  {
    to: '/admin/reviews',
    label: 'Interactive reviews',
    icon: 'mdi-book-open-page-variant-outline',
  },
  { to: '/admin/categories', label: 'Categories', icon: 'mdi-layers-outline' },
  { to: '/admin/subjects', label: 'Subjects', icon: 'mdi-folder-outline' },
  { to: '/admin/api-key', label: 'API key', icon: 'mdi-key-variant' },
  { to: '/admin/account', label: 'My account', icon: 'mdi-account-circle-outline' },
  ...(auth.isAdmin
    ? [
        { to: '/admin/users', label: 'Managers', icon: 'mdi-account-multiple-outline' },
        { to: '/admin/api-keys', label: 'All API keys', icon: 'mdi-key-chain-variant' },
      ]
    : []),
])
</script>

<template>
  <div class="mx-auto flex max-w-6xl flex-col gap-6 px-4 py-6 md:flex-row">
    <aside class="animate__animated animate__fadeInLeft animate__faster md:w-60 md:shrink-0">
      <v-card class="p-3 md:sticky md:top-20">
        <div class="flex items-center gap-3 px-2 pb-3">
          <v-avatar color="primary" variant="tonal" size="36">
            <v-icon icon="mdi-account-outline" />
          </v-avatar>
          <div class="min-w-0">
            <p class="truncate text-sm font-medium text-ctp-text">{{ auth.user?.display_name }}</p>
            <v-chip
              size="x-small"
              :color="auth.isAdmin ? 'ctp-peach' : 'info'"
              variant="tonal"
              class="mt-0.5 uppercase"
            >
              {{ auth.user?.role }}
            </v-chip>
          </div>
        </div>
        <v-divider class="mb-2" />
        <v-list
          nav
          density="compact"
          color="primary"
          class="flex gap-1 overflow-x-auto bg-transparent p-0 md:block"
          aria-label="Admin"
        >
          <v-list-item
            v-for="l in links"
            :key="l.to"
            :to="l.to"
            :prepend-icon="l.icon"
            :title="l.label"
            rounded="lg"
            class="shrink-0"
          />
        </v-list>
      </v-card>
    </aside>
    <section class="min-w-0 flex-1">
      <RouterView v-slot="{ Component }">
        <Transition
          mode="out-in"
          enter-active-class="animate__animated animate__fadeIn animate__faster"
        >
          <component :is="Component" />
        </Transition>
      </RouterView>
    </section>
  </div>
</template>
