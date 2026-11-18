<script setup lang="ts">
/** Step 1: pick a year (e.g. I3, I4, or a custom name). Step 2 is YearView (its subjects). */
import { onMounted } from 'vue'

import { api } from '@/api/client'
import type { Year } from '@/api/types'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import YearCard from '@/components/YearCard.vue'
import { useAsync } from '@/composables/useAsync'
import { lastYearSlug } from '@/composables/lastYear'

const { data: years, loading, error, run } = useAsync(() => api.get<Year[]>('/years'), [])
const last = lastYearSlug()

onMounted(run)
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8 sm:py-12">
    <section class="mb-8 sm:mb-10">
      <h1 class="text-3xl font-bold tracking-tight text-ctp-text sm:text-4xl">
        Study smarter for your exams
      </h1>
      <p class="mt-2 max-w-2xl text-ctp-subtext0">
        Choose your year, then a subject: read its interactive review or practice with quizzes. No
        account needed.
      </p>
    </section>

    <h2 class="mb-3 text-sm font-semibold tracking-wide text-ctp-subtext1 uppercase">
      Select your year
    </h2>
    <div v-if="loading" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <SkeletonBlock v-for="i in 4" :key="i" class="h-32" />
    </div>
    <EmptyState v-else-if="error" title="Couldn't load years" :message="error" icon="warning">
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <EmptyState
      v-else-if="years.length === 0"
      title="No years yet"
      message="Years will appear here once they are added."
    />
    <div v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <YearCard v-for="y in years" :key="y.id" :year="y" :last-visited="y.slug === last" />
    </div>
  </div>
</template>
