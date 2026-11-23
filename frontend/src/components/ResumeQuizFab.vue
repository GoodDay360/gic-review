<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

import { getQuizMode } from '@/quiz-modes/registry'
import { useProgressStore } from '@/stores/progress'

/** Floating shortcut back to the unfinished quiz, shown on every page except the dashboard and interactive reviews. */
const route = useRoute()
const progress = useProgressStore()

const active = computed(() => progress.state.activeQuiz)

const visible = computed(() => {
  const quiz = active.value
  if (!quiz) return false
  // Hidden on the staff dashboard.
  if (route.path === '/admin' || route.path.startsWith('/admin/')) return false
  // Hidden while reading an interactive review, so it doesn't cover the content.
  if (route.name === 'review') return false
  // Already playing this quiz: the button would point at the current page.
  if (route.name === 'play' && Number(route.params.quizId) === quiz.quizId) return false
  return true
})

const mode = computed(() => getQuizMode(active.value?.mode))

const stepText = computed(() => {
  const quiz = active.value
  if (!quiz) return ''
  const total = quiz.mode === 'qcm' ? quiz.questionIds.length : quiz.queue.length
  if (!total) return ''
  const unit = quiz.mode === 'qcm' ? 'Question' : 'Card'
  return `${unit} ${Math.min(quiz.index + 1, total)} / ${total}`
})

/** "I4 · Human Computer Interaction" (empty for progress saved before names were stored). */
const whereText = computed(() =>
  [active.value?.categoryName, active.value?.subjectName].filter(Boolean).join(' · '),
)

const target = computed(() =>
  active.value
    ? { path: `/quizzes/${active.value.quizId}/play`, query: { mode: active.value.mode } }
    : '/',
)
</script>

<template>
  <!-- Zero-height sticky anchor placed right before the footer: the button floats
       1rem above the viewport bottom, and stops above the footer once it scrolls in. -->
  <div
    class="pointer-events-none sticky z-[1005] h-0"
    :style="{ bottom: 'env(safe-area-inset-bottom)' }"
  >
    <Transition
      enter-active-class="animate__animated animate__fadeInUp animate__faster"
      leave-active-class="animate__animated animate__fadeOutDown animate__faster"
    >
      <RouterLink
        v-if="visible && active"
        :to="target"
        class="pointer-events-auto absolute right-4 bottom-4 flex max-w-[calc(100vw-2rem)] items-center gap-3 rounded-full bg-ctp-accent py-2 pr-5 pl-2 text-ctp-base shadow-lg transition-transform hover:-translate-y-0.5 hover:shadow-xl focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ctp-accent"
        :aria-label="`Continue unfinished quiz: ${active.quizTitle}${whereText ? ` (${whereText})` : ''}`"
      >
        <span
          class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-ctp-base/20"
        >
          <v-icon :icon="mode?.icon ?? 'mdi-play'" size="22" />
        </span>
        <span class="flex min-w-0 flex-col leading-tight">
          <span class="text-xs font-medium opacity-80">
            Continue {{ mode?.label ?? 'quiz' }}<span v-if="stepText"> · {{ stepText }}</span>
          </span>
          <span class="max-w-[14rem] truncate text-sm font-semibold sm:max-w-[18rem]">
            {{ active.quizTitle }}
          </span>
          <span
            v-if="whereText"
            class="flex max-w-[14rem] items-center gap-1 text-xs opacity-80 sm:max-w-[18rem]"
          >
            <v-icon icon="mdi-folder-outline" size="12" class="shrink-0" />
            <span class="truncate">{{ whereText }}</span>
          </span>
        </span>
        <v-icon icon="mdi-chevron-right" size="20" class="shrink-0 opacity-80" />
      </RouterLink>
    </Transition>
  </div>
</template>
