<script setup lang="ts">
withDefaults(defineProps<{ title: string; message?: string; icon?: string }>(), {
  message: '',
  icon: 'mdi-folder-open-outline',
})
</script>

<template>
  <v-card class="flex flex-col items-center gap-3 px-6 py-12 text-center" data-aos="fade-up">
    <v-avatar size="64" color="primary" variant="tonal">
      <v-icon :icon="icon" size="32" />
    </v-avatar>
    <h3 class="font-semibold text-ctp-text">{{ title }}</h3>
    <p v-if="message" class="max-w-md text-sm text-ctp-subtext0">{{ message }}</p>
    <slot />
  </v-card>
</template>
