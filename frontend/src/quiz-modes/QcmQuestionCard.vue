<script setup lang="ts">
/** Presentational QCM question (used by the player and by the admin preview). */
import { computed } from 'vue'

import type { Question, QuestionOption } from '@/api/types'
import ZoomableImage from '@/components/ZoomableImage.vue'

const props = defineProps<{
  question: Question
  options: QuestionOption[]
  selected: number[]
  submitted: boolean
}>()
const emit = defineEmits<{ toggle: [optionId: number] }>()

const hasImages = computed(() => props.options.some((o) => o.image_url))
const isCorrect = computed(() => {
  const correct = props.options.filter((o) => o.is_correct).map((o) => o.id)
  return (
    correct.length === props.selected.length && correct.every((id) => props.selected.includes(id))
  )
})

function state(o: QuestionOption): 'idle' | 'selected' | 'correct' | 'wrong' | 'missed' {
  const picked = props.selected.includes(o.id)
  if (!props.submitted) return picked ? 'selected' : 'idle'
  if (o.is_correct) return picked ? 'correct' : 'missed'
  return picked ? 'wrong' : 'idle'
}

const stateClass = {
  idle: 'border-ctp-surface1 hover:border-ctp-overlay0',
  selected: 'border-ctp-accent bg-ctp-accent/10 ring-1 ring-ctp-accent',
  correct: 'border-ctp-green bg-ctp-green/15 ring-1 ring-ctp-green',
  wrong: 'border-ctp-red bg-ctp-red/15 ring-1 ring-ctp-red',
  missed: 'border-ctp-green border-dashed',
}

const letter = (i: number) => String.fromCharCode(65 + i)
</script>

<template>
  <article class="space-y-5">
    <div class="space-y-3">
      <p
        v-if="question.prompt"
        class="text-lg leading-relaxed font-medium whitespace-pre-line text-ctp-text"
      >
        {{ question.prompt }}
      </p>
      <ZoomableImage
        v-if="question.image_url"
        :src="question.image_url"
        :alt="question.image_alt"
        img-class="max-h-[50vh] w-full rounded-lg bg-ctp-crust object-contain"
      />
      <p class="text-sm text-ctp-subtext0">
        {{ question.multi_select ? 'Select all answers that apply.' : 'Select one answer.' }}
      </p>
    </div>

    <div
      :role="question.multi_select ? 'group' : 'radiogroup'"
      aria-label="Answer options"
      :class="hasImages ? 'grid grid-cols-1 gap-3 sm:grid-cols-2' : 'flex flex-col gap-2'"
    >
      <v-card
        v-for="(o, i) in options"
        :key="o.id"
        :ripple="!submitted"
        :style="{ animationDelay: `${i * 60}ms` }"
        :role="question.multi_select ? 'checkbox' : 'radio'"
        :aria-checked="selected.includes(o.id)"
        :aria-disabled="submitted"
        :tabindex="submitted ? -1 : 0"
        class="animate__animated animate__fadeInUp animate__faster flex cursor-pointer gap-3 rounded-xl border-2 bg-ctp-mantle p-3 text-left transition"
        :class="[
          stateClass[state(o)],
          hasImages ? 'flex-col' : 'items-center',
          submitted ? 'cursor-default' : '',
        ]"
        @click="!submitted && emit('toggle', o.id)"
        @keydown.enter.space.prevent="!submitted && emit('toggle', o.id)"
      >
        <div class="flex items-center gap-3">
          <span
            class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full border border-ctp-surface2 text-xs font-semibold text-ctp-subtext1"
          >
            <v-icon
              v-if="state(o) === 'correct' || state(o) === 'missed'"
              icon="mdi-check"
              size="16"
              class="text-ctp-green"
            />
            <v-icon
              v-else-if="state(o) === 'wrong'"
              icon="mdi-close"
              size="16"
              class="text-ctp-red"
            />
            <template v-else>{{ letter(i) }}</template>
          </span>
          <span v-if="o.text" class="text-ctp-text">{{ o.text }}</span>
        </div>
        <ZoomableImage
          v-if="o.image_url"
          :src="o.image_url"
          :alt="o.image_alt"
          img-class="aspect-video w-full rounded-md bg-ctp-crust object-contain"
        />
      </v-card>
    </div>

    <v-alert
      v-if="submitted"
      :type="isCorrect ? 'success' : 'error'"
      :icon="isCorrect ? 'mdi-check-circle-outline' : 'mdi-close-circle-outline'"
      :title="isCorrect ? 'Correct!' : 'Not quite.'"
      border="start"
      class="animate__animated"
      :class="isCorrect ? 'animate__tada' : 'animate__headShake'"
      aria-live="polite"
    >
      <p v-if="question.explanation" class="mt-1 whitespace-pre-line text-ctp-text">
        {{ question.explanation }}
      </p>
      <ZoomableImage
        v-if="question.explanation_image_url"
        :src="question.explanation_image_url"
        alt="Explanation diagram"
        img-class="mt-3 max-h-80 w-full rounded-lg bg-ctp-crust object-contain"
      />
    </v-alert>
  </article>
</template>
