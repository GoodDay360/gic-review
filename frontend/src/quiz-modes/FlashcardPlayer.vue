<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Question, QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import ResultScreen from '@/components/ResultScreen.vue'
import { useLightboxStore } from '@/stores/lightbox'
import { useProgressStore, type FlashcardProgress, type ModeResult } from '@/stores/progress'

import FlashcardCard from './FlashcardCard.vue'
import { preloadQuestionImages, shuffle } from './utils'

const props = defineProps<{ quiz: QuizDetail }>()

const progress = useProgressStore()
const lightbox = useLightboxStore()

/** Cards still to go through this round; "Didn't know" re-appends a card at the end. */
const queue = ref<Question[]>([])
const index = ref(0)
const flipped = ref(false)
const deckSize = ref(0)
const missedIds = ref<Set<number>>(new Set())
/** Cards marked "I knew it" without ever being marked "Didn't know" this round. */
const knownIds = ref<Set<number>>(new Set())
const finished = ref(false)
const savedResult = ref<ModeResult | null>(null)

const current = computed(() => queue.value[index.value])
const knewCount = computed(() => knownIds.value.size)
const remaining = computed(() => queue.value.length - index.value)
const displayKnewCount = computed(() => savedResult.value?.lastScore ?? knewCount.value)
const displayDeckSize = computed(() => savedResult.value?.total ?? deckSize.value)
const displayMissedCards = computed(() => {
  const ids = savedResult.value?.missedQuestionIds
  return ids
    ? props.quiz.questions.filter((question) => ids.includes(question.id))
    : props.quiz.questions.filter((question) => missedIds.value.has(question.id))
})
const displayMissedCount = computed(
  () => savedResult.value?.missedQuestionIds.length ?? missedIds.value.size,
)
const displayPercentage = computed(() =>
  displayDeckSize.value ? Math.round((displayKnewCount.value / displayDeckSize.value) * 100) : 0,
)

function saveProgress() {
  progress.saveActiveQuiz({
    quizId: props.quiz.id,
    quizTitle: props.quiz.title,
    mode: 'flashcard',
    queue: queue.value.map((question) => question.id),
    index: index.value,
    flipped: flipped.value,
    deckSize: deckSize.value,
    missedIds: [...missedIds.value],
    knownIds: [...knownIds.value],
  })
}

function start(subset?: Question[]) {
  const base = subset ?? props.quiz.questions
  queue.value = progress.state.shuffleQuestions ? shuffle(base) : base.slice()
  deckSize.value = queue.value.length
  index.value = 0
  flipped.value = false
  missedIds.value = new Set()
  knownIds.value = new Set()
  finished.value = false
  savedResult.value = null
  saveProgress()
}

function resume(saved: FlashcardProgress): boolean {
  const questionsById = new Map(props.quiz.questions.map((question) => [question.id, question]))
  const restoredQueue = saved.queue
    .map((id) => questionsById.get(id))
    .filter((question): question is Question => !!question)
  if (restoredQueue.length === 0) return false

  queue.value = restoredQueue
  index.value = Math.min(Math.max(saved.index, 0), restoredQueue.length - 1)
  flipped.value = saved.flipped
  deckSize.value = Math.max(saved.deckSize, 0)
  missedIds.value = new Set(saved.missedIds.filter((id) => questionsById.has(id)))
  knownIds.value = new Set(saved.knownIds.filter((id) => questionsById.has(id)))
  finished.value = false
  savedResult.value = null
  return true
}

function showSavedResult() {
  const result = progress.resultFor(props.quiz.id, 'flashcard')
  if (result) {
    savedResult.value = result
    finished.value = true
  } else {
    start()
  }
}

function advance() {
  flipped.value = false
  if (index.value + 1 >= queue.value.length) {
    finished.value = true
    progress.record(props.quiz.id, 'flashcard', knewCount.value, deckSize.value, [
      ...missedIds.value,
    ])
    progress.clearActiveQuiz(props.quiz.id)
    savedResult.value = progress.resultFor(props.quiz.id, 'flashcard') ?? null
  } else {
    index.value++
    saveProgress()
  }
}

function knew() {
  const card = current.value
  if (!card) return
  if (!missedIds.value.has(card.id)) knownIds.value = new Set(knownIds.value).add(card.id)
  advance()
}

