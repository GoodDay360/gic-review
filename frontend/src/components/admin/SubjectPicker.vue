<script setup lang="ts">
import { computed, ref, watch } from 'vue'

import type { Subject } from '@/api/types'

/** Pick a subject in two steps (category, then subject), both searchable. */
const props = defineProps<{ subjects: Subject[] }>()
const subjectId = defineModel<number>({ required: true })

const categoryItems = computed(() => {
  const seen = new Map<number, string>()
  for (const s of props.subjects)
    if (!seen.has(s.category_id)) seen.set(s.category_id, s.category_name)
  return [...seen].map(([value, title]) => ({ title, value }))
})

const categoryOf = (id: number) => props.subjects.find((s) => s.id === id)?.category_id ?? null
const categoryId = ref<number | null>(null)

// Follow the subject when it's set from outside (form reset, edit dialog opening).
// With no valid subject, default to the first category and its first subject.
watch(
  [subjectId, () => props.subjects],
  ([id, subjects]) => {
    const c = categoryOf(id)
    if (c !== null) categoryId.value = c
    else if (subjects[0]) subjectId.value = subjects[0].id
    else categoryId.value = null
  },
  { immediate: true },
)

const subjectItems = computed(() =>
  props.subjects
    .filter((s) => s.category_id === categoryId.value)
    .map((s) => ({ title: s.name, value: s.id })),
)

function onCategoryChange(v: number | null) {
  categoryId.value = v
  if (categoryOf(subjectId.value) !== v) subjectId.value = subjectItems.value[0]?.value ?? 0
}
</script>

<template>
  <div class="grid gap-3 sm:grid-cols-2">
    <v-autocomplete
      :model-value="categoryId"
      :items="categoryItems"
      label="Category"
      prepend-inner-icon="mdi-shape-outline"
      auto-select-first
      no-data-text="No matching category"
      @update:model-value="onCategoryChange"
    />
    <v-autocomplete
      :model-value="subjectId || null"
      :items="subjectItems"
      label="Subject"
      prepend-inner-icon="mdi-book-outline"
      auto-select-first
      :disabled="categoryId === null"
      no-data-text="No matching subject"
      @update:model-value="(v: number | null) => (subjectId = v ?? 0)"
    />
  </div>
</template>
