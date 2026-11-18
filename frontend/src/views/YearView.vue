<script setup lang="ts">
/** Step 2: the subjects of one year. Clicking a subject opens the activity modal. */
import { onMounted, ref, watch } from 'vue'

import { api } from '@/api/client'
import type { Subject, YearWithSubjects } from '@/api/types'
import ActivityModal from '@/components/ActivityModal.vue'
import AppIcon from '@/components/AppIcon.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import SubjectCard from '@/components/SubjectCard.vue'
import { useAsync } from '@/composables/useAsync'
import { rememberYear } from '@/composables/lastYear'

const props = defineProps<{ yearSlug: string }>()

const {
  data: year,
  loading,
  error,
  run,
} = useAsync(
  () => api.get<YearWithSubjects>(`/years/${encodeURIComponent(props.yearSlug)}`),
  null as YearWithSubjects | null,
)
const selected = ref<Subject | null>(null)

onMounted(run)
watch(() => props.yearSlug, run)
watch(year, (y) => {
  if (y) {
    rememberYear(y.slug)
    document.title = `${y.name} · ${import.meta.env.VITE_APP_NAME || 'GIC Review'}`
  }
})
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8">
    <RouterLink to="/" class="btn-ghost -ml-3 mb-4"
      ><AppIcon name="arrow-left" :size="16" /> All years</RouterLink
    >

    <div v-if="loading" class="space-y-4">
      <SkeletonBlock class="h-10 w-48" />
      <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SkeletonBlock v-for="i in 3" :key="i" class="h-48" />
      </div>
    </div>
    <EmptyState v-else-if="error" title="Couldn't load this year" :message="error" icon="warning">
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <template v-else-if="year">
      <h1 class="text-3xl font-bold tracking-tight text-ctp-text">{{ year.name }}</h1>
      <p v-if="year.description" class="mt-1 max-w-2xl text-ctp-subtext0">{{ year.description }}</p>
      <h2 class="mt-8 mb-3 text-sm font-semibold tracking-wide text-ctp-subtext1 uppercase">
        Select a subject
      </h2>
      <EmptyState
        v-if="year.subjects.length === 0"
        title="No subjects in this year yet"
        message="Subjects will appear here once they are added."
      />
      <div v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <SubjectCard
          v-for="s in year.subjects"
          :key="s.id"
          :subject="s"
          @select="selected = $event"
        />
      </div>
    </template>

    <ActivityModal :subject="selected" @close="selected = null" />
  </div>
</template>
