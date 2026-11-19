<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import { api, errorMessage } from '@/api/client'
import type { QuizDetail } from '@/api/types'
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

const fileInput = ref<HTMLInputElement | null>(null)

async function onFile(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) text.value = await file.text()
  ;(e.target as HTMLInputElement).value = ''
}

async function submit() {
  if (importing.value) return
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
    toast.success(
      `"${quiz.title}" with ${quiz.question_count} questions. It stays a draft until you publish it.`,
      'Quiz imported',
    )
    router.push(`/admin/quizzes/${quiz.id}`)
  } catch (err) {
    toast.error(errorMessage(err), 'Import failed')
  } finally {
    importing.value = false
  }
}
</script>

<template>
  <div class="space-y-4">
    <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
      Bulk import
    </h1>
    <v-alert type="info" icon="mdi-code-json" class="text-sm">
      Paste or upload a quiz as JSON. The format (including every image field) is documented in
      <code class="rounded bg-ctp-surface0 px-1">docs/AI-HOW-ADD-QUIZ.md</code>. The whole quiz is
      created in a single transaction and starts unpublished.
    </v-alert>
    <div class="flex flex-wrap gap-2">
      <v-btn variant="tonal" prepend-icon="mdi-upload" @click="fileInput?.click()">
        Upload .json
      </v-btn>
      <input
        ref="fileInput"
        type="file"
        accept="application/json,.json"
        class="sr-only"
        tabindex="-1"
        aria-hidden="true"
        @change="onFile"
      />
      <v-btn variant="text" prepend-icon="mdi-creation" @click="loadSample">Load HCI sample </v-btn>
    </div>
    <v-textarea
      v-model="text"
      rows="22"
      :auto-grow="false"
      spellcheck="false"
      class="json-input"
      label="Quiz JSON"
      placeholder='{ "subject_slug": "hci", "title": "…", "questions": [ … ] }'
      :error="!!parseError"
    />
    <v-alert
      v-if="parseError"
      type="error"
      title="That isn't valid JSON"
      :text="parseError"
      role="alert"
      class="animate__animated animate__headShake"
    />
    <div class="flex justify-end">
      <v-btn
        color="primary"
        size="large"
        prepend-icon="mdi-import"
        :loading="importing"
        :disabled="!text.trim()"
        @click="submit"
      >
        Import quiz
      </v-btn>
    </div>
  </div>
</template>
