// Types mirroring the backend API (backend/src/models.rs, backend/src/auth.rs).

export type QuizModeId = 'qcm' | 'flashcard'
export type Role = 'admin' | 'manager'

export interface Subject {
  id: number
  slug: string
  name: string
  description: string | null
  color: string
  sort_order: number
  has_review: boolean
  quiz_count: number
  created_at: string
  updated_at: string
}

export interface Review {
  id: number
  subject_id: number
  title: string
  slug: string
  r2_key: string
  url: string
  sort_order: number
  is_published: boolean
  created_at: string
  updated_at: string
}

export interface QuizSummary {
  id: number
  subject_id: number
  title: string
  description: string | null
  cover_image_key: string | null
  cover_image_url: string | null
  is_published: boolean
  question_count: number
  available_modes: QuizModeId[]
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
  name: string
  slug: string
  description: string | null
  color: string
  sort_order: number
}
