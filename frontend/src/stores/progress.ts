import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

import type { QuizModeId } from '@/api/types'

/** Anonymous study progress, kept only in this browser (localStorage). */
export interface ModeResult {
  lastScore: number
  bestScore: number
  total: number
  attempts: number
  lastPlayedAt: string
  missedQuestionIds: number[]
}

interface ProgressState {
  results: Record<string, Partial<Record<QuizModeId, ModeResult>>>
  shuffleQuestions: boolean
}

const KEY = 'gic-progress-v1'

function load(): ProgressState {
  try {
    const raw = localStorage.getItem(KEY)
    if (raw) return { results: {}, shuffleQuestions: false, ...JSON.parse(raw) }
  } catch {
    /* ignore */
  }
  return { results: {}, shuffleQuestions: false }
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

  function record(
    quizId: number,
    mode: QuizModeId,
    score: number,
    total: number,
    missed: number[],
  ) {
    const key = String(quizId)
    const prev = state.value.results[key]?.[mode]
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

  return { state, resultFor, record }
})
