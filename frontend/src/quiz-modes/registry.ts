/**
 * Quiz mode registry. The mode-select modal and the play page both read from here.
 *
 * To add a mode (see docs/AI-HOW-ADD-QUIZ.md → "Adding a new quiz mode"):
 *   1. backend: add a `QuizMode` variant + availability rule in backend/src/quiz_modes.rs
 *   2. frontend: add the id to `QuizModeId` in src/api/types.ts
 *   3. create the player component in src/quiz-modes/
 *   4. add one entry to `QUIZ_MODES` below
 */
import { defineAsyncComponent, type Component } from 'vue'

import type { QuizModeId, QuizSummary } from '@/api/types'

export interface QuizModeDefinition {
  id: QuizModeId
  label: string
  description: string
  /** Icon name from components/AppIcon.vue */
  icon: string
  /** Player component; receives `quiz: QuizDetail` as a prop. */
  component: Component
  /** The backend computes availability; the frontend never guesses. */
  isAvailable(quiz: Pick<QuizSummary, 'available_modes'>): boolean
}

const fromBackend = (id: QuizModeId) => (quiz: Pick<QuizSummary, 'available_modes'>) =>
  quiz.available_modes.includes(id)

export const QUIZ_MODES: QuizModeDefinition[] = [
  {
    id: 'qcm',
    label: 'QCM',
    description: 'Multiple choice: pick the right answer(s) and see the explanation.',
    icon: 'list-check',
    component: defineAsyncComponent(() => import('./QcmPlayer.vue')),
    isAvailable: fromBackend('qcm'),
  },
  {
    id: 'flashcard',
    label: 'Flashcard',
    description: 'Flip cards, mark what you knew, and review the rest.',
    icon: 'cards',
    component: defineAsyncComponent(() => import('./FlashcardPlayer.vue')),
    isAvailable: fromBackend('flashcard'),
  },
]

export function getQuizMode(id: string | null | undefined): QuizModeDefinition | undefined {
  return QUIZ_MODES.find((m) => m.id === id)
}

export function availableModes(quiz: Pick<QuizSummary, 'available_modes'>): QuizModeDefinition[] {
  return QUIZ_MODES.filter((m) => m.isAvailable(quiz))
}
