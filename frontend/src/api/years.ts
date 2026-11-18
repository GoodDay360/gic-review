import { api } from './client'
import type { Subject, Year } from './types'

/** A year as returned by `GET /admin/years` (owner info + its subjects). */
export interface ManagedYear extends Year {
  owner_id: number | null
  owner_name: string | null
  /** You own this year (or are an admin): you can edit it and add content to its subjects. */
  can_manage: boolean
  subjects: Subject[]
}

export function loadManagedYears(): Promise<ManagedYear[]> {
  return api.get<ManagedYear[]>('/admin/years')
}

/** Subjects you may add new quizzes/reviews to: those in years you own (admins: all). */
export function addableSubjects(years: ManagedYear[]): Subject[] {
  return years.filter((y) => y.can_manage).flatMap((y) => y.subjects)
}

/** For an edit form: the addable subjects plus the item's current subject (if not already there). */
export function subjectsForEdit(years: ManagedYear[], currentSubjectId: number | null): Subject[] {
  const addable = addableSubjects(years)
  if (currentSubjectId == null || addable.some((s) => s.id === currentSubjectId)) return addable
  const current = years.flatMap((y) => y.subjects).find((s) => s.id === currentSubjectId)
  return current ? [current, ...addable] : addable
}
