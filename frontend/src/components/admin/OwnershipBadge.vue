<script setup lang="ts">
import type { ManagedFields } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'

defineProps<{ item: ManagedFields }>()
</script>

<template>
  <span
    v-if="item.access === 'shared'"
    class="inline-flex items-center gap-1 rounded-full bg-ctp-blue/15 px-2.5 py-1 text-xs font-medium text-ctp-blue"
    :title="`Shared with you by ${item.owner_name ?? 'another manager'}`"
  >
    <AppIcon name="users" :size="12" /> Shared by {{ item.owner_name ?? 'another manager' }}
  </span>
  <span
    v-else-if="item.access === 'admin'"
    class="inline-flex items-center gap-1 rounded-full bg-ctp-peach/15 px-2.5 py-1 text-xs font-medium text-ctp-peach"
    :title="`Owned by ${item.owner_name ?? 'nobody'}; you have admin access`"
  >
    <AppIcon name="users" :size="12" /> Owned by {{ item.owner_name ?? 'nobody' }}
  </span>
  <span
    v-else-if="item.share_count"
    class="inline-flex items-center gap-1 rounded-full bg-ctp-surface0 px-2.5 py-1 text-xs text-ctp-subtext1"
    :title="`You shared this with ${item.share_count} manager(s)`"
  >
    <AppIcon name="users" :size="12" /> Shared with {{ item.share_count }}
  </span>
</template>
