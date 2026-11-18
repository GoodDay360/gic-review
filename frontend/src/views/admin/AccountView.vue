<script setup lang="ts">
/** "My account": who you are, and change your own password (current password required). */
import { computed, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import AppIcon from '@/components/AppIcon.vue'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const auth = useAuthStore()
const toast = useToastStore()

const form = reactive({ current: '', next: '', confirm: '' })
const show = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

const problems = computed(() => {
  const p: string[] = []
  if (form.next && form.next.length < 8) p.push('The new password must be at least 8 characters.')
  if (form.next && form.current && form.next === form.current)
    p.push('The new password must be different from the current one.')
  if (form.confirm && form.next !== form.confirm) p.push("The two new passwords don't match.")
  return p
})
const canSubmit = computed(
  () => !!form.current && !!form.next && !!form.confirm && problems.value.length === 0,
)

async function submit() {
  if (!canSubmit.value) return
  saving.value = true
  error.value = null
  try {
    await api.post('/auth/change-password', {
      current_password: form.current,
      new_password: form.next,
    })
    Object.assign(form, { current: '', next: '', confirm: '' })
    toast.success('Password changed. Your other devices have been signed out.')
  } catch (err) {
    error.value = errorMessage(err).replace(/^current_password: /, 'Current password ')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="max-w-xl space-y-6">
    <h1 class="text-2xl font-bold text-ctp-text">My account</h1>

    <section class="card space-y-1 p-5">
      <p class="font-semibold text-ctp-text">{{ auth.user?.display_name }}</p>
      <p class="text-sm text-ctp-subtext0">{{ auth.user?.email }}</p>
      <p>
        <span class="rounded bg-ctp-surface0 px-1.5 py-0.5 text-[11px] uppercase text-ctp-subtext1">
          {{ auth.user?.role }}
        </span>
      </p>
    </section>

    <form class="card space-y-4 p-5" @submit.prevent="submit">
      <div>
        <h2 class="flex items-center gap-2 font-semibold text-ctp-text">
          <AppIcon name="key" :size="18" /> Change password
        </h2>
        <p class="mt-1 text-sm text-ctp-subtext0">
          Enter your current password to confirm it's you. After changing it, you stay signed in
          here and every other device is signed out.
        </p>
      </div>
      <!-- Hidden username field helps password managers save the new password correctly. -->
      <input
        type="text"
        :value="auth.user?.email"
        autocomplete="username"
        class="sr-only"
        tabindex="-1"
        aria-hidden="true"
        readonly
      />
      <div>
        <label class="label" for="pw-current">Current password</label>
        <input
          id="pw-current"
          v-model="form.current"
          :type="show ? 'text' : 'password'"
          class="input"
          autocomplete="current-password"
          required
        />
      </div>
      <div>
        <label class="label" for="pw-new">New password (min 8 characters)</label>
        <input
          id="pw-new"
          v-model="form.next"
          :type="show ? 'text' : 'password'"
          class="input"
          autocomplete="new-password"
          minlength="8"
          required
        />
      </div>
      <div>
        <label class="label" for="pw-confirm">Confirm new password</label>
        <input
          id="pw-confirm"
          v-model="form.confirm"
          :type="show ? 'text' : 'password'"
          class="input"
          autocomplete="new-password"
          required
        />
      </div>
      <label class="flex items-center gap-2 text-sm text-ctp-subtext1">
        <input v-model="show" type="checkbox" class="h-4 w-4 accent-ctp-accent" /> Show passwords
      </label>

      <ul v-if="problems.length" class="space-y-1 text-sm text-ctp-yellow" role="alert">
        <li v-for="p in problems" :key="p">{{ p }}</li>
      </ul>
      <p v-if="error" role="alert" class="rounded-lg bg-ctp-red/10 p-3 text-sm text-ctp-red">
        {{ error }}
      </p>

      <div class="flex justify-end">
        <button class="btn-primary" type="submit" :disabled="saving || !canSubmit">
          {{ saving ? 'Changing…' : 'Change password' }}
        </button>
      </div>
    </form>
  </div>
</template>
