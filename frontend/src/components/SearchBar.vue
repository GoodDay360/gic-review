<script setup lang="ts">
import AppIcon from './AppIcon.vue'

defineProps<{
  modelValue: string
  placeholder: string
  label: string
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <label class="relative block w-full">
    <span class="sr-only">{{ label }}</span>
    <AppIcon
      name="search"
      :size="18"
      class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ctp-overlay1"
    />
    <input
      :value="modelValue"
      type="search"
      class="input pl-10"
      :placeholder="placeholder"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
  </label>
</template>
