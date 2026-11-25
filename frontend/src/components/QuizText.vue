<script setup lang="ts">
/**
 * One piece of quiz text (prompt, option, answer or explanation), shown per the question's
 * `text_format`: `plain` keeps the text as typed, `markdown` renders code blocks, lists, tables, …
 */
import { computed } from 'vue'

import type { TextFormat } from '@/api/types'
import { renderMarkdown } from '@/utils/markdown'

const props = defineProps<{ source: string | null | undefined; format: TextFormat }>()

const html = computed(() => (props.format === 'markdown' ? renderMarkdown(props.source) : ''))

// Following a link must not also select an option or flip a flashcard.
function onClick(e: MouseEvent) {
  if ((e.target as HTMLElement | null)?.closest('a')) e.stopPropagation()
}
</script>

<template>
  <!-- Safe: markdown-it runs with `html: false`, so the source can't contain raw HTML. -->
  <!-- eslint-disable-next-line vue/no-v-html -->
  <div v-if="format === 'markdown'" class="md" @click="onClick" v-html="html" />
  <div v-else class="whitespace-pre-line">{{ source }}</div>
</template>
