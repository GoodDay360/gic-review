<script setup lang="ts">
/** Admin: see every manager's API keys and revoke them. Admins cannot create keys. */
import { computed, onMounted, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { ApiKey, User } from '@/api/types'
import BaseModal from '@/components/BaseModal.vue'
import EmptyState from '@/components/EmptyState.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const userFilter = ref<number | ''>('')
const statusFilter = ref<'active' | 'all'>('active')
const managers = ref<User[]>([])

const {
  data: keys,
  loading,
  run,
} = useAsync(
  () =>
    api.get<ApiKey[]>(`/admin/api-keys${userFilter.value ? `?user_id=${userFilter.value}` : ''}`),
  [],
)
onMounted(async () => {
  managers.value = (await api.get<User[]>('/admin/users').catch(() => [])).filter(() => true)
  run()
})

const shown = computed(() =>
  statusFilter.value === 'active' ? keys.value.filter((k) => k.status === 'active') : keys.value,
)

const revoking = ref<ApiKey | null>(null)
const revokingKey = ref(false)
async function confirmRevoke() {
  if (!revoking.value || revokingKey.value) return
  revokingKey.value = true
  try {
    await api.delete(`/admin/api-keys/${revoking.value.id}`)
    toast.success(`Revoked ${revoking.value.owner_name}'s key`)
    revoking.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    revokingKey.value = false
  }
}

const fmt = (d: string | null) => (d ? new Date(d.replace(' ', 'T')).toLocaleString() : 'Never')
const statusChip = {
  active: { color: 'success', icon: 'mdi-check-circle-outline' },
  revoked: { color: 'error', icon: 'mdi-cancel' },
  expired: { color: undefined, icon: 'mdi-clock-alert-outline' },
}
const managerItems = computed(() => [
  { title: 'All managers', value: '' as number | '' },
  ...managers.value.map((m) => ({ title: `${m.display_name} (${m.email})`, value: m.id })),
])
const statusItems = [
  { title: 'Active only', value: 'active' },
  { title: 'All keys', value: 'all' },
]
</script>

<template>
  <div class="space-y-5">
    <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">API keys</h1>
    <v-alert type="info" icon="mdi-information-outline" class="max-w-3xl text-sm">
      Each manager (and admin) can create one API key from their dashboard to let scripts or AI
      assistants add quizzes and interactive reviews. Revoke a key if it leaks or is no longer
      needed. Deactivating or deleting a manager also disables their key.
    </v-alert>

    <div class="flex flex-wrap items-center gap-3">
      <v-select
        v-model="userFilter"
        :items="managerItems"
        label="Manager"
        prepend-inner-icon="mdi-account-search-outline"
        hide-details
        class="max-w-sm min-w-60"
        @update:model-value="run"
      />
      <v-select
        v-model="statusFilter"
        :items="statusItems"
        label="Status"
        hide-details
        class="max-w-[12rem] min-w-40"
      />
    </div>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-16" />
    </div>
    <EmptyState
      v-else-if="!shown.length"
      title="No API keys"
      message="Managers create keys from their own dashboard."
      icon="mdi-key-variant"
    />
    <ul v-else class="space-y-2">
      <v-card
        v-for="(k, i) in shown"
        :key="k.id"
        tag="li"
        class="card-lift flex flex-wrap items-center gap-3 p-3"
        data-aos="fade-up"
        :data-aos-delay="Math.min(i, 6) * 40"
      >
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">
            {{ k.owner_name }}
            <span class="text-xs font-normal text-ctp-subtext0">{{ k.owner_email }}</span>
          </p>
          <p class="text-xs text-ctp-subtext0">
            <span class="font-mono">{{ k.key_prefix }}…</span> · {{ k.name }} ·
            {{ k.scopes.join(', ') }}
          </p>
          <p class="text-xs text-ctp-overlay1">
            Created {{ fmt(k.created_at) }} · last used {{ fmt(k.last_used_at) }} · expires
            {{ fmt(k.expires_at) }}
          </p>
        </div>
        <v-chip
          size="small"
          variant="tonal"
          class="capitalize"
          :color="statusChip[k.status].color"
          :prepend-icon="statusChip[k.status].icon"
          >{{ k.status }}</v-chip
        >
        <v-btn
          v-if="k.status === 'active'"
          variant="tonal"
          size="small"
          prepend-icon="mdi-key-remove"
          color="error"
          @click="revoking = k"
          >Revoke
        </v-btn>
      </v-card>
    </ul>

    <BaseModal :open="!!revoking" title="Revoke API key?" size="sm" @close="revoking = null">
      <v-alert type="warning" icon="mdi-key-alert-outline">
        {{ revoking?.owner_name }}'s key
        <span class="font-mono">{{ revoking?.key_prefix }}…</span> will stop working immediately.
        They can create a new one from their dashboard.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="revoking = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-key-remove"
          :loading="revokingKey"
          @click="confirmRevoke"
          >Revoke</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
