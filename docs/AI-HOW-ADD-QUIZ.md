# How to add quizzes (and new quiz modes)

Audience: AI assistants and developers working on this repo. Read `prompt.md` and
`docs/AI-SPEC.md` first. If anything you change here affects the schema, the JSON import
format, image fields or quiz modes, update this file in the same commit.

---

## Part 1: Adding quiz content

There are three ways, in order of preference.

### Key rules (apply to every method)

| Rule | Details |
|---|---|
| A question needs **`prompt` or `image_key`** (or both) | Image-only questions are fine (e.g. "Which principle does this screenshot violate?"). |
| An option needs **`text` or `image_key`** (or both) | Image-only options are shown as a grid of image cards. |
| `is_correct` | Mark every correct option `true`. All others `false` (the default). |
| `multi_select` | `false` (default): exactly one correct option, shown as radio-style choices. `true`: several options may be correct; the student must select **all** of them and nothing else. The API rejects a question with more than one correct option while `multi_select` is `false`. |
| **QCM** is available when… | **every** question has ≥ 2 options (text or image) and ≥ 1 correct option. |
| **Flashcard** is available when… | **every** question has `answer_text`, or `answer_image_key`, or ≥ 1 correct option (then the back shows the correct option text/images). |
| To support **both** modes | Give every question ≥ 2 options with ≥ 1 correct. Optionally add `answer_text` / `answer_image_key` for a nicer card back. |
| New quizzes start **unpublished** | Publish from the admin panel or with `PATCH /api/admin/quizzes/:id/publish {"is_published": true}`. Publishing is refused while no mode is available. |
| Images | Only PNG, JPEG, WebP, GIF. Max `MAX_IMAGE_SIZE_MB` (backend `.env`). Real bytes are checked (magic numbers). PNG/JPEG/WebP are re-encoded, resized to max 1600 px wide and stripped of EXIF. GIFs are kept as-is. |
| Limits | Title ≤ 200 chars, ≤ 12 options per question, ≤ 500 questions per import, alt text ≤ 255 chars. |

The backend computes `available_modes` (see `backend/src/quiz_modes.rs`); never hard-code
mode availability in the frontend.

### (a) Admin panel (preferred)

1. Log in at `/login` (admin or manager).
2. **Admin → Quizzes → New quiz**, pick the subject, enter a title.
3. On the quiz page, set the description and the **cover image**.
4. **Add question** for each question. Every image field (question, each option,
   flashcard answer, explanation) uses the `ImageUploader`: click to browse, drag & drop,
   or click the drop zone and paste a screenshot with **Ctrl+V**.
5. Use **Preview** to see the question exactly as students will, in QCM and Flashcard mode.
6. Reorder questions with drag & drop or the ↑/↓ buttons.
7. Click **Publish**.

### (b) Bulk import JSON (`POST /api/admin/quizzes/import`)

Admin panel: **Admin → Bulk import** (paste, or upload a `.json` file; "Load HCI sample"
fills in `docs/examples/hci-sample-quiz.json`).

The import runs in **one database transaction**: either the whole quiz is created or
nothing is. Images downloaded from URLs during a failed import are deleted again.

#### Schema

```jsonc
{
  "subject_slug": "hci",              // required unless subject_id is given
  "subject_id": 1,                    // optional alternative to subject_slug
  "title": "string (1–200)",          // required
  "description": "string | null",     // optional, ≤ 5000
  "cover_image_key": "string | null", // EITHER an uploaded key …
  "cover_image_url": "string | null", // … OR a public http(s) URL to download (not both)
  "questions": [                      // required, 1–500 items, kept in this order
    {
      "prompt": "string | null",      // prompt and/or image required
      "image_key": "…", "image_url": "…", "image_alt": "string | null",
      "answer_text": "string | null", // flashcard back
      "answer_image_key": "…", "answer_image_url": "…", "answer_image_alt": "string | null",
      "explanation": "string | null", // shown after answering (QCM) and on the card back
      "explanation_image_key": "…", "explanation_image_url": "…",
      "multi_select": false,          // default false
      "options": [                    // default []
        {
          "text": "string | null",    // text and/or image required
          "image_key": "…", "image_url": "…", "image_alt": "string | null",
          "is_correct": false         // default false
        }
      ]
    }
  ]
}
```

Unknown fields are rejected (`deny_unknown_fields`), which catches typos such as
`"is_corect"`. Every image field comes in a `*_key` / `*_url` pair; give at most one of
the two.

#### Two ways to supply images

