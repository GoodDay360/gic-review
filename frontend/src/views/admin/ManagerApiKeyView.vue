<script setup lang="ts">
/**
 * A manager's personal API key (max one active). Scripts and AI assistants send it as
 * `Authorization: Bearer gic_…` to add quizzes and interactive reviews.
 */
import { computed, onMounted, reactive, ref } from 'vue'

import { api, errorMessage } from '@/api/client'
import type { ApiKey, ApiScope, CreatedApiKey } from '@/api/types'
import AppIcon from '@/components/AppIcon.vue'
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
  if (!form.scopes.length) {
    toast.error('Choose at least one permission')
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
  if (!active.value) return
  try {
    await api.delete(`/admin/api-keys/${active.value.id}`)
    toast.success('API key revoked. It no longer works.')
    revoking.value = false
    run()
  } catch (err) {
    toast.error(errorMessage(err))
  }
}

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    toast.success('Copied to clipboard')
  } catch {
    toast.error('Copy failed; select the text and copy it manually')
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
    <h1 class="text-2xl font-bold text-ctp-text">API key</h1>
    <p class="max-w-2xl text-sm text-ctp-subtext0">
      Use an API key to let a script or an AI assistant add quizzes and interactive reviews for you.
      It acts as you, with only the permissions you choose. You can have
      <strong class="text-ctp-text">one active key</strong>; regenerate it if it leaks. Your admin
      can see and revoke it.
    </p>

    <section v-if="created" class="card space-y-3 border-ctp-green p-5" aria-live="polite">
      <h2 class="flex items-center gap-2 font-semibold text-ctp-green">
        <AppIcon name="key" /> Your new API key
      </h2>
      <p class="text-sm text-ctp-subtext1">
        Copy it now. <strong class="text-ctp-text">It will not be shown again.</strong> Store it
        like a password.
      </p>
      <div class="flex gap-2">
        <code class="input flex-1 overflow-x-auto font-mono text-xs whitespace-nowrap select-all">{{
          created
        }}</code>
        <button class="btn-primary" @click="copy(created!)">
          <AppIcon name="copy" :size="16" /> Copy
        </button>
      </div>
      <pre class="overflow-x-auto rounded-lg bg-ctp-crust p-3 text-xs text-ctp-subtext1">{{
        example
      }}</pre>
      <button class="btn-ghost btn-sm" @click="created = null">I've saved it, hide the key</button>
    </section>

    <SkeletonBlock v-if="loading" class="h-40" />
    <section v-else-if="active" class="card space-y-4 p-5">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p class="font-semibold text-ctp-text">{{ active.name }}</p>
          <p class="font-mono text-sm text-ctp-subtext0">{{ active.key_prefix }}…</p>
        </div>
        <span class="rounded-full bg-ctp-green/15 px-2.5 py-1 text-xs font-medium text-ctp-green"
          >Active</span
        >
      </div>
      <div class="flex flex-wrap gap-2">
        <span
          v-for="s in active.scopes"
          :key="s"
          class="rounded-full bg-ctp-accent/15 px-2.5 py-1 text-xs font-medium text-ctp-accent"
        >
          {{ s }}
        </span>
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
        <button class="btn-secondary" @click="openForm(true)">
          <AppIcon name="refresh" :size="16" /> Regenerate
        </button>
        <button class="btn-ghost text-ctp-red" @click="revoking = true">
          <AppIcon name="trash" :size="16" /> Revoke
        </button>
      </div>
    </section>
    <section v-else class="card flex flex-col items-start gap-3 p-5">
      <p class="text-sm text-ctp-subtext1">You don't have an active API key.</p>
      <button class="btn-primary" @click="openForm(false)">
        <AppIcon name="key" :size="16" /> Create API key
      </button>
    </section>

    <section v-if="history.length" class="space-y-2">
      <h2 class="text-sm font-semibold text-ctp-subtext1">Previous keys</h2>
      <ul class="space-y-1 text-sm">
        <li
          v-for="k in history"
          :key="k.id"
          class="flex flex-wrap gap-x-3 rounded-lg bg-ctp-mantle px-3 py-2 text-ctp-subtext0"
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
      <form id="api-key-form" class="space-y-4" @submit.prevent="submit">
        <div>
          <label class="label" for="ak-name">Name</label>
          <input
            id="ak-name"
            v-model="form.name"
            class="input"
            required
            maxlength="100"
            placeholder="e.g. Claude assistant"
          />
        </div>
        <fieldset class="space-y-2">
          <legend class="label">Permissions</legend>
          <label
            v-for="s in SCOPES"
            :key="s.id"
            class="flex cursor-pointer items-start gap-3 rounded-lg border border-ctp-surface1 p-3"
          >
            <input
              v-model="form.scopes"
              type="checkbox"
              :value="s.id"
              class="mt-1 h-4 w-4 accent-ctp-accent"
            />
            <span>
              <span class="block text-sm font-medium text-ctp-text"
                >{{ s.label }} <code class="text-xs text-ctp-subtext0">{{ s.id }}</code></span
              >
              <span class="block text-xs text-ctp-subtext0">{{ s.hint }}</span>
            </span>
          </label>
        </fieldset>
        <div>
          <label class="label" for="ak-exp">Expires</label>
          <select id="ak-exp" v-model.number="form.expiry" class="input">
            <option v-for="e in EXPIRY" :key="e.value" :value="e.value">{{ e.label }}</option>
          </select>
        </div>
      </form>
      <template #footer>
        <button class="btn-ghost" @click="formOpen = false">Cancel</button>
        <button class="btn-primary" type="submit" form="api-key-form" :disabled="saving">
          {{ saving ? 'Creating…' : regenerating ? 'Regenerate key' : 'Create key' }}
        </button>
      </template>
    </BaseModal>

    <BaseModal :open="revoking" title="Revoke API key?" size="sm" @close="revoking = false">
      <p class="text-sm text-ctp-subtext1">
        Anything using this key (scripts, AI assistants) will stop working immediately.
      </p>
      <template #footer>
        <button class="btn-ghost" @click="revoking = false">Cancel</button>
        <button class="btn-danger" @click="revoke">Revoke</button>
      </template>
    </BaseModal>
  </div>
</template>
