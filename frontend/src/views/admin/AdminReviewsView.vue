<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'

import { api, errorMessage, uploadWithProgress } from '@/api/client'
import {
  addableSubjects,
  loadManagedCategories,
  subjectsForEdit,
  type ManagedCategory,
} from '@/api/categories'
import type { Review, Subject } from '@/api/types'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const categories = ref<ManagedCategory[]>([])
/** Every subject (for labels in the list). */
const subjects = computed<Subject[]>(() => categories.value.flatMap((y) => y.subjects))
/** Subjects offered in the form: addable ones (+ the review's current subject when editing). */
const formSubjects = computed<Subject[]>(() =>
  editing.value && editing.value !== 'new'
    ? subjectsForEdit(categories.value, editing.value.subject_id)
    : addableSubjects(categories.value),
)
const reviews = ref<Review[]>([])
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    ;[categories.value, reviews.value] = await Promise.all([
      loadManagedCategories(),
      api.get<Review[]>('/admin/reviews'),
    ])
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    loading.value = false
  }
}
onMounted(load)

const subjectName = (id: number) => subjects.value.find((s) => s.id === id)?.name ?? '?'

// ----- create / edit -----
const editing = ref<Review | 'new' | null>(null)
const form = reactive({ subject_id: 0, title: '', slug: '', sort_order: 0, is_published: false })
const file = ref<File | null>(null)
const progress = ref<number | null>(null)
const deletingReview = ref(false)

function openNew() {
  Object.assign(form, {
    subject_id: addableSubjects(categories.value)[0]?.id ?? 0,
    title: '',
    slug: '',
    sort_order: 0,
    is_published: false,
  })
  file.value = null
  editing.value = 'new'
}
function openEdit(r: Review) {
  Object.assign(form, {
    subject_id: r.subject_id,
    title: r.title,
    slug: r.slug,
    sort_order: r.sort_order,
    is_published: r.is_published,
  })
  file.value = null
  editing.value = r
}
function onFile(picked: File | File[] | null | undefined) {
  const f = (Array.isArray(picked) ? picked[0] : picked) ?? null
  if (f && !f.name.toLowerCase().endsWith('.html')) {
    toast.warning('Interactive reviews must be a single .html file.', 'Wrong file type')
    file.value = null
    return
  }
  file.value = f
}

const formSubjectItems = computed(() =>
  formSubjects.value.map((s) => ({ title: `${s.category_name} · ${s.name}`, value: s.id })),
)

async function save() {
  if (progress.value !== null) return
  const isNew = editing.value === 'new'
  if (isNew && !file.value) {
    toast.error('Choose the .html file to upload')
    return
  }
  const data = new FormData()
  data.append('subject_id', String(form.subject_id))
  data.append('title', form.title)
  data.append('slug', form.slug)
  data.append('sort_order', String(form.sort_order))
  data.append('is_published', String(form.is_published))
  if (file.value) data.append('file', file.value, file.value.name)
  progress.value = 0
  try {
    const path = isNew ? '/admin/reviews' : `/admin/reviews/${(editing.value as Review).id}`
    await uploadWithProgress<Review>(
      path,
      data,
      (f) => (progress.value = f),
      isNew ? 'POST' : 'PUT',
    )
    toast.success(isNew ? 'Review uploaded' : 'Review updated')
    editing.value = null
    load()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    progress.value = null
  }
}

const deleting = ref<Review | null>(null)
async function confirmDelete() {
  if (!deleting.value || deletingReview.value) return
  deletingReview.value = true
  try {
    await api.delete(`/admin/reviews/${deleting.value.id}`)
    toast.success('Review deleted')
    deleting.value = null
    load()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingReview.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
        Interactive reviews
      </h1>
      <v-btn color="primary" prepend-icon="mdi-upload" :disabled="!subjects.length" @click="openNew"
        >Upload review
      </v-btn>
    </div>
    <v-alert type="info" icon="mdi-information-outline" class="text-sm">
      A review is one self-contained <code class="rounded bg-ctp-surface0 px-1">.html</code> file
      shown in a sandboxed frame. See
      <code class="rounded bg-ctp-surface0 px-1">docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md</code> for
      the rules and a starter template. Replacing the file keeps the same URL.
    </v-alert>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-14" />
    </div>
    <EmptyState
      v-else-if="!reviews.length"
      title="No interactive reviews yet"
      icon="mdi-book-open-page-variant-outline"
    />
    <ul v-else class="space-y-2">
      <v-card
        v-for="(r, i) in reviews"
        :key="r.id"
        tag="li"
        class="card-lift flex flex-wrap items-center gap-3 p-3"
        data-aos="fade-up"
        :data-aos-delay="Math.min(i, 6) * 40"
      >
        <v-avatar color="primary" variant="tonal" rounded="lg">
          <v-icon icon="mdi-language-html5" />
        </v-avatar>
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">{{ r.title }}</p>
          <p class="text-xs text-ctp-subtext0">
            {{ subjectName(r.subject_id) }} · /{{ r.slug }} · order {{ r.sort_order }}
          </p>
        </div>
        <v-chip
          size="small"
          :color="r.is_published ? 'success' : undefined"
          variant="tonal"
          :prepend-icon="r.is_published ? 'mdi-earth' : 'mdi-eye-off-outline'"
        >
          {{ r.is_published ? 'Published' : 'Hidden' }}
        </v-chip>
        <v-btn
          variant="text"
          size="small"
          icon="mdi-code-tags"
          :href="r.url"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Open raw HTML"
        />
        <v-btn
          variant="text"
          size="small"
          icon="mdi-pencil-outline"
          aria-label="Edit"
          @click="openEdit(r)"
        />
        <OwnershipBadge :item="r" />
        <v-btn
          v-if="r.access !== 'shared'"
          variant="text"
          size="small"
          icon="mdi-delete-outline"
          color="error"
          aria-label="Delete"
          @click="deleting = r"
        />
      </v-card>
    </ul>

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'Upload interactive review' : 'Edit interactive review'"
      @close="editing = null"
    >
      <v-form id="review-form" class="space-y-4" @submit.prevent="save">
        <v-select v-model="form.subject_id" :items="formSubjectItems" label="Subject" />
        <v-text-field v-model="form.title" label="Title" required maxlength="200" />
        <div class="grid grid-cols-2 gap-3">
          <v-text-field
            v-model="form.slug"
            label="Slug (optional)"
            placeholder="from title"
            pattern="[a-z0-9]+(-[a-z0-9]+)*"
          />
          <v-text-field v-model.number="form.sort_order" label="Order" type="number" />
        </div>
        <v-switch v-model="form.is_published" label="Published" color="success" />
        <v-file-input
          :model-value="file"
          :label="editing === 'new' ? 'HTML file' : 'Replace HTML file (optional)'"
          accept=".html,text/html"
          prepend-inner-icon="mdi-language-html5"
          show-size
          @update:model-value="onFile"
        />
        <v-progress-linear
          v-if="progress !== null"
          :model-value="progress * 100"
          color="primary"
          height="6"
          rounded
          striped
        />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="editing = null">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="review-form"
          prepend-icon="mdi-content-save-outline"
          :loading="progress !== null"
          >Save</v-btn
        >
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete review?" size="sm" @close="deleting = null">
      <v-alert type="error" icon="mdi-delete-alert-outline">
        "{{ deleting?.title }}" and its HTML file will be permanently deleted.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingReview"
          @click="confirmDelete"
          >Delete</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
