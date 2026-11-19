<script setup lang="ts">
/**
 * Reusable image field: click to browse, drag & drop, or paste (Ctrl+V) a screenshot.
 * Uploads to POST /api/admin/uploads/image and exposes the returned key via v-model.
 */
import { computed, ref, useId } from 'vue'

import { errorMessage, uploadWithProgress } from '@/api/client'
import type { UploadedImage } from '@/api/types'
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
    <label :for="inputId" class="block text-sm font-medium text-ctp-subtext1">{{ label }}</label>
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
      class="animate__animated animate__fadeIn flex items-start gap-3 rounded-lg border border-ctp-surface1 bg-ctp-surface0 p-2"
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
        <v-text-field
          v-if="withAlt"
          v-model="alt"
          label="Alt text"
          placeholder="Describe the image for screen readers"
          maxlength="255"
          density="compact"
          prepend-inner-icon="mdi-text-short"
          hide-details
        />
        <div class="flex gap-2">
          <v-btn
            variant="tonal"
            size="small"
            prepend-icon="mdi-upload"
            type="button"
            :disabled="uploading"
            @click="browse"
            >Replace
          </v-btn>
          <v-btn
            variant="text"
            size="small"
            prepend-icon="mdi-delete-outline"
            type="button"
            color="error"
            :disabled="uploading"
            @click="remove"
            >Remove
          </v-btn>
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
      <v-icon
        :icon="dragging ? 'mdi-image-move' : 'mdi-image-plus-outline'"
        :size="compact ? 22 : 30"
        :class="dragging ? 'text-ctp-accent' : 'text-ctp-overlay1'"
      />
      <span class="text-ctp-subtext1">
        <span class="font-medium text-ctp-accent">Browse</span>, drag & drop, or click here and
        press Ctrl+V
      </span>
      <span v-if="!compact" class="text-xs text-ctp-overlay1">PNG, JPG, WebP or GIF</span>
    </div>

    <v-progress-linear
      v-if="uploading"
      :model-value="(progress ?? 0) * 100"
      color="primary"
      height="6"
      rounded
      striped
      aria-label="Upload progress"
    />
    <v-alert v-if="error" type="error" density="compact" role="alert" :text="error" />
  </div>
</template>
