<script setup lang="ts">
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
</script>

<template>
  <v-snackbar-queue
    v-model="toast.queue"
    location="bottom end"
    variant="flat"
    rounded="lg"
    :total-visible="4"
    closable
    close-text="Dismiss"
    transition="slide-y-reverse-transition"
  />
</template>
