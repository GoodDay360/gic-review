<script lang="ts">
/** Open modals, innermost last: only the top one reacts to Esc / Tab. */
const modalStack: symbol[] = []
</script>

<script setup lang="ts">
/**
 * Accessible modal: focus trap, Esc / backdrop click to close, aria labelling,
 * and focus is restored to the element that opened it.
 */
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

import { useLightboxStore } from '@/stores/lightbox'

import AppIcon from './AppIcon.vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    description?: string
    size?: 'sm' | 'md' | 'lg' | 'xl'
  }>(),
  { size: 'md', description: undefined },
)
const emit = defineEmits<{ close: [] }>()

const panel = ref<HTMLElement | null>(null)
const titleId = useId()
const descId = useId()
let previouslyFocused: HTMLElement | null = null
const token = Symbol('modal')
const lightbox = useLightboxStore()

function popStack() {
  const i = modalStack.indexOf(token)
  if (i >= 0) modalStack.splice(i, 1)
}

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

function focusables(): HTMLElement[] {
  return panel.value ? Array.from(panel.value.querySelectorAll<HTMLElement>(FOCUSABLE)) : []
}

function onKeydown(e: KeyboardEvent) {
  // The image lightbox sits above modals and handles its own keys.
  if (!props.open || lightbox.src || modalStack[modalStack.length - 1] !== token) return
  if (e.key === 'Escape') {
    e.stopPropagation()
    emit('close')
    return
  }
  if (e.key !== 'Tab') return
  const items = focusables()
  if (items.length === 0) {
    e.preventDefault()
    return
  }
  const first = items[0]!
  const last = items[items.length - 1]!
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault()
    first.focus()
  }
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previouslyFocused = document.activeElement as HTMLElement | null
      modalStack.push(token)
      document.addEventListener('keydown', onKeydown, true)
      document.body.style.overflow = 'hidden'
      await nextTick()
      ;(focusables()[0] ?? panel.value)?.focus()
    } else {
      document.removeEventListener('keydown', onKeydown, true)
      popStack()
      if (modalStack.length === 0) document.body.style.overflow = ''
      previouslyFocused?.focus?.()
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown, true)
  popStack()
  if (modalStack.length === 0) document.body.style.overflow = ''
})

const sizes = { sm: 'max-w-sm', md: 'max-w-lg', lg: 'max-w-2xl', xl: 'max-w-4xl' }
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-150 ease-out"
      enter-from-class="opacity-0"
      leave-active-class="transition duration-100 ease-in"
      leave-to-class="opacity-0"
    >
      <div
        v-if="open"
        class="fixed inset-0 z-40 flex items-end justify-center bg-ctp-crust/70 p-0 backdrop-blur-sm sm:items-center sm:p-4"
        @mousedown.self="emit('close')"
      >
        <div
          ref="panel"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="titleId"
          :aria-describedby="description ? descId : undefined"
          tabindex="-1"
          class="flex max-h-[92vh] w-full flex-col rounded-t-2xl border border-ctp-surface0 bg-ctp-base shadow-2xl sm:rounded-2xl"
          :class="sizes[size]"
        >
          <header class="flex items-start justify-between gap-4 border-b border-ctp-surface0 p-5">
            <div>
              <h2 :id="titleId" class="text-lg font-semibold text-ctp-text">{{ title }}</h2>
              <p v-if="description" :id="descId" class="mt-1 text-sm text-ctp-subtext0">
                {{ description }}
              </p>
            </div>
            <button class="btn-ghost -m-2 p-2" aria-label="Close dialog" @click="emit('close')">
              <AppIcon name="close" />
            </button>
          </header>
          <div class="overflow-y-auto p-5">
            <slot />
          </div>
          <footer
            v-if="$slots.footer"
            class="flex justify-end gap-2 border-t border-ctp-surface0 p-4"
          >
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
