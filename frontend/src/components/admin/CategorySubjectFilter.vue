<script lang="ts">
import type { ManagedCategory } from '@/api/categories'

/** '' = "All". */
export type FilterId = number | ''

/** Keep the items whose subject matches the chosen category / subject. */
export function filterBySubject<T extends { subject_id: number }>(
  items: T[],
  categories: ManagedCategory[],
  categoryId: FilterId,
  subjectId: FilterId,
): T[] {
  const subjectCategory = new Map(
    categories.flatMap((c) => c.subjects).map((s) => [s.id, s.category_id]),
  )
  return items.filter(
    (it) =>
      (!subjectId || it.subject_id === subjectId) &&
      (!categoryId || subjectCategory.get(it.subject_id) === categoryId),
  )
}
</script>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ categories: ManagedCategory[] }>()
const category = defineModel<FilterId>('category', { default: '' })
const subject = defineModel<FilterId>('subject', { default: '' })

const subjects = computed(() => props.categories.flatMap((c) => c.subjects))

const categoryItems = computed(() => [
  { title: 'All categories', value: '' as FilterId },
  ...props.categories.map((c) => ({ title: c.name, value: c.id as FilterId })),
])
/** Subjects narrowed to the chosen category. */
const subjectItems = computed(() => [
  { title: 'All subjects', value: '' as FilterId, subtitle: undefined as string | undefined },
  ...subjects.value
    .filter((s) => !category.value || s.category_id === category.value)
    .map((s) => ({
      title: s.name,
      value: s.id as FilterId,
      subtitle: category.value ? undefined : s.category_name,
    })),
])

function onCategoryChange(v: FilterId | null) {
  category.value = v ?? ''
  const current = subjects.value.find((s) => s.id === subject.value)
  if (current && category.value && current.category_id !== category.value) subject.value = ''
}
function onSubjectChange(v: FilterId | null) {
  subject.value = v ?? ''
}
</script>

<template>
  <div class="flex flex-wrap gap-3">
    <v-autocomplete
      :model-value="category"
      :items="categoryItems"
      label="Category"
      prepend-inner-icon="mdi-shape-outline"
      hide-details
      auto-select-first
      class="w-full sm:max-w-xs"
      @update:model-value="onCategoryChange"
    />
    <v-autocomplete
      :model-value="subject"
      :items="subjectItems"
      label="Subject"
      prepend-inner-icon="mdi-filter-variant"
      hide-details
      auto-select-first
      item-props
      class="w-full sm:max-w-xs"
      @update:model-value="onSubjectChange"
    />
  </div>
</template>
