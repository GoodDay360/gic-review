import { api } from './client'
import type { Subject, Category } from './types'

/** A subject as returned inside `GET /admin/categories`. */
export interface ManagedSubject extends Subject {
  /** You may add quizzes/reviews here (your category, shared category/subject, or admin). */
  can_add_content: boolean
  /** This subject was shared with you on its own. */
  shared_with_you: boolean
}

/** A category as returned by `GET /admin/categories` (owner info, sharing + its subjects). */
export interface ManagedCategory extends Category {
  owner_id: number | null
  owner_name: string | null
  /** You own this category (or are an admin): edit it, manage its subjects and who it's shared with. */
  can_manage: boolean
  /** The whole category was shared with you. */
  shared_with_you: boolean
  /** You may add content to at least one of its subjects. */
  can_add_content: boolean
  subjects: ManagedSubject[]
}

export function loadManagedCategories(): Promise<ManagedCategory[]> {
  return api.get<ManagedCategory[]>('/admin/categories')
}

/** Subjects you may add new quizzes/reviews to (own categories, shared categories/subjects; admins: all). */
export function addableSubjects(categories: ManagedCategory[]): Subject[] {
  return categories.flatMap((y) => y.subjects).filter((s) => s.can_add_content)
}

/** For an edit form: the addable subjects plus the item's current subject (if not already there). */
export function subjectsForEdit(
  categories: ManagedCategory[],
  currentSubjectId: number | null,
): Subject[] {
  const addable = addableSubjects(categories)
  if (currentSubjectId == null || addable.some((s) => s.id === currentSubjectId)) return addable
  const current = categories.flatMap((y) => y.subjects).find((s) => s.id === currentSubjectId)
  return current ? [current, ...addable] : addable
}
