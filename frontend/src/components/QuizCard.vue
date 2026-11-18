<script setup lang="ts">
import { computed } from 'vue'

import type { QuizSummary } from '@/api/types'
import { getQuizMode } from '@/quiz-modes/registry'
import { useProgressStore } from '@/stores/progress'

import AppIcon from './AppIcon.vue'

const props = defineProps<{ quiz: QuizSummary }>()
defineEmits<{ select: [quiz: QuizSummary] }>()

const progress = useProgressStore()
const best = computed(() => {
  const r = progress.resultFor(props.quiz.id, 'qcm')
  return r && r.total ? Math.round((r.bestScore / r.total) * 100) : null
})
</script>

<template>
  <button
    class="card group flex h-full flex-col overflow-hidden text-left transition hover:-translate-y-0.5 hover:border-ctp-surface2 hover:shadow-lg"
    @click="$emit('select', quiz)"
  >
    <div class="aspect-[16/7] w-full bg-ctp-surface0">
      <img
        v-if="quiz.cover_image_url"
        :src="quiz.cover_image_url"
        alt=""
        loading="lazy"
        class="h-full w-full object-cover"
      />
      <div v-else class="flex h-full items-center justify-center text-ctp-overlay0">
        <AppIcon name="list-check" :size="36" />
      </div>
    </div>
    <div class="flex flex-1 flex-col gap-2 p-4">
      <h3 class="font-semibold text-ctp-text group-hover:text-ctp-accent">{{ quiz.title }}</h3>
      <p v-if="quiz.description" class="line-clamp-2 text-sm text-ctp-subtext0">
        {{ quiz.description }}
      </p>
      <div class="mt-auto flex flex-wrap items-center gap-2 pt-2 text-xs">
        <span class="rounded-full bg-ctp-surface0 px-2.5 py-1 text-ctp-subtext1">
          {{ quiz.question_count }} question{{ quiz.question_count === 1 ? '' : 's' }}
        </span>
        <span
          v-for="m in quiz.available_modes"
          :key="m"
          class="rounded-full bg-ctp-accent/15 px-2.5 py-1 font-medium text-ctp-accent"
        >
          {{ getQuizMode(m)?.label ?? m }}
        </span>
        <span v-if="best !== null" class="ml-auto text-ctp-subtext0">Best: {{ best }}%</span>
      </div>
    </div>
  </button>
</template>
