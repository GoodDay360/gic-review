# Developer guide: adding a new quiz mode

Audience: developers (and coding assistants) working **inside this repository**. This is not
for API clients; those use `docs/AI-HOW-ADD-QUIZ.md`. Read `prompt.md` (build brief) and
`docs/AI-SPEC.md` (status) first, and follow the "Keep features in sync" rules: a new mode is
a feature, so add it to `prompt.md` and to the Features list in `AI-SPEC.md` in the same commit.

Quiz data is mode-agnostic: one quiz can be played in several modes. Adding a mode means
writing a new **player** and an **availability rule**; the stored data normally stays the same.

### Checklist

1. **Backend enum + rule:** `backend/src/quiz_modes.rs`
   - add a variant to `enum QuizMode` (it serializes in lowercase, e.g. `TrueFalse` → `"truefalse"`;
     add `#[serde(rename = "true_false")]` if you want a different id),
   - add it to `QuizMode::ALL` (order = display order),
   - write its rule in `QuizMode::is_available` and add a unit test in the same file.
2. **API:** nothing else to do: `available_modes` is computed from `QuizMode::ALL` for
   every quiz in `GET /api/subjects/:slug/quizzes` and `GET /api/quizzes/:id`.
3. **Frontend type:** add the id to `QuizModeId` in `frontend/src/api/types.ts`.
4. **Player component:** `frontend/src/quiz-modes/<Name>Player.vue`. It receives
   `quiz: QuizDetail` as a prop. Reuse `ResultScreen`, `ZoomableImage`,
   `preloadQuestionImages` and `useProgressStore().record(...)`.
5. **Registry entry:** add one object to `QUIZ_MODES` in
   `frontend/src/quiz-modes/registry.ts`. The mode modal and the play page pick it up
   automatically; do not edit `QuizModeModal.vue` or `QuizPlayView.vue`.
6. **Schema (only if needed):** add a **new** migration in `backend/migrations/`
   (never edit old ones), update `backend/src/models.rs`, the import format in
   `backend/src/routes/import.rs`, and this document.
7. **Docs:** add the mode as a requirement in `prompt.md`, as a Features line in
   `docs/AI-SPEC.md` (✅ + a Changelog entry), and list it under "Quiz modes" in
   `docs/AI-HOW-ADD-QUIZ.md` so API clients know when it becomes available.

### Worked example: True/False

A question is playable as True/False when it has **exactly two options** and exactly one
is correct (e.g. "True" / "False").

```rust
// backend/src/quiz_modes.rs
pub enum QuizMode {
    Qcm,
    Flashcard,
    #[serde(rename = "truefalse")]
    TrueFalse,
}

impl QuizMode {
    pub const ALL: &'static [QuizMode] = &[QuizMode::Qcm, QuizMode::Flashcard, QuizMode::TrueFalse];

    pub fn is_available(self, questions: &[QuestionDto]) -> bool {
        if questions.is_empty() {
            return false;
        }
        match self {
            // … existing arms …
            QuizMode::TrueFalse => questions.iter().all(|q| {
                q.options.len() == 2 && q.options.iter().filter(|o| o.is_correct).count() == 1
            }),
        }
    }
}
```

```ts
// frontend/src/api/types.ts
export type QuizModeId = 'qcm' | 'flashcard' | 'truefalse'

// frontend/src/quiz-modes/registry.ts  (add to QUIZ_MODES)
{
  id: 'truefalse',
  label: 'True / False',
  description: 'Decide quickly whether each statement is true or false.',
  icon: 'check',
  component: defineAsyncComponent(() => import('./TrueFalsePlayer.vue')),
  isAvailable: fromBackend('truefalse'),
},
```

`TrueFalsePlayer.vue` shows `question.prompt` (and `image_url`), two big buttons built
from `question.options`, then the explanation, and ends with `ResultScreen`. No migration
is needed because the data already fits the existing tables.
