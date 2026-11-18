<script setup lang="ts">
/**
 * Share a whole year, or one subject of a year, with other people by email.
 * Year owner / admin: add and remove people. Someone it is shared with: see who has access, or leave.
 */
import { ref, watch } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { Shares } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const props = defineProps<{
  /** What is shared: a whole `years` entry or one `subjects` entry. */
  kind: 'years' | 'subjects'
  /** The year/subject to share; `null` closes the dialog. */
  item: { id: number; title: string } | null
}>()
const emit = defineEmits<{ close: []; changed: []; left: [] }>()

const auth = useAuthStore()
const toast = useToastStore()
const shares = ref<Shares | null>(null)
const loading = ref(false)
const email = ref('')
const busy = ref(false)

const base = () => `/admin/${props.kind}/${props.item!.id}/shares`

const description = () =>
  props.kind === 'years'
    ? 'People you add can add quizzes and reviews to every subject of this year, and edit and publish all content in it. Only the year owner (or an admin) can change the year, its subjects and who has access.'
    : "People you add can add quizzes and reviews to this subject, and edit and publish all content in it. Only the year's owner (or an admin) can change it and who has access."

watch(
  () => props.item,
  async (item) => {
    shares.value = null
    email.value = ''
    if (!item) return
    loading.value = true
    try {
      shares.value = await api.get<Shares>(base())
    } catch (err) {
      toast.error(errorMessage(err))
      emit('close')
    } finally {
      loading.value = false
    }
  },
)

async function add() {
  if (!email.value.trim() || busy.value) return
  busy.value = true
  try {
    shares.value = await api.post<Shares>(base(), { email: email.value.trim() })
    toast.success(`Shared with ${email.value.trim()}`)
    email.value = ''
    emit('changed')
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    busy.value = false
  }
}

async function remove(userId: number) {
  if (busy.value) return
  const leaving = userId === auth.user?.id
  busy.value = true
  try {
    shares.value = await api.delete<Shares>(`${base()}/${userId}`)
    if (leaving && !shares.value?.can_manage) {
      toast.success('You no longer have access to it')
      emit('left')
      emit('close')
    } else {
      toast.success('Access removed')
      emit('changed')
    }
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <BaseModal
    :open="!!item"
    :title="`Share “${item?.title ?? ''}”`"
    :description="description()"
    @close="emit('close')"
  >
    <div v-if="loading" class="space-y-2">
      <SkeletonBlock v-for="i in 2" :key="i" class="h-12" />
    </div>
    <div v-else-if="shares" class="space-y-5">
      <form v-if="shares.can_manage" class="flex gap-2" @submit.prevent="add">
        <label for="share-email" class="sr-only">Email</label>
        <input
          id="share-email"
          v-model="email"
          type="email"
          required
          class="input flex-1"
          placeholder="Email of a manager or admin, e.g. dara@school.edu"
          autocomplete="off"
        />
        <button class="btn-primary" type="submit" :disabled="busy || !email.trim()">
          <AppIcon name="plus" :size="16" /> Share
        </button>
      </form>

      <ul class="divide-y divide-ctp-surface0 rounded-lg border border-ctp-surface0">
        <li v-if="shares.owner" class="flex items-center gap-3 p-3">
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ctp-text">
              {{ shares.owner.display_name }}
              <span v-if="shares.owner.user_id === auth.user?.id" class="text-ctp-subtext0"
                >(you)</span
              >
            </p>
            <p class="text-xs text-ctp-subtext0">{{ shares.owner.email }}</p>
          </div>
          <span
            class="rounded-full bg-ctp-accent/15 px-2.5 py-1 text-xs font-medium text-ctp-accent"
          >
            Year owner
          </span>
        </li>
        <li v-for="p in shares.shared_with" :key="p.user_id" class="flex items-center gap-3 p-3">
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ctp-text">
              {{ p.display_name }}
              <span v-if="p.user_id === auth.user?.id" class="text-ctp-subtext0">(you)</span>
            </p>
            <p class="text-xs text-ctp-subtext0">{{ p.email }}</p>
          </div>
          <span class="text-xs text-ctp-subtext0">Has access</span>
          <button
            v-if="shares.can_manage"
            class="btn-ghost btn-sm text-ctp-red"
            :disabled="busy"
            :aria-label="`Remove ${p.display_name}`"
            @click="remove(p.user_id)"
          >
            <AppIcon name="x" :size="14" /> Remove
          </button>
          <button
            v-else-if="p.user_id === auth.user?.id"
            class="btn-ghost btn-sm text-ctp-red"
            :disabled="busy"
            @click="remove(p.user_id)"
          >
            Leave
          </button>
        </li>
        <li v-if="!shares.shared_with.length" class="p-3 text-sm text-ctp-subtext0">
          Not shared with anyone yet.
        </li>
      </ul>
    </div>
    <template #footer>
      <button class="btn-secondary" @click="emit('close')">Done</button>
    </template>
  </BaseModal>
</template>
