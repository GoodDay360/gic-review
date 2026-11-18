<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Question, QuestionOption, QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import ResultScreen from '@/components/ResultScreen.vue'
import { useLightboxStore } from '@/stores/lightbox'
import { useProgressStore, type ModeResult, type QcmProgress } from '@/stores/progress'

import QcmQuestionCard from './QcmQuestionCard.vue'
import { preloadQuestionImages, questionLabel, shuffle } from './utils'

const props = defineProps<{ quiz: QuizDetail }>()

const progress = useProgressStore()
const lightbox = useLightboxStore()

interface Round {
  questions: Question[]
  options: Record<number, QuestionOption[]>
}

const round = ref<Round>({ questions: [], options: {} })
const index = ref(0)
const selected = ref<number[]>([])
const submitted = ref(false)
const results = ref<Record<number, boolean>>({})
const finished = ref(false)
const savedResult = ref<ModeResult | null>(null)

const current = computed(() => round.value.questions[index.value])
const answeredCount = computed(() => Object.keys(results.value).length)
const score = computed(() => Object.values(results.value).filter(Boolean).length)
const total = computed(() => round.value.questions.length)
const missed = computed(() => round.value.questions.filter((q) => results.value[q.id] === false))
const displayScore = computed(() => savedResult.value?.lastScore ?? score.value)
const displayTotal = computed(() => savedResult.value?.total ?? total.value)
const displayPercentage = computed(() =>
  displayTotal.value ? Math.round((displayScore.value / displayTotal.value) * 100) : 0,
)
const displayMissed = computed(() =>
  savedResult.value
    ? props.quiz.questions.filter((q) => savedResult.value!.missedQuestionIds.includes(q.id))
    : missed.value,
)

function saveProgress() {
  progress.saveActiveQuiz({
    quizId: props.quiz.id,
    quizTitle: props.quiz.title,
    mode: 'qcm',
    questionIds: round.value.questions.map((q) => q.id),
    optionIds: Object.fromEntries(
      Object.entries(round.value.options).map(([id, options]) => [
        id,
        options.map((option) => option.id),
      ]),
    ),
    index: index.value,
    selectedOptionIds: selected.value,
    submitted: submitted.value,
    results: Object.fromEntries(
      Object.entries(results.value).map(([id, correct]) => [id, correct]),
    ),
  })
}

function start(subset?: Question[]) {
  const base = subset ?? props.quiz.questions
  const questions = progress.state.shuffleQuestions ? shuffle(base) : base.slice()
  // Option order is shuffled on every attempt.
  const options: Record<number, QuestionOption[]> = {}
  for (const q of questions) options[q.id] = shuffle(q.options)
  round.value = { questions, options }
  index.value = 0
  selected.value = []
  submitted.value = false
  results.value = {}
  finished.value = false
  savedResult.value = null
  saveProgress()
}

function resume(saved: QcmProgress): boolean {
  const questionsById = new Map(props.quiz.questions.map((q) => [q.id, q]))
  const questions = saved.questionIds
    .map((id) => questionsById.get(id))
    .filter((q): q is Question => !!q)
  if (questions.length === 0) return false

  const options = Object.fromEntries(
    questions.map((question) => {
      const savedIds = saved.optionIds[String(question.id)] ?? []
      const optionsById = new Map(question.options.map((option) => [option.id, option]))
      const ordered = savedIds
        .map((id) => optionsById.get(id))
        .filter((option): option is QuestionOption => !!option)
      for (const option of question.options) {
        if (!ordered.some((savedOption) => savedOption.id === option.id)) ordered.push(option)
      }
      return [question.id, ordered]
    }),
  )
  const indexValue = Math.min(Math.max(saved.index, 0), questions.length - 1)
  const activeQuestion = questions[indexValue]!
  const selectedIds = new Set(activeQuestion.options.map((option) => option.id))
  const roundQuestionIds = new Set(questions.map((question) => question.id))
  const resultsById = Object.fromEntries(
    Object.entries(saved.results)
      .filter(([id]) => roundQuestionIds.has(Number(id)))
      .map(([id, correct]) => [Number(id), correct]),
  )
  round.value = { questions, options }
  index.value = indexValue
  selected.value = saved.selectedOptionIds.filter((id) => selectedIds.has(id))
  submitted.value = saved.submitted && Object.hasOwn(resultsById, String(activeQuestion.id))
  results.value = resultsById
  finished.value = false
  savedResult.value = null
  return true
}

function showSavedResult() {
  const result = progress.resultFor(props.quiz.id, 'qcm')
  if (result) {
    savedResult.value = result
    finished.value = true
  } else {
    start()
  }
}

function toggle(optionId: number) {
  if (submitted.value || !current.value) return
  if (current.value.multi_select) {
    selected.value = selected.value.includes(optionId)
      ? selected.value.filter((id) => id !== optionId)
      : [...selected.value, optionId]
  } else {
    selected.value = [optionId]
  }
  saveProgress()
}

function submit() {
  const q = current.value
  if (!q || submitted.value || selected.value.length === 0) return
  const correct = q.options.filter((o) => o.is_correct).map((o) => o.id)
  results.value[q.id] =
    correct.length === selected.value.length && correct.every((id) => selected.value.includes(id))
  submitted.value = true
  saveProgress()
}

