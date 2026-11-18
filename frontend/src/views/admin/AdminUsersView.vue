<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { User } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const { data: users, loading, run } = useAsync(() => api.get<User[]>('/admin/users'), [])
onMounted(run)

const creating = ref(false)
const form = reactive({ email: '', display_name: '', password: '' })
async function create() {
  try {
    await api.post('/admin/users', { ...form })
    toast.success('Manager account created')
    creating.value = false
    Object.assign(form, { email: '', display_name: '', password: '' })
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

async function toggleActive(u: User) {
  try {
    await api.patch(`/admin/users/${u.id}`, { is_active: !u.is_active })
    toast.success(u.is_active ? 'Account deactivated' : 'Account reactivated')
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

const resetting = ref<User | null>(null)
const newPassword = ref('')
async function resetPassword() {
  if (!resetting.value) return
  try {
    await api.post(`/admin/users/${resetting.value.id}/reset-password`, {
      password: newPassword.value,
    })
    toast.success('Password reset')
    resetting.value = null
    newPassword.value = ''
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

const deleting = ref<User | null>(null)
async function confirmDelete() {
  if (!deleting.value) return
  try {
    await api.delete(`/admin/users/${deleting.value.id}`)
    toast.success('Manager deleted')
    deleting.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-bold text-ctp-text">Managers</h1>
      <button class="btn-primary" @click="creating = true">
        <AppIcon name="plus" :size="16" /> New manager
      </button>
    </div>
    <p class="text-sm text-ctp-subtext0">
      Managers can create quizzes and interactive reviews. Only admins can manage accounts and
      subjects.
    </p>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-14" />
    </div>
    <ul v-else class="space-y-2">
      <li v-for="u in users" :key="u.id" class="card flex flex-wrap items-center gap-3 p-3">
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">{{ u.display_name }}</p>
          <p class="text-xs text-ctp-subtext0">{{ u.email }}</p>
        </div>
        <span class="rounded-full bg-ctp-surface0 px-2.5 py-1 text-xs uppercase">{{ u.role }}</span>
        <span
          v-if="!u.is_active"
          class="rounded-full bg-ctp-red/15 px-2.5 py-1 text-xs text-ctp-red"
          >Inactive</span
        >
        <template v-if="u.role === 'manager'">
          <button class="btn-secondary btn-sm" @click="toggleActive(u)">
            {{ u.is_active ? 'Deactivate' : 'Activate' }}
          </button>
          <button class="btn-secondary btn-sm" @click="resetting = u">Reset password</button>
          <button class="btn-ghost btn-sm text-ctp-red" aria-label="Delete" @click="deleting = u">
            <AppIcon name="trash" :size="16" />
          </button>
        </template>
      </li>
    </ul>

    <BaseModal :open="creating" title="New manager" @close="creating = false">
      <form id="new-user" class="space-y-4" @submit.prevent="create">
        <div>
          <label class="label" for="nu-name">Display name</label>
          <input id="nu-name" v-model="form.display_name" class="input" required maxlength="100" />
        </div>
        <div>
          <label class="label" for="nu-email">Email</label>
          <input
            id="nu-email"
            v-model="form.email"
            type="email"
            class="input"
            required
            autocomplete="off"
          />
        </div>
        <div>
          <label class="label" for="nu-pass">Initial password (min 8 characters)</label>
          <input
            id="nu-pass"
            v-model="form.password"
            type="password"
            class="input"
            required
            minlength="8"
            autocomplete="new-password"
          />
        </div>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="creating = false">Cancel</button>
        <button class="btn-primary" type="submit" form="new-user">Create manager</button>
      </template>
    </BaseModal>

    <BaseModal
      :open="!!resetting"
      :title="`Reset password for ${resetting?.display_name ?? ''}`"
      size="sm"
      @close="resetting = null"
    >
      <form id="reset-pass" @submit.prevent="resetPassword">
        <label class="label" for="rp-pass">New password (min 8 characters)</label>
        <input
          id="rp-pass"
          v-model="newPassword"
          type="password"
          class="input"
          required
          minlength="8"
          autocomplete="new-password"
        />
      </form>
      <template #footer>
        <button class="btn-ghost" @click="resetting = null">Cancel</button>
        <button class="btn-primary" type="submit" form="reset-pass">Reset</button>
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete manager?" size="sm" @close="deleting = null">
      <p class="text-sm text-ctp-subtext1">
        {{ deleting?.email }} will be deleted. Quizzes they created are kept.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="deleting = null">Cancel</button>
        <button class="btn-danger" @click="confirmDelete">Delete</button>
      </template>
    </BaseModal>
  </div>
</template>
