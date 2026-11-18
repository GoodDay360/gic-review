<script setup lang="ts">
import AppIcon from './AppIcon.vue'

withDefaults(defineProps<{ title: string; message?: string; icon?: string }>(), {
  message: '',
  icon: 'folder',
})
</script>

<template>
  <div class="card flex flex-col items-center gap-3 px-6 py-12 text-center">
    <span class="rounded-full bg-ctp-surface0 p-3 text-ctp-overlay2">
      <AppIcon :name="icon" :size="28" />
    </span>
    <h3 class="font-semibold text-ctp-text">{{ title }}</h3>
    <p v-if="message" class="max-w-md text-sm text-ctp-subtext0">{{ message }}</p>
    <slot />
  </div>
</template>
