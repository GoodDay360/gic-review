<script setup lang="ts">
import { useRouter } from 'vue-router'

import type { Subject } from '@/api/types'
import { subjectPagePath, type SubjectPage } from '@/utils/subjectPaths'

import BaseModal from './BaseModal.vue'

const props = defineProps<{ subject: Subject | null }>()
const emit = defineEmits<{ close: [] }>()
const router = useRouter()

function go(page: SubjectPage) {
  if (!props.subject) return
  emit('close')
  router.push(subjectPagePath(props.subject.category_slug, props.subject.slug, page))
}
</script>

<template>
  <BaseModal
    :open="!!props.subject"
    :title="props.subject?.name ?? ''"
    description="What would you like to do?"
    @close="emit('close')"
  >
    <div v-if="props.subject" class="grid gap-3 sm:grid-cols-2">
      <v-card
        tag="button"
        type="button"
        hover
        class="w-full card-lift flex flex-col items-start gap-2 p-5 text-left"
        :disabled="!props.subject.has_review"
        @click="go('review')"
      >
        <v-avatar color="primary" variant="tonal" rounded="lg">
          <v-icon icon="mdi-book-open-page-variant-outline" />
        </v-avatar>
        <span class="font-semibold text-ctp-text">Interactive Review</span>
        <span class="text-sm text-ctp-subtext0">
          {{ props.subject.has_review ? 'Read the interactive study guide.' : 'Coming soon' }}
        </span>
      </v-card>
      <v-card
        tag="button"
        type="button"
        hover
        class="w-full card-lift flex flex-col items-start gap-2 p-5 text-left"
        @click="go('quizzes')"
      >
        <v-avatar color="primary" variant="tonal" rounded="lg">
          <v-icon icon="mdi-format-list-checks" />
        </v-avatar>
        <span class="font-semibold text-ctp-text">Quiz</span>
        <span class="text-sm text-ctp-subtext0">Practice with QCM or flashcards.</span>
      </v-card>
    </div>
  </BaseModal>
</template>
