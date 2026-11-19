<script setup lang="ts">
defineProps<{
  modelValue: string
  placeholder: string
  label: string
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
</script>

<template>
  <v-text-field
    :model-value="modelValue"
    type="search"
    :placeholder="placeholder"
    :aria-label="label"
    prepend-inner-icon="mdi-magnify"
    clearable
    hide-details
    class="w-full"
    @update:model-value="emit('update:modelValue', $event ?? '')"
  />
</template>
