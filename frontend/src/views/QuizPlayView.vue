<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'

import { api } from '@/api/client'
import type { QuizDetail } from '@/api/types'
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
      <v-btn
        variant="text"
        prepend-icon="mdi-arrow-left"
        :to="quiz ? `/subjects/${quiz.subject_slug}/quizzes` : '/'"
        class="-ml-3"
        >Back to quizzes
      </v-btn>
      <v-btn-toggle
        v-if="quiz && otherModes.length > 1"
        :model-value="mode?.id"
        mandatory
        density="comfortable"
        variant="outlined"
        divided
        rounded="lg"
        color="primary"
        aria-label="Quiz mode"
      >
        <v-btn
          v-for="m in otherModes"
          :key="m.id"
          :value="m.id"
          :prepend-icon="m.icon"
          @click="switchMode(m.id)"
        >
          {{ m.label }}
        </v-btn>
      </v-btn-toggle>
    </div>

    <div v-if="loading" class="space-y-4">
      <SkeletonBlock class="h-8 w-2/3" />
      <SkeletonBlock class="h-72" />
    </div>
    <EmptyState
      v-else-if="error"
      title="Couldn't load this quiz"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="run"
        >Try again</v-btn
      >
    </EmptyState>
    <template v-else-if="quiz">
      <header class="animate__animated animate__fadeInDown mb-6">
        <p class="text-sm text-ctp-subtext0">
          {{ quiz.subject_name }}<span v-if="mode"> · {{ mode.label }}</span>
        </p>
        <h1 class="text-2xl font-bold text-ctp-text">{{ quiz.title }}</h1>
        <v-chip
          v-if="!quiz.is_published"
          color="warning"
          variant="tonal"
          size="small"
          prepend-icon="mdi-eye-off-outline"
          class="mt-2"
        >
          Draft: only its editors can see this quiz
        </v-chip>
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
        icon="mdi-information-outline"
      >
        <div class="flex flex-wrap justify-center gap-2">
          <v-btn v-for="m in otherModes" :key="m.id" color="primary" @click="switchMode(m.id)">
            Play as {{ m.label }}
          </v-btn>
        </div>
      </EmptyState>
    </template>

    <BaseModal
      :open="!!pendingProgress"
      title="Unfinished quiz"
      description="Your current progress is saved in this browser."
      @close="cancelNewQuiz"
    >
      <v-alert type="warning" icon="mdi-progress-clock" :text="progressPrompt" />
      <template #footer>
        <div class="grid w-full grid-cols-1 gap-2 sm:grid-cols-[auto_1fr_1fr]">
          <v-btn variant="text" class="w-full whitespace-nowrap" @click="cancelNewQuiz"
            >Go back</v-btn
          >
          <v-btn
            variant="tonal"
            color="primary"
            prepend-icon="mdi-play"
            class="w-full whitespace-nowrap"
            @click="continuePrevious"
          >
            Continue previous quiz
          </v-btn>
          <v-btn
            color="error"
            prepend-icon="mdi-restart"
            class="w-full whitespace-nowrap"
            @click="clearAndStartSelected"
          >
            {{ startSelectedLabel }}
          </v-btn>
        </div>
      </template>
    </BaseModal>
  </div>
</template>
