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
import CategorySubjectFilter from '@/components/admin/CategorySubjectFilter.vue'
import SubjectPicker from '@/components/admin/SubjectPicker.vue'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useManagedList } from '@/composables/useManagedList'
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
/** Filtered, searched and paged on the server (20 per page, infinite scroll). */
const {
  search,
  category: categoryFilter,
  subject: subjectFilter,
  items: reviews,
  loading,
  loadingMore,
  hasMore,
  error,
  loadMore,
  retry,
  reset,
  isFiltered,
  replaceItem,
  removeItem,
} = useManagedList<Review>('/admin/reviews')

onMounted(async () => {
  try {
    categories.value = await loadManagedCategories()
  } catch (err) {
    toast.error(errorMessage(err))
  }
})

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
    reset()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    progress.value = null
  }
}

const publishingId = ref<number | null>(null)
async function togglePublish(r: Review) {
  if (publishingId.value !== null) return
  publishingId.value = r.id
  try {
    // The update endpoint takes a partial form: only the published flag changes.
    const data = new FormData()
    data.append('is_published', String(!r.is_published))
    const updated = await api.put<Review>(`/admin/reviews/${r.id}`, data)
    toast.success(r.is_published ? 'Review unpublished' : 'Review published')
    replaceItem(updated)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    publishingId.value = null
  }
}

const deleting = ref<Review | null>(null)
async function confirmDelete() {
  if (!deleting.value || deletingReview.value) return
  deletingReview.value = true
  try {
    await api.delete(`/admin/reviews/${deleting.value.id}`)
    toast.success('Review deleted')
    removeItem(deleting.value.id)
    deleting.value = null
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

    <CategorySubjectFilter
      v-model:category="categoryFilter"
      v-model:subject="subjectFilter"
      :categories="categories"
    />
    <SearchBar
      v-model="search"
      label="Search interactive reviews"
      placeholder="Search interactive reviews…"
    />

    <div v-if="loading && !reviews.length" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-14" />
    </div>
    <EmptyState
      v-else-if="error && !reviews.length"
      title="Couldn't load interactive reviews"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="retry"
        >Try again</v-btn
      >
    </EmptyState>
    <EmptyState
      v-else-if="!reviews.length && isFiltered()"
      title="No matching reviews"
      message="No interactive review matches the search and the selected category and subject."
      icon="mdi-filter-remove-outline"
    />
    <EmptyState
      v-else-if="!reviews.length"
      title="No interactive reviews yet"
      icon="mdi-book-open-page-variant-outline"
    />
    <template v-else>
      <ul class="space-y-2">
        <v-card
          v-for="(r, i) in reviews"
          :key="r.id"
          tag="li"
          class="card-lift flex flex-wrap items-center gap-3 p-3"
          data-aos="fade-up"
          :data-aos-delay="(i % 20 < 6 ? i % 20 : 6) * 40"
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
            variant="tonal"
            size="small"
            :color="r.is_published ? undefined : 'success'"
            :prepend-icon="r.is_published ? 'mdi-eye-off-outline' : 'mdi-publish'"
            :loading="publishingId === r.id"
            :disabled="publishingId !== null && publishingId !== r.id"
            @click="togglePublish(r)"
          >
            {{ r.is_published ? 'Unpublish' : 'Publish' }}
          </v-btn>
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
      <InfiniteScrollSentinel
        :has-more="hasMore"
        :loading="loadingMore"
        :error="error"
        @load-more="error ? retry() : loadMore()"
      />
    </template>

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'Upload interactive review' : 'Edit interactive review'"
      @close="editing = null"
    >
      <v-form id="review-form" class="space-y-4" @submit.prevent="save">
        <SubjectPicker v-model="form.subject_id" :subjects="formSubjects" />
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
