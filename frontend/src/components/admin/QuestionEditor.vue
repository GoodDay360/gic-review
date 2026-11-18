<script setup lang="ts">
/** Create / edit one question with its options, flashcard answer and explanation. */
import { computed, onBeforeUnmount, reactive, ref } from 'vue'

import { discardUpload, questionKeys, saveQuestion } from '@/api/admin'
import { errorMessage } from '@/api/client'
import type { Question, QuestionInput, QuizDetail } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import { useToastStore } from '@/stores/toast'

import ImageUploader from './ImageUploader.vue'
import QuestionPreview from './QuestionPreview.vue'

const props = defineProps<{ quizId: number; question: Question | null }>()
const emit = defineEmits<{ saved: [quiz: QuizDetail]; cancel: [] }>()
const toast = useToastStore()

interface Img {
  key: string | null
  url: string | null
  alt: string | null
}
interface OptionForm {
  uid: number
  text: string
  image: Img
  is_correct: boolean
}

let uid = 1
const q = props.question
const form = reactive({
  prompt: q?.prompt ?? '',
  image: { key: q?.image_key ?? null, url: q?.image_url ?? null, alt: q?.image_alt ?? null } as Img,
  answer_text: q?.answer_text ?? '',
  answer_image: {
    key: q?.answer_image_key ?? null,
    url: q?.answer_image_url ?? null,
    alt: q?.answer_image_alt ?? null,
  } as Img,
  explanation: q?.explanation ?? '',
  explanation_image: {
    key: q?.explanation_image_key ?? null,
    url: q?.explanation_image_url ?? null,
    alt: null,
  } as Img,
  multi_select: q?.multi_select ?? false,
  options: (q?.options ?? []).map<OptionForm>((o) => ({
    uid: uid++,
    text: o.text ?? '',
    image: { key: o.image_key, url: o.image_url, alt: o.image_alt },
    is_correct: o.is_correct,
  })),
})
if (!q) {
  form.options.push(
    { uid: uid++, text: '', image: { key: null, url: null, alt: null }, is_correct: true },
    { uid: uid++, text: '', image: { key: null, url: null, alt: null }, is_correct: false },
  )
}

/** Keys uploaded during this edit session (deleted again if they end up unused). */
const uploaded = new Set<string>()
const originalKeys = new Set(q ? questionKeys(q) : [])
const saving = ref(false)
const previewing = ref<Question | null>(null)

function addOption() {
  form.options.push({
    uid: uid++,
    text: '',
    image: { key: null, url: null, alt: null },
    is_correct: false,
  })
}
function removeOption(i: number) {
  form.options.splice(i, 1)
}
function moveOption(i: number, dir: -1 | 1) {
  const j = i + dir
  if (j < 0 || j >= form.options.length) return
  const [o] = form.options.splice(i, 1)
  form.options.splice(j, 0, o!)
}
function setCorrect(i: number, value: boolean) {
  if (!form.multi_select && value) form.options.forEach((o, k) => (o.is_correct = k === i))
  else form.options[i]!.is_correct = value
}

const blank = (s: string) => (s.trim() ? s.trim() : null)

const input = computed<QuestionInput>(() => ({
  prompt: blank(form.prompt),
  image_key: form.image.key,
  image_alt: form.image.key ? blank(form.image.alt ?? '') : null,
  answer_text: blank(form.answer_text),
  answer_image_key: form.answer_image.key,
  answer_image_alt: form.answer_image.key ? blank(form.answer_image.alt ?? '') : null,
  explanation: blank(form.explanation),
  explanation_image_key: form.explanation_image.key,
  multi_select: form.multi_select,
  options: form.options
    .filter((o) => o.text.trim() || o.image.key)
    .map((o) => ({
      text: blank(o.text),
      image_key: o.image.key,
      image_alt: o.image.key ? blank(o.image.alt ?? '') : null,
      is_correct: o.is_correct,
    })),
}))

const problems = computed(() => {
  const p: string[] = []
  if (!input.value.prompt && !input.value.image_key) p.push('Add a prompt or a question image.')
  const correct = input.value.options.filter((o) => o.is_correct).length
  if (!form.multi_select && correct > 1)
    p.push('Several options are correct: enable "multiple answers".')
  return p
})

/** Build a Question-shaped object for the student preview. */
function asQuestion(): Question {
  const i = input.value
  return {
    id: q?.id ?? 0,
    position: q?.position ?? 0,
    prompt: i.prompt,
    image_key: i.image_key,
    image_url: form.image.url,
    image_alt: i.image_alt,
    answer_text: i.answer_text,
    answer_image_key: i.answer_image_key,
    answer_image_url: form.answer_image.url,
    answer_image_alt: i.answer_image_alt,
    explanation: i.explanation,
    explanation_image_key: i.explanation_image_key,
    explanation_image_url: form.explanation_image.url,
    multi_select: i.multi_select,
    options: form.options
      .filter((o) => o.text.trim() || o.image.key)
      .map((o, idx) => ({
        id: idx + 1,
        position: idx,
        text: blank(o.text),
        image_key: o.image.key,
        image_url: o.image.url,
        image_alt: o.image.alt,
        is_correct: o.is_correct,
      })),
  }
}

function discardUnused(keep: Set<string>) {
  for (const k of uploaded) if (!keep.has(k) && !originalKeys.has(k)) discardUpload(k)
}

let saved = false

async function save() {
  if (problems.value.length) return
  saving.value = true
  try {
    const quiz = await saveQuestion(props.quizId, q?.id ?? null, input.value)
    saved = true
    discardUnused(new Set(questionKeys(input.value)))
    toast.success(q ? 'Question saved' : 'Question added')
    emit('saved', quiz)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    saving.value = false
  }
}

