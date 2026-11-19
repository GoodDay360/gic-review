<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { api, errorMessage } from '@/api/client'
import { addableSubjects, loadManagedCategories, type ManagedCategory } from '@/api/categories'
import type { QuizDetail, QuizSummary, Subject } from '@/api/types'
import CategorySubjectFilter, {
  filterBySubject,
  type FilterId,
} from '@/components/admin/CategorySubjectFilter.vue'
import SubjectPicker from '@/components/admin/SubjectPicker.vue'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { getQuizMode } from '@/quiz-modes/registry'
import { useToastStore } from '@/stores/toast'

const router = useRouter()
const toast = useToastStore()

const categoryFilter = ref<FilterId>('')
const subjectFilter = ref<FilterId>('')
const subjects = ref<Subject[]>([])
const categories = ref<ManagedCategory[]>([])
/** New quizzes may only go into subjects of categories you own (admins: any). */
const addable = computed(() => addableSubjects(categories.value))
const { data: quizzes, loading, run } = useAsync(() => api.get<QuizSummary[]>('/admin/quizzes'), [])

onMounted(async () => {
  categories.value = await loadManagedCategories().catch(() => [])
  subjects.value = categories.value.flatMap((y) => y.subjects)
  run()
})

const filteredQuizzes = computed(() =>
  filterBySubject(quizzes.value, categories.value, categoryFilter.value, subjectFilter.value),
)
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
    toast.success('It stays a draft until you publish it.', 'Quiz created')
    router.push(`/admin/quizzes/${quiz.id}`)
  } catch (err) {
    toast.error(errorMessage(err), 'Couldn’t create the quiz')
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
      <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
        Quizzes
      </h1>
      <div class="flex flex-wrap gap-2">
        <v-btn variant="tonal" prepend-icon="mdi-import" to="/admin/import">Bulk import</v-btn>
        <v-btn
          color="primary"
          prepend-icon="mdi-plus"
          :disabled="!addable.length"
          :title="
            addable.length ? '' : 'Create a category and a subject first (Categories / Subjects)'
          "
          @click="openCreate"
          >New quiz
        </v-btn>
      </div>
    </div>

    <CategorySubjectFilter
      v-model:category="categoryFilter"
      v-model:subject="subjectFilter"
      :categories="categories"
    />

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 4" :key="i" class="h-16" />
    </div>
    <EmptyState
      v-else-if="!quizzes.length"
      title="No quizzes yet"
      message="Create one, or use Bulk import to paste a whole quiz as JSON."
      icon="mdi-format-list-checks"
    />
    <EmptyState
      v-else-if="!filteredQuizzes.length"
      title="No matching quizzes"
      message="No quiz matches the selected category and subject."
      icon="mdi-filter-remove-outline"
    />
    <ul v-else class="space-y-2">
      <v-card
        v-for="(q, i) in filteredQuizzes"
        :key="q.id"
        tag="li"
        class="card-lift flex flex-wrap items-center gap-3 p-3"
        data-aos="fade-up"
        :data-aos-delay="Math.min(i, 6) * 40"
      >
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
            <span v-else class="text-ctp-peach">no playable mode yet</span>
          </p>
        </div>
        <v-chip
          size="small"
          :color="q.is_published ? 'success' : undefined"
          variant="tonal"
          :prepend-icon="q.is_published ? 'mdi-earth' : 'mdi-file-edit-outline'"
        >
          {{ q.is_published ? 'Published' : 'Draft' }}
        </v-chip>
        <OwnershipBadge :item="q" />
        <div class="flex gap-1">
          <v-btn
            variant="tonal"
            size="small"
            :color="q.is_published ? undefined : 'success'"
            :prepend-icon="q.is_published ? 'mdi-eye-off-outline' : 'mdi-publish'"
            :loading="publishingId === q.id"
            :disabled="publishingId !== null && publishingId !== q.id"
            @click="togglePublish(q)"
          >
            {{ q.is_published ? 'Unpublish' : 'Publish' }}
          </v-btn>
          <v-btn
            variant="text"
            size="small"
            icon="mdi-pencil-outline"
            :to="`/admin/quizzes/${q.id}`"
            aria-label="Edit"
          />
          <v-btn
            v-if="q.access !== 'shared'"
            variant="text"
            size="small"
            icon="mdi-delete-outline"
            color="error"
            aria-label="Delete"
            @click="deleting = q"
          />
        </div>
      </v-card>
    </ul>

    <BaseModal :open="creating" title="New quiz" @close="creating = false">
      <v-form id="create-quiz" class="space-y-4" @submit.prevent="create">
        <SubjectPicker v-model="newQuiz.subject_id" :subjects="addable" />
        <v-text-field v-model="newQuiz.title" label="Title" required maxlength="200" />
        <v-textarea v-model="newQuiz.description" label="Description" rows="3" />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="creating = false">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="create-quiz"
          prepend-icon="mdi-plus"
          :loading="creatingQuiz"
        >
          Create
        </v-btn>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete quiz?" size="sm" @close="deleting = null">
      <v-alert type="error" icon="mdi-delete-alert-outline">
        "{{ deleting?.title }}" and all its questions and images will be permanently deleted.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingQuiz"
          @click="confirmDelete"
        >
          Delete
        </v-btn>
      </template>
    </BaseModal>
  </div>
</template>
