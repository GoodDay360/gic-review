import { api } from './client'
import type { Subject, Year } from './types'

/** A subject as returned inside `GET /admin/years`. */
export interface ManagedSubject extends Subject {
  /** You may add quizzes/reviews here (your year, shared year/subject, or admin). */
  can_add_content: boolean
  /** This subject was shared with you on its own. */
  shared_with_you: boolean
}

/** A year as returned by `GET /admin/years` (owner info, sharing + its subjects). */
export interface ManagedYear extends Year {
  owner_id: number | null
  owner_name: string | null
  /** You own this year (or are an admin): edit it, manage its subjects and who it's shared with. */
  can_manage: boolean
  /** The whole year was shared with you. */
  shared_with_you: boolean
  /** You may add content to at least one of its subjects. */
  can_add_content: boolean
  subjects: ManagedSubject[]
}

export function loadManagedYears(): Promise<ManagedYear[]> {
  return api.get<ManagedYear[]>('/admin/years')
}

/** Subjects you may add new quizzes/reviews to (own years, shared years/subjects; admins: all). */
export function addableSubjects(years: ManagedYear[]): Subject[] {
  return years.flatMap((y) => y.subjects).filter((s) => s.can_add_content)
}

/** For an edit form: the addable subjects plus the item's current subject (if not already there). */
export function subjectsForEdit(years: ManagedYear[], currentSubjectId: number | null): Subject[] {
  const addable = addableSubjects(years)
  if (currentSubjectId == null || addable.some((s) => s.id === currentSubjectId)) return addable
  const current = years.flatMap((y) => y.subjects).find((s) => s.id === currentSubjectId)
  return current ? [current, ...addable] : addable
}
