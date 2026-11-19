<script setup lang="ts">
import type { Category } from '@/api/types'

defineProps<{ category: Category; lastVisited?: boolean }>()
</script>

<template>
  <v-card
    :to="`/categories/${category.slug}`"
    hover
    class="card-lift group flex h-full flex-col gap-2 p-5"
    :class="lastVisited ? 'border-ctp-accent/60' : ''"
  >
    <span class="flex items-start justify-between gap-3">
      <span class="flex items-center gap-3">
        <v-avatar color="primary" variant="tonal" rounded="lg" size="44">
          <v-icon icon="mdi-layers-outline" size="24" />
        </v-avatar>
        <span class="text-2xl font-bold tracking-tight text-ctp-text group-hover:text-ctp-accent">
          {{ category.name }}
        </span>
      </span>
      <v-icon
        icon="mdi-chevron-right"
        class="mt-1 text-ctp-overlay1 transition-transform group-hover:translate-x-1 group-hover:text-ctp-accent"
      />
    </span>
    <span v-if="category.description" class="line-clamp-2 text-sm text-ctp-subtext0">
      {{ category.description }}
    </span>
    <span class="mt-auto flex flex-wrap items-center gap-2 pt-2">
      <v-chip size="small" variant="tonal" prepend-icon="mdi-book-multiple-outline">
        {{ category.subject_count }} subject{{ category.subject_count === 1 ? '' : 's' }}
      </v-chip>
      <v-chip v-if="lastVisited" size="small" color="primary" prepend-icon="mdi-history">
        Last opened
      </v-chip>
    </span>
  </v-card>
</template>
