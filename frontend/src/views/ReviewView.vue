<script setup lang="ts">
/**
 * Interactive review viewer. The HTML comes from the R2 public domain and runs in a
 * sandboxed iframe WITHOUT allow-same-origin, so its scripts get an opaque origin and
 * cannot read this app's cookies, storage or DOM. The current theme is sent with
 * postMessage: { type: 'gic-theme', flavor, accent }.
 */
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { api } from '@/api/client'
import type { Review, Subject } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useThemeStore } from '@/stores/theme'

const props = defineProps<{ subjectSlug: string }>()
const route = useRoute()
const router = useRouter()
const theme = useThemeStore()
const frame = ref<HTMLIFrameElement | null>(null)

const { data, loading, error, run } = useAsync(
  async () => {
    const slug = encodeURIComponent(props.subjectSlug)
    const [subject, reviews] = await Promise.all([
      api.get<Subject>(`/subjects/${slug}`),
      api.get<Review[]>(`/subjects/${slug}/reviews`),
    ])
    return { subject, reviews }
  },
  { subject: null as Subject | null, reviews: [] as Review[] },
)
onMounted(run)

const selected = computed<Review | null>(() => {
  const list = data.value.reviews
  if (list.length === 1) return list[0]!
  const r = route.query.r
  return list.find((x) => x.slug === r) ?? null
})

/** Never render a review from our own origin (it would not be isolated). */
const safeUrl = computed(() => {
  const url = selected.value?.url
  if (!url) return null
  try {
    return new URL(url).origin !== window.location.origin ? url : null
  } catch {
    return null
  }
})

function sendTheme() {
  // The sandboxed frame has an opaque ("null") origin, so '*' is the only usable target.
  // The message contains nothing sensitive.
  frame.value?.contentWindow?.postMessage(
    { type: 'gic-theme', flavor: theme.flavor, accent: theme.accent },
    '*',
  )
}
watch(() => [theme.flavor, theme.accent], sendTheme)

function open(r: Review) {
  router.push({ query: { r: r.slug } })
}
</script>

<template>
  <div class="mx-auto flex max-w-6xl flex-col px-4 py-6">
    <div class="mb-4 flex flex-wrap items-center gap-2">
      <RouterLink
        v-if="selected && data.reviews.length > 1"
        :to="{ query: {} }"
        class="btn-ghost -ml-3"
      >
        <AppIcon name="arrow-left" :size="16" /> All reviews
      </RouterLink>
      <RouterLink
        v-else
        :to="data.subject ? `/years/${data.subject.year_slug}` : '/'"
        class="btn-ghost -ml-3"
        ><AppIcon name="arrow-left" :size="16" />
        {{ data.subject ? `${data.subject.year_name} subjects` : 'Home' }}</RouterLink
      >
      <h1 class="text-xl font-bold text-ctp-text">
        {{ data.subject?.name
        }}<span v-if="selected" class="font-normal text-ctp-subtext0"> · {{ selected.title }}</span>
      </h1>
    </div>

    <SkeletonBlock v-if="loading" class="h-[75vh]" />
    <EmptyState v-else-if="error" title="Couldn't load the review" :message="error" icon="warning">
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <EmptyState
      v-else-if="data.reviews.length === 0"
      title="Coming soon"
      message="There is no interactive review for this subject yet."
      icon="book"
    />
    <ul v-else-if="!selected" class="grid gap-3 sm:grid-cols-2">
      <li v-for="r in data.reviews" :key="r.id">
        <button
          class="card flex w-full items-center gap-3 p-4 text-left hover:border-ctp-accent"
          @click="open(r)"
        >
          <span class="rounded-lg bg-ctp-surface0 p-2 text-ctp-accent"
            ><AppIcon name="book"
          /></span>
          <span class="font-medium text-ctp-text">{{ r.title }}</span>
          <AppIcon name="chevron-right" class="ml-auto text-ctp-overlay1" />
        </button>
      </li>
    </ul>
    <iframe
      v-else-if="safeUrl"
      ref="frame"
      :key="safeUrl"
      :src="safeUrl"
      :title="selected.title"
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
      loading="lazy"
      class="h-[calc(100vh-10rem)] min-h-[60vh] w-full rounded-xl border border-ctp-surface0 bg-ctp-base"
      @load="sendTheme"
    />
    <EmptyState
      v-else
      title="This review cannot be displayed safely"
      message="Its URL is not on the file storage domain."
      icon="warning"
    />
  </div>
</template>
