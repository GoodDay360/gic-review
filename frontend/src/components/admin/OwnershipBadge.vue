<script setup lang="ts">
import type { ManagedFields } from '@/api/types'

defineProps<{ item: ManagedFields }>()
</script>

<template>
  <v-chip
    v-if="item.access === 'shared'"
    size="small"
    color="info"
    variant="tonal"
    prepend-icon="mdi-account-multiple-outline"
    title="Its category or subject is shared with you: you can edit and publish it, but not delete it"
  >
    Shared · by {{ item.owner_name ?? 'unknown' }}
  </v-chip>
  <v-chip
    v-else-if="item.access === 'admin' || item.access === 'category_owner'"
    size="small"
    color="ctp-peach"
    variant="tonal"
    prepend-icon="mdi-account-outline"
    :title="item.access === 'admin' ? 'You have admin access' : 'It is in a category you own'"
  >
    By {{ item.owner_name ?? 'a deleted account' }}
  </v-chip>
</template>
