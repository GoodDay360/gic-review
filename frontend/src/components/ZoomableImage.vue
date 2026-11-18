<script setup lang="ts">
/** An image that opens the full-screen lightbox when clicked (click does not bubble). */
import { useLightboxStore } from '@/stores/lightbox'

const props = withDefaults(defineProps<{ src: string; alt?: string | null; imgClass?: string }>(), {
  alt: '',
  imgClass: 'max-h-80 w-full object-contain',
})
const lightbox = useLightboxStore()
</script>

<template>
  <button
    type="button"
    class="group relative block w-full cursor-zoom-in overflow-hidden rounded-lg"
    :aria-label="`Enlarge image${props.alt ? ': ' + props.alt : ''}`"
    @click.stop="lightbox.open(props.src, props.alt ?? '')"
    @keydown.space.stop
  >
    <img :src="src" :alt="alt ?? ''" loading="lazy" decoding="async" :class="imgClass" />
  </button>
</template>
