<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Question, QuizDetail } from '@/api/types'
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
    categoryName: props.quiz.category_name,
    subjectName: props.quiz.subject_name,
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
  queue.value = progress.state.shuffleQuestionOrder ? shuffle(base) : base.slice()
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
        <span class="flex flex-wrap gap-1.5">
          <v-chip size="small" color="success" variant="tonal" prepend-icon="mdi-check">
            {{ knewCount }} known
          </v-chip>
          <v-chip size="small" color="error" variant="tonal" prepend-icon="mdi-replay">
            {{ missedIds.size }} to review
          </v-chip>
          <v-chip size="small" variant="tonal">{{ remaining }} left</v-chip>
        </span>
      </div>
      <v-progress-linear
        :model-value="(index / queue.length) * 100"
        color="primary"
        bg-color="ctp-surface1"
        height="8"
        rounded
        aria-label="Deck progress"
      />

      <FlashcardCard
        :key="`${current.id}-${index}`"
        :question="current"
        :flipped="flipped"
        @flip="toggleFlip"
      />

      <div class="flex items-center justify-between gap-2">
        <v-btn
          variant="text"
          icon="mdi-chevron-left"
          :disabled="index === 0"
          aria-label="Previous card"
          @click="prev"
        />
        <div class="flex flex-1 justify-center gap-2">
          <v-btn prepend-icon="mdi-close" color="error" variant="tonal" @click="didntKnow">
            Didn't know
          </v-btn>
          <v-btn prepend-icon="mdi-check" color="success" variant="tonal" @click="knew">
            I knew it
          </v-btn>
        </div>
        <v-btn
          variant="text"
          icon="mdi-chevron-right"
          :disabled="index + 1 >= queue.length"
          aria-label="Next card"
          @click="next"
        />
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
        <v-btn
          v-if="displayMissedCards.length"
          color="primary"
          prepend-icon="mdi-target"
          @click="start(displayMissedCards)"
        >
          Study missed cards again
        </v-btn>
        <v-btn variant="tonal" prepend-icon="mdi-refresh" @click="start()">Redo quiz </v-btn>
      </template>
    </ResultScreen>
  </div>
</template>
