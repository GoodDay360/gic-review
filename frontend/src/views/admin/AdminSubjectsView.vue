<script setup lang="ts">
/**
 * Subjects live inside years. You can add/edit subjects only in years you own (admins: all).
 * Subjects can only be created here, never through the API.
 */
import { computed, onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { Subject, SubjectInput } from '@/api/types'
import { loadManagedYears, type ManagedSubject, type ManagedYear } from '@/api/years'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { ACCENT_NAMES } from '@/stores/theme'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const auth = useAuthStore()
const { data: years, loading, run } = useAsync(loadManagedYears, [] as ManagedYear[])
onMounted(run)
const subjects = computed<ManagedSubject[]>(() => years.value.flatMap((y) => y.subjects))
/** Years you can add subjects to. */
const myYears = computed(() => years.value.filter((y) => y.can_manage))
const canManage = (s: Subject) => years.value.some((y) => y.id === s.year_id && y.can_manage)
const sharing = ref<ManagedSubject | null>(null)

const editing = ref<Subject | 'new' | null>(null)
const saving = ref(false)
const form = reactive<SubjectInput>({
  year_id: 0,
  name: '',
  slug: '',
  description: '',
  color: 'mauve',
  sort_order: 0,
})
let slugTouched = false

function openNew() {
  Object.assign(form, {
    year_id: myYears.value[0]?.id ?? 0,
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
    year_id: s.year_id,
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
    run()
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
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingSubject.value = false
  }
}

// Maintenance: remove R2 files that no row references (admin only).
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
      <h1 class="text-2xl font-bold text-ctp-text">Subjects</h1>
      <div class="flex flex-wrap gap-2">
        <button
          v-if="auth.isAdmin"
          class="btn-secondary"
          :disabled="cleaning"
          title="Delete R2 files that no quiz, question, option or review uses (keeps files younger than 1 hour)"
          @click="cleanupOrphans"
        >
          <AppIcon name="trash" :size="16" />
          {{ cleaning ? 'Cleaning…' : 'Clean up orphaned images' }}
        </button>
        <button
          class="btn-primary"
          :disabled="!myYears.length"
          :title="myYears.length ? '' : 'Create a year first (Years page)'"
          @click="openNew"
        >
          <AppIcon name="plus" :size="16" /> New subject
        </button>
      </div>
    </div>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 2" :key="i" class="h-14" />
    </div>
    <ul v-else class="space-y-2">
      <li v-for="s in subjects" :key="s.id" class="card flex flex-wrap items-center gap-3 p-3">
        <span
          class="h-8 w-8 rounded-lg"
          :style="{ backgroundColor: `var(--ctp-${s.color})` }"
          aria-hidden="true"
        />
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">{{ s.name }}</p>
          <p class="text-xs text-ctp-subtext0">
            <span class="font-medium text-ctp-subtext1">{{ s.year_name }}</span> · /{{ s.slug }} ·
            order {{ s.sort_order }} · {{ s.quiz_count }} published quizzes
          </p>
        </div>
        <span
          v-if="s.shared_with_you"
          class="rounded-full bg-ctp-blue/15 px-2 py-0.5 text-[11px] font-medium text-ctp-blue"
          >Shared with you</span
        >
        <span v-else-if="s.can_add_content && !canManage(s)" class="text-xs text-ctp-subtext0"
          >Year shared with you</span
        >
        <span v-else-if="!canManage(s)" class="text-xs text-ctp-overlay1">Not your year</span>
        <button
          v-if="canManage(s) || s.shared_with_you"
          class="btn-ghost btn-sm"
          :aria-label="canManage(s) ? 'Share subject' : 'Who has access'"
          :title="
            canManage(s) ? 'Share this subject with other managers' : 'Who has access / leave'
          "
          @click="sharing = s"
        >
          <AppIcon name="users" :size="16" />
        </button>
        <button v-if="canManage(s)" class="btn-ghost btn-sm" aria-label="Edit" @click="openEdit(s)">
          <AppIcon name="pencil" :size="16" />
        </button>
        <button
          v-if="canManage(s)"
          class="btn-ghost btn-sm text-ctp-red"
          aria-label="Delete"
          @click="deleting = s"
        >
          <AppIcon name="trash" :size="16" />
        </button>
      </li>
    </ul>

    <ShareDialog
      kind="subjects"
      :item="sharing ? { id: sharing.id, title: `${sharing.year_name} · ${sharing.name}` } : null"
      @close="sharing = null"
      @changed="run"
      @left="run"
    />

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'New subject' : 'Edit subject'"
      @close="editing = null"
    >
      <form id="subject-form" class="space-y-4" @submit.prevent="save">
        <div>
          <label class="label" for="sj-year">Year</label>
          <select id="sj-year" v-model="form.year_id" class="input" required>
            <option v-for="y in myYears" :key="y.id" :value="y.id">{{ y.name }}</option>
          </select>
          <p v-if="!myYears.length" class="mt-1 text-xs text-ctp-yellow">
            Create a year first (Years page). You can only add subjects to your own years.
          </p>
        </div>
        <div>
          <label class="label" for="sj-name">Name</label>
          <input
            id="sj-name"
            v-model="form.name"
            class="input"
            required
            maxlength="150"
            @input="onName"
          />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="label" for="sj-slug">Slug</label>
            <input
              id="sj-slug"
              v-model="form.slug"
              class="input"
              required
              pattern="[a-z0-9]+(-[a-z0-9]+)*"
              @input="slugTouched = true"
            />
          </div>
          <div>
            <label class="label" for="sj-order">Order</label>
            <input id="sj-order" v-model.number="form.sort_order" type="number" class="input" />
          </div>
        </div>
        <div>
          <label class="label" for="sj-desc">Description</label>
          <textarea id="sj-desc" v-model="form.description" class="input" rows="3" />
        </div>
        <fieldset>
          <legend class="label">Color</legend>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="c in ACCENT_NAMES"
              :key="c"
              type="button"
              :aria-label="c"
              :aria-pressed="form.color === c"
              :title="c"
              class="h-7 w-7 rounded-full ring-offset-2 ring-offset-ctp-base"
              :class="form.color === c ? 'ring-2 ring-ctp-text' : ''"
              :style="{ backgroundColor: `var(--ctp-${c})` }"
              @click="form.color = c"
            />
          </div>
        </fieldset>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="editing = null">Cancel</button>
        <button class="btn-primary" type="submit" form="subject-form" :disabled="saving">
          {{ saving ? 'Saving…' : 'Save' }}
        </button>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete subject?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        "{{ deleting?.name }}" can only be deleted when it has no quizzes and no interactive
        reviews.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" :disabled="deletingSubject" @click="confirmDelete">
          {{ deletingSubject ? 'Deleting…' : 'Delete' }}
        </button>
      </template>
    </BaseModal>
  </div>
</template>
