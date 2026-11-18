<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'

import { api } from '@/api/client'
import type { QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useProgressStore, type ActiveQuizProgress } from '@/stores/progress'
import { useAsync } from '@/composables/useAsync'
import { availableModes, getQuizMode } from '@/quiz-modes/registry'

const props = defineProps<{ quizId: string; mode?: string }>()
const router = useRouter()
const progress = useProgressStore()

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
const pendingProgress = computed<ActiveQuizProgress | null>(() => {
  const active = progress.state.activeQuiz
  const selectedMode = mode.value
  if (
    !active ||
    !selectedMode ||
    !quiz.value ||
    (active.quizId === Number(props.quizId) && active.mode === selectedMode.id)
  ) {
    return null
  }
  return active
})
const progressPrompt = computed(() => {
  const active = pendingProgress.value
  if (!active || !quiz.value) return ''
  const oldMode = getQuizMode(active.mode)?.label ?? active.mode
  if (active.quizId === quiz.value.id) {
    return `Your ${oldMode} attempt for “${active.quizTitle}” is unfinished. Continue where you left off, or clear its progress and switch to ${mode.value?.label}?`
  }
  return `Your ${oldMode} attempt for “${active.quizTitle}” is unfinished. Continue it, or clear its progress and start “${quiz.value.title}”?`
})
const startSelectedLabel = computed(() =>
  pendingProgress.value?.quizId === quiz.value?.id ? 'Clear & switch mode' : 'Clear & start quiz',
)

function switchMode(id: string) {
  router.replace({ query: { mode: id } })
}

function continuePrevious() {
  const active = pendingProgress.value
  if (!active) return
  router.replace({ path: `/quizzes/${active.quizId}/play`, query: { mode: active.mode } })
}

function clearAndStartSelected() {
  const active = pendingProgress.value
  if (!active) return
  progress.clearActiveQuiz(active.quizId)
}

function cancelNewQuiz() {
  if (quiz.value) router.replace(`/subjects/${quiz.value.subject_slug}/quizzes`)
  else router.back()
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
          Draft: only its editors can see this quiz
        </p>
      </header>
      <component
        :is="mode!.component"
        v-if="modeOk && !pendingProgress"
        :key="mode!.id"
        :quiz="quiz"
      />
      <EmptyState
        v-else-if="!pendingProgress"
        title="This mode isn't available for this quiz"
        icon="info"
      >
        <div class="flex flex-wrap justify-center gap-2">
          <button v-for="m in otherModes" :key="m.id" class="btn-primary" @click="switchMode(m.id)">
            Play as {{ m.label }}
          </button>
        </div>
      </EmptyState>
    </template>

    <BaseModal
      :open="!!pendingProgress"
      title="Unfinished quiz"
      description="Your current progress is saved in this browser."
      @close="cancelNewQuiz"
    >
      <div
        class="flex items-start gap-3 rounded-xl border border-ctp-yellow/30 bg-ctp-yellow/10 p-4"
      >
        <span
          class="grid size-9 shrink-0 place-items-center rounded-full bg-ctp-yellow/15 text-ctp-yellow"
          aria-hidden="true"
        >
          <AppIcon name="info" :size="18" />
        </span>
        <p class="text-sm leading-6 text-ctp-subtext1">{{ progressPrompt }}</p>
      </div>
      <template #footer>
        <div class="grid w-full grid-cols-1 gap-2 sm:grid-cols-[auto_1fr_1fr]">
          <button class="btn-ghost w-full whitespace-nowrap" @click="cancelNewQuiz">Go back</button>
          <button class="btn-secondary w-full whitespace-nowrap" @click="continuePrevious">
            Continue previous quiz
          </button>
          <button class="btn-danger w-full whitespace-nowrap" @click="clearAndStartSelected">
            {{ startSelectedLabel }}
          </button>
        </div>
      </template>
    </BaseModal>
  </div>
</template>
