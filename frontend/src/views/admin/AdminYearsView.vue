<script setup lang="ts">
/**
 * Years group subjects (e.g. "I3", "I4" or any custom name). Students pick a year first.
 * Any editor can create a year (they own it); only its owner or an admin can change it.
 * New quizzes/reviews can only go into subjects of years you own.
 */
import { onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import { loadManagedYears, type ManagedYear } from '@/api/years'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const auth = useAuthStore()
const { data: years, loading, run } = useAsync(loadManagedYears, [] as ManagedYear[])
onMounted(run)

const editing = ref<ManagedYear | 'new' | null>(null)
const form = reactive({ name: '', slug: '', description: '', sort_order: 0 })
let slugTouched = false

function toSlug(name: string) {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function openNew() {
  Object.assign(form, { name: '', slug: '', description: '', sort_order: years.value.length })
  slugTouched = false
  editing.value = 'new'
}
function openEdit(y: ManagedYear) {
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
  const body = {
    name: form.name,
    slug: form.slug.trim() || null,
    description: form.description.trim() || null,
    sort_order: form.sort_order,
  }
  try {
    if (editing.value === 'new') await api.post('/admin/years', body)
    else if (editing.value) await api.put(`/admin/years/${editing.value.id}`, body)
    toast.success('Year saved')
    editing.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

const deleting = ref<ManagedYear | null>(null)
const sharing = ref<ManagedYear | null>(null)
async function confirmDelete() {
  if (!deleting.value) return
  try {
    await api.delete(`/admin/years/${deleting.value.id}`)
    toast.success('Year deleted')
    deleting.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-bold text-ctp-text">Years</h1>
      <button class="btn-primary" @click="openNew">
        <AppIcon name="plus" :size="16" /> New year
      </button>
    </div>
    <p class="max-w-2xl text-sm text-ctp-subtext0">
      Students choose a year first (e.g. <strong>I3</strong>, <strong>I4</strong> or any name), then
      a subject of that year. A year belongs to whoever created it: only its owner (and admins) can
      change it, add subjects, and decide who else may use it. Share a year (people icon) to let
      other managers add and edit content in all its subjects, or share a single subject on the
      Subjects page. Years can only be created here, never through the API.
    </p>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 2" :key="i" class="h-14" />
    </div>
    <EmptyState
      v-else-if="!years.length"
      title="No years yet"
      message="Create the first year."
      icon="calendar"
    />
    <ul v-else class="space-y-2">
      <li v-for="y in years" :key="y.id" class="card flex flex-wrap items-center gap-3 p-3">
        <span
          class="flex h-9 min-w-9 items-center justify-center rounded-lg bg-ctp-accent/15 px-2 text-sm font-bold text-ctp-accent"
        >
          {{ y.name.slice(0, 4) }}
        </span>
        <div class="min-w-0 flex-1">
          <p class="flex flex-wrap items-center gap-2 font-medium text-ctp-text">
            {{ y.name }}
            <span
              v-if="y.can_manage && y.owner_id === auth.user?.id"
              class="rounded-full bg-ctp-green/15 px-2 py-0.5 text-[11px] font-medium text-ctp-green"
              >Yours</span
            >
            <span
              v-else
              class="rounded-full bg-ctp-surface0 px-2 py-0.5 text-[11px] text-ctp-subtext0"
            >
              {{ y.owner_name ? `Owned by ${y.owner_name}` : 'No owner (admins only)' }}
            </span>
            <span
              v-if="y.shared_with_you"
              class="rounded-full bg-ctp-blue/15 px-2 py-0.5 text-[11px] font-medium text-ctp-blue"
              >Shared with you</span
            >
          </p>
          <p class="text-xs text-ctp-subtext0">
            /years/{{ y.slug }} · order {{ y.sort_order }} · {{ y.subject_count }} subject{{
              y.subject_count === 1 ? '' : 's'
            }}
          </p>
        </div>
        <button
          v-if="y.can_manage || y.shared_with_you"
          class="btn-ghost btn-sm"
          :aria-label="y.can_manage ? 'Share year' : 'Who has access'"
          :title="y.can_manage ? 'Share this year with other managers' : 'Who has access / leave'"
          @click="sharing = y"
        >
          <AppIcon name="users" :size="16" />
        </button>
        <button v-if="y.can_manage" class="btn-ghost btn-sm" aria-label="Edit" @click="openEdit(y)">
          <AppIcon name="pencil" :size="16" />
        </button>
        <button
          v-if="y.can_manage"
          class="btn-ghost btn-sm text-ctp-red"
          aria-label="Delete"
          :disabled="y.subject_count > 0"
          :title="y.subject_count > 0 ? 'Move or delete its subjects first' : 'Delete'"
          @click="deleting = y"
        >
          <AppIcon name="trash" :size="16" />
        </button>
      </li>
    </ul>

    <ShareDialog
      kind="years"
      :item="sharing ? { id: sharing.id, title: sharing.name } : null"
      @close="sharing = null"
      @changed="run"
      @left="run"
    />

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'New year' : 'Edit year'"
      @close="editing = null"
    >
      <form id="year-form" class="space-y-4" @submit.prevent="save">
        <div>
          <label class="label" for="yr-name">Name</label>
          <input
            id="yr-name"
            v-model="form.name"
            class="input"
            required
            maxlength="100"
            placeholder="e.g. I4 or Very Cool Year"
            @input="onName"
          />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="label" for="yr-slug">Slug</label>
            <input
              id="yr-slug"
              v-model="form.slug"
              class="input"
              placeholder="from the name"
              pattern="[a-z0-9]+(-[a-z0-9]+)*"
              @input="slugTouched = true"
            />
          </div>
          <div>
            <label class="label" for="yr-order">Order</label>
            <input id="yr-order" v-model.number="form.sort_order" type="number" class="input" />
          </div>
        </div>
        <div>
          <label class="label" for="yr-desc">Description (optional)</label>
          <textarea id="yr-desc" v-model="form.description" class="input" rows="2" />
        </div>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="editing = null">Cancel</button>
        <button class="btn-primary" type="submit" form="year-form">Save</button>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete year?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">"{{ deleting?.name }}" will be deleted.</p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" @click="confirmDelete">Delete</button>
      </template>
    </BaseModal>
  </div>
</template>
