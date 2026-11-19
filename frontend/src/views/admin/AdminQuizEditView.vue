<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { discardUpload } from '@/api/admin'
import { api, errorMessage } from '@/api/client'
import { loadManagedCategories, subjectsForEdit } from '@/api/categories'
import type { Question, QuizDetail, Subject } from '@/api/types'
import SubjectPicker from '@/components/admin/SubjectPicker.vue'
import ImageUploader from '@/components/admin/ImageUploader.vue'
import QuestionEditor from '@/components/admin/QuestionEditor.vue'
import QuestionPreview from '@/components/admin/QuestionPreview.vue'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { getQuizMode } from '@/quiz-modes/registry'
import { questionLabel } from '@/quiz-modes/utils'
import { useToastStore } from '@/stores/toast'

const props = defineProps<{ id: string }>()
const toast = useToastStore()

const quiz = ref<QuizDetail | null>(null)
const subjects = ref<Subject[]>([])
const loading = ref(true)
const meta = reactive({
  subject_id: 0,
  title: '',
  description: '',
  cover: { key: null as string | null, url: null as string | null },
})
let savedCoverKey: string | null = null
const savingMeta = ref(false)
const togglingPublish = ref(false)
const deletingQuestion = ref(false)
const reordering = ref(false)

function setQuiz(q: QuizDetail) {
  quiz.value = q
  meta.subject_id = q.subject_id
  meta.title = q.title
  meta.description = q.description ?? ''
  meta.cover = { key: q.cover_image_key, url: q.cover_image_url }
  savedCoverKey = q.cover_image_key
}

async function load() {
  loading.value = true
  try {
    const [q, s] = await Promise.all([
      api.get<QuizDetail>(`/admin/quizzes/${props.id}`),
      loadManagedCategories(),
    ])
    subjects.value = subjectsForEdit(s, q.subject_id)
    setQuiz(q)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    loading.value = false
  }
}
onMounted(load)

async function saveMeta() {
  if (!quiz.value || savingMeta.value) return
  savingMeta.value = true
  try {
    setQuiz(
      await api.put<QuizDetail>(`/admin/quizzes/${quiz.value.id}`, {
        subject_id: meta.subject_id,
        title: meta.title,
        description: meta.description.trim() || null,
        cover_image_key: meta.cover.key,
      }),
    )
    toast.success('Quiz details saved')
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    savingMeta.value = false
  }
}

function onCoverUploaded(key: string) {
  // A replaced-but-unsaved upload would become an orphan; drop the previous unsaved one.
  if (pendingCover && pendingCover !== savedCoverKey) discardUpload(pendingCover)
  pendingCover = key
}
let pendingCover: string | null = null

async function togglePublish() {
  if (!quiz.value || togglingPublish.value) return
  togglingPublish.value = true
  try {
    setQuiz(
      await api.patch<QuizDetail>(`/admin/quizzes/${quiz.value.id}/publish`, {
        is_published: !quiz.value.is_published,
      }),
    )
    toast.success(quiz.value.is_published ? 'Published' : 'Unpublished')
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    togglingPublish.value = false
  }
}

// ----- questions -----
const editing = ref<{ question: Question | null } | null>(null)
const previewing = ref<Question | null>(null)
const deleting = ref<Question | null>(null)

function onSaved(q: QuizDetail) {
  setQuiz(q)
  editing.value = null
}

async function confirmDeleteQuestion() {
  if (!quiz.value || !deleting.value || deletingQuestion.value) return
  deletingQuestion.value = true
  try {
    setQuiz(
      await api.delete<QuizDetail>(
        `/admin/quizzes/${quiz.value.id}/questions/${deleting.value.id}`,
      ),
    )
    toast.success('Question deleted')
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deleting.value = null
    deletingQuestion.value = false
  }
}

async function reorder(ids: number[]) {
  if (!quiz.value || reordering.value) return
  reordering.value = true
  try {
    setQuiz(
      await api.put<QuizDetail>(`/admin/quizzes/${quiz.value.id}/questions/reorder`, {
        question_ids: ids,
      }),
    )
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    reordering.value = false
  }
}

