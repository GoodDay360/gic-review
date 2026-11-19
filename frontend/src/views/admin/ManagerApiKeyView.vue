<script setup lang="ts">
/**
 * A manager's personal API key (max one active). Scripts and AI assistants send it as
 * `Authorization: Bearer gic_…` to add quizzes and interactive reviews.
 */
import { computed, onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { ApiKey, ApiScope, CreatedApiKey } from '@/api/types'
import BaseModal from '@/components/BaseModal.vue'
import SkeletonBlock from '@/components/SkeletonBlock.vue'
import { API_BASE_URL } from '@/config'
import { useAsync } from '@/composables/useAsync'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const { data: keys, loading, run } = useAsync(() => api.get<ApiKey[]>('/admin/api-keys/mine'), [])
onMounted(run)

const active = computed(() => keys.value.find((k) => k.status === 'active') ?? null)
const history = computed(() => keys.value.filter((k) => k.status !== 'active'))

const SCOPES: { id: ApiScope; label: string; hint: string }[] = [
  {
    id: 'quizzes:write',
    label: 'Quizzes',
    hint: 'Create, edit, import and publish quizzes; upload quiz images.',
  },
  {
    id: 'reviews:write',
    label: 'Interactive reviews',
    hint: 'Upload, replace and delete interactive review HTML.',
  },
]
const EXPIRY = [
  { value: 30, label: '30 days' },
  { value: 90, label: '90 days' },
  { value: 365, label: '1 year' },
  { value: 0, label: 'Never expires' },
]

const formOpen = ref(false)
const regenerating = ref(false)
const form = reactive({
  name: 'AI assistant',
  scopes: ['quizzes:write', 'reviews:write'] as ApiScope[],
  expiry: 90,
})
const saving = ref(false)
const revokingKey = ref(false)
const created = ref<string | null>(null)

function openForm(regen: boolean) {
  regenerating.value = regen
  if (regen && active.value) {
    form.name = active.value.name
    form.scopes = [...active.value.scopes]
  }
  formOpen.value = true
}

async function submit() {
  if (saving.value) return
  if (!form.scopes.length) {
    toast.warning('Pick at least one permission for the key.', 'No permissions selected')
    return
  }
  saving.value = true
  try {
    const res = await api.post<CreatedApiKey>('/admin/api-keys', {
      name: form.name,
      scopes: form.scopes,
      expires_in_days: form.expiry || null,
      replace_existing: regenerating.value,
    })
    created.value = res.key
    formOpen.value = false
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    saving.value = false
  }
}

const revoking = ref(false)
async function revoke() {
  if (!active.value || revokingKey.value) return
  revokingKey.value = true
  try {
    await api.delete(`/admin/api-keys/${active.value.id}`)
    toast.success('API key revoked. It no longer works.')
    revoking.value = false
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  } finally {
    revokingKey.value = false
  }
}

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    toast.success('Copied to clipboard')
  } catch {
    toast.warning('Select the key and copy it manually.', 'Couldn’t copy')
  }
}

const fmt = (d: string | null) => (d ? new Date(d.replace(' ', 'T')).toLocaleString() : 'Never')
const example = computed(
  () => `# Check the key
curl -H "Authorization: Bearer ${created.value ?? 'gic_…'}" ${API_BASE_URL}/auth/me

# Create a whole quiz from JSON (see docs/AI-HOW-ADD-QUIZ.md)
curl -H "Authorization: Bearer ${created.value ?? 'gic_…'}" \\
     -H "Content-Type: application/json" \\
     --data @quiz.json ${API_BASE_URL}/admin/quizzes/import`,
)
</script>

