<script setup lang="ts">
import type { Year } from '@/api/types'

import AppIcon from './AppIcon.vue'

defineProps<{ year: Year; lastVisited?: boolean }>()
</script>

<template>
  <RouterLink
    :to="`/years/${year.slug}`"
    class="card group flex h-full flex-col gap-2 p-5 transition hover:-translate-y-0.5 hover:border-ctp-surface2 hover:shadow-lg"
    :class="lastVisited ? 'border-ctp-accent/60' : ''"
  >
    <span class="flex items-start justify-between gap-3">
      <span class="text-2xl font-bold tracking-tight text-ctp-text group-hover:text-ctp-accent">
        {{ year.name }}
      </span>
      <AppIcon name="chevron-right" class="mt-1 text-ctp-overlay1 group-hover:text-ctp-accent" />
    </span>
    <span v-if="year.description" class="line-clamp-2 text-sm text-ctp-subtext0">
      {{ year.description }}
    </span>
    <span class="mt-auto flex flex-wrap items-center gap-2 pt-2 text-xs">
      <span class="rounded-full bg-ctp-surface0 px-2.5 py-1 text-ctp-subtext1">
        {{ year.subject_count }} subject{{ year.subject_count === 1 ? '' : 's' }}
      </span>
      <span
        v-if="lastVisited"
        class="rounded-full bg-ctp-accent/15 px-2.5 py-1 font-medium text-ctp-accent"
      >
        Last opened
      </span>
    </span>
  </RouterLink>
</template>