function move(i: number, dir: -1 | 1) {
  if (!quiz.value) return
  const ids = quiz.value.questions.map((q) => q.id)
  const j = i + dir
  if (j < 0 || j >= ids.length) return
  ;[ids[i], ids[j]] = [ids[j]!, ids[i]!]
  reorder(ids)
}

// Drag & drop reordering.
const dragIndex = ref<number | null>(null)
function onDrop(target: number) {
  if (!quiz.value || dragIndex.value === null || dragIndex.value === target) return
  const ids = quiz.value.questions.map((q) => q.id)
  const [moved] = ids.splice(dragIndex.value, 1)
  ids.splice(target, 0, moved!)
  dragIndex.value = null
  reorder(ids)
}
</script>

<template>
  <div v-if="loading" class="space-y-3">
    <SkeletonBlock class="h-10 w-1/2" /><SkeletonBlock class="h-64" />
  </div>
  <EmptyState v-else-if="!quiz" title="Quiz not found" icon="mdi-alert-outline">
    <v-btn variant="tonal" prepend-icon="mdi-arrow-left" to="/admin/quizzes">Back to quizzes</v-btn>
  </EmptyState>
  <div v-else class="space-y-6">
    <div class="flex flex-wrap items-center gap-2">
      <v-btn variant="text" prepend-icon="mdi-arrow-left" to="/admin/quizzes" class="-ml-3"
        >Quizzes</v-btn
      >
      <v-chip
        size="small"
        :color="quiz.is_published ? 'success' : undefined"
        variant="tonal"
        :prepend-icon="quiz.is_published ? 'mdi-earth' : 'mdi-file-edit-outline'"
      >
        {{ quiz.is_published ? 'Published' : 'Draft' }}
      </v-chip>
      <OwnershipBadge :item="quiz" />
      <div class="ml-auto flex flex-wrap gap-2">
        <v-btn
          v-for="m in quiz.available_modes"
          :key="m"
          variant="tonal"
          size="small"
          prepend-icon="mdi-play-circle-outline"
          :to="{ path: `/quizzes/${quiz.id}/play`, query: { mode: m } }"
          target="_blank"
          >Play {{ getQuizMode(m)?.label ?? m }}
        </v-btn>
        <v-btn
          :color="quiz.is_published ? undefined : 'success'"
          :variant="quiz.is_published ? 'tonal' : 'flat'"
          size="small"
          :prepend-icon="quiz.is_published ? 'mdi-eye-off-outline' : 'mdi-publish'"
          :loading="togglingPublish"
          @click="togglePublish"
        >
          {{ quiz.is_published ? 'Unpublish' : 'Publish' }}
        </v-btn>
      </div>
    </div>

    <v-card tag="form" class="space-y-4 p-5" data-aos="fade-up" @submit.prevent="saveMeta">
      <h1 class="flex items-center gap-2 text-xl font-bold text-ctp-text">
        <v-icon icon="mdi-clipboard-text-outline" color="primary" /> Quiz details
      </h1>
      <div class="grid gap-4 md:grid-cols-2">
        <div class="space-y-4">
          <v-text-field v-model="meta.title" label="Title" required maxlength="200" />
          <SubjectPicker v-model="meta.subject_id" :subjects="subjects" />
          <v-textarea v-model="meta.description" label="Description" rows="3" />
        </div>
        <ImageUploader
          v-model:image-key="meta.cover.key"
          v-model:image-url="meta.cover.url"
          label="Cover image (shown on the quiz card)"
          :quiz-id="quiz.id"
          :with-alt="false"
          @uploaded="onCoverUploaded"
        />
      </div>
      <p class="text-xs text-ctp-subtext0">
        Available modes:
        <strong v-if="quiz.available_modes.length" class="text-ctp-text">
          {{ quiz.available_modes.map((m) => getQuizMode(m)?.label ?? m).join(', ') }}
        </strong>
        <span v-else class="text-ctp-peach"
          >none yet: add questions with options (QCM) or answers (Flashcard).</span
        >
      </p>
      <div class="flex justify-end">
        <v-btn
          color="primary"
          type="submit"
          prepend-icon="mdi-content-save-outline"
          :loading="savingMeta"
          >Save details</v-btn
        >
      </div>
    </v-card>

    <section class="space-y-3" data-aos="fade-up">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-ctp-text">Questions ({{ quiz.questions.length }})</h2>
        <v-btn
          color="primary"
          size="small"
          prepend-icon="mdi-plus"
          @click="editing = { question: null }"
          >Add question
        </v-btn>
      </div>
      <EmptyState
        v-if="!quiz.questions.length"
        title="No questions yet"
        message="Add your first question."
        icon="mdi-format-list-checks"
      />
      <ol v-else class="space-y-2">
        <v-card
          v-for="(q, i) in quiz.questions"
          :key="q.id"
          tag="li"
          draggable="true"
          class="card-lift flex items-center gap-3 p-3"
          :class="dragIndex === i ? 'opacity-50' : ''"
          @dragstart="dragIndex = i"
          @dragend="dragIndex = null"
          @dragover.prevent
          @drop.prevent="onDrop(i)"
        >
          <span class="cursor-grab text-ctp-overlay1" aria-hidden="true"
            ><v-icon icon="mdi-drag" size="18"
          /></span>
          <span class="w-6 text-sm text-ctp-subtext0">{{ i + 1 }}.</span>
          <img
            v-if="q.image_url"
            :src="q.image_url"
            alt=""
            class="h-10 w-14 rounded object-cover"
            loading="lazy"
          />
          <div class="min-w-0 flex-1">
            <p class="truncate text-sm text-ctp-text">{{ questionLabel(q) }}</p>
            <p class="text-xs text-ctp-subtext0">
              {{ q.options.length }} options ·
              {{ q.options.filter((o) => o.is_correct).length }} correct
              <span v-if="q.multi_select">· multi</span>
              <span v-if="q.answer_text || q.answer_image_url">· flashcard answer</span>
            </p>
          </div>
          <div class="flex">
            <v-btn
              variant="text"
              size="small"
              icon="mdi-arrow-up"
              :disabled="i === 0 || reordering"
              aria-label="Move up"
              @click="move(i, -1)"
            />
            <v-btn
              variant="text"
              size="small"
              icon="mdi-arrow-down"
              :disabled="i === quiz.questions.length - 1 || reordering"
              aria-label="Move down"
              @click="move(i, 1)"
            />
            <v-btn
              variant="text"
              size="small"
              icon="mdi-eye-outline"
              aria-label="Preview"
              @click="previewing = q"
            />
            <v-btn
              variant="text"
              size="small"
              icon="mdi-pencil-outline"
              aria-label="Edit"
              @click="editing = { question: q }"
            />
            <v-btn
              variant="text"
              size="small"
              color="error"
              icon="mdi-delete-outline"
              aria-label="Delete"
              @click="deleting = q"
            />
          </div>
        </v-card>
      </ol>
    </section>

    <BaseModal
      :open="!!editing"
      :title="editing?.question ? 'Edit question' : 'New question'"
      size="xl"
      @close="editing = null"
    >
      <QuestionEditor
        v-if="editing"
        :key="editing.question?.id ?? 'new'"
        :quiz-id="quiz.id"
        :question="editing.question"
        @saved="onSaved"
        @cancel="editing = null"
      />
    </BaseModal>

    <QuestionPreview :question="previewing" @close="previewing = null" />

    <BaseModal :open="!!deleting" title="Delete question?" size="sm" @close="deleting = null">
      <v-alert type="error" icon="mdi-delete-alert-outline">
        The question, its options and its images will be permanently deleted.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingQuestion"
          @click="confirmDeleteQuestion"
          >Delete</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
