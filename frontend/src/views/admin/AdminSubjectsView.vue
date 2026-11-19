<script setup lang="ts">
/**
 * Subjects live inside categories. You can add subjects to categories you own or that are shared with
 * you, and edit/delete them only in categories you own (admins: all).
 * Subjects can only be created here, never through the API.
 */
import { computed, onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { Subject, SubjectInput } from '@/api/types'
import { loadManagedCategories, type ManagedSubject, type ManagedCategory } from '@/api/categories'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useManagedList } from '@/composables/useManagedList'
import { ACCENT_NAMES } from '@/stores/theme'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const auth = useAuthStore()
/** Full category list: for the form, ownership checks and the next sort order. */
const { data: categories, run } = useAsync(loadManagedCategories, [] as ManagedCategory[])
onMounted(run)
const subjects = computed<ManagedSubject[]>(() => categories.value.flatMap((y) => y.subjects))
/** Searched and paged on the server (20 per page, infinite scroll). */
const { search, items, loading, loadingMore, hasMore, error, loadMore, retry, reset } =
  useManagedList<ManagedSubject>('/admin/subjects')
/** Reload both the category data and the subject list after a change. */
function refresh() {
  run()
  reset()
}
/** Categories you can add subjects to. */
const myCategories = computed(() =>
  categories.value.filter((y) => y.can_manage || y.shared_with_you),
)
const canManage = (s: Subject) =>
  categories.value.some((y) => y.id === s.category_id && y.can_manage)
const sharing = ref<ManagedSubject | null>(null)

const editing = ref<Subject | 'new' | null>(null)
const saving = ref(false)
const form = reactive<SubjectInput>({
  category_id: 0,
  name: '',
  slug: '',
  description: '',
  color: 'mauve',
  sort_order: 0,
})
let slugTouched = false

function openNew() {
  Object.assign(form, {
    category_id: myCategories.value[0]?.id ?? 0,
    name: '',
    slug: '',
    description: '',
    color: 'mauve',
    sort_order: subjects.value.length,
  })
  slugTouched = false
  editing.value = 'new'
}
function openEdit(s: Subject) {
  Object.assign(form, {
    category_id: s.category_id,
    name: s.name,
    slug: s.slug,
    description: s.description ?? '',
    color: s.color,
    sort_order: s.sort_order,
  })
  slugTouched = true
  editing.value = s
}
function onName() {
  if (!slugTouched)
    form.slug = form.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
}

async function save() {
  if (saving.value) return
  saving.value = true
  const body = { ...form, description: form.description?.trim() || null }
  try {
    if (editing.value === 'new') await api.post('/admin/subjects', body)
    else if (editing.value) await api.put(`/admin/subjects/${editing.value.id}`, body)
    toast.success('Subject saved')
    editing.value = null
    refresh()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    saving.value = false
  }
}

const deleting = ref<Subject | null>(null)
const deletingSubject = ref(false)
async function confirmDelete() {
  if (!deleting.value || deletingSubject.value) return
  deletingSubject.value = true
  try {
    await api.delete(`/admin/subjects/${deleting.value.id}`)
    toast.success('Subject deleted')
    deleting.value = null
    refresh()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingSubject.value = false
  }
}

