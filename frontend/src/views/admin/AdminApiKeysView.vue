<script setup lang="ts">
/** Admin: see every manager's API keys and revoke them. Admins cannot create keys. */
import { computed, onMounted, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { ApiKey, User } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
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
async function confirmRevoke() {
  if (!revoking.value) return
  try {
    await api.delete(`/admin/api-keys/${revoking.value.id}`)
    toast.success(`Revoked ${revoking.value.owner_name}'s key`)
    revoking.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

const fmt = (d: string | null) => (d ? new Date(d.replace(' ', 'T')).toLocaleString() : 'Never')
const statusClass = {
  active: 'bg-ctp-green/15 text-ctp-green',
  revoked: 'bg-ctp-red/15 text-ctp-red',
  expired: 'bg-ctp-surface0 text-ctp-subtext0',
}
</script>

<template>
  <div class="space-y-5">
    <h1 class="text-2xl font-bold text-ctp-text">API keys</h1>
    <p class="max-w-2xl text-sm text-ctp-subtext0">
      Each manager (and admin) can create one API key from their dashboard to let scripts or AI
      assistants add quizzes and interactive reviews. Revoke a key if it leaks or is no longer
      needed. Deactivating or deleting a manager also disables their key.
    </p>

    <div class="flex flex-wrap items-center gap-3">
      <select
        v-model="userFilter"
        class="input max-w-xs"
        aria-label="Filter by manager"
        @change="run"
      >
        <option value="">All managers</option>
        <option v-for="m in managers" :key="m.id" :value="m.id">
          {{ m.display_name }} ({{ m.email }})
        </option>
      </select>
      <select v-model="statusFilter" class="input max-w-[10rem]" aria-label="Filter by status">
        <option value="active">Active only</option>
        <option value="all">All keys</option>
      </select>
    </div>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-16" />
    </div>
    <EmptyState
      v-else-if="!shown.length"
      title="No API keys"
      message="Managers create keys from their own dashboard."
      icon="key"
    />
    <ul v-else class="space-y-2">
      <li v-for="k in shown" :key="k.id" class="card flex flex-wrap items-center gap-3 p-3">
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
        <span
          class="rounded-full px-2.5 py-1 text-xs font-medium capitalize"
          :class="statusClass[k.status]"
          >{{ k.status }}</span
        >
        <button
          v-if="k.status === 'active'"
          class="btn-ghost btn-sm text-ctp-red"
          @click="revoking = k"
        >
          <AppIcon name="trash" :size="16" /> Revoke
        </button>
      </li>
    </ul>

    <BaseModal :open="!!revoking" title="Revoke API key?" size="sm" @close="revoking = null">
      <p class="text-sm text-ctp-subtext1">
        {{ revoking?.owner_name }}'s key
        <span class="font-mono">{{ revoking?.key_prefix }}…</span> will stop working immediately.
        They can create a new one from their dashboard.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="revoking = null">Cancel</button>
        <button class="btn-danger" @click="confirmRevoke">Revoke</button>
      </template>
    </BaseModal>
  </div>
</template>
