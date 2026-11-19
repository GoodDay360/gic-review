<script setup lang="ts">
/**
 * App dialog built on `<v-dialog>` (focus trap, Esc / outside click to close, focus restore).
 * Full-screen-ish sheet on phones, centered card on larger screens.
 */
import { computed, useId } from 'vue'
import { useDisplay } from 'vuetify'

import { useLightboxStore } from '@/stores/lightbox'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    size?: 'sm' | 'md' | 'lg' | 'xl'
    /** Keep the dialog open on outside click / Esc (e.g. while saving). */
    persistent?: boolean
  }>(),
  { size: 'md', description: undefined, persistent: false },
)
const emit = defineEmits<{ close: [] }>()

const lightbox = useLightboxStore()
const { xs } = useDisplay()
const titleId = useId()
const sizes = { sm: 400, md: 540, lg: 720, xl: 960 }

const model = computed({
  get: () => props.open,
  set: (v) => {
    // The image lightbox sits above dialogs and handles its own Esc.
    if (!v && !lightbox.src) emit('close')
  },
})
</script>

<template>
  <v-dialog
    v-model="model"
    :max-width="sizes[size]"
    :persistent="persistent"
    :fullscreen="xs && size !== 'sm'"
    :transition="xs ? 'dialog-bottom-transition' : 'scale-transition'"
    :aria-labelledby="titleId"
  >
    <v-card :rounded="xs && size !== 'sm' ? 0 : 'xl'" class="bg-ctp-base">
      <v-card-item class="border-b border-ctp-surface0 px-5 py-4">
        <v-card-title :id="titleId" class="p-0 text-lg font-semibold text-wrap">
          {{ title }}
        </v-card-title>
        <v-card-subtitle v-if="description" class="mt-1 p-0 text-sm text-wrap opacity-100">
          <span class="text-ctp-subtext0">{{ description }}</span>
        </v-card-subtitle>
        <template #append>
          <v-btn
            icon="mdi-close"
            variant="text"
            size="small"
            aria-label="Close dialog"
            @click="emit('close')"
          />
        </template>
      </v-card-item>
      <v-card-text class="p-5">
        <slot />
      </v-card-text>
      <!-- A plain footer (not v-card-actions, which would force every button to variant="text"). -->
      <div
        v-if="$slots.footer"
        class="flex flex-wrap items-center justify-end gap-2 border-t border-ctp-surface0 px-4 py-3"
      >
        <slot name="footer" />
      </div>
    </v-card>
  </v-dialog>
</template>
