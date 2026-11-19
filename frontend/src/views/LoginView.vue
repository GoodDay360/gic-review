<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { ApiError } from '@/api/client'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

type Notice = { type: 'error' | 'warning' | 'info' | 'success'; title: string; text: string }

const auth = useAuthStore()
const toast = useToastStore()
const router = useRouter()
const route = useRoute()

const form = ref<{ validate: () => Promise<{ valid: boolean }> } | null>(null)
const email = ref('')
const password = ref('')
const showPassword = ref(false)
const capsLock = ref(false)
const submitting = ref(false)
const notice = ref<Notice | null>(null)
/** Bumped on every new notice so animate.css replays its entrance. */
const noticeKey = ref(0)

const rules = {
  email: [
    (v: string) => !!v.trim() || 'Enter your email.',
    (v: string) =>
      /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v.trim()) || 'That doesn’t look like an email address.',
  ],
  password: [(v: string) => !!v || 'Enter your password.'],
}

const redirectTarget = computed(() => {
  const r = typeof route.query.redirect === 'string' ? route.query.redirect : ''
  // Only allow internal redirects.
  return r.startsWith('/') && !r.startsWith('//') ? r : ''
})

// Sent here from a protected page: say why.
if (redirectTarget.value) {
  notice.value = {
    type: 'info',
    title: 'Sign in to continue',
    text: 'That page is for staff only. Sign in and you’ll go straight there.',
  }
}

function show(n: Notice) {
  notice.value = n
  noticeKey.value++
}

/** Turn a failed login into a clear, specific message. */
function describe(err: unknown): Notice {
  if (err instanceof ApiError) {
    switch (err.code) {
      case 'invalid_credentials':
        return {
          type: 'error',
          title: 'Wrong email or password',
          text: 'Check both and try again. Passwords are case-sensitive.',
        }
      case 'too_many_requests':
        return {
          type: 'warning',
          title: 'Too many attempts',
          text: 'For your security, sign-in is paused for a few minutes. Please wait, then try again.',
        }
      case 'validation_error':
        return { type: 'warning', title: 'Check your details', text: err.message }
      case 'network_error':
        return {
          type: 'error',
          title: 'Can’t reach the server',
          text: 'Check your internet connection, then try again.',
        }
    }
    if (err.status >= 500)
      return {
        type: 'error',
        title: 'Something went wrong on our side',
        text: 'The server had a problem. Please try again in a moment.',
      }
    return { type: 'error', title: 'Sign-in failed', text: err.message }
  }
  return { type: 'error', title: 'Sign-in failed', text: 'Something went wrong. Please try again.' }
}

function onPasswordKey(e: KeyboardEvent) {
  capsLock.value = e.getModifierState?.('CapsLock') ?? false
}

async function submit() {
  if (submitting.value) return
  const result = await form.value?.validate()
  if (result && !result.valid) {
    show({ type: 'warning', title: 'Almost there', text: 'Fill in both fields to sign in.' })
    return
  }
  submitting.value = true
  notice.value = null
  try {
    await auth.login(email.value.trim(), password.value)
    show({ type: 'success', title: 'Signed in', text: 'Taking you to your dashboard…' })
    toast.success(`Good to see you, ${auth.user?.display_name ?? 'there'}!`, 'Welcome back')
    router.push(redirectTarget.value || '/admin')
  } catch (err) {
    show(describe(err))
    if (err instanceof ApiError && err.code === 'invalid_credentials') password.value = ''
  } finally {
    submitting.value = false
  }
}

const noticeIcon = {
  error: 'mdi-alert-octagon-outline',
  warning: 'mdi-alert-outline',
  info: 'mdi-information-outline',
  success: 'mdi-check-circle-outline',
}
const noticeAnim = {
  error: 'animate__headShake',
  warning: 'animate__headShake',
  info: 'animate__fadeIn',
  success: 'animate__bounceIn',
}
</script>

<template>
  <div class="mx-auto flex max-w-md flex-col px-4 py-12 sm:py-16">
    <div class="animate__animated animate__fadeInDown mb-6 flex flex-col items-center text-center">
      <v-avatar color="primary" variant="tonal" size="64" class="mb-4">
        <v-icon icon="mdi-shield-account-outline" size="34" />
      </v-avatar>
      <h1 class="text-2xl font-bold text-ctp-text">Staff login</h1>
      <p class="mt-1 text-sm text-ctp-subtext0">
        For admins and managers only. Students don't need an account.
      </p>
    </div>

    <v-card class="animate__animated animate__fadeInUp p-6">
      <v-form ref="form" validate-on="submit lazy" @submit.prevent="submit">
        <div class="space-y-4">
          <v-alert
            v-if="notice"
            :key="noticeKey"
            :type="notice.type"
            :icon="noticeIcon[notice.type]"
            :title="notice.title"
            :text="notice.text"
            role="alert"
            class="animate__animated"
            :class="noticeAnim[notice.type]"
            :closable="notice.type !== 'success'"
            @click:close="notice = null"
          />
          <v-text-field
            v-model="email"
            label="Email"
            type="email"
            autocomplete="username"
            prepend-inner-icon="mdi-email-outline"
            :rules="rules.email"
            :disabled="submitting"
            autofocus
          />
          <v-text-field
            v-model="password"
            label="Password"
            :type="showPassword ? 'text' : 'password'"
            autocomplete="current-password"
            prepend-inner-icon="mdi-lock-outline"
            :append-inner-icon="showPassword ? 'mdi-eye-off-outline' : 'mdi-eye-outline'"
            :rules="rules.password"
            :disabled="submitting"
            @click:append-inner="showPassword = !showPassword"
            @keydown="onPasswordKey"
            @keyup="onPasswordKey"
          />
          <v-expand-transition>
            <v-alert
              v-if="capsLock"
              type="warning"
              density="compact"
              icon="mdi-keyboard-caps"
              text="Caps Lock is on."
            />
          </v-expand-transition>
          <v-btn
            type="submit"
            color="primary"
            size="large"
            block
            :loading="submitting"
            append-icon="mdi-arrow-right"
          >
            Sign in
          </v-btn>
        </div>
      </v-form>
    </v-card>

    <v-btn
      to="/"
      variant="text"
      prepend-icon="mdi-arrow-left"
      class="animate__animated animate__fadeIn mx-auto mt-6"
    >
      Back to studying
    </v-btn>
  </div>
</template>
