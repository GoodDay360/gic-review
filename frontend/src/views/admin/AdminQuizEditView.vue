<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'

import { discardUpload } from '@/api/admin'
import { api, errorMessage } from '@/api/client'
import { loadManagedYears, subjectsForEdit } from '@/api/years'
import type { Question, QuizDetail, Subject } from '@/api/types'
import ImageUploader from '@/components/admin/ImageUploader.vue'
import QuestionEditor from '@/components/admin/QuestionEditor.vue'
import QuestionPreview from '@/components/admin/QuestionPreview.vue'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { getQuizMode } from '@/quiz-modes/registry'
import { questionLabel } from '@/quiz-modes/utils'
import { useToastStore } from '@/stores/toast'

const props = defineProps<{ id: string }>()
const toast = useToastStore()
const router = useRouter()

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
      loadManagedYears(),
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
  if (!quiz.value) return
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
  if (!quiz.value) return
  try {
    setQuiz(
      await api.patch<QuizDetail>(`/admin/quizzes/${quiz.value.id}/publish`, {
        is_published: !quiz.value.is_published,
      }),
    )
    toast.success(quiz.value.is_published ? 'Published' : 'Unpublished')
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

// ----- questions -----
const editing = ref<{ question: Question | null } | null>(null)
const previewing = ref<Question | null>(null)
const deleting = ref<Question | null>(null)
const sharing = ref(false)

function onSaved(q: QuizDetail) {
  setQuiz(q)
  editing.value = null
}

async function confirmDeleteQuestion() {
  if (!quiz.value || !deleting.value) return
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
  }
}

async function reorder(ids: number[]) {
  if (!quiz.value) return
  try {
    setQuiz(
      await api.put<QuizDetail>(`/admin/quizzes/${quiz.value.id}/questions/reorder`, {
        question_ids: ids,
      }),
    )
  } catch (err) {
    toast.error(errorMessage(err))
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
  <EmptyState v-else-if="!quiz" title="Quiz not found" icon="warning">
    <RouterLink to="/admin/quizzes" class="btn-secondary">Back to quizzes</RouterLink>
  </EmptyState>
  <div v-else class="space-y-6">
    <div class="flex flex-wrap items-center gap-2">
      <RouterLink to="/admin/quizzes" class="btn-ghost -ml-3"
        ><AppIcon name="arrow-left" :size="16" /> Quizzes</RouterLink
      >
      <span
        class="rounded-full px-2.5 py-1 text-xs font-medium"
        :class="
          quiz.is_published ? 'bg-ctp-green/15 text-ctp-green' : 'bg-ctp-surface0 text-ctp-subtext0'
        "
      >
        {{ quiz.is_published ? 'Published' : 'Draft' }}
      </span>
      <OwnershipBadge :item="quiz" />
      <div class="ml-auto flex flex-wrap gap-2">
        <button class="btn-secondary btn-sm" @click="sharing = true">
          <AppIcon name="users" :size="14" /> Share
        </button>
        <RouterLink
          v-for="m in quiz.available_modes"
          :key="m"
          :to="{ path: `/quizzes/${quiz.id}/play`, query: { mode: m } }"
          class="btn-secondary btn-sm"
          target="_blank"
        >
          <AppIcon name="eye" :size="14" /> Play {{ getQuizMode(m)?.label ?? m }}
        </RouterLink>
        <button class="btn-primary btn-sm" @click="togglePublish">
          {{ quiz.is_published ? 'Unpublish' : 'Publish' }}
        </button>
      </div>
    </div>

    <form class="card space-y-4 p-5" @submit.prevent="saveMeta">
      <h1 class="text-xl font-bold text-ctp-text">Quiz details</h1>
      <div class="grid gap-4 md:grid-cols-2">
        <div class="space-y-4">
          <div>
            <label class="label" for="qe-title">Title</label>
            <input id="qe-title" v-model="meta.title" class="input" required maxlength="200" />
          </div>
          <div>
            <label class="label" for="qe-subject">Subject</label>
            <select id="qe-subject" v-model="meta.subject_id" class="input">
              <option v-for="s in subjects" :key="s.id" :value="s.id">
                {{ s.year_name }} · {{ s.name }}
              </option>
            </select>
          </div>
          <div>
            <label class="label" for="qe-desc">Description</label>
            <textarea id="qe-desc" v-model="meta.description" class="input" rows="3" />
          </div>
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
        <span v-else class="text-ctp-yellow"
          >none yet: add questions with options (QCM) or answers (Flashcard).</span
        >
      </p>
      <div class="flex justify-end">
        <button class="btn-primary" type="submit" :disabled="savingMeta">
          {{ savingMeta ? 'Saving…' : 'Save details' }}
        </button>
      </div>
    </form>

    <section class="space-y-3">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold text-ctp-text">Questions ({{ quiz.questions.length }})</h2>
        <button class="btn-primary btn-sm" @click="editing = { question: null }">
          <AppIcon name="plus" :size="14" /> Add question
        </button>
      </div>
      <EmptyState
        v-if="!quiz.questions.length"
        title="No questions yet"
        message="Add your first question."
        icon="list-check"
      />
      <ol v-else class="space-y-2">
        <li
          v-for="(q, i) in quiz.questions"
          :key="q.id"
          draggable="true"
          class="card flex items-center gap-3 p-3 transition"
          :class="dragIndex === i ? 'opacity-50' : ''"
          @dragstart="dragIndex = i"
          @dragend="dragIndex = null"
          @dragover.prevent
          @drop.prevent="onDrop(i)"
        >
          <span class="cursor-grab text-ctp-overlay1" aria-hidden="true"
            ><AppIcon name="grip" :size="18"
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
            <button
              class="btn-ghost p-2"
              :disabled="i === 0"
              aria-label="Move up"
              @click="move(i, -1)"
            >
              <AppIcon name="arrow-up" :size="16" />
            </button>
            <button
              class="btn-ghost p-2"
              :disabled="i === quiz.questions.length - 1"
              aria-label="Move down"
              @click="move(i, 1)"
            >
              <AppIcon name="arrow-down" :size="16" />
            </button>
            <button class="btn-ghost p-2" aria-label="Preview" @click="previewing = q">
              <AppIcon name="eye" :size="16" />
            </button>
            <button class="btn-ghost p-2" aria-label="Edit" @click="editing = { question: q }">
              <AppIcon name="pencil" :size="16" />
            </button>
            <button class="btn-ghost p-2 text-ctp-red" aria-label="Delete" @click="deleting = q">
              <AppIcon name="trash" :size="16" />
            </button>
          </div>
        </li>
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

    <ShareDialog
      kind="quizzes"
      :item="sharing ? quiz : null"
      @close="sharing = false"
      @changed="load"
      @left="router.push('/admin/quizzes')"
    />

    <QuestionPreview :question="previewing" @close="previewing = null" />

    <BaseModal :open="!!deleting" title="Delete question?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        The question, its options and its images will be permanently deleted.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" @click="confirmDeleteQuestion">Delete</button>
      </template>
    </BaseModal>
  </div>
</template>
