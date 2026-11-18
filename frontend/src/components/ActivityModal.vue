<script setup lang="ts">
import { useRouter } from 'vue-router'

import type { Subject } from '@/api/types'

import AppIcon from './AppIcon.vue'
import BaseModal from './BaseModal.vue'

const props = defineProps<{ subject: Subject | null }>()
const emit = defineEmits<{ close: [] }>()
const router = useRouter()

function go(path: string) {
  emit('close')
  router.push(path)
}
</script>

<template>
  <BaseModal
    :open="!!props.subject"
    :title="props.subject?.name ?? ''"
    description="What would you like to do?"
    @close="emit('close')"
  >
    <div v-if="props.subject" class="grid gap-3 sm:grid-cols-2">
      <button
        class="card flex flex-col items-start gap-2 p-5 text-left transition hover:border-ctp-accent disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:border-ctp-surface0"
        :disabled="!props.subject.has_review"
        @click="go(`/subjects/${props.subject.slug}/review`)"
      >
        <span class="rounded-lg bg-ctp-surface0 p-2 text-ctp-accent"><AppIcon name="book" /></span>
        <span class="font-semibold text-ctp-text">Interactive Review</span>
        <span class="text-sm text-ctp-subtext0">
          {{ props.subject.has_review ? 'Read the interactive study guide.' : 'Coming soon' }}
        </span>
      </button>
      <button
        class="card flex flex-col items-start gap-2 p-5 text-left transition hover:border-ctp-accent"
        @click="go(`/subjects/${props.subject.slug}/quizzes`)"
      >
        <span class="rounded-lg bg-ctp-surface0 p-2 text-ctp-accent"
          ><AppIcon name="list-check"
        /></span>
        <span class="font-semibold text-ctp-text">Quiz</span>
        <span class="text-sm text-ctp-subtext0">Practice with QCM or flashcards.</span>
      </button>
    </div>
  </BaseModal>
</template>
