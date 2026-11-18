<script setup lang="ts">
import { onMounted } from 'vue'

import AppHeader from '@/components/AppHeader.vue'
import ImageLightbox from '@/components/ImageLightbox.vue'
import ToastHost from '@/components/ToastHost.vue'
import { useAuthStore } from '@/stores/auth'

// Know whether a staff session exists (shows the Admin link in the header).
onMounted(() => useAuthStore().fetchMe())
</script>

<template>
  <div class="flex min-h-screen flex-col">
    <AppHeader />
    <main class="flex-1">
      <RouterView />
    </main>
    <footer
      class="border-t border-ctp-surface0 bg-ctp-mantle py-4 text-center text-xs text-ctp-subtext0"
    >
      GIC Review · study at your own pace ·
      <a
        href="https://github.com/GoodDay360"
        target="_blank"
        rel="noopener noreferrer"
        class="font-medium text-ctp-accent hover:underline"
      >
        Made by: @GoodDay360
      </a>
    </footer>
    <ImageLightbox />
    <ToastHost />
  </div>
</template>
