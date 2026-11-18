<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { Subject, SubjectInput } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { ACCENT_NAMES } from '@/stores/theme'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const { data: subjects, loading, run } = useAsync(() => api.get<Subject[]>('/subjects'), [])
onMounted(run)

const editing = ref<Subject | 'new' | null>(null)
const form = reactive<SubjectInput>({
  name: '',
  slug: '',
  description: '',
  color: 'mauve',
  sort_order: 0,
})
let slugTouched = false

function openNew() {
  Object.assign(form, {
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
  const body = { ...form, description: form.description?.trim() || null }
  try {
    if (editing.value === 'new') await api.post('/admin/subjects', body)
    else if (editing.value) await api.put(`/admin/subjects/${editing.value.id}`, body)
    toast.success('Subject saved')
    editing.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

const deleting = ref<Subject | null>(null)
async function confirmDelete() {
  if (!deleting.value) return
  try {
    await api.delete(`/admin/subjects/${deleting.value.id}`)
    toast.success('Subject deleted')
    deleting.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

// Maintenance: remove R2 files that no row references (admin only).
const cleaning = ref(false)
async function cleanupOrphans() {
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
          class="btn-secondary"
          :disabled="cleaning"
          title="Delete R2 files that no quiz, question, option or review uses (keeps files younger than 1 hour)"
          @click="cleanupOrphans"
        >
          <AppIcon name="trash" :size="16" />
          {{ cleaning ? 'Cleaning…' : 'Clean up orphaned images' }}
        </button>
        <button class="btn-primary" @click="openNew">
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
            /{{ s.slug }} · order {{ s.sort_order }} · {{ s.quiz_count }} published quizzes
          </p>
        </div>
        <button class="btn-ghost btn-sm" aria-label="Edit" @click="openEdit(s)">
          <AppIcon name="pencil" :size="16" />
        </button>
        <button class="btn-ghost btn-sm text-ctp-red" aria-label="Delete" @click="deleting = s">
          <AppIcon name="trash" :size="16" />
        </button>
      </li>
    </ul>

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'New subject' : 'Edit subject'"
      @close="editing = null"
    >
      <form id="subject-form" class="space-y-4" @submit.prevent="save">
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
        <button class="btn-primary" type="submit" form="subject-form">Save</button>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete subject?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        "{{ deleting?.name }}" can only be deleted when it has no quizzes and no interactive
        reviews.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" @click="confirmDelete">Delete</button>
      </template>
    </BaseModal>
  </div>
</template>
