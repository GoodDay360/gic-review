<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { api, errorMessage, uploadWithProgress } from '@/api/client'
import type { Review, Subject } from '@/api/types'
import OwnershipBadge from '@/components/admin/OwnershipBadge.vue'
import ShareDialog from '@/components/admin/ShareDialog.vue'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const subjects = ref<Subject[]>([])
const reviews = ref<Review[]>([])
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    ;[subjects.value, reviews.value] = await Promise.all([
      api.get<Subject[]>('/subjects'),
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

function openNew() {
  Object.assign(form, {
    subject_id: subjects.value[0]?.id ?? 0,
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
function onFile(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0] ?? null
  if (f && !f.name.toLowerCase().endsWith('.html')) {
    toast.error('Please choose an .html file')
    ;(e.target as HTMLInputElement).value = ''
    return
  }
  file.value = f
}

async function save() {
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

const sharing = ref<Review | null>(null)
const deleting = ref<Review | null>(null)
async function confirmDelete() {
  if (!deleting.value) return
  try {
    await api.delete(`/admin/reviews/${deleting.value.id}`)
    toast.success('Review deleted')
    deleting.value = null
    load()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-bold text-ctp-text">Interactive reviews</h1>
      <button class="btn-primary" :disabled="!subjects.length" @click="openNew">
        <AppIcon name="upload" :size="16" /> Upload review
      </button>
    </div>
    <p class="text-sm text-ctp-subtext0">
      A review is one self-contained <code class="rounded bg-ctp-surface0 px-1">.html</code> file
      shown in a sandboxed frame. See
      <code class="rounded bg-ctp-surface0 px-1">docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md</code> for
      the rules and a starter template. Replacing the file keeps the same URL.
    </p>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-14" />
    </div>
    <EmptyState v-else-if="!reviews.length" title="No interactive reviews yet" icon="book" />
    <ul v-else class="space-y-2">
      <li v-for="r in reviews" :key="r.id" class="card flex flex-wrap items-center gap-3 p-3">
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">{{ r.title }}</p>
          <p class="text-xs text-ctp-subtext0">
            {{ subjectName(r.subject_id) }} · /{{ r.slug }} · order {{ r.sort_order }}
          </p>
        </div>
        <span
          class="rounded-full px-2.5 py-1 text-xs font-medium"
          :class="
            r.is_published ? 'bg-ctp-green/15 text-ctp-green' : 'bg-ctp-surface0 text-ctp-subtext0'
          "
        >
          {{ r.is_published ? 'Published' : 'Hidden' }}
        </span>
        <a
          :href="r.url"
          target="_blank"
          rel="noopener noreferrer"
          class="btn-ghost btn-sm"
          aria-label="Open raw HTML"
          ><AppIcon name="code" :size="16"
        /></a>
        <button class="btn-ghost btn-sm" aria-label="Edit" @click="openEdit(r)">
          <AppIcon name="pencil" :size="16" />
        </button>
        <OwnershipBadge :item="r" />
        <button
          class="btn-ghost btn-sm"
          aria-label="Share"
          title="Share with managers"
          @click="sharing = r"
        >
          <AppIcon name="users" :size="16" />
        </button>
        <button
          v-if="r.access === 'owner' || r.access === 'admin'"
          class="btn-ghost btn-sm text-ctp-red"
          aria-label="Delete"
          @click="deleting = r"
        >
          <AppIcon name="trash" :size="16" />
        </button>
      </li>
    </ul>

    <BaseModal
      :open="!!editing"
      :title="editing === 'new' ? 'Upload interactive review' : 'Edit interactive review'"
      @close="editing = null"
    >
      <form id="review-form" class="space-y-4" @submit.prevent="save">
        <div>
          <label class="label" for="rv-subject">Subject</label>
          <select id="rv-subject" v-model="form.subject_id" class="input">
            <option v-for="s in subjects" :key="s.id" :value="s.id">
              {{ s.year_name }} · {{ s.name }}
            </option>
          </select>
        </div>
        <div>
          <label class="label" for="rv-title">Title</label>
          <input id="rv-title" v-model="form.title" class="input" required maxlength="200" />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="label" for="rv-slug">Slug (optional)</label>
            <input
              id="rv-slug"
              v-model="form.slug"
              class="input"
              placeholder="from title"
              pattern="[a-z0-9]+(-[a-z0-9]+)*"
            />
          </div>
          <div>
            <label class="label" for="rv-order">Order</label>
            <input id="rv-order" v-model.number="form.sort_order" type="number" class="input" />
          </div>
        </div>
        <label class="flex items-center gap-2 text-sm text-ctp-subtext1">
          <input v-model="form.is_published" type="checkbox" class="h-4 w-4 accent-ctp-accent" />
          Published
        </label>
        <div>
          <label class="label" for="rv-file">{{
            editing === 'new' ? 'HTML file' : 'Replace HTML file (optional)'
          }}</label>
          <input id="rv-file" type="file" accept=".html,text/html" class="input" @change="onFile" />
        </div>
        <div v-if="progress !== null" class="h-1.5 overflow-hidden rounded-full bg-ctp-surface0">
          <div class="h-full bg-ctp-accent" :style="{ width: `${progress * 100}%` }" />
        </div>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="editing = null">Cancel</button>
        <button class="btn-primary" type="submit" form="review-form" :disabled="progress !== null">
          Save
        </button>
      </template>
    </BaseModal>

    <ShareDialog
      kind="reviews"
      :item="sharing"
      @close="sharing = null"
      @changed="load"
      @left="load"
    />

    <BaseModal :open="!!deleting" title="Delete review?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        "{{ deleting?.title }}" and its HTML file will be permanently deleted.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" @click="confirmDelete">Delete</button>
      </template>
    </BaseModal>
  </div>
</template>
