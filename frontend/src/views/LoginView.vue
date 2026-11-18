<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { errorMessage } from '@/api/client'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
const router = useRouter()
const route = useRoute()

const email = ref('')
const password = ref('')
const submitting = ref(false)
const error = ref<string | null>(null)

async function submit() {
  if (submitting.value) return
  submitting.value = true
  error.value = null
  try {
    await auth.login(email.value, password.value)
    const redirect = typeof route.query.redirect === 'string' ? route.query.redirect : '/admin'
    // Only allow internal redirects.
    router.push(redirect.startsWith('/') && !redirect.startsWith('//') ? redirect : '/admin')
  } catch (err) {
    error.value = errorMessage(err)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto flex max-w-sm flex-col px-4 py-16">
    <h1 class="text-2xl font-bold text-ctp-text">Staff login</h1>
    <p class="mt-1 text-sm text-ctp-subtext0">
      For admins and managers only. Students don't need an account.
    </p>
    <form class="card mt-6 space-y-4 p-5" @submit.prevent="submit">
      <div>
        <label for="email" class="label">Email</label>
        <input
          id="email"
          v-model="email"
          type="email"
          autocomplete="username"
          required
          class="input"
        />
      </div>
      <div>
        <label for="password" class="label">Password</label>
        <input
          id="password"
          v-model="password"
          type="password"
          autocomplete="current-password"
          required
          class="input"
        />
      </div>
      <p v-if="error" role="alert" class="rounded-lg bg-ctp-red/10 p-3 text-sm text-ctp-red">
        {{ error }}
      </p>
      <button type="submit" class="btn-primary w-full" :disabled="submitting">
        {{ submitting ? 'Signing in…' : 'Sign in' }}
      </button>
    </form>
  </div>
</template>
