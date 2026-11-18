<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = defineProps<{
  hasMore: boolean
  loading: boolean
  error?: string | null
}>()
const emit = defineEmits<{ loadMore: [] }>()
const target = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | undefined

function loadIfNearViewport() {
  const rect = target.value?.getBoundingClientRect()
  if (
    rect &&
    rect.top <= window.innerHeight + 320 &&
    rect.bottom >= -320 &&
    props.hasMore &&
    !props.loading &&
    !props.error
  ) {
    emit('loadMore')
  }
}

onMounted(() => {
  if (!target.value || !('IntersectionObserver' in window)) return
  observer = new IntersectionObserver(
    ([entry]) => {
      if (entry?.isIntersecting) loadIfNearViewport()
    },
    { rootMargin: '320px' },
  )
  observer.observe(target.value)
})

watch(
  () => [props.hasMore, props.loading, props.error],
  ([hasMore, loading, error]) => {
    if (hasMore && !loading && !error) requestAnimationFrame(loadIfNearViewport)
  },
)

onBeforeUnmount(() => observer?.disconnect())
</script>

<template>
  <div ref="target" class="flex min-h-12 items-center justify-center py-3" aria-live="polite">
    <span v-if="loading" class="text-sm text-ctp-subtext0">Loading more…</span>
    <button v-else-if="error" type="button" class="btn-secondary btn-sm" @click="emit('loadMore')">
      Retry
    </button>
    <button v-else-if="hasMore" type="button" class="btn-ghost btn-sm" @click="emit('loadMore')">
      Load more
    </button>
  </div>
</template>