function next() {
  if (!submitted.value) return
  if (index.value + 1 >= total.value) {
    finished.value = true
    progress.record(
      props.quiz.id,
      'qcm',
      score.value,
      total.value,
      missed.value.map((q) => q.id),
    )
    progress.clearActiveQuiz(props.quiz.id)
    savedResult.value = progress.resultFor(props.quiz.id, 'qcm') ?? null
    return
  }
  index.value++
  selected.value = []
  submitted.value = false
  saveProgress()
}

// Preload the next question's images.
watch(index, (i) => preloadQuestionImages(round.value.questions[i + 1]), { immediate: true })

function onKey(e: KeyboardEvent) {
  if (finished.value || lightbox.src) return
  const tag = (e.target as HTMLElement | null)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA') return
  const q = current.value
  if (!q) return
  const n = Number.parseInt(e.key, 10)
  if (!Number.isNaN(n) && n >= 1 && n <= 9) {
    const opt = round.value.options[q.id]?.[n - 1]
    if (opt) toggle(opt.id)
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (submitted.value) next()
    else submit()
  }
}

onMounted(() => {
  const active = progress.activeQuizFor(props.quiz.id)
  if (active?.mode === 'qcm') {
    if (!resume(active)) {
      progress.clearActiveQuiz(props.quiz.id)
      showSavedResult()
    }
  } else {
    showSavedResult()
  }
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

function toggleShuffle() {
  progress.state.shuffleQuestions = !progress.state.shuffleQuestions
  start()
}
</script>

<template>
  <div class="space-y-6">
    <template v-if="!finished && current">
      <div class="space-y-2">
        <div class="flex flex-wrap items-center justify-between gap-2 text-sm text-ctp-subtext0">
          <span>Question {{ index + 1 }} / {{ total }}</span>
          <span class="font-medium text-ctp-text">Score: {{ score }} / {{ answeredCount }}</span>
          <button
            class="btn-ghost btn-sm"
            :aria-pressed="progress.state.shuffleQuestions"
            title="Shuffle question order (restarts the quiz)"
            @click="toggleShuffle"
          >
            <AppIcon name="shuffle" :size="16" />
            Shuffle questions: {{ progress.state.shuffleQuestions ? 'on' : 'off' }}
          </button>
        </div>
        <div
          class="h-2 overflow-hidden rounded-full bg-ctp-surface0"
          role="progressbar"
          :aria-valuenow="index + (submitted ? 1 : 0)"
          aria-valuemin="0"
          :aria-valuemax="total"
          aria-label="Quiz progress"
        >
          <div
            class="h-full rounded-full bg-ctp-accent transition-all duration-300"
            :style="{ width: `${((index + (submitted ? 1 : 0)) / total) * 100}%` }"
          />
        </div>
      </div>

      <QcmQuestionCard
        :key="current.id"
        :question="current"
        :options="round.options[current.id] ?? []"
        :selected="selected"
        :submitted="submitted"
        @toggle="toggle"
      />

      <div class="flex items-center justify-between gap-3">
        <p class="hidden text-xs text-ctp-overlay1 sm:block">
          Keys: 1–9 select · Enter submit / next
        </p>
        <button
          v-if="!submitted"
          class="btn-primary ml-auto"
          :disabled="selected.length === 0"
          @click="submit"
        >
          Submit answer
        </button>
        <button v-else class="btn-primary ml-auto" @click="next">
          {{ index + 1 >= total ? 'See results' : 'Next' }}
          <AppIcon name="chevron-right" :size="16" />
        </button>
      </div>
    </template>

    <ResultScreen
      v-else-if="finished"
      title="Quiz complete"
      :percentage="displayPercentage"
      :stats="[
        { label: 'Score', value: `${displayScore} / ${displayTotal}`, tone: 'accent' },
        { label: 'Correct', value: displayScore, tone: 'green' },
        { label: 'Missed', value: displayMissed.length, tone: 'red' },
      ]"
    >
      <div v-if="displayMissed.length" class="text-left">
        <h3 class="mb-2 text-sm font-semibold text-ctp-subtext1">Missed questions</h3>
        <ul class="space-y-2">
          <li
            v-for="q in displayMissed"
            :key="q.id"
            class="flex items-center gap-3 rounded-lg bg-ctp-surface0 p-3 text-sm"
          >
            <img
              v-if="q.image_url"
              :src="q.image_url"
              :alt="q.image_alt ?? ''"
              loading="lazy"
              class="h-10 w-14 rounded object-cover"
            />
            <span class="line-clamp-2 text-ctp-text">{{ questionLabel(q) }}</span>
          </li>
        </ul>
      </div>
      <template #actions>
        <button class="btn-primary" @click="start()">
          <AppIcon name="refresh" :size="16" /> Redo quiz
        </button>
        <button v-if="displayMissed.length" class="btn-secondary" @click="start(displayMissed)">
          Retry missed only
        </button>
      </template>
    </ResultScreen>
  </div>
</template>
