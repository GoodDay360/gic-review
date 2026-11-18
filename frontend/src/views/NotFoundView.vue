<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
</script>

<template>
  <div class="mx-auto max-w-xl px-4 py-16">
    <EmptyState
      title="Page not found"
      message="The page you're looking for doesn't exist."
      icon="warning"
    >
      <RouterLink to="/" class="btn-primary">Go home</RouterLink>
    </EmptyState>
  </div>
</template>
