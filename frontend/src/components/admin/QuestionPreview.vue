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
      <v-tabs v-model="tab" color="primary" grow density="comfortable">
        <v-tab value="qcm" prepend-icon="mdi-format-list-checks">QCM</v-tab>
        <v-tab value="flashcard" prepend-icon="mdi-cards-outline">Flashcard</v-tab>
      </v-tabs>

      <template v-if="tab === 'qcm'">
        <v-alert v-if="!qcmReady" type="warning" density="compact">
          Not playable as QCM yet: it needs at least 2 options and 1 correct option.
        </v-alert>
        <QcmQuestionCard
          :question="question"
          :options="question.options"
          :selected="selected"
          :submitted="submitted"
          @toggle="toggle"
        />
        <div class="flex justify-end gap-2">
          <v-btn
            variant="tonal"
            size="small"
            prepend-icon="mdi-restart"
            @click="((selected = []), (submitted = false))"
          >
            Reset
          </v-btn>
          <v-btn
            color="primary"
            size="small"
            prepend-icon="mdi-check"
            :disabled="submitted || !selected.length"
            @click="submitted = true"
          >
            Submit
          </v-btn>
        </div>
      </template>
      <template v-else>
        <v-alert v-if="!flashReady" type="warning" density="compact">
          Not playable as a flashcard yet: add an answer (text or image) or mark a correct option.
        </v-alert>
        <FlashcardCard :question="question" :flipped="flipped" @flip="flipped = !flipped" />
      </template>
    </div>
  </BaseModal>
</template>
