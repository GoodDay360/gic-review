<script setup lang="ts">
import { computed } from 'vue'

import type { QuizSummary } from '@/api/types'
import { getQuizMode } from '@/quiz-modes/registry'
import { useProgressStore } from '@/stores/progress'

const props = defineProps<{ quiz: QuizSummary }>()
defineEmits<{ select: [quiz: QuizSummary] }>()

const progress = useProgressStore()
const best = computed(() => {
  const r = progress.resultFor(props.quiz.id, 'qcm')
  return r && r.total ? Math.round((r.bestScore / r.total) * 100) : null
})
</script>

<template>
  <v-card
    tag="button"
    type="button"
    hover
    class="w-full card-lift group flex h-full flex-col overflow-hidden text-left"
    @click="$emit('select', quiz)"
  >
    <div class="aspect-[16/7] w-full overflow-hidden bg-ctp-surface0">
      <img
        v-if="quiz.cover_image_url"
        :src="quiz.cover_image_url"
        alt=""
        loading="lazy"
        class="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
      />
      <div v-else class="flex h-full items-center justify-center text-ctp-overlay0">
        <v-icon icon="mdi-format-list-checks" size="40" />
      </div>
    </div>
    <div class="flex flex-1 flex-col gap-2 p-4">
      <h3 class="font-semibold text-ctp-text group-hover:text-ctp-accent">{{ quiz.title }}</h3>
      <p v-if="quiz.description" class="line-clamp-2 text-sm text-ctp-subtext0">
        {{ quiz.description }}
      </p>
      <div class="mt-auto flex flex-wrap items-center gap-2 pt-2">
        <v-chip size="small" variant="tonal" prepend-icon="mdi-help-circle-outline">
          {{ quiz.question_count }} question{{ quiz.question_count === 1 ? '' : 's' }}
        </v-chip>
        <v-chip
          v-for="m in quiz.available_modes"
          :key="m"
          size="small"
          color="primary"
          variant="tonal"
          :prepend-icon="getQuizMode(m)?.icon"
        >
          {{ getQuizMode(m)?.label ?? m }}
        </v-chip>
        <v-chip
          v-if="best !== null"
          size="small"
          :color="best >= 50 ? 'success' : 'warning'"
          variant="text"
          prepend-icon="mdi-trophy-outline"
          class="ml-auto"
        >
          Best: {{ best }}%
        </v-chip>
      </div>
    </div>
  </v-card>
</template>
