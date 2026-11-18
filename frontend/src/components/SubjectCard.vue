<script setup lang="ts">
import type { Subject } from '@/api/types'

import AppIcon from './AppIcon.vue'

defineProps<{ subject: Subject }>()
defineEmits<{ select: [subject: Subject] }>()
</script>

<template>
  <button
    class="card group flex h-full flex-col gap-3 p-5 text-left transition hover:-translate-y-0.5 hover:border-ctp-surface2 hover:shadow-lg"
    :aria-label="`Open ${subject.name}`"
    @click="$emit('select', subject)"
  >
    <span
      class="flex h-11 w-11 items-center justify-center rounded-xl"
      :style="{
        color: `var(--ctp-${subject.color})`,
        backgroundColor: `color-mix(in srgb, var(--ctp-${subject.color}) 18%, transparent)`,
      }"
    >
      <AppIcon name="book" :size="22" />
    </span>
    <span class="text-lg font-semibold text-ctp-text group-hover:text-ctp-accent">{{
      subject.name
    }}</span>
    <span v-if="subject.description" class="line-clamp-3 text-sm text-ctp-subtext0">
      {{ subject.description }}
    </span>
    <span class="mt-auto flex flex-wrap gap-2 pt-2 text-xs text-ctp-subtext1">
      <span class="rounded-full bg-ctp-surface0 px-2.5 py-1">
        {{ subject.quiz_count }} quiz{{ subject.quiz_count === 1 ? '' : 'zes' }}
      </span>
      <span v-if="subject.has_review" class="rounded-full bg-ctp-surface0 px-2.5 py-1">
        Interactive review
      </span>
    </span>
  </button>
</template>
