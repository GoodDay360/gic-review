<script setup lang="ts">
defineProps<{
  title: string
  percentage?: number
  stats: { label: string; value: string | number; tone?: 'green' | 'red' | 'accent' }[]
}>()

const toneClass = { green: 'text-ctp-green', red: 'text-ctp-red', accent: 'text-ctp-accent' }
</script>

<template>
  <v-card class="space-y-6 p-6 text-center sm:p-8" aria-live="polite">
    <div class="flex flex-col items-center gap-2">
      <v-avatar size="64" color="primary" variant="tonal" class="animate__animated animate__tada">
        <v-icon icon="mdi-trophy-outline" size="32" />
      </v-avatar>
      <h2 class="text-2xl font-bold text-ctp-text">{{ title }}</h2>
      <div v-if="percentage !== undefined" class="animate__animated animate__zoomIn my-2">
        <v-progress-circular
          :model-value="percentage"
          :indeterminate="false"
          :size="132"
          :width="12"
          :color="percentage >= 50 ? 'success' : 'error'"
          bg-color="ctp-surface1"
        >
          <span class="text-4xl font-extrabold text-ctp-text">{{ percentage }}%</span>
        </v-progress-circular>
      </div>
    </div>
    <dl class="grid grid-cols-2 gap-3 sm:grid-cols-3">
      <div
        v-for="(s, i) in stats"
        :key="s.label"
        class="animate__animated animate__fadeInUp rounded-xl bg-ctp-surface0 p-3"
        :style="{ animationDelay: `${150 + i * 80}ms` }"
      >
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
  </v-card>
</template>
