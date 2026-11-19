<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { User } from '@/api/types'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAsync } from '@/composables/useAsync'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const { data: users, loading, run } = useAsync(() => api.get<User[]>('/admin/users'), [])
onMounted(run)

const creating = ref(false)
const saving = ref(false)
const togglingId = ref<number | null>(null)
const resettingPassword = ref(false)
const deletingManager = ref(false)
const form = reactive({ email: '', display_name: '', password: '' })
async function create() {
  if (saving.value) return
  saving.value = true
  try {
    await api.post('/admin/users', { ...form })
    toast.success('Manager account created')
    creating.value = false
    Object.assign(form, { email: '', display_name: '', password: '' })
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    saving.value = false
  }
}

async function toggleActive(u: User) {
  if (togglingId.value !== null) return
  togglingId.value = u.id
  try {
    await api.patch(`/admin/users/${u.id}`, { is_active: !u.is_active })
    toast.success(u.is_active ? 'Account deactivated' : 'Account reactivated')
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    togglingId.value = null
  }
}

const resetting = ref<User | null>(null)
const newPassword = ref('')
async function resetPassword() {
  if (!resetting.value || resettingPassword.value) return
  resettingPassword.value = true
  try {
    await api.post(`/admin/users/${resetting.value.id}/reset-password`, {
      password: newPassword.value,
    })
    toast.success('Password reset')
    resetting.value = null
    newPassword.value = ''
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    resettingPassword.value = false
  }
}

const deleting = ref<User | null>(null)
async function confirmDelete() {
  if (!deleting.value || deletingManager.value) return
  deletingManager.value = true
  try {
    await api.delete(`/admin/users/${deleting.value.id}`)
    toast.success('Manager deleted')
    deleting.value = null
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    deletingManager.value = false
  }
}
</script>

<template>
  <div class="space-y-5">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
        Managers
      </h1>
      <v-btn color="primary" prepend-icon="mdi-account-plus-outline" @click="creating = true"
        >New manager
      </v-btn>
    </div>
    <v-alert type="info" icon="mdi-information-outline" class="text-sm">
      Managers can create quizzes and interactive reviews. Only admins can manage accounts and
      subjects.
    </v-alert>

    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 3" :key="i" class="h-14" />
    </div>
    <ul v-else class="space-y-2">
      <v-card
        v-for="(u, i) in users"
        :key="u.id"
        tag="li"
        class="card-lift flex flex-wrap items-center gap-3 p-3"
        :class="u.is_active ? '' : 'opacity-70'"
        data-aos="fade-up"
        :data-aos-delay="Math.min(i, 6) * 40"
      >
        <v-avatar :color="u.role === 'admin' ? 'ctp-peach' : 'primary'" variant="tonal">
          {{ u.display_name.slice(0, 1).toUpperCase() }}
        </v-avatar>
        <div class="min-w-0 flex-1">
          <p class="font-medium text-ctp-text">{{ u.display_name }}</p>
          <p class="text-xs text-ctp-subtext0">{{ u.email }}</p>
        </div>
        <v-chip size="small" variant="tonal" class="uppercase">{{ u.role }}</v-chip>
        <v-chip
          v-if="!u.is_active"
          size="small"
          color="error"
          variant="tonal"
          prepend-icon="mdi-account-off-outline"
        >
          Inactive
        </v-chip>
        <template v-if="u.role === 'manager'">
          <v-btn
            variant="tonal"
            size="small"
            :color="u.is_active ? 'warning' : 'success'"
            :prepend-icon="u.is_active ? 'mdi-account-cancel-outline' : 'mdi-account-check-outline'"
            :loading="togglingId === u.id"
            @click="toggleActive(u)"
          >
            {{ u.is_active ? 'Deactivate' : 'Activate' }}
          </v-btn>
          <v-btn variant="tonal" size="small" prepend-icon="mdi-lock-reset" @click="resetting = u">
            Reset password
          </v-btn>
          <v-btn
            variant="text"
            size="small"
            icon="mdi-delete-outline"
            color="error"
            aria-label="Delete"
            @click="deleting = u"
          />
        </template>
      </v-card>
    </ul>

    <BaseModal :open="creating" title="New manager" @close="creating = false">
      <v-form id="new-user" class="space-y-4" @submit.prevent="create">
        <v-text-field v-model="form.display_name" label="Display name" required maxlength="100" />
        <v-text-field v-model="form.email" label="Email" type="email" required autocomplete="off" />
        <v-text-field
          v-model="form.password"
          label="Initial password (min 8 characters)"
          type="password"
          required
          minlength="8"
          autocomplete="new-password"
        />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="creating = false">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="new-user"
          prepend-icon="mdi-plus"
          :loading="saving"
          >Create manager</v-btn
        >
      </template>
    </BaseModal>

    <BaseModal
      :open="!!resetting"
      :title="`Reset password for ${resetting?.display_name ?? ''}`"
      size="sm"
      @close="resetting = null"
    >
      <v-form id="reset-pass" @submit.prevent="resetPassword">
        <v-text-field
          v-model="newPassword"
          label="New password (min 8 characters)"
          type="password"
          prepend-inner-icon="mdi-lock-outline"
          required
          minlength="8"
          autocomplete="new-password"
        />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="resetting = null">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="reset-pass"
          prepend-icon="mdi-lock-reset"
          :loading="resettingPassword"
          >Reset</v-btn
        >
      </template>
    </BaseModal>

    <BaseModal :open="!!deleting" title="Delete manager?" size="sm" @close="deleting = null">
      <v-alert type="warning" icon="mdi-account-remove-outline">
        {{ deleting?.email }} will be deleted. Their quizzes and reviews go to the manager each one
        was shared with first; anything never shared stays online (if published) but nobody can edit
        it. Consider deactivating instead.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="deleting = null">Cancel</v-btn>
        <v-btn
          color="error"
          prepend-icon="mdi-delete-outline"
          :loading="deletingManager"
          @click="confirmDelete"
          >Delete</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
