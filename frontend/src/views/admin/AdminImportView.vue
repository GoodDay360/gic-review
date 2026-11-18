<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import { api, errorMessage } from '@/api/client'
import type { QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import { useToastStore } from '@/stores/toast'

import sample from './sample-quiz.json'

const router = useRouter()
const toast = useToastStore()
const text = ref('')
const importing = ref(false)
const parseError = ref<string | null>(null)

function loadSample() {
  text.value = JSON.stringify(sample, null, 2)
  parseError.value = null
}

async function onFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) text.value = await file.text()
  ;(e.target as HTMLInputElement).value = ''
}

async function submit() {
  parseError.value = null
  let body: unknown
  try {
    body = JSON.parse(text.value)
  } catch (err) {
    parseError.value = `Invalid JSON: ${(err as Error).message}`
    return
  }
  importing.value = true
  try {
    const quiz = await api.post<QuizDetail>('/admin/quizzes/import', body)
    toast.success(`Imported "${quiz.title}" with ${quiz.question_count} questions (unpublished).`)
    router.push(`/admin/quizzes/${quiz.id}`)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <div class="space-y-4">
    <h1 class="text-2xl font-bold text-ctp-text">Bulk import</h1>
    <p class="text-sm text-ctp-subtext0">
      Paste or upload a quiz as JSON. The format (including every image field) is documented in
      <code class="rounded bg-ctp-surface0 px-1">docs/AI-HOW-ADD-QUIZ.md</code>. The whole quiz is
      created in a single transaction and starts unpublished.
    </p>
    <div class="flex flex-wrap gap-2">
      <label class="btn-secondary cursor-pointer">
        <AppIcon name="upload" :size="16" /> Upload .json
        <input type="file" accept="application/json,.json" class="sr-only" @change="onFile" />
      </label>
      <button class="btn-ghost" @click="loadSample">
        <AppIcon name="sparkles" :size="16" /> Load HCI sample
      </button>
    </div>
    <textarea
      v-model="text"
      rows="22"
      spellcheck="false"
      class="input font-mono text-xs"
      aria-label="Quiz JSON"
      placeholder='{ "subject_slug": "hci", "title": "…", "questions": [ … ] }'
    />
    <p v-if="parseError" role="alert" class="text-sm text-ctp-red">{{ parseError }}</p>
    <div class="flex justify-end">
      <button class="btn-primary" :disabled="importing || !text.trim()" @click="submit">
        {{ importing ? 'Importing…' : 'Import quiz' }}
      </button>
    </div>
  </div>
</template>
