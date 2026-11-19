<script setup lang="ts">
/** Full-screen, zoomable image viewer. Esc closes; wheel/buttons zoom; drag to pan. */
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'

import { useLightboxStore } from '@/stores/lightbox'

const lightbox = useLightboxStore()
const scale = ref(1)
const offset = ref({ x: 0, y: 0 })
const closeBtn = ref<{ $el: HTMLElement } | null>(null)
let drag: { x: number; y: number; ox: number; oy: number } | null = null
let previouslyFocused: HTMLElement | null = null

function reset() {
  scale.value = 1
  offset.value = { x: 0, y: 0 }
}

function zoom(delta: number) {
  scale.value = Math.min(5, Math.max(1, +(scale.value + delta).toFixed(2)))
  if (scale.value === 1) offset.value = { x: 0, y: 0 }
}

function onWheel(e: WheelEvent) {
  e.preventDefault()
  zoom(e.deltaY < 0 ? 0.25 : -0.25)
}

function onPointerDown(e: PointerEvent) {
  if (scale.value === 1) return
  drag = { x: e.clientX, y: e.clientY, ox: offset.value.x, oy: offset.value.y }
  ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
}
function onPointerMove(e: PointerEvent) {
  if (!drag) return
  offset.value = { x: drag.ox + e.clientX - drag.x, y: drag.oy + e.clientY - drag.y }
}
function onPointerUp() {
  drag = null
}

function onKey(e: KeyboardEvent) {
  if (!lightbox.src) return
  if (e.key === 'Escape') {
    e.stopPropagation()
    lightbox.close()
  } else if (e.key === '+' || e.key === '=') zoom(0.5)
  else if (e.key === '-') zoom(-0.5)
  else if (e.key === 'Tab') {
    e.preventDefault()
    closeBtn.value?.$el.focus()
  }
}

watch(
  () => lightbox.src,
  async (src) => {
    if (src) {
      reset()
      previouslyFocused = document.activeElement as HTMLElement | null
      document.addEventListener('keydown', onKey, true)
      await nextTick()
      closeBtn.value?.$el.focus()
    } else {
      document.removeEventListener('keydown', onKey, true)
      previouslyFocused?.focus?.()
    }
  },
)
onBeforeUnmount(() => document.removeEventListener('keydown', onKey, true))
</script>

<template>
  <Teleport to="body">
    <div
      v-if="lightbox.src"
      class="animate__animated animate__fadeIn animate__faster fixed inset-0 z-[3000] flex items-center justify-center bg-ctp-crust/95"
      role="dialog"
      aria-modal="true"
      :aria-label="lightbox.alt || 'Image viewer'"
      @click.self="lightbox.close()"
      @wheel="onWheel"
    >
      <div class="absolute top-3 right-3 flex gap-2">
        <v-btn
          icon="mdi-magnify-minus-outline"
          variant="tonal"
          aria-label="Zoom out"
          @click="zoom(-0.5)"
        />
        <v-btn
          icon="mdi-magnify-plus-outline"
          variant="tonal"
          aria-label="Zoom in"
          @click="zoom(0.5)"
        />
        <v-btn
          ref="closeBtn"
          icon="mdi-close"
          variant="tonal"
          aria-label="Close image"
          @click="lightbox.close()"
        />
      </div>
      <img
        :src="lightbox.src"
        :alt="lightbox.alt"
        class="max-h-[90vh] max-w-[95vw] touch-none object-contain select-none"
        :class="scale > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-zoom-in'"
        :style="{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${scale})` }"
        draggable="false"
        @click.stop="scale === 1 && zoom(1)"
        @dblclick.stop="reset"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
      />
      <p
        v-if="lightbox.alt"
        class="absolute bottom-4 max-w-[90vw] rounded-lg bg-ctp-mantle/90 px-3 py-1.5 text-sm text-ctp-subtext1"
      >
        {{ lightbox.alt }}
      </p>
    </div>
  </Teleport>
</template>
