<script setup lang="ts">
/**
 * Share a whole category, or one subject of a category, with other people by email.
 * Category owner / admin: add and remove people, or share it with every manager at once.
 * Someone it is shared with: see who has access, or leave.
 */
import { ref, watch } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { Shares } from '@/api/types'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { useAuthStore } from '@/stores/auth'
import { useToastStore } from '@/stores/toast'

const props = defineProps<{
  /** What is shared: a whole `categories` entry or one `subjects` entry. */
  kind: 'categories' | 'subjects'
  /** The category/subject to share; `null` closes the dialog. */
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
  props.kind === 'categories'
    ? 'People you add can add quizzes and reviews to every subject of this category, and edit and publish all content in it. Only the category owner (or an admin) can change the category, its subjects and who has access.'
    : "People you add can add quizzes and reviews to this subject, and edit and publish all content in it. Only the category's owner (or an admin) can change it and who has access."

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
    toast.success(`${email.value.trim()} can now add and edit content here.`, 'Shared')
    email.value = ''
    emit('changed')
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    busy.value = false
  }
}

async function setAll(enabled: boolean | null) {
  if (busy.value) return
  busy.value = true
  try {
    shares.value = await api.put<Shares>(`${base()}/all`, { enabled: !!enabled })
    toast.success(
      enabled
        ? 'Every manager can now add and edit content here.'
        : 'No longer shared with all managers.',
      enabled ? 'Shared' : undefined,
    )
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
    const stillShared = shares.value?.shared_with_all || shares.value?.category_shared_with_all
    if (leaving && !shares.value?.can_manage && stillShared) {
      toast.success('Removed — you still have access because it is shared with all managers')
      emit('changed')
    } else if (leaving && !shares.value?.can_manage) {
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
      <v-form v-if="shares.can_manage" class="flex gap-2" @submit.prevent="add">
        <v-text-field
          v-model="email"
          type="email"
          required
          label="Email"
          prepend-inner-icon="mdi-email-outline"
          class="flex-1"
          placeholder="Email of a manager or admin, e.g. dara@school.edu"
          autocomplete="off"
          hide-details
        />
        <v-btn
          color="primary"
          prepend-icon="mdi-share-variant-outline"
          type="submit"
          height="48"
          :loading="busy"
          :disabled="!email.trim()"
          >Share</v-btn
        >
      </v-form>

      <div
        v-if="shares.can_manage"
        class="flex items-center gap-3 rounded-lg border border-ctp-surface0 p-3"
      >
        <v-avatar color="secondary" variant="tonal" size="36">
          <v-icon icon="mdi-account-group-outline" size="20" />
        </v-avatar>
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium text-ctp-text">Share with all managers</p>
          <p class="text-xs text-ctp-subtext0">
            Every manager can add, edit and publish content here, without adding them one by one.
          </p>
        </div>
        <v-switch
          :model-value="shares.shared_with_all"
          color="primary"
          inset
          hide-details
          density="compact"
          :disabled="busy"
          aria-label="Share with all managers"
          @update:model-value="setAll"
        />
      </div>
      <p
        v-if="kind === 'subjects' && shares.category_shared_with_all"
        class="flex items-center gap-2 text-sm text-ctp-subtext0"
      >
        <v-icon icon="mdi-information-outline" size="18" />
        Its whole category is shared with all managers, so every manager already has access.
      </p>

      <ul class="divide-y divide-ctp-surface0 rounded-lg border border-ctp-surface0">
        <li v-if="shares.owner" class="flex items-center gap-3 p-3">
          <v-avatar color="primary" variant="tonal" size="36">
            <v-icon icon="mdi-crown-outline" size="20" />
          </v-avatar>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ctp-text">
              {{ shares.owner.display_name }}
              <span v-if="shares.owner.user_id === auth.user?.id" class="text-ctp-subtext0"
                >(you)</span
              >
            </p>
            <p class="text-xs text-ctp-subtext0">{{ shares.owner.email }}</p>
          </div>
          <v-chip size="small" color="primary" variant="tonal">Category owner</v-chip>
        </li>
        <li v-if="shares.shared_with_all && !shares.can_manage" class="flex items-center gap-3 p-3">
          <v-avatar color="secondary" variant="tonal" size="36">
            <v-icon icon="mdi-account-group-outline" size="20" />
          </v-avatar>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ctp-text">All managers</p>
            <p class="text-xs text-ctp-subtext0">Everyone with a manager account</p>
          </div>
          <v-chip size="x-small" color="success" variant="tonal" prepend-icon="mdi-check"
            >Has access</v-chip
          >
        </li>
        <li
          v-for="p in shares.shared_with"
          :key="p.user_id"
          class="animate__animated animate__fadeIn flex items-center gap-3 p-3"
        >
          <v-avatar variant="tonal" size="36">
            {{ p.display_name.slice(0, 1).toUpperCase() }}
          </v-avatar>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-ctp-text">
              {{ p.display_name }}
              <span v-if="p.user_id === auth.user?.id" class="text-ctp-subtext0">(you)</span>
            </p>
            <p class="text-xs text-ctp-subtext0">{{ p.email }}</p>
          </div>
          <v-chip size="x-small" color="success" variant="tonal" prepend-icon="mdi-check"
            >Has access</v-chip
          >
          <v-btn
            v-if="shares.can_manage"
            variant="text"
            size="small"
            prepend-icon="mdi-close"
            color="error"
            :disabled="busy"
            :aria-label="`Remove ${p.display_name}`"
            @click="remove(p.user_id)"
            >Remove
          </v-btn>
          <v-btn
            v-else-if="p.user_id === auth.user?.id"
            variant="text"
            size="small"
            color="error"
            prepend-icon="mdi-exit-run"
            :disabled="busy"
            @click="remove(p.user_id)"
          >
            Leave
          </v-btn>
        </li>
        <li
          v-if="!shares.shared_with.length && !(shares.shared_with_all && !shares.can_manage)"
          class="flex items-center gap-2 p-3 text-sm text-ctp-subtext0"
        >
          <v-icon icon="mdi-account-off-outline" size="18" /> Not shared with anyone yet.
        </li>
      </ul>
    </div>
    <template #footer>
      <v-btn variant="tonal" prepend-icon="mdi-check" @click="emit('close')">Done</v-btn>
    </template>
  </BaseModal>
</template>
