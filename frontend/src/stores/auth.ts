import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import { ApiError, api } from '@/api/client'
import type { User } from '@/api/types'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const loaded = ref(false)
  let pending: Promise<void> | null = null

  const isStaff = computed(() => !!user.value)
  const isAdmin = computed(() => user.value?.role === 'admin')
  const isManager = computed(() => user.value?.role === 'manager')

  /** Load the current session once (the cookie is httpOnly, so we must ask the API). */
  function fetchMe(force = false): Promise<void> {
    if (loaded.value && !force) return Promise.resolve()
    if (pending) return pending
    pending = api
      .get<User>('/auth/me')
      .then((u) => {
        user.value = u
      })
      .catch((err) => {
        if (!(err instanceof ApiError) || err.status !== 401) console.warn(err)
        user.value = null
      })
      .finally(() => {
        loaded.value = true
        pending = null
      })
    return pending
  }

  async function login(email: string, password: string) {
    user.value = await api.post<User>('/auth/login', { email, password })
    loaded.value = true
  }

  async function logout() {
    try {
      await api.post('/auth/logout')
    } finally {
      user.value = null
    }
  }

  return { user, loaded, isStaff, isAdmin, isManager, fetchMe, login, logout }
})
