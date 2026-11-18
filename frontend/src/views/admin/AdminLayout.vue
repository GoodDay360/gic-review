<script setup lang="ts">
import { computed } from 'vue'

import AppIcon from '@/components/AppIcon.vue'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()

const links = computed(() => [
  { to: '/admin/quizzes', label: 'Quizzes', icon: 'list-check' },
  { to: '/admin/import', label: 'Bulk import', icon: 'import' },
  { to: '/admin/reviews', label: 'Interactive reviews', icon: 'book' },
  ...(auth.isAdmin
    ? [
        { to: '/admin/subjects', label: 'Subjects', icon: 'folder' },
        { to: '/admin/users', label: 'Managers', icon: 'users' },
      ]
    : []),
])
</script>

<template>
  <div class="mx-auto flex max-w-6xl flex-col gap-6 px-4 py-6 md:flex-row">
    <aside class="md:w-56 md:shrink-0">
      <div class="card p-3">
        <p class="px-2 pb-2 text-xs text-ctp-subtext0">
          Signed in as <span class="font-medium text-ctp-text">{{ auth.user?.display_name }}</span>
          <span class="ml-1 rounded bg-ctp-surface0 px-1.5 py-0.5 text-[10px] uppercase">{{
            auth.user?.role
          }}</span>
        </p>
        <nav class="flex gap-1 overflow-x-auto md:flex-col" aria-label="Admin">
          <RouterLink
            v-for="l in links"
            :key="l.to"
            :to="l.to"
            class="flex shrink-0 items-center gap-2 rounded-lg px-3 py-2 text-sm text-ctp-subtext1 hover:bg-ctp-surface0 hover:text-ctp-text"
            active-class="!bg-ctp-surface0 !text-ctp-accent font-medium"
          >
            <AppIcon :name="l.icon" :size="18" /> {{ l.label }}
          </RouterLink>
        </nav>
      </div>
    </aside>
    <section class="min-w-0 flex-1">
      <RouterView />
    </section>
  </div>
</template>
