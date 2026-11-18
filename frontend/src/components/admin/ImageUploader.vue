<script setup lang="ts">
/**
 * Reusable image field: click to browse, drag & drop, or paste (Ctrl+V) a screenshot.
 * Uploads to POST /api/admin/uploads/image and exposes the returned key via v-model.
 */
import { computed, ref, useId } from 'vue'

import { errorMessage, uploadWithProgress } from '@/api/client'
import type { UploadedImage } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import { useLightboxStore } from '@/stores/lightbox'

const props = withDefaults(
  defineProps<{
    label: string
    quizId?: number | null
    withAlt?: boolean
    compact?: boolean
  }>(),
  { quizId: null, withAlt: true, compact: false },
)

/** R2 object key saved on the record. */
const key = defineModel<string | null>('imageKey', { default: null })
/** Public URL for the preview. */
const url = defineModel<string | null>('imageUrl', { default: null })
const alt = defineModel<string | null>('alt', { default: null })

const emit = defineEmits<{ uploaded: [key: string] }>()

const ACCEPT = ['image/png', 'image/jpeg', 'image/webp', 'image/gif']
const fileInput = ref<HTMLInputElement | null>(null)
const dragging = ref(false)
const progress = ref<number | null>(null)
const error = ref<string | null>(null)
const inputId = useId()
const lightbox = useLightboxStore()

const uploading = computed(() => progress.value !== null)

async function upload(file: File) {
  if (uploading.value) return
  error.value = null
  if (!ACCEPT.includes(file.type)) {
    error.value = 'Only PNG, JPEG, WebP or GIF images are allowed.'
    return
  }
  const form = new FormData()
  // Pasted screenshots have a generic name; give them an extension that matches the type.
  const ext = file.type.split('/')[1]?.replace('jpeg', 'jpg') ?? 'png'
  const name = file.name && file.name.includes('.') ? file.name : `pasted.${ext}`
  form.append('file', file, name)
  if (props.quizId) form.append('quiz_id', String(props.quizId))
  progress.value = 0
  try {
    const res = await uploadWithProgress<UploadedImage>('/admin/uploads/image', form, (f) => {
      progress.value = f
    })
    key.value = res.key
    url.value = res.url
    emit('uploaded', res.key)
  } catch (err) {
    error.value = errorMessage(err)
  } finally {
    progress.value = null
  }
}

function onFileChange(e: Event) {
  const file = (e.target as HTMLInputElement).files?.[0]
  if (file) upload(file)
  ;(e.target as HTMLInputElement).value = ''
}

function onDrop(e: DragEvent) {
  dragging.value = false
  const file = e.dataTransfer?.files?.[0]
  if (file) upload(file)
}

function onPaste(e: ClipboardEvent) {
  const item = Array.from(e.clipboardData?.items ?? []).find((i) => i.type.startsWith('image/'))
  const file = item?.getAsFile()
  if (file) {
    e.preventDefault()
    upload(file)
  }
}

function remove() {
  key.value = null
  url.value = null
  alt.value = null
}

function browse() {
  fileInput.value?.click()
}
</script>

<template>
  <div class="space-y-2" @paste="onPaste">
    <label :for="inputId" class="label">{{ label }}</label>
    <input
      :id="inputId"
      ref="fileInput"
      type="file"
      :accept="ACCEPT.join(',')"
      class="sr-only"
      @change="onFileChange"
    />

    <div
      v-if="url"
      class="flex items-start gap-3 rounded-lg border border-ctp-surface1 bg-ctp-surface0 p-2"
    >
      <button
        type="button"
        class="shrink-0 cursor-zoom-in overflow-hidden rounded-md bg-ctp-crust"
        :aria-label="`Preview ${label}`"
        @click="lightbox.open(url!, alt ?? '')"
      >
        <img
          :src="url"
          :alt="alt ?? ''"
          class="object-contain"
          :class="compact ? 'h-14 w-20' : 'h-24 w-32'"
        />
      </button>
      <div class="flex flex-1 flex-col gap-2">
        <input
          v-if="withAlt"
          v-model="alt"
          class="input"
          placeholder="Alt text (describe the image for screen readers)"
          maxlength="255"
          :aria-label="`${label} alt text`"
        />
        <div class="flex gap-2">
          <button type="button" class="btn-secondary btn-sm" :disabled="uploading" @click="browse">
            <AppIcon name="upload" :size="14" /> Replace
          </button>
          <button
            type="button"
            class="btn-ghost btn-sm text-ctp-red"
            :disabled="uploading"
            @click="remove"
          >
            <AppIcon name="trash" :size="14" /> Remove
          </button>
        </div>
      </div>
    </div>

    <div
      v-else
      role="button"
      tabindex="0"
      :aria-label="`${label}: click to browse, drop an image, or focus and paste with Ctrl+V`"
      class="flex cursor-pointer flex-col items-center justify-center gap-1 rounded-lg border-2 border-dashed px-4 text-center text-sm transition"
      :class="[
        dragging
          ? 'border-ctp-accent bg-ctp-accent/10'
          : 'border-ctp-surface1 hover:border-ctp-overlay0',
        compact ? 'py-3' : 'py-6',
      ]"
      @click="browse"
      @keydown.enter.prevent="browse"
      @keydown.space.prevent="browse"
      @dragover.prevent="dragging = true"
      @dragleave.prevent="dragging = false"
      @drop.prevent="onDrop"
    >
      <AppIcon name="image" class="text-ctp-overlay1" />
      <span class="text-ctp-subtext1">
        <span class="font-medium text-ctp-accent">Browse</span>, drag & drop, or click here and
        press Ctrl+V
      </span>
      <span v-if="!compact" class="text-xs text-ctp-overlay1">PNG, JPG, WebP or GIF</span>
    </div>

    <div
      v-if="uploading"
      class="h-1.5 overflow-hidden rounded-full bg-ctp-surface0"
      role="progressbar"
      :aria-valuenow="Math.round((progress ?? 0) * 100)"
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <div
        class="h-full bg-ctp-accent transition-all"
        :style="{ width: `${(progress ?? 0) * 100}%` }"
      />
    </div>
    <p v-if="error" role="alert" class="text-xs text-ctp-red">{{ error }}</p>
  </div>
</template>
