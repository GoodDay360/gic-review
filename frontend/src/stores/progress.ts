import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

import type { QuizModeId } from '@/api/types'
import { MAX_SAVED_RESULTS } from '@/config'

/** Anonymous study progress, kept only in this browser (localStorage). */
export interface ModeResult {
  lastScore: number
  bestScore: number
  total: number
  attempts: number
  lastPlayedAt: string
  missedQuestionIds: number[]
}

interface QuizProgressBase {
  quizId: number
  quizTitle: string
}

export interface QcmProgress extends QuizProgressBase {
  mode: 'qcm'
  questionIds: number[]
  optionIds: Record<string, number[]>
  index: number
  selectedOptionIds: number[]
  submitted: boolean
  results: Record<string, boolean>
}

export interface FlashcardProgress extends QuizProgressBase {
  mode: 'flashcard'
  queue: number[]
  index: number
  flipped: boolean
  deckSize: number
  missedIds: number[]
  knownIds: number[]
}

export type ActiveQuizProgress = QcmProgress | FlashcardProgress

interface ProgressState {
  results: Record<string, Partial<Record<QuizModeId, ModeResult>>>
  shuffleQuestions: boolean
  activeQuiz: ActiveQuizProgress | null
}

const KEY = 'gic-progress-v1'

function pruneOldestResults(results: ProgressState['results'], maxResults: number): boolean {
  let count = 0
  let oldest: { quizId: string; mode: QuizModeId; playedAt: number } | undefined
  for (const [quizId, modes] of Object.entries(results)) {
    for (const mode of ['qcm', 'flashcard'] as const) {
      const result = modes?.[mode]
      if (result) {
        count++
        const parsedDate = Date.parse(result.lastPlayedAt)
        const entry = {
          quizId,
          mode,
          playedAt: Number.isFinite(parsedDate) ? parsedDate : Number.NEGATIVE_INFINITY,
        }
        if (!oldest || entry.playedAt < oldest.playedAt) oldest = entry
      }
    }
  }

  const removeCount = count - maxResults
  if (removeCount <= 0) return false

  const oldestEntries =
    removeCount === 1
      ? [oldest!]
      : Object.entries(results)
          .flatMap(([quizId, modes]) =>
            (['qcm', 'flashcard'] as const).flatMap((mode) => {
              const result = modes?.[mode]
              if (!result) return []
              const parsedDate = Date.parse(result.lastPlayedAt)
              return [
                {
                  quizId,
                  mode,
                  playedAt: Number.isFinite(parsedDate) ? parsedDate : Number.NEGATIVE_INFINITY,
                },
              ]
            }),
          )
          .sort((a, b) => a.playedAt - b.playedAt)
          .slice(0, removeCount)

  for (const { quizId, mode } of oldestEntries) {
    const remaining = { ...results[quizId] }
    delete remaining[mode]
    if (Object.keys(remaining).length) results[quizId] = remaining
    else delete results[quizId]
  }
  return true
}

function isNumberArray(value: unknown): value is number[] {
  return Array.isArray(value) && value.every((item) => Number.isSafeInteger(item))
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isActiveQuizProgress(value: unknown): value is ActiveQuizProgress {
  if (
    !isRecord(value) ||
    !Number.isSafeInteger(value.quizId) ||
    typeof value.quizTitle !== 'string' ||
    !Number.isSafeInteger(value.index) ||
    (value.index as number) < 0
  ) {
    return false
  }

  if (value.mode === 'qcm') {
    return (
      isNumberArray(value.questionIds) &&
      isNumberArray(value.selectedOptionIds) &&
      typeof value.submitted === 'boolean' &&
      isRecord(value.optionIds) &&
      Object.values(value.optionIds).every(isNumberArray) &&
      isRecord(value.results) &&
      Object.values(value.results).every((result) => typeof result === 'boolean')
    )
  }

  return (
    value.mode === 'flashcard' &&
    isNumberArray(value.queue) &&
    typeof value.flipped === 'boolean' &&
    Number.isSafeInteger(value.deckSize) &&
    (value.deckSize as number) >= 0 &&
    isNumberArray(value.missedIds) &&
    isNumberArray(value.knownIds)
  )
}

function load(): ProgressState {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw) {
      const saved: unknown = JSON.parse(raw)
      if (isRecord(saved)) {
        const results = isRecord(saved.results) ? (saved.results as ProgressState['results']) : {}
        const wasPruned = pruneOldestResults(results, MAX_SAVED_RESULTS)
        const progress = {
          results,
          shuffleQuestions:
            typeof saved.shuffleQuestions === 'boolean' ? saved.shuffleQuestions : false,
          activeQuiz: isActiveQuizProgress(saved.activeQuiz) ? saved.activeQuiz : null,
        }
        if (wasPruned) {
          try {
            localStorage.setItem(KEY, JSON.stringify(progress))
          } catch {
            /* ignore */
          }
        }
        return progress
      }
    }
  } catch {
    /* ignore */
  }
  return { activeQuiz: null, results: {}, shuffleQuestions: false }
}

export const useProgressStore = defineStore('progress', () => {
  const state = ref<ProgressState>(load())

  watch(
    state,
    (s) => {
      try {
        localStorage.setItem(KEY, JSON.stringify(s))
      } catch {
        /* ignore */
      }
    },
    { deep: true },
  )

  function resultFor(quizId: number, mode: QuizModeId): ModeResult | undefined {
    return state.value.results[String(quizId)]?.[mode]
  }

  function activeQuizFor(quizId: number): ActiveQuizProgress | null {
    return state.value.activeQuiz?.quizId === quizId ? state.value.activeQuiz : null
  }

  function saveActiveQuiz(progress: ActiveQuizProgress) {
    state.value.activeQuiz = progress
  }

  function clearActiveQuiz(quizId?: number) {
    if (quizId === undefined || state.value.activeQuiz?.quizId === quizId) {
      state.value.activeQuiz = null
    }
  }

  function record(
    quizId: number,
    mode: QuizModeId,
    score: number,
    total: number,
    missed: number[],
  ) {
    const key = String(quizId)
    const prev = state.value.results[key]?.[mode]
    if (!prev) pruneOldestResults(state.value.results, MAX_SAVED_RESULTS - 1)
    state.value.results[key] = {
      ...state.value.results[key],
      [mode]: {
        lastScore: score,
        bestScore: Math.max(score, prev?.bestScore ?? 0),
        total,
        attempts: (prev?.attempts ?? 0) + 1,
        lastPlayedAt: new Date().toISOString(),
        missedQuestionIds: missed,
      },
    }
  }

  return { state, resultFor, activeQuizFor, saveActiveQuiz, clearActiveQuiz, record }
})
