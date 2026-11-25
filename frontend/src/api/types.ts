// Types mirroring the backend API (backend/src/models.rs, backend/src/auth.rs).

export type QuizModeId = 'qcm' | 'flashcard'
export type Role = 'admin' | 'manager'

export interface Category {
  id: number
  slug: string
  /** Free text, e.g. "I4" or "Very Cool Category". */
  name: string
  description: string | null
  sort_order: number
  subject_count: number
  created_at: string
  updated_at: string
}

export interface CategoryWithSubjects extends Category {
  subjects: Subject[]
  subjects_has_more: boolean
}

export interface Page<T> {
  items: T[]
  has_more: boolean
}

export interface Subject {
  id: number
  category_id: number
  category_slug: string
  category_name: string
  slug: string
  name: string
  description: string | null
  color: string
  sort_order: number
  has_review: boolean
  review_count: number
  quiz_count: number
  created_at: string
  updated_at: string
}

/** Present on items returned by the manager API (`/admin/...`). */
export interface ManagedFields {
  /**
   * How you can reach this item: `owner` (you created it), `admin`, `category_owner` (it's in your
   * category) or `shared` (its category or subject is shared with you: edit, but not delete).
   */
  access?: 'owner' | 'admin' | 'category_owner' | 'shared'
  owner_id?: number | null
  owner_name?: string | null
}

export interface SharePerson {
  user_id: number
  email: string
  display_name: string
}

export interface Shares {
  owner: SharePerson | null
  shared_with: (SharePerson & { shared_at: string })[]
  /** Shared with every manager (on top of `shared_with`). */
  shared_with_all: boolean
  /** For a subject: its whole category is shared with every manager. */
  category_shared_with_all: boolean
  you_are_owner: boolean
  /** Category owner or admin: may add/remove people. */
  can_manage: boolean
}

export interface Review extends ManagedFields {
  id: number
  subject_id: number
  title: string
  slug: string
  r2_key: string
  url: string
  sort_order: number
  is_published: boolean
  /** Creator's display name ("Created by …"); null if that account was deleted. */
  created_by_name: string | null
  created_at: string
  updated_at: string
}

export interface QuizSummary extends ManagedFields {
  id: number
  subject_id: number
  title: string
  description: string | null
  cover_image_key: string | null
  cover_image_url: string | null
  is_published: boolean
  question_count: number
  available_modes: QuizModeId[]
  /** Creator's display name ("Created by …"); null if that account was deleted. */
  created_by_name: string | null
  created_at: string
  updated_at: string
}

export interface QuestionOption {
  id: number
  position: number
  text: string | null
  image_key: string | null
  image_url: string | null
  image_alt: string | null
  is_correct: boolean
}

export interface Question {
  id: number
  position: number
  prompt: string | null
  image_key: string | null
  image_url: string | null
  image_alt: string | null
  answer_text: string | null
  answer_image_key: string | null
  answer_image_url: string | null
  answer_image_alt: string | null
  explanation: string | null
  explanation_image_key: string | null
  explanation_image_url: string | null
  multi_select: boolean
  options: QuestionOption[]
}

export interface QuizDetail extends QuizSummary {
  subject_slug: string
  subject_name: string
  /** Subject slugs are unique per category: links need both. */
  category_slug: string
  category_name: string
  questions: Question[]
}

export interface User {
  id: number
  email: string
  display_name: string
  role: Role
  is_active: boolean
  created_by: number | null
  created_at: string
  updated_at: string
}

export type ApiScope = 'quizzes:write' | 'reviews:write'

export interface ApiKey {
  id: number
  user_id: number
  owner_email: string
  owner_name: string
  name: string
  key_prefix: string
  scopes: ApiScope[]
  status: 'active' | 'revoked' | 'expired'
  expires_at: string | null
  last_used_at: string | null
  revoked_at: string | null
  created_at: string
}

export interface CreatedApiKey {
  /** Full key, shown only once. */
  key: string
  api_key: ApiKey
}

export interface UploadedImage {
  key: string
  url: string
}

// ----- Inputs -----

export interface QuizInput {
  subject_id: number
  title: string
  description: string | null
  cover_image_key: string | null
}

export interface OptionInput {
  text: string | null
  image_key: string | null
  image_alt: string | null
  is_correct: boolean
}

export interface QuestionInput {
  prompt: string | null
  image_key: string | null
  image_alt: string | null
  answer_text: string | null
  answer_image_key: string | null
  answer_image_alt: string | null
  explanation: string | null
  explanation_image_key: string | null
  multi_select: boolean
  options: OptionInput[]
}

export interface SubjectInput {
  category_id: number
  name: string
  slug: string
  description: string | null
  color: string
  sort_order: number
}
