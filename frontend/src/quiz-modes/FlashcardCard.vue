<script setup lang="ts">
/** Presentational flashcard (used by the player and by the admin preview). */
import { computed } from 'vue'

import type { Question } from '@/api/types'
import ZoomableImage from '@/components/ZoomableImage.vue'

import { correctOptions } from './utils'

const props = defineProps<{ question: Question; flipped: boolean }>()
const emit = defineEmits<{ flip: [] }>()

// Without an explicit answer, the back shows the correct option(s).
const fallbackOptions = computed(() =>
  !props.question.answer_text && !props.question.answer_image_url
    ? correctOptions(props.question)
    : [],
)
</script>

<template>
  <div class="[perspective:1600px]">
    <div
      role="button"
      tabindex="0"
      :aria-label="
        flipped ? 'Card back. Press to show the front.' : 'Card front. Press to reveal the answer.'
      "
      :aria-pressed="flipped"
      class="relative min-h-[22rem] w-full cursor-pointer transition-transform duration-500 [transform-style:preserve-3d] sm:min-h-[26rem]"
      :class="flipped ? '[transform:rotateY(180deg)]' : ''"
      @click="emit('flip')"
      @keydown.enter.prevent="emit('flip')"
    >
      <!-- Front -->
      <section
        class="card absolute inset-0 flex flex-col items-center justify-center gap-4 overflow-auto p-6 [backface-visibility:hidden]"
        :aria-hidden="flipped"
      >
        <span
          class="absolute top-3 left-4 text-xs font-semibold tracking-wide text-ctp-overlay1 uppercase"
          >Question</span
        >
        <p
          v-if="question.prompt"
          class="text-center text-xl leading-relaxed font-medium whitespace-pre-line text-ctp-text"
        >
          {{ question.prompt }}
        </p>
        <div v-if="question.image_url" class="w-full">
          <ZoomableImage
            :src="question.image_url"
            :alt="question.image_alt"
            img-class="mx-auto max-h-[18rem] w-full object-contain"
          />
        </div>
        <span class="absolute bottom-3 text-xs text-ctp-overlay1"
          >Click or press Space to flip</span
        >
      </section>

      <!-- Back -->
      <section
        class="card absolute inset-0 flex flex-col items-center justify-center gap-4 overflow-auto border-ctp-accent/50 p-6 [backface-visibility:hidden] [transform:rotateY(180deg)]"
        :aria-hidden="!flipped"
      >
        <span
          class="absolute top-3 left-4 text-xs font-semibold tracking-wide text-ctp-accent uppercase"
          >Answer</span
        >
        <p
          v-if="question.answer_text"
          class="text-center text-xl leading-relaxed font-medium whitespace-pre-line text-ctp-text"
        >
          {{ question.answer_text }}
        </p>
        <div v-if="question.answer_image_url" class="w-full">
          <ZoomableImage
            :src="question.answer_image_url"
            :alt="question.answer_image_alt"
            img-class="mx-auto max-h-[16rem] w-full object-contain"
          />
        </div>
        <ul v-if="fallbackOptions.length" class="flex w-full flex-col items-center gap-3">
          <li
            v-for="o in fallbackOptions"
            :key="o.id"
            class="flex w-full flex-col items-center gap-2"
          >
            <span v-if="o.text" class="text-center text-xl font-medium text-ctp-text">{{
              o.text
            }}</span>
            <ZoomableImage
              v-if="o.image_url"
              :src="o.image_url"
              :alt="o.image_alt"
              img-class="mx-auto max-h-48 w-full object-contain"
            />
          </li>
        </ul>
        <div
          v-if="question.explanation || question.explanation_image_url"
          class="w-full border-t border-ctp-surface0 pt-3"
        >
          <p
            v-if="question.explanation"
            class="text-center text-sm whitespace-pre-line text-ctp-subtext1"
          >
            {{ question.explanation }}
          </p>
          <ZoomableImage
            v-if="question.explanation_image_url"
            :src="question.explanation_image_url"
            alt="Explanation diagram"
            img-class="mx-auto mt-2 max-h-40 w-full object-contain"
          />
        </div>
      </section>
    </div>
  </div>
</template>
