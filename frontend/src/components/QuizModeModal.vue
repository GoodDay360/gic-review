<script setup lang="ts">
/** Mode picker built entirely from the quiz-mode registry. */
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import type { QuizSummary } from '@/api/types'
import { availableModes } from '@/quiz-modes/registry'

import AppIcon from './AppIcon.vue'
import BaseModal from './BaseModal.vue'

const props = defineProps<{ quiz: QuizSummary | null }>()
const emit = defineEmits<{ close: [] }>()
const router = useRouter()

const modes = computed(() => (props.quiz ? availableModes(props.quiz) : []))

function choose(modeId: string) {
  if (!props.quiz) return
  const id = props.quiz.id
  emit('close')
  router.push({ path: `/quizzes/${id}/play`, query: { mode: modeId } })
}
</script>

<template>
  <BaseModal
    :open="!!quiz"
    :title="quiz?.title ?? ''"
    description="Choose how you want to study."
    @close="emit('close')"
  >
    <div v-if="modes.length" class="grid gap-3 sm:grid-cols-2">
      <button
        v-for="m in modes"
        :key="m.id"
        class="card flex flex-col items-start gap-2 p-5 text-left transition hover:border-ctp-accent"
        @click="choose(m.id)"
      >
        <span class="rounded-lg bg-ctp-surface0 p-2 text-ctp-accent"
          ><AppIcon :name="m.icon"
        /></span>
        <span class="font-semibold text-ctp-text">{{ m.label }}</span>
        <span class="text-sm text-ctp-subtext0">{{ m.description }}</span>
      </button>
    </div>
    <p v-else class="text-sm text-ctp-subtext0">This quiz has no playable mode yet.</p>
  </BaseModal>
</template>
