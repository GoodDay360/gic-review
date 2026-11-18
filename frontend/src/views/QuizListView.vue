<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { api } from '@/api/client'
import type { QuizSummary, Subject } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import QuizCard from '@/components/QuizCard.vue'
import QuizModeModal from '@/components/QuizModeModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'

const props = defineProps<{ subjectSlug: string }>()

const { data, loading, error, run } = useAsync(
  async () => {
    const [subject, quizzes] = await Promise.all([
      api.get<Subject>(`/subjects/${encodeURIComponent(props.subjectSlug)}`),
      api.get<QuizSummary[]>(`/subjects/${encodeURIComponent(props.subjectSlug)}/quizzes`),
    ])
    return { subject, quizzes }
  },
  { subject: null as Subject | null, quizzes: [] as QuizSummary[] },
)
const selected = ref<QuizSummary | null>(null)
onMounted(run)
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8">
    <RouterLink to="/" class="btn-ghost -ml-3 mb-4"
      ><AppIcon name="arrow-left" :size="16" /> All subjects</RouterLink
    >
    <h1 class="text-2xl font-bold text-ctp-text sm:text-3xl">
      {{ data.subject?.name ?? 'Quizzes' }}
      <span class="font-normal text-ctp-subtext0">· Quizzes</span>
    </h1>

    <div class="mt-6">
      <div v-if="loading" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SkeletonBlock v-for="i in 3" :key="i" class="h-60" />
      </div>
      <EmptyState v-else-if="error" title="Couldn't load quizzes" :message="error" icon="warning">
        <button class="btn-secondary" @click="run">Try again</button>
      </EmptyState>
      <EmptyState
        v-else-if="data.quizzes.length === 0"
        title="No quizzes yet for this subject"
        message="Check back soon, new quizzes are on the way."
        icon="list-check"
      />
      <div v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <QuizCard v-for="q in data.quizzes" :key="q.id" :quiz="q" @select="selected = $event" />
      </div>
    </div>

    <QuizModeModal :quiz="selected" @close="selected = null" />
  </div>
</template>
