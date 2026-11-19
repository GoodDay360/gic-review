<script setup lang="ts">
/**
 * Categories group subjects (e.g. "I3", "I4" or any custom name). Students pick a category first.
 * Any editor can create a category (they own it); only its owner or an admin can change it.
 * New quizzes/reviews can only go into subjects of categories you own.
 */
import { reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { ManagedCategory } from '@/api/categories'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useManagedList } from '@/composables/useManagedList'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const auth = useAuthStore()
/** Searched and paged on the server (20 per page, infinite scroll). */
const { search, items, loading, loadingMore, hasMore, error, loadMore, retry, reset } =
  useManagedList<ManagedCategory>('/admin/categories')

const editing = ref<ManagedCategory | 'new' | null>(null)
const saving = ref(false)
const form = reactive({ name: '', slug: '', description: '', sort_order: 0 })
let slugTouched = false

function toSlug(name: string) {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function openNew() {
  Object.assign(form, { name: '', slug: '', description: '', sort_order: items.value.length })
  slugTouched = false
  editing.value = 'new'
}
function openEdit(y: ManagedCategory) {
  Object.assign(form, {
    name: y.name,
    slug: y.slug,
    description: y.description ?? '',
    sort_order: y.sort_order,
  })
  slugTouched = true
  editing.value = y
}
function onName() {
  if (!slugTouched) form.slug = toSlug(form.name)
}

async function save() {
  if (saving.value) return
  saving.value = true
  const body = {
    name: form.name,
    slug: form.slug.trim() || null,
    description: form.description.trim() || null,
    sort_order: form.sort_order,
  }
  try {
    if (editing.value === 'new') await api.post('/admin/categories', body)
    else if (editing.value) await api.put(`/admin/categories/${editing.value.id}`, body)
    toast.success('Category saved')
    editing.value = null
    reset()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    saving.value = false
  }
}

const deleting = ref<ManagedCategory | null>(null)
const deletingCategory = ref(false)
const sharing = ref<ManagedCategory | null>(null)
async function confirmDelete() {
  if (!deleting.value || deletingCategory.value) return
  deletingCategory.value = true
  try {
    await api.delete(`/admin/categories/${deleting.value.id}`)
    toast.success('Category deleted')
    deleting.value = null
    reset()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingCategory.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
        Categories
      </h1>
      <v-btn color="primary" prepend-icon="mdi-plus" @click="openNew">New category </v-btn>
    </div>
    <v-alert type="info" variant="tonal" icon="mdi-information-outline" class="max-w-3xl text-sm">
      Students choose a category first (e.g. <strong>I3</strong>, <strong>I4</strong> or any name),
      then a subject of that category. A category belongs to whoever created it: only its owner (and
      admins) can change it, add subjects, and decide who else may use it. Share a category (people
      icon) to let other managers add and edit content in all its subjects, or share a single
      subject on the Subjects page. Categories can only be created here, never through the API.
    </v-alert>

    <SearchBar v-model="search" label="Search categories" placeholder="Search categories…" />

    <div v-if="loading && !items.length" class="space-y-2">
      <SkeletonBlock v-for="i in 2" :key="i" class="h-14" />
    </div>
    <EmptyState
      v-else-if="error && !items.length"
      title="Couldn't load categories"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="retry"
        >Try again</v-btn
      >
    </EmptyState>
    <EmptyState
      v-else-if="!items.length && search.trim()"
      title="No matching categories"
      message="Try another search."
      icon="mdi-magnify-close"
    />
    <EmptyState
      v-else-if="!items.length"
      title="No categories yet"
      message="Create the first category."
      icon="mdi-layers-outline"
    />
    <template v-else>
      <ul class="space-y-2">
        <v-card
          v-for="(y, i) in items"
          :key="y.id"
          tag="li"
          class="card-lift flex flex-wrap items-center gap-3 p-3"
          data-aos="fade-up"
          :data-aos-delay="(i % 20 < 6 ? i % 20 : 6) * 40"
        >
          <v-avatar color="primary" variant="tonal" rounded="lg" class="px-2 text-sm font-bold">
            {{ y.name.slice(0, 4) }}
          </v-avatar>
          <div class="min-w-0 flex-1">
            <p class="flex flex-wrap items-center gap-2 font-medium text-ctp-text">
              {{ y.name }}
              <v-chip
                v-if="y.can_manage && y.owner_id === auth.user?.id"
                size="x-small"
                color="success"
                variant="tonal"
                prepend-icon="mdi-star-outline"
                >Yours</v-chip
              >
              <v-chip v-else size="x-small" variant="tonal">
                {{ y.owner_name ? `Owned by ${y.owner_name}` : 'No owner (admins only)' }}
              </v-chip>
              <v-chip
                v-if="y.shared_with_you"
                size="x-small"
                color="info"
                variant="tonal"
                prepend-icon="mdi-account-multiple-outline"
                >Shared with you</v-chip
              >
            </p>
            <p class="text-xs text-ctp-subtext0">
              /categories/{{ y.slug }} · order {{ y.sort_order }} · {{ y.subject_count }} subject{{
                y.subject_count === 1 ? '' : 's'
              }}
            </p>
          </div>
          <v-btn
            v-if="y.can_manage || y.shared_with_you"
            variant="text"
            size="small"
            icon="mdi-account-multiple-outline"
            :aria-label="y.can_manage ? 'Share category' : 'Who has access'"
            :title="
              y.can_manage ? 'Share this category with other managers' : 'Who has access / leave'
            "
            @click="sharing = y"
          />
          <v-btn
            v-if="y.can_manage"
            variant="text"
            size="small"
            icon="mdi-pencil-outline"
            aria-label="Edit"
            @click="openEdit(y)"
          />
          <v-btn
            v-if="y.can_manage"
            variant="text"
            size="small"
            icon="mdi-delete-outline"
            color="error"
            aria-label="Delete"
            :disabled="y.subject_count > 0"
            :title="y.subject_count > 0 ? 'Move or delete its subjects first' : 'Delete'"
            @click="deleting = y"
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
      kind="categories"
      :item="sharing ? { id: sharing.id, title: sharing.name } : null"
      @close="sharing = null"
      @changed="reset"
      @left="reset"
    />

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'New category' : 'Edit category'"
      @close="editing = null"
    >
      <v-form id="category-form" class="space-y-4" @submit.prevent="save">
        <v-text-field
          v-model="form.name"
          label="Name"
          required
          maxlength="100"
          placeholder="e.g. I4 or Very Cool Category"
          @input="onName"
        />
        <div class="grid grid-cols-2 gap-3">
          <v-text-field
            v-model="form.slug"
            label="Slug"
            placeholder="from the name"
            pattern="[a-z0-9]+(-[a-z0-9]+)*"
            @input="slugTouched = true"
          />
          <v-text-field v-model.number="form.sort_order" label="Order" type="number" />
        </div>
        <v-textarea v-model="form.description" label="Description (optional)" rows="2" />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="editing = null">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="category-form"
          prepend-icon="mdi-content-save-outline"
          :loading="saving"
          >Save</v-btn
        >
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete category?" size="sm" @close="deleting = null">
      <v-alert type="error" icon="mdi-delete-alert-outline"
        >"{{ deleting?.name }}" will be deleted.</v-alert
      >
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingCategory"
          @click="confirmDelete"
          >Delete</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