<template>
  <div class="space-y-5">
    <h1 class="animate__animated animate__fadeInDown text-2xl font-bold text-ctp-text">API key</h1>
    <v-alert type="info" icon="mdi-robot-outline" class="max-w-3xl text-sm">
      Use an API key to let a script or an AI assistant add quizzes and interactive reviews for you.
      It acts as you, with only the permissions you choose. You can have
      <strong class="text-ctp-text">one active key</strong>; regenerate it if it leaks. Your admin
      can see and revoke it.
    </v-alert>

    <v-card
      v-if="created"
      tag="section"
      class="animate__animated animate__bounceIn space-y-3 border-ctp-green p-5"
      aria-live="polite"
    >
      <h2 class="flex items-center gap-2 font-semibold text-ctp-green">
        <v-icon icon="mdi-key-variant" /> Your new API key
      </h2>
      <v-alert type="warning" density="compact" icon="mdi-eye-off-outline">
        Copy it now. <strong>It will not be shown again.</strong> Store it like a password.
      </v-alert>
      <div class="flex gap-2">
        <code
          class="flex-1 overflow-x-auto rounded-lg border border-ctp-surface1 bg-ctp-surface0 px-3 py-2.5 font-mono text-xs whitespace-nowrap select-all"
          >{{ created }}</code
        >
        <v-btn color="primary" prepend-icon="mdi-content-copy" @click="copy(created!)">Copy </v-btn>
      </div>
      <pre class="overflow-x-auto rounded-lg bg-ctp-crust p-3 text-xs text-ctp-subtext1">{{
        example
      }}</pre>
      <v-btn variant="text" size="small" prepend-icon="mdi-check" @click="created = null">
        I've saved it, hide the key
      </v-btn>
    </v-card>

    <SkeletonBlock v-if="loading" class="h-40" />
    <v-card v-else-if="active" tag="section" class="space-y-4 p-5" data-aos="fade-up">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p class="font-semibold text-ctp-text">{{ active.name }}</p>
          <p class="font-mono text-sm text-ctp-subtext0">{{ active.key_prefix }}…</p>
        </div>
        <v-chip
          size="small"
          color="success"
          variant="tonal"
          prepend-icon="mdi-check-circle-outline"
        >
          Active
        </v-chip>
      </div>
      <div class="flex flex-wrap gap-2">
        <v-chip
          v-for="s in active.scopes"
          :key="s"
          size="small"
          color="primary"
          variant="tonal"
          prepend-icon="mdi-shield-key-outline"
        >
          {{ s }}
        </v-chip>
      </div>
      <dl class="grid gap-2 text-sm sm:grid-cols-3">
        <div>
          <dt class="text-ctp-subtext0">Created</dt>
          <dd class="text-ctp-text">{{ fmt(active.created_at) }}</dd>
        </div>
        <div>
          <dt class="text-ctp-subtext0">Last used</dt>
          <dd class="text-ctp-text">{{ fmt(active.last_used_at) }}</dd>
        </div>
        <div>
          <dt class="text-ctp-subtext0">Expires</dt>
          <dd class="text-ctp-text">{{ fmt(active.expires_at) }}</dd>
        </div>
      </dl>
      <div class="flex flex-wrap gap-2">
        <v-btn variant="tonal" prepend-icon="mdi-refresh" @click="openForm(true)"
          >Regenerate
        </v-btn>
        <v-btn
          variant="text"
          prepend-icon="mdi-delete-outline"
          color="error"
          @click="revoking = true"
          >Revoke
        </v-btn>
      </div>
    </v-card>
    <v-card v-else tag="section" class="flex flex-col items-start gap-3 p-5" data-aos="fade-up">
      <p class="flex items-center gap-2 text-sm text-ctp-subtext1">
        <v-icon icon="mdi-key-remove" /> You don't have an active API key.
      </p>
      <v-btn color="primary" prepend-icon="mdi-key-variant" @click="openForm(false)"
        >Create API key
      </v-btn>
    </v-card>

    <section v-if="history.length" class="space-y-2">
      <h2 class="text-sm font-semibold text-ctp-subtext1">Previous keys</h2>
      <ul class="space-y-1 text-sm">
        <li
          v-for="k in history"
          :key="k.id"
          class="flex flex-wrap gap-x-3 rounded-lg border border-ctp-surface0 bg-ctp-mantle px-3 py-2 text-ctp-subtext0"
          data-aos="fade-up"
        >
          <span class="font-mono">{{ k.key_prefix }}…</span>
          <span>{{ k.name }}</span>
          <span class="ml-auto capitalize"
            >{{ k.status }} · {{ fmt(k.revoked_at ?? k.expires_at) }}</span
          >
        </li>
      </ul>
    </section>

    <BaseModal
      :open="formOpen"
      :title="regenerating ? 'Regenerate API key' : 'Create API key'"
      :description="regenerating ? 'Your current key stops working immediately.' : undefined"
      @close="formOpen = false"
    >
      <v-form id="api-key-form" class="space-y-4" @submit.prevent="submit">
        <v-text-field
          v-model="form.name"
          label="Name"
          required
          maxlength="100"
          placeholder="e.g. Claude assistant"
        />
        <fieldset class="space-y-2">
          <legend class="mb-1 text-sm font-medium text-ctp-subtext1">Permissions</legend>
          <v-card
            v-for="s in SCOPES"
            :key="s.id"
            class="px-2 py-1 transition-colors"
            :class="form.scopes.includes(s.id) ? 'border-ctp-accent/60' : ''"
          >
            <v-checkbox v-model="form.scopes" :value="s.id" density="compact">
              <template #label>
                <span class="py-1">
                  <span class="block text-sm font-medium text-ctp-text"
                    >{{ s.label }} <code class="text-xs text-ctp-subtext0">{{ s.id }}</code></span
                  >
                  <span class="block text-xs text-ctp-subtext0">{{ s.hint }}</span>
                </span>
              </template>
            </v-checkbox>
          </v-card>
        </fieldset>
        <v-select
          v-model="form.expiry"
          :items="EXPIRY"
          item-title="label"
          item-value="value"
          label="Expires"
          prepend-inner-icon="mdi-calendar-clock"
        />
      </v-form>
      <template #footer>
        <v-btn variant="text" @click="formOpen = false">Cancel</v-btn>
        <v-btn
          color="primary"
          type="submit"
          form="api-key-form"
          :prepend-icon="regenerating ? 'mdi-refresh' : 'mdi-key-plus'"
          :loading="saving"
        >
          {{ regenerating ? 'Regenerate key' : 'Create key' }}
        </v-btn>
      </template>
    </BaseModal>

    <BaseModal :open="revoking" title="Revoke API key?" size="sm" @close="revoking = false">
      <v-alert type="warning" icon="mdi-key-alert-outline">
        Anything using this key (scripts, AI assistants) will stop working immediately.
      </v-alert>
      <template #footer>
        <v-btn variant="text" @click="revoking = false">Cancel</v-btn>
        <v-btn color="error" prepend-icon="mdi-key-remove" :loading="revokingKey" @click="revoke"
          >Revoke</v-btn
        >
      </template>
    </BaseModal>
  </div>
</template>
