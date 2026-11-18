import { api } from './client'
import type { Question, QuestionInput, QuizDetail } from './types'

/** Remove an uploaded image that was never saved on a record (best effort). */
export async function discardUpload(key: string) {
  try {
    await api.delete(`/admin/uploads/image/${key.split('/').map(encodeURIComponent).join('/')}`)
  } catch {
    /* the admin "clean up orphaned images" action catches anything left over */
  }
}

export function saveQuestion(quizId: number, questionId: number | null, input: QuestionInput) {
  return questionId
    ? api.put<QuizDetail>(`/admin/quizzes/${quizId}/questions/${questionId}`, input)
    : api.post<QuizDetail>(`/admin/quizzes/${quizId}/questions`, input)
}

/** Every image key used by a question (to find uploads that ended up unused). */
export function questionKeys(
  q: Pick<Question, 'image_key' | 'answer_image_key' | 'explanation_image_key'> & {
    options: { image_key: string | null }[]
  },
) {
  return [
    q.image_key,
    q.answer_image_key,
    q.explanation_image_key,
    ...q.options.map((o) => o.image_key),
  ].filter((k): k is string => !!k)
}
