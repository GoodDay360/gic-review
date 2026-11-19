<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
</script>

<template>
  <div class="mx-auto max-w-xl px-4 py-16">
    <EmptyState
      title="Page not found"
      message="The page you're looking for doesn't exist."
      icon="mdi-map-marker-question-outline"
    >
      <v-btn to="/" color="primary" prepend-icon="mdi-home-outline">Go home</v-btn>
    </EmptyState>
  </div>
</template>
