<script setup lang="ts">
/** Step 1: pick a year (e.g. I3, I4, or a custom name). Step 2 is YearView (its subjects). */
import { api } from '@/api/client'
import type { Page, Year } from '@/api/types'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import YearCard from '@/components/YearCard.vue'
import { useInfiniteSearch } from '@/composables/useInfiniteSearch'
import { lastYearSlug } from '@/composables/lastYear'

const {
  search,
  items: years,
  loading,
  loadingMore,
  hasMore,
  error,
  loadMore,
  retry,
} = useInfiniteSearch<Year>((search, offset, limit) => {
  const query = new URLSearchParams({ search, offset: String(offset), limit: String(limit) })
  return api.get<Page<Year>>(`/years?${query}`)
})
const last = lastYearSlug()
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
    <SearchBar v-model="search" class="mb-5" label="Search years" placeholder="Search years…" />
    <div v-if="loading && years.length === 0" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <SkeletonBlock v-for="i in 4" :key="i" class="h-32" />
    </div>
    <EmptyState
      v-else-if="error && years.length === 0"
      title="Couldn't load years"
      :message="error"
      icon="warning"
    >
      <button class="btn-secondary" @click="retry">Try again</button>
    </EmptyState>
    <EmptyState
      v-else-if="years.length === 0"
      :title="search ? 'No matching years' : 'No years yet'"
      :message="search ? 'Try another search.' : 'Years will appear here once they are added.'"
    />
    <template v-else>
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <YearCard v-for="y in years" :key="y.id" :year="y" :last-visited="y.slug === last" />
      </div>
      <InfiniteScrollSentinel
        :has-more="hasMore"
        :loading="loadingMore"
        :error="error"
        @load-more="error ? retry() : loadMore()"
      />
    </template>
  </div>
</template>
