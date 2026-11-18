<script setup lang="ts">
import AppIcon from './AppIcon.vue'

defineProps<{
  title: string
  percentage?: number
  stats: { label: string; value: string | number; tone?: 'green' | 'red' | 'accent' }[]
}>()

const toneClass = { green: 'text-ctp-green', red: 'text-ctp-red', accent: 'text-ctp-accent' }
</script>

<template>
  <section class="card space-y-6 p-6 text-center sm:p-8" aria-live="polite">
    <div class="flex flex-col items-center gap-2">
      <span class="rounded-full bg-ctp-accent/15 p-3 text-ctp-accent"
        ><AppIcon name="sparkles" :size="28"
      /></span>
      <h2 class="text-2xl font-bold text-ctp-text">{{ title }}</h2>
      <p v-if="percentage !== undefined" class="text-5xl font-extrabold text-ctp-accent">
        {{ percentage }}%
      </p>
    </div>
    <dl class="grid grid-cols-2 gap-3 sm:grid-cols-3">
      <div v-for="s in stats" :key="s.label" class="rounded-xl bg-ctp-surface0 p-3">
        <dt class="text-xs text-ctp-subtext0">{{ s.label }}</dt>
        <dd class="text-xl font-semibold" :class="s.tone ? toneClass[s.tone] : 'text-ctp-text'">
          {{ s.value }}
        </dd>
      </div>
    </dl>
    <slot />
    <div class="flex flex-wrap justify-center gap-2">
      <slot name="actions" />
    </div>
  </section>
</template>
