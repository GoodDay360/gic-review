<script setup lang="ts">
import { onMounted, ref } from 'vue'

import { api } from '@/api/client'
import type { Subject } from '@/api/types'
import ActivityModal from '@/components/ActivityModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import SubjectCard from '@/components/SubjectCard.vue'
import { useAsync } from '@/composables/useAsync'

const { data: subjects, loading, error, run } = useAsync(() => api.get<Subject[]>('/subjects'), [])
const selected = ref<Subject | null>(null)

onMounted(run)
</script>

<template>
  <div class="mx-auto max-w-6xl px-4 py-8 sm:py-12">
    <section class="mb-8 sm:mb-10">
      <h1 class="text-3xl font-bold tracking-tight text-ctp-text sm:text-4xl">
        Study smarter for your exams
      </h1>
      <p class="mt-2 max-w-2xl text-ctp-subtext0">
        Pick a subject, then read its interactive review or practice with quizzes. No account
        needed.
      </p>
    </section>

    <div v-if="loading" class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-48" />
    </div>
    <EmptyState v-else-if="error" title="Couldn't load subjects" :message="error" icon="warning">
      <button class="btn-secondary" @click="run">Try again</button>
    </EmptyState>
    <EmptyState
      v-else-if="subjects.length === 0"
      title="No subjects yet"
      message="Subjects will appear here once they are added."
    />
    <div v-else class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <SubjectCard v-for="s in subjects" :key="s.id" :subject="s" @select="selected = $event" />
    </div>

    <ActivityModal :subject="selected" @close="selected = null" />
  </div>
</template>
