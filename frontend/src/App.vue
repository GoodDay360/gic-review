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
  <v-app>
    <AppHeader />
    <v-main class="flex flex-col">
      <div class="flex-1">
        <RouterView v-slot="{ Component }">
          <Transition
            mode="out-in"
            enter-active-class="animate__animated animate__fadeIn animate__faster"
          >
            <component :is="Component" />
          </Transition>
        </RouterView>
      </div>
      <v-footer
        class="flex-grow-0 justify-center border-t border-ctp-surface0 bg-ctp-mantle py-4 text-center text-xs text-ctp-subtext0"
      >
        <span>
          GIC Review · study at your own pace ·
          <a
            href="https://github.com/GoodDay360"
            target="_blank"
            rel="noopener noreferrer"
            class="font-medium text-ctp-accent hover:underline"
          >
            <v-icon icon="mdi-github" size="14" class="mb-0.5" /> Made by: @GoodDay360
          </a>
        </span>
      </v-footer>
    </v-main>
    <ImageLightbox />
    <ToastHost />
  </v-app>
</template>
