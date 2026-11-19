<script setup lang="ts">
/** "My account": who you are, and change your own password (current password required). */
import { computed, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
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
  if (!canSubmit.value || saving.value) return
  saving.value = true
  error.value = null
  try {
    await api.post('/auth/change-password', {
      current_password: form.current,
      new_password: form.next,
    })
    Object.assign(form, { current: '', next: '', confirm: '' })
    toast.success('Your other devices have been signed out.', 'Password changed')
  } catch (err) {
    error.value = errorMessage(err).replace(/^current_password: /, 'Current password ')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="max-w-xl space-y-6">
    <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">
      My account
    </h1>

    <v-card tag="section" class="flex items-center gap-4 p-5" data-aos="fade-up">
      <v-avatar color="primary" size="56" class="text-xl font-bold">
        {{ auth.user?.display_name?.slice(0, 1).toUpperCase() }}
      </v-avatar>
      <div class="min-w-0 space-y-1">
        <p class="font-semibold text-ctp-text">{{ auth.user?.display_name }}</p>
        <p class="truncate text-sm text-ctp-subtext0">{{ auth.user?.email }}</p>
        <v-chip size="x-small" variant="tonal" class="uppercase">{{ auth.user?.role }}</v-chip>
      </div>
    </v-card>

    <v-card
      tag="form"
      class="space-y-4 p-5"
      data-aos="fade-up"
      data-aos-delay="80"
      @submit.prevent="submit"
    >
      <div>
        <h2 class="flex items-center gap-2 font-semibold text-ctp-text">
          <v-icon icon="mdi-key-variant" size="18" /> Change password
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
      <v-text-field
        v-model="form.current"
        label="Current password"
        prepend-inner-icon="mdi-lock-outline"
        :type="show ? 'text' : 'password'"
        autocomplete="current-password"
        required
      />
      <v-text-field
        v-model="form.next"
        label="New password (min 8 characters)"
        prepend-inner-icon="mdi-lock-plus-outline"
        :type="show ? 'text' : 'password'"
        autocomplete="new-password"
        minlength="8"
        required
      />
      <v-text-field
        v-model="form.confirm"
        label="Confirm new password"
        prepend-inner-icon="mdi-lock-check-outline"
        :type="show ? 'text' : 'password'"
        autocomplete="new-password"
        required
      />
      <v-checkbox v-model="show" label="Show passwords" />

      <v-expand-transition>
        <v-alert v-if="problems.length" type="warning" density="compact" role="alert">
          <ul class="space-y-0.5">
            <li v-for="p in problems" :key="p">{{ p }}</li>
          </ul>
        </v-alert>
      </v-expand-transition>
      <v-alert
        v-if="error"
        type="error"
        title="Password not changed"
        :text="error"
        role="alert"
        class="animate__animated animate__headShake"
        closable
        @click:close="error = null"
      />

      <div class="flex justify-end">
        <v-btn
          color="primary"
          type="submit"
          prepend-icon="mdi-lock-check-outline"
          :loading="saving"
          :disabled="!canSubmit"
        >
          Change password
        </v-btn>
      </div>
    </v-card>
  </div>
</template>
