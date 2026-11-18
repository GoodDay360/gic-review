<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { api, errorMessage } from '@/api/client'
import { addableSubjects, loadManagedYears, type ManagedYear } from '@/api/years'
import type { QuizDetail, QuizSummary, Subject } from '@/api/types'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { getQuizMode } from '@/quiz-modes/registry'
import { useToastStore } from '@/stores/toast'

const router = useRouter()
const toast = useToastStore()

const subjectFilter = ref<number | ''>('')
const subjects = ref<Subject[]>([])
const years = ref<ManagedYear[]>([])
/** New quizzes may only go into subjects of years you own (admins: any). */
const addable = computed(() => addableSubjects(years.value))
const {
  data: quizzes,
  loading,
  run,
} = useAsync(
  () =>
    api.get<QuizSummary[]>(
      `/admin/quizzes${subjectFilter.value ? `?subject_id=${subjectFilter.value}` : ''}`,
    ),
  [],
)

onMounted(async () => {
  years.value = await loadManagedYears().catch(() => [])
  subjects.value = years.value.flatMap((y) => y.subjects)
  run()
})

const subjectName = computed(() => new Map(subjects.value.map((s) => [s.id, s.name])))

// ----- create -----
const creating = ref(false)
const creatingQuiz = ref(false)
const publishingId = ref<number | null>(null)
const deletingQuiz = ref(false)
const newQuiz = ref({ subject_id: 0, title: '', description: '' })
function openCreate() {
  newQuiz.value = { subject_id: addable.value[0]?.id ?? 0, title: '', description: '' }
  creating.value = true
}
async function create() {
  if (creatingQuiz.value) return
  creatingQuiz.value = true
  try {
    const quiz = await api.post<QuizDetail>('/admin/quizzes', {
      ...newQuiz.value,
      description: newQuiz.value.description || null,
      cover_image_key: null,
    })
    creating.value = false
    toast.success('Quiz created (unpublished)')
    router.push(`/admin/quizzes/${quiz.id}`)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    creatingQuiz.value = false
  }
}

async function togglePublish(q: QuizSummary) {
  if (publishingId.value !== null) return
  publishingId.value = q.id
  try {
    await api.patch(`/admin/quizzes/${q.id}/publish`, { is_published: !q.is_published })
    toast.success(q.is_published ? 'Quiz unpublished' : 'Quiz published')
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    publishingId.value = null
  }
}

const deleting = ref<QuizSummary | null>(null)
async function confirmDelete() {
  if (!deleting.value || deletingQuiz.value) return
  deletingQuiz.value = true
  try {
    await api.delete(`/admin/quizzes/${deleting.value.id}`)
    toast.success('Quiz deleted')
    deleting.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingQuiz.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-bold text-ctp-text">Quizzes</h1>
      <div class="flex flex-wrap gap-2">
        <button
          class="btn-primary"
          :disabled="!addable.length"
          :title="addable.length ? '' : 'Create a year and a subject first (Years / Subjects)'"
          @click="openCreate"
        >
          <AppIcon name="plus" :size="16" /> New quiz
        </button>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <label for="subject-filter" class="text-sm text-ctp-subtext1">Subject</label>
      <select id="subject-filter" v-model="subjectFilter" class="input max-w-xs" @change="run">
        <option value="">All subjects</option>
        <option v-for="s in subjects" :key="s.id" :value="s.id">
          {{ s.year_name }} · {{ s.name }}
        </option>
      </select>
    </div>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 4" :key="i" class="h-16" />
    </div>
    <EmptyState
      v-else-if="!quizzes.length"
      title="No quizzes yet"
      message="Create one, or use Bulk import to paste a whole quiz as JSON."
      icon="list-check"
    />
    <ul v-else class="space-y-2">
      <li v-for="q in quizzes" :key="q.id" class="card flex flex-wrap items-center gap-3 p-3">
        <img
          v-if="q.cover_image_url"
          :src="q.cover_image_url"
          alt=""
          class="h-12 w-20 rounded-md object-cover"
          loading="lazy"
        />
        <div class="min-w-0 flex-1">
          <RouterLink
            :to="`/admin/quizzes/${q.id}`"
            class="font-medium text-ctp-text hover:text-ctp-accent"
            >{{ q.title }}</RouterLink
          >
          <p class="text-xs text-ctp-subtext0">
            {{ subjectName.get(q.subject_id) ?? 'Unknown subject' }} ·
            {{ q.question_count }} questions ·
            <span v-if="q.available_modes.length">{{
              q.available_modes.map((m) => getQuizMode(m)?.label ?? m).join(', ')
            }}</span>
            <span v-else class="text-ctp-yellow">no playable mode yet</span>
          </p>
        </div>
        <span
          class="rounded-full px-2.5 py-1 text-xs font-medium"
          :class="
            q.is_published ? 'bg-ctp-green/15 text-ctp-green' : 'bg-ctp-surface0 text-ctp-subtext0'
          "
        >
          {{ q.is_published ? 'Published' : 'Draft' }}
        </span>
        <OwnershipBadge :item="q" />
        <div class="flex gap-1">
          <button
            class="btn-secondary btn-sm"
            :disabled="publishingId === q.id"
            @click="togglePublish(q)"
          >
            {{ publishingId === q.id ? 'Working…' : q.is_published ? 'Unpublish' : 'Publish' }}
          </button>
          <RouterLink :to="`/admin/quizzes/${q.id}`" class="btn-ghost btn-sm" aria-label="Edit"
            ><AppIcon name="pencil" :size="16"
          /></RouterLink>
          <button
            v-if="q.access !== 'shared'"
            class="btn-ghost btn-sm text-ctp-red"
            aria-label="Delete"
            @click="deleting = q"
          >
            <AppIcon name="trash" :size="16" />
          </button>
        </div>
      </li>
    </ul>

    <BaseModal :open="creating" title="New quiz" @close="creating = false">
      <form id="create-quiz" class="space-y-4" @submit.prevent="create">
        <div>
          <label class="label" for="nq-subject">Subject</label>
          <select id="nq-subject" v-model="newQuiz.subject_id" class="input" required>
            <option v-for="s in addable" :key="s.id" :value="s.id">
              {{ s.year_name }} · {{ s.name }}
            </option>
          </select>
        </div>
        <div>
          <label class="label" for="nq-title">Title</label>
          <input id="nq-title" v-model="newQuiz.title" class="input" required maxlength="200" />
        </div>
        <div>
          <label class="label" for="nq-desc">Description</label>
          <textarea id="nq-desc" v-model="newQuiz.description" class="input" rows="3" />
        </div>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="creating = false">Cancel</button>
        <button class="btn-primary" type="submit" form="create-quiz" :disabled="creatingQuiz">
          {{ creatingQuiz ? 'Creating…' : 'Create' }}
        </button>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete quiz?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        "{{ deleting?.title }}" and all its questions and images will be permanently deleted.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" :disabled="deletingQuiz" @click="confirmDelete">
          {{ deletingQuiz ? 'Deleting…' : 'Delete' }}
        </button>
      </template>
    </BaseModal>
  </div>
</template>
