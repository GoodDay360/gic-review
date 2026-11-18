<script setup lang="ts">
import type { ManagedFields } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'

defineProps<{ item: ManagedFields }>()
</script>

<template>
  <span
    v-if="item.access === 'shared'"
    class="inline-flex items-center gap-1 rounded-full bg-ctp-blue/15 px-2.5 py-1 text-xs font-medium text-ctp-blue"
    title="Its year or subject is shared with you: you can edit and publish it, but not delete it"
  >
    <AppIcon name="users" :size="12" /> Shared · by {{ item.owner_name ?? 'unknown' }}
  </span>
  <span
    v-else-if="item.access === 'admin' || item.access === 'year_owner'"
    class="inline-flex items-center gap-1 rounded-full bg-ctp-peach/15 px-2.5 py-1 text-xs font-medium text-ctp-peach"
    :title="item.access === 'admin' ? 'You have admin access' : 'It is in a year you own'"
  >
    <AppIcon name="users" :size="12" /> By {{ item.owner_name ?? 'a deleted account' }}
  </span>
</template>
