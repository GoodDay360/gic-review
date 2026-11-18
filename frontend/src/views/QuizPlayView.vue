<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'

import { api } from '@/api/client'
import type { QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { availableModes, getQuizMode } from '@/quiz-modes/registry'

const props = defineProps<{ quizId: string; mode?: string }>()
const router = useRouter()

const {
  data: quiz,
  loading,
  error,
  run,
} = useAsync(
  () => api.get<QuizDetail>(`/quizzes/${encodeURIComponent(props.quizId)}`),
  null as QuizDetail | null,
)
onMounted(run)
watch(() => props.quizId, run)

const mode = computed(() => getQuizMode(props.mode))
const modeOk = computed(() => !!quiz.value && !!mode.value && mode.value.isAvailable(quiz.value))
const otherModes = computed(() => (quiz.value ? availableModes(quiz.value) : []))

function switchMode(id: string) {
  router.replace({ query: { mode: id } })
}
</script>

<template>
  <div class="mx-auto max-w-3xl px-4 py-6 sm:py-8">
    <div class="mb-4 flex flex-wrap items-center justify-between gap-2">
      <RouterLink
        :to="quiz ? `/subjects/${quiz.subject_slug}/quizzes` : '/'"
        class="btn-ghost -ml-3"
      >
        <AppIcon name="arrow-left" :size="16" /> Back to quizzes
      </RouterLink>
      <div
        v-if="quiz && otherModes.length > 1"
        class="flex gap-1 rounded-lg bg-ctp-surface0 p-1"
        role="tablist"
        aria-label="Quiz mode"
      >
        <button
          v-for="m in otherModes"
          :key="m.id"
          role="tab"
          :aria-selected="m.id === mode?.id"
          class="rounded-md px-3 py-1 text-sm"
          :class="
            m.id === mode?.id
              ? 'bg-ctp-base font-medium text-ctp-text shadow'
              : 'text-ctp-subtext0 hover:text-ctp-text'
          "
          @click="switchMode(m.id)"
        >
          {{ m.label }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="space-y-4">
      <SkeletonBlock class="h-8 w-2/3" />
      <SkeletonBlock class="h-72" />
    </div>
    <EmptyState v-else-if="error" title="Couldn't load this quiz" :message="error" icon="warning">
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <template v-else-if="quiz">
      <header class="mb-6">
        <p class="text-sm text-ctp-subtext0">
          {{ quiz.subject_name }}<span v-if="mode"> · {{ mode.label }}</span>
        </p>
        <h1 class="text-2xl font-bold text-ctp-text">{{ quiz.title }}</h1>
        <p
          v-if="!quiz.is_published"
          class="mt-2 inline-block rounded-md bg-ctp-yellow/20 px-2 py-1 text-xs font-medium text-ctp-yellow"
        >
          Draft: only staff can see this quiz
        </p>
      </header>
      <component :is="mode!.component" v-if="modeOk" :key="mode!.id" :quiz="quiz" />
      <EmptyState v-else title="This mode isn't available for this quiz" icon="info">
        <div class="flex flex-wrap justify-center gap-2">
          <button v-for="m in otherModes" :key="m.id" class="btn-primary" @click="switchMode(m.id)">
            Play as {{ m.label }}
          </button>
        </div>
      </EmptyState>
    </template>
  </div>
</template>
