<script setup lang="ts">
/** Shows a question exactly as students see it, in QCM and Flashcard mode. */
import { computed, ref, watch } from 'vue'

import type { Question } from '@/api/types'
import BaseModal from '@/components/BaseModal.vue'
import FlashcardCard from '@/quiz-modes/FlashcardCard.vue'
import QcmQuestionCard from '@/quiz-modes/QcmQuestionCard.vue'

const props = defineProps<{ question: Question | null }>()
const emit = defineEmits<{ close: [] }>()

const tab = ref<'qcm' | 'flashcard'>('qcm')
const selected = ref<number[]>([])
const submitted = ref(false)
const flipped = ref(false)

watch(
  () => props.question,
  () => {
    selected.value = []
    submitted.value = false
    flipped.value = false
  },
)

const qcmReady = computed(
  () =>
    !!props.question &&
    props.question.options.length >= 2 &&
    props.question.options.some((o) => o.is_correct),
)
const flashReady = computed(
  () =>
    !!props.question &&
    (!!props.question.answer_text ||
      !!props.question.answer_image_url ||
      props.question.options.some((o) => o.is_correct)),
)

function toggle(id: number) {
  if (!props.question) return
  selected.value = props.question.multi_select
    ? selected.value.includes(id)
      ? selected.value.filter((x) => x !== id)
      : [...selected.value, id]
    : [id]
}
</script>

<template>
  <BaseModal :open="!!question" title="Student preview" size="lg" @close="emit('close')">
    <div v-if="question" class="space-y-4">
      <div class="flex gap-1 rounded-lg bg-ctp-surface0 p-1" role="tablist">
        <button
          v-for="t in ['qcm', 'flashcard'] as const"
          :key="t"
          role="tab"
          :aria-selected="tab === t"
          class="flex-1 rounded-md px-3 py-1.5 text-sm"
          :class="tab === t ? 'bg-ctp-base font-medium text-ctp-text shadow' : 'text-ctp-subtext0'"
          @click="tab = t"
        >
          {{ t === 'qcm' ? 'QCM' : 'Flashcard' }}
        </button>
      </div>

      <template v-if="tab === 'qcm'">
        <p v-if="!qcmReady" class="rounded-lg bg-ctp-yellow/15 p-3 text-sm text-ctp-yellow">
          Not playable as QCM yet: it needs at least 2 options and 1 correct option.
        </p>
        <QcmQuestionCard
          :question="question"
          :options="question.options"
          :selected="selected"
          :submitted="submitted"
          @toggle="toggle"
        />
        <div class="flex justify-end gap-2">
          <button class="btn-secondary btn-sm" @click="((selected = []), (submitted = false))">
            Reset
          </button>
          <button
            class="btn-primary btn-sm"
            :disabled="submitted || !selected.length"
            @click="submitted = true"
          >
            Submit
          </button>
        </div>
      </template>
      <template v-else>
        <p v-if="!flashReady" class="rounded-lg bg-ctp-yellow/15 p-3 text-sm text-ctp-yellow">
          Not playable as a flashcard yet: add an answer (text or image) or mark a correct option.
        </p>
        <FlashcardCard :question="question" :flipped="flipped" @flip="flipped = !flipped" />
      </template>
    </div>
  </BaseModal>
</template>