// Maintenance: remove R2 files that no row references (admin only, production builds only:
// a dev server may share the bucket with production but not its database).
const cleanupAllowed = import.meta.env.PROD
const cleaning = ref(false)
async function cleanupOrphans() {
  if (cleaning.value) return
  cleaning.value = true
  try {
    const res = await api.post<{ scanned: number; deleted: number }>(
      '/admin/uploads/cleanup-orphans',
    )
    toast.success(`Scanned ${res.scanned} files, deleted ${res.deleted} orphaned file(s).`)
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    cleaning.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
        Subjects
      </h1>
      <div class="flex flex-wrap gap-2">
        <v-btn
          v-if="auth.isAdmin"
          variant="tonal"
          prepend-icon="mdi-broom"
          :title="
            cleanupAllowed
              ? 'Delete R2 files that no quiz, question, option or review uses (keeps files younger than 1 hour)'
              : 'Only available in production: a dev server could delete images production still uses'
          "
          :disabled="!cleanupAllowed"
          :loading="cleaning"
          @click="cleanupOrphans"
          >Clean up orphaned images</v-btn
        >
        <v-btn
          color="primary"
          prepend-icon="mdi-plus"
          :disabled="!myCategories.length"
          :title="
            myCategories.length
              ? ''
              : 'Create a category first (Categories page), or ask an owner to share one'
          "
          @click="openNew"
          >New subject
        </v-btn>
      </div>
    </div>

    <SearchBar v-model="search" label="Search subjects" placeholder="Search subjects…" />

    <div v-if="loading && !items.length" class="space-y-2">
      <SkeletonBlock v-for="i in 2" :key="i" class="h-14" />
    </div>
    <EmptyState
      v-else-if="error && !items.length"
      title="Couldn't load subjects"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="retry"
        >Try again</v-btn
      >
    </EmptyState>
    <EmptyState
      v-else-if="!items.length && search.trim()"
      title="No matching subjects"
      message="Try another search."
      icon="mdi-magnify-close"
    />
    <EmptyState
      v-else-if="!items.length"
      title="No subjects yet"
      message="Create the first subject."
      icon="mdi-book-open-page-variant-outline"
    />
    <template v-else>
      <ul class="space-y-2">
        <v-card
          v-for="(s, i) in items"
          :key="s.id"
          tag="li"
          class="card-lift flex flex-wrap items-center gap-3 p-3"
          data-aos="fade-up"
          :data-aos-delay="(i % 20 < 6 ? i % 20 : 6) * 40"
        >
          <v-avatar :color="`ctp-${s.color}`" rounded="lg" size="36" aria-hidden="true">
            <v-icon icon="mdi-book-open-page-variant-outline" size="20" />
          </v-avatar>
          <div class="min-w-0 flex-1">
            <p class="font-medium text-ctp-text">{{ s.name }}</p>
            <p class="text-xs text-ctp-subtext0">
              <span class="font-medium text-ctp-subtext1">{{ s.category_name }}</span> · /{{
                s.slug
              }}
              · order {{ s.sort_order }} · {{ s.quiz_count }} published quizzes
            </p>
          </div>
          <v-chip
            v-if="s.shared_with_you"
            size="x-small"
            color="info"
            variant="tonal"
            prepend-icon="mdi-account-multiple-outline"
            >Shared with you</v-chip
          >
          <span v-else-if="s.can_add_content && !canManage(s)" class="text-xs text-ctp-subtext0"
            >Category shared with you</span
          >
          <span v-else-if="!canManage(s)" class="text-xs text-ctp-overlay1">Not your category</span>
          <v-btn
            v-if="canManage(s) || s.shared_with_you"
            variant="text"
            size="small"
            icon="mdi-account-multiple-outline"
            :aria-label="canManage(s) ? 'Share subject' : 'Who has access'"
            :title="
              canManage(s) ? 'Share this subject with other managers' : 'Who has access / leave'
            "
            @click="sharing = s"
          />
          <v-btn
            v-if="canManage(s)"
            variant="text"
            size="small"
            icon="mdi-pencil-outline"
            aria-label="Edit"
            @click="openEdit(s)"
          />
          <v-btn
            v-if="canManage(s)"
            variant="text"
            size="small"
            icon="mdi-delete-outline"
            color="error"
            aria-label="Delete"
            @click="deleting = s"
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

    <ShareDialog
      kind="subjects"
      :item="
        sharing ? { id: sharing.id, title: `${sharing.category_name} · ${sharing.name}` } : null
      "
      @close="sharing = null"
      @changed="refresh"
      @left="refresh"
    />

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'New subject' : 'Edit subject'"
      @close="editing = null"
    >
      <v-form id="subject-form" class="space-y-4" @submit.prevent="save">
        <v-select
          v-model="form.category_id"
          :items="myCategories"
          item-title="name"
          item-value="id"
          label="Category"
          prepend-inner-icon="mdi-layers-outline"
        />
        <v-alert v-if="!myCategories.length" type="warning" density="compact">
          Create a category first (Categories page). You can add subjects to your own categories and
          to categories shared with you.
        </v-alert>
        <v-text-field v-model="form.name" label="Name" required maxlength="150" @input="onName" />
        <div class="grid grid-cols-2 gap-3">
          <v-text-field
            v-model="form.slug"
            label="Slug"
            required
            pattern="[a-z0-9]+(-[a-z0-9]+)*"
            @input="slugTouched = true"
          />
          <v-text-field v-model.number="form.sort_order" label="Order" type="number" />
        </div>
        <v-textarea v-model="form.description" label="Description" rows="3" />
        <fieldset>
          <legend class="mb-2 text-sm font-medium text-ctp-subtext1">Color</legend>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="c in ACCENT_NAMES"
              :key="c"
              v-ripple
              type="button"
              :aria-label="c"
              :aria-pressed="form.color === c"
              :title="c"
              class="flex h-8 w-8 items-center justify-center rounded-full ring-offset-2 ring-offset-ctp-base transition"
              :class="form.color === c ? 'ring-2 ring-ctp-text' : 'hover:scale-110'"
              :style="{ backgroundColor: `var(--ctp-${c})`, color: 'var(--ctp-base)' }"
              @click="form.color = c"
            >
              <v-icon v-if="form.color === c" icon="mdi-check" size="16" />
            </button>
          </div>
        </fieldset>
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="editing = null">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="subject-form"
          prepend-icon="mdi-content-save-outline"
          :loading="saving"
          >Save</v-btn
        >
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete subject?" size="sm" @close="deleting = null">
      <v-alert type="warning" icon="mdi-delete-alert-outline">
        "{{ deleting?.name }}" can only be deleted when it has no quizzes and no interactive
        reviews.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingSubject"
          @click="confirmDelete"
          >Delete</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
