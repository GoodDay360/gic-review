import { defineStore } from 'pinia'
import { ref } from 'vue'

/** One app-wide full-screen image viewer. */
export const useLightboxStore = defineStore('lightbox', () => {
  const src = ref<string | null>(null)
  const alt = ref('')

  function open(url: string, altText = '') {
    src.value = url
    alt.value = altText
  }

  function close() {
    src.value = null
  }

  return { src, alt, open, close }
})
