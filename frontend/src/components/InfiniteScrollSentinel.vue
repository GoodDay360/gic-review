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
    <span v-if="loading" class="flex items-center gap-2 text-sm text-ctp-subtext0">
      <v-progress-circular size="18" width="2" /> Loading more…
    </span>
    <v-btn
      v-else-if="error"
      variant="tonal"
      color="error"
      size="small"
      prepend-icon="mdi-refresh"
      @click="emit('loadMore')"
    >
      Retry
    </v-btn>
    <v-btn
      v-else-if="hasMore"
      variant="text"
      size="small"
      append-icon="mdi-chevron-down"
      @click="emit('loadMore')"
    >
      Load more
    </v-btn>
  </div>
</template>