function cancel() {
  emit('cancel')
}

// Closing without saving (Cancel, Esc, backdrop): drop images uploaded in this session.
onBeforeUnmount(() => {
  if (!saved) discardUnused(new Set())
})
</script>

<template>
  <form class="space-y-6" @submit.prevent="save">
    <section class="space-y-3">
      <h3 class="font-semibold text-ctp-text">Question (QCM question / flashcard front)</h3>
      <div>
        <label class="label" for="q-prompt">Prompt</label>
        <textarea
          id="q-prompt"
          v-model="form.prompt"
          rows="3"
          class="input"
          placeholder="Leave empty for an image-only question"
        />
      </div>
      <ImageUploader
        v-model:image-key="form.image.key"
        v-model:image-url="form.image.url"
        v-model:alt="form.image.alt"
        label="Question image"
        :quiz-id="quizId"
        @uploaded="uploaded.add($event)"
      />
    </section>

    <section class="space-y-3">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <h3 class="font-semibold text-ctp-text">Options (QCM)</h3>
        <label class="flex items-center gap-2 text-sm text-ctp-subtext1">
          <input v-model="form.multi_select" type="checkbox" class="h-4 w-4 accent-ctp-accent" />
          Multiple correct answers
        </label>
      </div>
      <p class="text-xs text-ctp-subtext0">
        QCM needs at least 2 options and 1 correct option. Options can be text, an image, or both.
      </p>
      <ul class="space-y-3">
        <li
          v-for="(o, i) in form.options"
          :key="o.uid"
          class="rounded-lg border border-ctp-surface1 p-3"
        >
          <div class="flex items-start gap-2">
            <label
              class="mt-2 flex items-center gap-1.5 text-xs font-medium"
              :class="o.is_correct ? 'text-ctp-green' : 'text-ctp-subtext0'"
            >
              <input
                :type="form.multi_select ? 'checkbox' : 'radio'"
                :name="`correct-${quizId}`"
                :checked="o.is_correct"
                class="h-4 w-4 accent-ctp-green"
                @change="setCorrect(i, ($event.target as HTMLInputElement).checked)"
              />
              Correct
            </label>
            <input
              v-model="o.text"
              class="input flex-1"
              :placeholder="`Option ${i + 1} text`"
              :aria-label="`Option ${i + 1} text`"
            />
            <div class="flex">
              <button
                type="button"
                class="btn-ghost p-2"
                :disabled="i === 0"
                aria-label="Move option up"
                @click="moveOption(i, -1)"
              >
                <AppIcon name="arrow-up" :size="16" />
              </button>
              <button
                type="button"
                class="btn-ghost p-2"
                :disabled="i === form.options.length - 1"
                aria-label="Move option down"
                @click="moveOption(i, 1)"
              >
                <AppIcon name="arrow-down" :size="16" />
              </button>
              <button
                type="button"
                class="btn-ghost p-2 text-ctp-red"
                aria-label="Remove option"
                @click="removeOption(i)"
              >
                <AppIcon name="trash" :size="16" />
              </button>
            </div>
          </div>
          <div class="mt-2">
            <ImageUploader
              v-model:image-key="o.image.key"
              v-model:image-url="o.image.url"
              v-model:alt="o.image.alt"
              :label="`Option ${i + 1} image (optional)`"
              :quiz-id="quizId"
              compact
              @uploaded="uploaded.add($event)"
            />
          </div>
        </li>
      </ul>
      <button
        type="button"
        class="btn-secondary btn-sm"
        :disabled="form.options.length >= 12"
        @click="addOption"
      >
        <AppIcon name="plus" :size="14" /> Add option
      </button>
    </section>

    <section class="space-y-3">
      <h3 class="font-semibold text-ctp-text">Flashcard back</h3>
      <p class="text-xs text-ctp-subtext0">
        Optional. If empty, the back shows the correct option(s).
      </p>
      <div>
        <label class="label" for="q-answer">Answer text</label>
        <textarea id="q-answer" v-model="form.answer_text" rows="2" class="input" />
      </div>
      <ImageUploader
        v-model:image-key="form.answer_image.key"
        v-model:image-url="form.answer_image.url"
        v-model:alt="form.answer_image.alt"
        label="Answer image"
        :quiz-id="quizId"
        @uploaded="uploaded.add($event)"
      />
    </section>

    <section class="space-y-3">
      <h3 class="font-semibold text-ctp-text">Explanation (shown after answering)</h3>
      <textarea v-model="form.explanation" rows="3" class="input" aria-label="Explanation" />
      <ImageUploader
        v-model:image-key="form.explanation_image.key"
        v-model:image-url="form.explanation_image.url"
        label="Explanation image (e.g. a diagram)"
        :quiz-id="quizId"
        :with-alt="false"
        @uploaded="uploaded.add($event)"
      />
    </section>

    <ul
      v-if="problems.length"
      class="space-y-1 rounded-lg bg-ctp-yellow/15 p-3 text-sm text-ctp-yellow"
      role="alert"
    >
      <li v-for="p in problems" :key="p">{{ p }}</li>
    </ul>

    <div
      class="sticky bottom-0 -mx-5 -mb-5 flex flex-wrap justify-end gap-2 border-t border-ctp-surface0 bg-ctp-base p-4"
    >
      <button type="button" class="btn-secondary mr-auto" @click="previewing = asQuestion()">
        <AppIcon name="eye" :size="16" /> Preview
      </button>
      <button type="button" class="btn-ghost" @click="cancel">Cancel</button>
      <button type="submit" class="btn-primary" :disabled="saving || problems.length > 0">
        {{ saving ? 'Saving…' : 'Save question' }}
      </button>
    </div>

    <QuestionPreview :question="previewing" @close="previewing = null" />
  </form>
</template>