1. **Upload first, then reference the key** (works for private images too):

   ```bash
   # Log in once and keep the session cookie
   curl -c cookies.txt -H 'Content-Type: application/json' \
     -d '{"email":"you@example.com","password":"…"}' \
     http://localhost:8080/api/auth/login

   # Upload an image → returns { "key": "tmp/<uuid>.png", "url": "https://files…/tmp/<uuid>.png" }
   curl -b cookies.txt -F 'file=@screenshot.png' \
     http://localhost:8080/api/admin/uploads/image
   ```

   Then put the returned `key` in `image_key` (or `answer_image_key`, `cover_image_key`, …).
   Pass `-F quiz_id=12` when the quiz already exists, to get a `quizzes/12/<uuid>.png` key.

2. **Give a public URL** in `image_url` (or `answer_image_url`, …). The import downloads it,
   validates and re-encodes it, and stores it in R2 as `quizzes/<quiz_id>/<uuid>.<ext>`.
   URLs must be public `http(s)`. Private/loopback addresses and redirects are refused.

#### Complete example (every image field)

```json
{
  "subject_slug": "hci",
  "title": "Visual Design Principles",
  "description": "Recognise layout and usability principles from real screens.",
  "cover_image_url": "https://example.com/images/hci-cover.png",
  "questions": [
    {
      "prompt": "Which of Nielsen's heuristics does this dialog violate?",
      "image_url": "https://example.com/images/delete-dialog.png",
      "image_alt": "A dialog with Delete and Save buttons styled identically side by side",
      "answer_text": "Error prevention",
      "explanation": "Destructive actions should be visually distinct and confirmed.",
      "explanation_image_url": "https://example.com/images/error-prevention-diagram.png",
      "options": [
        { "text": "Error prevention", "is_correct": true },
        { "text": "Help and documentation", "is_correct": false },
        { "text": "Flexibility and efficiency of use", "is_correct": false }
      ]
    },
    {
      "prompt": "Which of these layouts follows Fitts's law best for the primary action?",
      "options": [
        {
          "image_url": "https://example.com/images/layout-a.png",
          "image_alt": "Small primary button in the far corner",
          "is_correct": false
        },
        {
          "image_url": "https://example.com/images/layout-b.png",
          "image_alt": "Large primary button next to the form fields",
          "is_correct": true
        }
      ],
      "explanation": "Bigger and closer targets are faster to reach."
    },
    {
      "prompt": "Sketch the shape of Fitts's law (movement time vs. index of difficulty).",
      "answer_text": "A straight line: MT grows linearly with log2(D/W + 1).",
      "answer_image_key": "tmp/2f1c7a8e-3b5d-4c1e-9f0a-6d2b8e4c7a11.png",
      "answer_image_alt": "Line chart with movement time increasing linearly",
      "options": [
        { "text": "A straight line", "is_correct": true },
        { "text": "An exponential curve", "is_correct": false }
      ]
    }
  ]
}
```

The `answer_image_key` above must be a key returned by the upload endpoint; this example
key will not exist on your bucket. A complete, **text-only** sample that imports without
any images is in `docs/examples/hci-sample-quiz.json`.

Import with curl:

```bash
curl -b cookies.txt -H 'Content-Type: application/json' \
  --data @docs/examples/hci-sample-quiz.json \
  http://localhost:8080/api/admin/quizzes/import
```

### (c) SQL seed / migration (last resort)

Use this only for fixtures that must exist in every environment. Create a **new**
migration (never edit an applied one):

```bash
cd backend
sqlx migrate add seed_my_quiz   # creates migrations/<timestamp>_seed_my_quiz.sql
```

Insert in the order **quiz → questions → options**, inside a transaction. Images must
already be in R2; only store their keys.

```sql
START TRANSACTION;

INSERT INTO quizzes (subject_id, title, description, cover_image_key, is_published)
SELECT id, 'HCI Basics', 'Seeded quiz', NULL, FALSE FROM subjects WHERE slug = 'hci';
SET @quiz_id = LAST_INSERT_ID();

INSERT INTO questions (quiz_id, position, prompt, answer_text, explanation, multi_select)
VALUES (@quiz_id, 0, 'What does Hick''s law describe?',
        'Decision time grows with the number of choices.', NULL, FALSE);
SET @q1 = LAST_INSERT_ID();

INSERT INTO question_options (question_id, position, text, is_correct) VALUES
  (@q1, 0, 'Decision time vs. number of choices', TRUE),
  (@q1, 1, 'Pointing time vs. distance and size', FALSE);

COMMIT;
```

Run it as a one-off script with `mysql gic_review < seed.sql`, which uses the explicit
`START TRANSACTION … COMMIT` above. **Inside a sqlx migration file, drop the
`START TRANSACTION` / `COMMIT` lines**: sqlx already runs each migration in a transaction,
and in MySQL a nested `START TRANSACTION` silently commits the outer one. Keep seed
migrations to plain `INSERT`s, with no DDL, because MySQL commits DDL implicitly.

---

## Part 2: Adding a new quiz mode

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
7. **Docs:** add the mode to `prompt.md` (requirements) and mark it ✅ in
   `docs/AI-SPEC.md` (with a Changelog line) in the same commit.

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
