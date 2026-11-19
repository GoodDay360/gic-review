<script setup lang="ts">
/** Step 1: pick a category (e.g. I3, I4, or a custom name). Step 2 is CategoryView (its subjects). */
import { api } from '@/api/client'
import type { Page, Category } from '@/api/types'
import EmptyState from '@/components/EmptyState.vue'
import InfiniteScrollSentinel from '@/components/InfiniteScrollSentinel.vue'
import SearchBar from '@/components/SearchBar.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import CategoryCard from '@/components/CategoryCard.vue'
import { useInfiniteSearch } from '@/composables/useInfiniteSearch'
import { lastCategorySlug } from '@/composables/lastCategory'

const {
  search,
  items: categories,
  loading,
  loadingMore,
  hasMore,
  error,
  loadMore,
  retry,
} = useInfiniteSearch<Category>((search, offset, limit) => {
  const query = new URLSearchParams({ search, offset: String(offset), limit: String(limit) })
  return api.get<Page<Category>>(`/categories?${query}`)
})
const last = lastCategorySlug()
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8 sm:py-12">
    <section
      class="hero relative mb-8 overflow-hidden rounded-3xl border border-ctp-surface0 p-6 sm:mb-10 sm:p-10"
    >
      <div class="relative">
        <v-chip
          color="primary"
          variant="tonal"
          prepend-icon="mdi-school-outline"
          class="animate__animated animate__fadeInDown mb-4"
        >
          Free · no account needed
        </v-chip>
        <h1
          class="animate__animated animate__fadeInUp text-3xl font-extrabold tracking-tight text-ctp-text sm:text-5xl"
        >
          Study smarter for your <span class="text-ctp-accent">exams</span>
        </h1>
        <p
          class="animate__animated animate__fadeInUp mt-3 max-w-2xl text-ctp-subtext0 [animation-delay:120ms]"
        >
          Choose your category, then a subject: read its interactive review or practice with
          quizzes. No account needed.
        </p>
        <div
          class="animate__animated animate__fadeInUp mt-6 flex flex-wrap gap-2 [animation-delay:220ms]"
        >
          <v-chip variant="outlined" prepend-icon="mdi-book-open-page-variant-outline"
            >Interactive reviews</v-chip
          >
          <v-chip variant="outlined" prepend-icon="mdi-format-list-checks">QCM quizzes</v-chip>
          <v-chip variant="outlined" prepend-icon="mdi-cards-outline">Flashcards</v-chip>
        </div>
      </div>
    </section>

    <h2 class="mb-3 text-sm font-semibold tracking-wide text-ctp-subtext1 uppercase">
      Select your category
    </h2>
    <SearchBar
      v-model="search"
      class="mb-5"
      label="Search categories"
      placeholder="Search categories…"
    />
    <div v-if="loading && categories.length === 0" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <SkeletonBlock v-for="i in 4" :key="i" class="h-32" />
    </div>
    <EmptyState
      v-else-if="error && categories.length === 0"
      title="Couldn't load categories"
      :message="error"
      icon="mdi-alert-outline"
    >
      <v-btn variant="tonal" color="primary" prepend-icon="mdi-refresh" @click="retry"
        >Try again</v-btn
      >
    </EmptyState>
    <EmptyState
      v-else-if="categories.length === 0"
      :title="search ? 'No matching categories' : 'No categories yet'"
      :message="search ? 'Try another search.' : 'Categories will appear here once they are added.'"
    />
    <template v-else>
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <CategoryCard
          v-for="(y, i) in categories"
          :key="y.id"
          :category="y"
          :last-visited="y.slug === last"
          data-aos="fade-up"
          :data-aos-delay="(i % 4) * 70"
        />
      </div>
      <InfiniteScrollSentinel
        :has-more="hasMore"
        :loading="loadingMore"
        :error="error"
        @load-more="error ? retry() : loadMore()"
      />
    </template>
  </div>
</template>