function didntKnow() {
  const card = current.value
  if (!card) return
  if (!missedIds.value.has(card.id)) {
    missedIds.value = new Set(missedIds.value).add(card.id)
    knownIds.value = new Set(knownIds.value)
    knownIds.value.delete(card.id)
    queue.value.push(card) // comes back once at the end of the round
  }
  advance()
}

function toggleFlip() {
  flipped.value = !flipped.value
  saveProgress()
}

function prev() {
  if (index.value > 0) {
    index.value--
    flipped.value = false
    saveProgress()
  }
}
function next() {
  if (index.value + 1 < queue.value.length) {
    index.value++
    flipped.value = false
    saveProgress()
  }
}

watch(index, (i) => preloadQuestionImages(queue.value[i + 1]), { immediate: true })

function onKey(e: KeyboardEvent) {
  if (finished.value || lightbox.src) return
  const tag = (e.target as HTMLElement | null)?.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return
  switch (e.key) {
    case ' ':
      e.preventDefault()
      toggleFlip()
      break
    case 'ArrowLeft':
      prev()
      break
    case 'ArrowRight':
      next()
      break
    case '1':
      knew()
      break
    case '2':
      didntKnow()
      break
  }
}

onMounted(() => {
  const active = progress.activeQuizFor(props.quiz.id)
  if (active?.mode === 'flashcard') {
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
</script>

<template>
  <div class="space-y-5">
    <template v-if="!finished && current">
      <div class="flex flex-wrap items-center justify-between gap-2 text-sm text-ctp-subtext0">
        <span>Card {{ index + 1 }} / {{ queue.length }}</span>
        <span>
          <span class="text-ctp-green">{{ knewCount }} known</span> ·
          <span class="text-ctp-red">{{ missedIds.size }} to review</span> · {{ remaining }} left
        </span>
      </div>
      <div
        class="h-2 overflow-hidden rounded-full bg-ctp-surface0"
        role="progressbar"
        :aria-valuenow="index"
        aria-valuemin="0"
        :aria-valuemax="queue.length"
        aria-label="Deck progress"
      >
        <div
          class="h-full rounded-full bg-ctp-accent transition-all"
          :style="{ width: `${(index / queue.length) * 100}%` }"
        />
      </div>

      <FlashcardCard
        :key="`${current.id}-${index}`"
        :question="current"
        :flipped="flipped"
        @flip="toggleFlip"
      />

      <div class="flex items-center justify-between gap-2">
        <button class="btn-ghost" :disabled="index === 0" aria-label="Previous card" @click="prev">
          <AppIcon name="chevron-left" />
        </button>
        <div class="flex flex-1 justify-center gap-2">
          <button
            class="btn border border-ctp-red text-ctp-red hover:bg-ctp-red/10"
            @click="didntKnow"
          >
            <AppIcon name="x" :size="16" /> Didn't know
          </button>
          <button
            class="btn border border-ctp-green text-ctp-green hover:bg-ctp-green/10"
            @click="knew"
          >
            <AppIcon name="check" :size="16" /> I knew it
          </button>
        </div>
        <button
          class="btn-ghost"
          :disabled="index + 1 >= queue.length"
          aria-label="Next card"
          @click="next"
        >
          <AppIcon name="chevron-right" />
        </button>
      </div>
      <p class="hidden text-center text-xs text-ctp-overlay1 sm:block">
        Space flip · ← / → previous / next · 1 knew it · 2 didn't know
      </p>
    </template>

    <ResultScreen
      v-else-if="finished"
      title="Deck complete"
      :percentage="displayPercentage"
      :stats="[
        { label: 'Cards', value: displayDeckSize },
        { label: 'Knew first time', value: displayKnewCount, tone: 'green' },
        { label: 'Needed review', value: displayMissedCount, tone: 'red' },
      ]"
    >
      <template #actions>
        <button
          v-if="displayMissedCards.length"
          class="btn-primary"
          @click="start(displayMissedCards)"
        >
          Study missed cards again
        </button>
        <button class="btn-secondary" @click="start()">
          <AppIcon name="refresh" :size="16" /> Redo quiz
        </button>
      </template>
    </ResultScreen>
  </div>
</template>
