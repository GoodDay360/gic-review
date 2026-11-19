<script setup lang="ts">
import type { Subject } from '@/api/types'

defineProps<{ subject: Subject }>()
defineEmits<{ select: [subject: Subject] }>()
</script>

<template>
  <v-card
    tag="button"
    type="button"
    hover
    class="w-full card-lift group flex h-full flex-col gap-3 p-5 text-left"
    :aria-label="`Open ${subject.name}`"
    @click="$emit('select', subject)"
  >
    <span
      class="flex h-11 w-11 items-center justify-center rounded-xl transition-transform group-hover:scale-110 group-hover:-rotate-6"
      :style="{
        color: `var(--ctp-${subject.color})`,
        backgroundColor: `color-mix(in srgb, var(--ctp-${subject.color}) 18%, transparent)`,
      }"
    >
      <v-icon icon="mdi-book-open-page-variant-outline" size="24" />
    </span>
    <span class="text-lg font-semibold text-ctp-text group-hover:text-ctp-accent">{{
      subject.name
    }}</span>
    <span v-if="subject.description" class="line-clamp-3 text-sm text-ctp-subtext0">
      {{ subject.description }}
    </span>
    <span class="mt-auto flex flex-wrap gap-2 pt-2">
      <v-chip size="small" variant="tonal" prepend-icon="mdi-format-list-checks">
        {{ subject.quiz_count }} quiz{{ subject.quiz_count === 1 ? '' : 'zes' }}
      </v-chip>
      <v-chip
        v-if="subject.has_review"
        size="small"
        variant="tonal"
        prepend-icon="mdi-book-open-variant"
      >
        Interactive review
      </v-chip>
    </span>
  </v-card>
</template>
