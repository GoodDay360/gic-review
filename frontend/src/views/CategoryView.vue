<script setup lang="ts">
/** Step 2: the subjects of one category. Clicking a subject opens the activity modal. */
import { ref, watch } from 'vue'

import { api } from '@/api/client'
import type { Subject, CategoryWithSubjects } from '@/api/types'
import ActivityModal from '@/components/ActivityModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import SubjectCard from '@/components/SubjectCard.vue'
import { useInfiniteSearch } from '@/composables/useInfiniteSearch'
import { rememberCategory } from '@/composables/lastCategory'

const props = defineProps<{ categorySlug: string }>()
const category = ref<CategoryWithSubjects | null>(null)
const {
  search,
  items: subjects,
  loading,
  loadingMore,
  hasMore,
  error,
  loadMore,
  retry,
  reset,
} = useInfiniteSearch<Subject>(async (search, offset, limit) => {
  const query = new URLSearchParams({ search, offset: String(offset), limit: String(limit) })
  const result = await api.get<CategoryWithSubjects>(
    `/categories/${encodeURIComponent(props.categorySlug)}?${query}`,
  )
  category.value = result
  return { items: result.subjects, has_more: result.subjects_has_more }
})
const selected = ref<Subject | null>(null)

watch(
  () => props.categorySlug,
  () => {
    category.value = null
    selected.value = null
    if (search.value) search.value = ''
    else reset()
  },
)
watch(category, (y) => {
  if (y) {
    rememberCategory(y.slug)
    document.title = `${y.name} · ${import.meta.env.VITE_APP_NAME || 'GIC Review'}`
  }
})
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8">
    <v-btn variant="text" prepend-icon="mdi-arrow-left" to="/" class="-ml-3 mb-4"
      >All categories</v-btn
    >

    <div v-if="loading && !category" class="space-y-4">
      <SkeletonBlock class="h-10 w-48" />
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SkeletonBlock v-for="i in 3" :key="i" class="h-48" />
      </div>
    </div>
    <EmptyState
      v-else-if="error && !category"
      title="Couldn't load this category"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="retry"
        >Try again</v-btn
      >
    </EmptyState>
    <template v-else-if="category">
      <h1
        class="animate__animated animate__fadeInDown text-3xl font-bold tracking-tight text-ctp-text"
      >
        {{ category.name }}
      </h1>
      <p v-if="category.description" class="mt-1 max-w-2xl text-ctp-subtext0">
        {{ category.description }}
      </p>
      <h2 class="mt-8 mb-3 text-sm font-semibold tracking-wide text-ctp-subtext1 uppercase">
        Select a subject
      </h2>
      <SearchBar
        v-model="search"
        class="mb-5"
        label="Search subjects"
        placeholder="Search subjects…"
      />
      <div v-if="loading && subjects.length === 0" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SkeletonBlock v-for="i in 3" :key="i" class="h-48" />
      </div>
      <EmptyState
        v-else-if="error && subjects.length === 0"
        title="Couldn't load subjects"
        :message="error"
        icon="mdi-alert-outline"
      />
      <EmptyState
        v-else-if="subjects.length === 0"
        :title="search ? 'No matching subjects' : 'No subjects in this category yet'"
        :message="search ? 'Try another search.' : 'Subjects will appear here once they are added.'"
      />
      <div v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SubjectCard
          v-for="(s, i) in subjects"
          :key="s.id"
          :subject="s"
          data-aos="fade-up"
          :data-aos-delay="(i % 3) * 70"
          @select="selected = $event"
        />
      </div>
      <InfiniteScrollSentinel
        :has-more="hasMore"
        :loading="loadingMore"
        :error="error"
        @load-more="error ? retry() : loadMore()"
      />
    </template>

    <ActivityModal :subject="selected" @close="selected = null" />
  </div>
</template>
