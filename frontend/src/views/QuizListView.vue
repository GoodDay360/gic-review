<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'

import { api } from '@/api/client'
import type { Page, QuizSummary, Subject } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import QuizCard from '@/components/QuizCard.vue'
import QuizModeModal from '@/components/QuizModeModal.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useInfiniteSearch } from '@/composables/useInfiniteSearch'
import { useAsync } from '@/composables/useAsync'

const props = defineProps<{ subjectSlug: string }>()
const {
  data: subject,
  loading,
  error,
  run,
} = useAsync(
  () => api.get<Subject>(`/subjects/${encodeURIComponent(props.subjectSlug)}`),
  null as Subject | null,
)
const {
  search,
  items: quizzes,
  loading: quizzesLoading,
  loadingMore,
  hasMore,
  error: quizzesError,
  loadMore,
  retry,
  reset,
} = useInfiniteSearch<QuizSummary>((search, offset, limit) => {
  const query = new URLSearchParams({ search, offset: String(offset), limit: String(limit) })
  return api.get<Page<QuizSummary>>(
    `/subjects/${encodeURIComponent(props.subjectSlug)}/quizzes?${query}`,
  )
})
const selected = ref<QuizSummary | null>(null)
watch(
  () => props.subjectSlug,
  () => {
    selected.value = null
    if (search.value) search.value = ''
    else reset()
    run()
  },
)
onMounted(run)
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8">
    <RouterLink :to="subject ? `/years/${subject.year_slug}` : '/'" class="btn-ghost -ml-3 mb-4"
      ><AppIcon name="arrow-left" :size="16" />
      {{ subject ? `${subject.year_name} subjects` : 'Home' }}</RouterLink
    >
    <h1 class="text-2xl font-bold text-ctp-text sm:text-3xl">
      {{ subject?.name ?? 'Quizzes' }}
      <span class="font-normal text-ctp-subtext0">· Quizzes</span>
    </h1>

    <div class="mt-6">
      <SearchBar
        v-model="search"
        class="mb-5"
        label="Search quizzes"
        placeholder="Search quizzes…"
      />
      <div v-if="loading && !subject" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SkeletonBlock v-for="i in 3" :key="i" class="h-60" />
      </div>
      <EmptyState
        v-else-if="error && !subject"
        title="Couldn't load quizzes"
        :message="error"
        icon="warning"
      >
        <button class="btn-secondary" @click="run">Try again</button>
      </EmptyState>
      <div
        v-else-if="quizzesLoading && quizzes.length === 0"
        class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
      >
        <SkeletonBlock v-for="i in 3" :key="i" class="h-60" />
      </div>
      <EmptyState
        v-else-if="quizzesError && quizzes.length === 0"
        title="Couldn't load quizzes"
        :message="quizzesError"
        icon="warning"
      >
        <button class="btn-secondary" @click="retry">Try again</button>
      </EmptyState>
      <EmptyState
        v-else-if="quizzes.length === 0"
        :title="search ? 'No matching quizzes' : 'No quizzes yet for this subject'"
        :message="search ? 'Try another search.' : 'Check back soon, new quizzes are on the way.'"
        icon="list-check"
      />
      <template v-else>
        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <QuizCard v-for="q in quizzes" :key="q.id" :quiz="q" @select="selected = $event" />
        </div>
        <InfiniteScrollSentinel
          :has-more="hasMore"
          :loading="loadingMore"
          :error="quizzesError"
          @load-more="quizzesError ? retry() : loadMore()"
        />
      </template>
    </div>

    <QuizModeModal :quiz="selected" @close="selected = null" />
  </div>
</template>
