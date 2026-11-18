<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import type { Question, QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import ResultScreen from '@/components/ResultScreen.vue'
import { useLightboxStore } from '@/stores/lightbox'
import { useProgressStore } from '@/stores/progress'

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

const current = computed(() => queue.value[index.value])
const knewCount = computed(() => knownIds.value.size)
const remaining = computed(() => queue.value.length - index.value)

function start(subset?: Question[]) {
  const base = subset ?? props.quiz.questions
  queue.value = progress.state.shuffleQuestions ? shuffle(base) : base.slice()
  deckSize.value = queue.value.length
  index.value = 0
  flipped.value = false
  missedIds.value = new Set()
  knownIds.value = new Set()
  finished.value = false
}

function advance() {
  flipped.value = false
  if (index.value + 1 >= queue.value.length) {
    finished.value = true
    progress.record(props.quiz.id, 'flashcard', knewCount.value, deckSize.value, [
      ...missedIds.value,
    ])
  } else {
    index.value++
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
  missedIds.value = new Set(missedIds.value).add(card.id)
  queue.value.push(card) // comes back at the end of the round
  advance()
}

function prev() {
  if (index.value > 0) {
    index.value--
    flipped.value = false
  }
}
function next() {
  if (index.value + 1 < queue.value.length) {
    index.value++
    flipped.value = false
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
      flipped.value = !flipped.value
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
  start()
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

const missedCards = computed(() => props.quiz.questions.filter((q) => missedIds.value.has(q.id)))
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
        @flip="flipped = !flipped"
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
      :percentage="deckSize ? Math.round((knewCount / deckSize) * 100) : 0"
      :stats="[
        { label: 'Cards', value: deckSize },
        { label: 'Knew first time', value: knewCount, tone: 'green' },
        { label: 'Needed review', value: missedIds.size, tone: 'red' },
      ]"
    >
      <template #actions>
        <button v-if="missedCards.length" class="btn-primary" @click="start(missedCards)">
          Study missed cards again
        </button>
        <button class="btn-secondary" @click="start()">
          <AppIcon name="refresh" :size="16" /> Restart all cards
        </button>
      </template>
    </ResultScreen>
  </div>
</template>
