<script setup lang="ts">
/**
 * Interactive review viewer. The HTML comes from the R2 public domain and runs in a
 * sandboxed iframe WITHOUT allow-same-origin, so its scripts get an opaque origin and
 * cannot read this app's cookies, storage or DOM. The current theme is sent with
 * postMessage: { type: 'gic-theme', flavor, accent } and per-review scroll progress.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { api, errorMessage } from '@/api/client'
import type { Page, Review, Subject } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useInfiniteSearch } from '@/composables/useInfiniteSearch'
import { useAsync } from '@/composables/useAsync'
import { useThemeStore } from '@/stores/theme'
import {
  getReviewScrollPosition,
  isReviewScrollMessage,
  saveReviewScrollPosition,
} from '@/utils/reviewProgress'

const props = defineProps<{ subjectSlug: string }>()
const route = useRoute()
const router = useRouter()
const theme = useThemeStore()
const frame = ref<HTMLIFrameElement | null>(null)
const directReview = ref<Review | null>(null)
const directReviewError = ref<string | null>(null)

const {
  data: subject,
  loading,
  error,
  run,
} = useAsync(
  () => api.get<Subject>(`/subjects/${encodeURIComponent(props.subjectSlug)}`),
  null as Subject | null,
)
const {
  search,
  items: reviews,
  loading: reviewsLoading,
  loadingMore,
  hasMore,
  error: reviewsError,
  loadMore,
  retry,
  reset,
} = useInfiniteSearch<Review>((search, offset, limit) => {
  const query = new URLSearchParams({ search, offset: String(offset), limit: String(limit) })
  return api.get<Page<Review>>(
    `/subjects/${encodeURIComponent(props.subjectSlug)}/reviews?${query}`,
  )
})
watch(
  () => props.subjectSlug,
  () => {
    if (search.value) search.value = ''
    else reset()
    run()
  },
)

const selected = computed<Review | null>(() => {
  const r = route.query.r
  return reviews.value.find((x) => x.slug === r) ?? directReview.value
})
watch(
  () => [props.subjectSlug, route.query.r] as const,
  async ([subjectSlug, slug]) => {
    directReview.value = null
    directReviewError.value = null
    if (typeof slug !== 'string' || reviews.value.some((review) => review.slug === slug)) return
    const query = new URLSearchParams({ slug, offset: '0', limit: '1' })
    try {
      const page = await api.get<Page<Review>>(
        `/subjects/${encodeURIComponent(subjectSlug)}/reviews?${query}`,
      )
      if (route.query.r === slug) directReview.value = page.items[0] ?? null
    } catch (err) {
      if (route.query.r === slug) directReviewError.value = errorMessage(err)
    }
  },
  { immediate: true },
)

function handleReviewMessage(event: MessageEvent<unknown>) {
  const review = selected.value
  if (
    !review ||
    event.source !== frame.value?.contentWindow ||
    !isReviewScrollMessage(event.data)
  ) {
    return
  }
  saveReviewScrollPosition(review.id, event.data.scrollY)
}

onMounted(() => {
  window.addEventListener('message', handleReviewMessage)
  run()
})
onBeforeUnmount(() => window.removeEventListener('message', handleReviewMessage))

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
  const review = selected.value
  const target = frame.value?.contentWindow
  if (!review || !target) return
  target.postMessage({ type: 'gic-theme', flavor: theme.flavor, accent: theme.accent }, '*')
  target.postMessage(
    { type: 'gic-review-restore', scrollY: getReviewScrollPosition(review.id) },
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
      <RouterLink v-if="selected" :to="{ query: {} }" class="btn-ghost -ml-3">
        <AppIcon name="arrow-left" :size="16" /> All reviews
      </RouterLink>
      <RouterLink v-else :to="subject ? `/years/${subject.year_slug}` : '/'" class="btn-ghost -ml-3"
        ><AppIcon name="arrow-left" :size="16" />
        {{ subject ? `${subject.year_name} subjects` : 'Home' }}</RouterLink
      >
      <h1 class="text-xl font-bold text-ctp-text">
        {{ subject?.name
        }}<span v-if="selected" class="font-normal text-ctp-subtext0"> · {{ selected.title }}</span>
      </h1>
    </div>

    <SkeletonBlock v-if="loading && !subject" class="h-[75vh]" />
    <EmptyState
      v-else-if="error && !subject"
      title="Couldn't load the review"
      :message="error"
      icon="warning"
    >
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <EmptyState
      v-else-if="directReviewError"
      title="Couldn't load this review"
      :message="directReviewError"
      icon="warning"
    />
    <template v-else-if="!selected">
      <SearchBar
        v-model="search"
        class="mb-5"
        label="Search interactive reviews"
        placeholder="Search interactive reviews…"
      />
      <div v-if="reviewsLoading && reviews.length === 0" class="grid gap-3 sm:grid-cols-2">
        <SkeletonBlock v-for="i in 4" :key="i" class="h-20" />
      </div>
      <EmptyState
        v-else-if="reviewsError && reviews.length === 0"
        title="Couldn't load interactive reviews"
        :message="reviewsError"
        icon="warning"
      >
        <button class="btn-secondary" @click="retry">Try again</button>
      </EmptyState>
      <EmptyState
        v-else-if="reviews.length === 0"
        :title="search ? 'No matching interactive reviews' : 'Coming soon'"
        :message="
          search ? 'Try another search.' : 'There is no interactive review for this subject yet.'
        "
        icon="book"
      />
      <template v-else>
        <ul class="grid gap-3 sm:grid-cols-2">
          <li v-for="r in reviews" :key="r.id">
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
        <InfiniteScrollSentinel
          :has-more="hasMore"
          :loading="loadingMore"
          :error="reviewsError"
          @load-more="reviewsError ? retry() : loadMore()"
        />
      </template>
    </template>
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
      v-else-if="selected"
      title="This review cannot be displayed safely"
      message="Its URL is not on the file storage domain."
      icon="warning"
    />
  </div>
</template>
