<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Question, QuestionOption, QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import ResultScreen from '@/components/ResultScreen.vue'
import { useLightboxStore } from '@/stores/lightbox'
import { useProgressStore } from '@/stores/progress'

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

const current = computed(() => round.value.questions[index.value])
const answeredCount = computed(() => Object.keys(results.value).length)
const score = computed(() => Object.values(results.value).filter(Boolean).length)
const total = computed(() => round.value.questions.length)
const percentage = computed(() => (total.value ? Math.round((score.value / total.value) * 100) : 0))
const missed = computed(() => round.value.questions.filter((q) => results.value[q.id] === false))

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
}

function submit() {
  const q = current.value
  if (!q || submitted.value || selected.value.length === 0) return
  const correct = q.options.filter((o) => o.is_correct).map((o) => o.id)
  results.value[q.id] =
    correct.length === selected.value.length && correct.every((id) => selected.value.includes(id))
  submitted.value = true
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
    return
  }
  index.value++
  selected.value = []
  submitted.value = false
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
  start()
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
      :percentage="percentage"
      :stats="[
        { label: 'Score', value: `${score} / ${total}`, tone: 'accent' },
        { label: 'Correct', value: score, tone: 'green' },
        { label: 'Missed', value: missed.length, tone: 'red' },
      ]"
    >
      <div v-if="missed.length" class="text-left">
        <h3 class="mb-2 text-sm font-semibold text-ctp-subtext1">Missed questions</h3>
        <ul class="space-y-2">
          <li
            v-for="q in missed"
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
          <AppIcon name="refresh" :size="16" /> Retry
        </button>
        <button v-if="missed.length" class="btn-secondary" @click="start(missed)">
          Retry missed only
        </button>
      </template>
    </ResultScreen>
  </div>
</template>
