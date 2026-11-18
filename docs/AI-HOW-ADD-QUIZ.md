# GIC Review API: how to add quizzes

**Audience:** an AI assistant (or any script) that creates quizzes in GIC Review **through the
REST API only**. You do not have, and do not need, access to the source code. A typical job:
*"Read this PDF and create a quiz for the HCI subject."*

This document is self-contained: it has everything you need (authentication, the workflow,
the full JSON format, every endpoint, and error codes). Interactive reviews are covered in the
companion document `AI-HOW-ADD-INTERACTIVE-REVIEW.md`.

---

## 1. What you need from the human

| Value | Example | Where it comes from |
|---|---|---|
| **API base URL** | `https://api.example.com/api` | The site owner. Every path below is relative to it. |
| **API key** | `gic_3f9a…` (68 characters) | A **manager** creates it in the dashboard: **Dashboard → API key**, permission `quizzes:write`. |

```bash
export GIC_API=https://api.example.com/api   # no trailing slash
export GIC_KEY=gic_…                         # keep it secret
```

Send the key on **every** request:

```
Authorization: Bearer <API key>
```

Treat the key like a password: never print it in answers, never put it in files or quiz
content. If a request returns `401`, the key is wrong, expired or revoked: ask the human for a
new one.

---

## 2. What your key can and cannot do

- The key acts **as the manager who created it**.
- You can manage the quizzes that manager **owns** (created by them or by you with their key) and
  quizzes **another manager shared with them**.
- You **cannot see** other managers' quizzes (you get `404`, as if they did not exist).
- On a **shared** quiz you can edit and publish, but you **cannot delete** it (`403 owner_only`).
- Sharing, user accounts, subjects and API keys are managed by humans in the dashboard. Your key
  is refused there (`403 api_key_not_allowed`).
- Subjects (e.g. "Human Computer Interaction") are created by an admin. If the subject you need
  does not exist, stop and ask the human.

---

## 3. Recommended workflow (e.g. from a PDF)

1. **Check the key:** `GET /auth/me`. Expect `"role": "manager"` and `"api_key_scopes"` containing
   `"quizzes:write"`.
2. **Pick the subject:** `GET /subjects` (public). Use its `slug` (e.g. `hci`) as `subject_slug`.
3. **Read the source and plan the questions** (see section 4 for writing rules).
4. **Upload images** you need (figures, screenshots, diagrams): `POST /admin/uploads/image`
   returns a `key`. Alternatively give a public `https://` URL in the import (`*_url` fields).
5. **Create the whole quiz in one call:** `POST /admin/quizzes/import` with the JSON from section 5.
   It is created **unpublished** (a draft that students can't see).
6. **Check the response:** `available_modes` should be `["qcm", "flashcard"]`. If a mode is
   missing, fix the questions (section 4, "Both modes").
7. **Report back to the human** with the quiz `id` and `title`, and ask whether to publish.
   Publish only when asked: `PATCH /admin/quizzes/{id}/publish {"is_published": true}`.
8. **Fix mistakes** later with the question endpoints (section 7) instead of importing again.
   Don't create duplicates: `GET /admin/quizzes` first to see what already exists.

---

## 4. Writing good questions

**Content rules (from the source material):**
- Use only facts from the provided material; don't invent. Keep the source's language
  (e.g. French in → French out) unless told otherwise.
- One concept per question. Short, unambiguous prompts.
- 3–5 options per question, exactly one correct unless the question says "select all".
  Wrong options (distractors) must be plausible and about the same length as the right one.
- Avoid "all of the above" / "none of the above" (options are shuffled for students).
- Always add an `explanation` (1–3 sentences: why the answer is right). You may cite the source,
  e.g. "(slide 12)".
- Add `answer_text` (a short answer) so the flashcard back reads well on its own.
- 10–30 questions per quiz is a good size; split big documents into several quizzes by chapter.

**Format rules:**

| Rule | Details |
|---|---|
| Question content | `prompt` **or** an image (or both). Image-only questions are fine ("Which principle does this screen violate?"). |
| Option content | `text` **or** an image (or both). Image options are shown as a grid of image cards. |
| `is_correct` | `true` on every correct option, `false` (default) on the others. |
| `multi_select` | `false` (default): exactly one option may be correct. `true`: several may be correct and students must select all of them. Marking 2+ correct options while `multi_select` is `false` is rejected. |
| Both modes | **QCM** needs every question to have ≥ 2 options and ≥ 1 correct option. **Flashcard** needs every question to have `answer_text`, an answer image, or ≥ 1 correct option. → Give every question ≥ 2 options with ≥ 1 correct, plus `answer_text`, and both modes work. |
| Limits | title ≤ 200 chars, description ≤ 5000, prompt/answer ≤ 5000, explanation ≤ 10000, option text ≤ 2000, alt text ≤ 255, ≤ 12 options per question, 1–500 questions per import, request ≤ 4 MB. |
| Images | PNG, JPEG, WebP or GIF, max 5 MB each (the site owner may change this). They are resized to max 1600 px wide. Always add `*_alt` text describing the image for screen readers. |

---

## 5. Import format (`POST /admin/quizzes/import`)

```jsonc
{
  "subject_slug": "hci",              // required (or "subject_id": 1)
  "title": "string",                  // required, 1–200 chars
  "description": "string | null",
  "cover_image_key": "…",             // optional cover: EITHER an uploaded key…
  "cover_image_url": "https://…",     // …OR a public URL to download (never both)
  "questions": [                      // required, 1–500, kept in this order
    {
      "prompt": "string | null",                    // prompt and/or image required
      "image_key": "…", "image_url": "…",           // question image (= flashcard front)
      "image_alt": "string | null",
      "answer_text": "string | null",               // flashcard back
      "answer_image_key": "…", "answer_image_url": "…", "answer_image_alt": "string | null",
      "explanation": "string | null",               // shown after answering / on the card back
      "explanation_image_key": "…", "explanation_image_url": "…",
      "multi_select": false,
      "options": [
        {
          "text": "string | null",                  // text and/or image required
          "image_key": "…", "image_url": "…", "image_alt": "string | null",
          "is_correct": false
        }
      ]
    }
  ]
}
```

- Unknown fields are **rejected** (this catches typos like `"is_corect"`).
- Every image comes as a `*_key` / `*_url` pair; give at most one of the two. Omit both for no image.
- `*_url` must be a public `http(s)` URL (no private/internal addresses, no redirects). It is
  downloaded and stored by the server.
- The import is all-or-nothing: on any error nothing is created, and the response says exactly
  which field is wrong (e.g. `questions[3].options[1]: an option needs text or an image`).

### Example: text-only (works in QCM and Flashcard)

```json
{
  "subject_slug": "hci",
  "title": "Nielsen's Heuristics: basics",
  "description": "Questions on usability heuristics and Fitts's law.",
  "questions": [
    {
      "prompt": "Which heuristic is violated when a system gives no feedback after the user clicks \"Save\"?",
      "answer_text": "Visibility of system status",
      "explanation": "The system should keep users informed about what is going on, through timely feedback.",
      "options": [
        { "text": "Visibility of system status", "is_correct": true },
        { "text": "Aesthetic and minimalist design" },
        { "text": "Help and documentation" },
        { "text": "Flexibility and efficiency of use" }
      ]
    },
    {
      "prompt": "According to Fitts's law, the time to reach a target depends on… (select all)",
      "answer_text": "The distance to the target and its size.",
      "explanation": "MT = a + b · log2(D / W + 1): bigger, closer targets are faster to hit.",
      "multi_select": true,
      "options": [
        { "text": "The distance to the target", "is_correct": true },
        { "text": "The size of the target", "is_correct": true },
        { "text": "The color of the target" },
        { "text": "The number of items on screen" }
      ]
    }
  ]
}
```

### Example: every image field

```json
{
  "subject_slug": "hci",
  "title": "Visual Design Principles",
  "description": "Recognise layout and usability principles from real screens.",
  "cover_image_url": "https://example.com/images/hci-cover.png",
  "questions": [
    {
      "prompt": "Which heuristic does this dialog violate?",
      "image_key": "tmp/2f1c7a8e-3b5d-4c1e-9f0a-6d2b8e4c7a11.png",
      "image_alt": "A dialog with Delete and Save buttons styled identically side by side",
      "answer_text": "Error prevention",
      "explanation": "Destructive actions should be visually distinct and confirmed.",
      "explanation_image_url": "https://example.com/images/error-prevention-diagram.png",
      "options": [
        { "text": "Error prevention", "is_correct": true },
        { "text": "Help and documentation" },
        { "text": "Flexibility and efficiency of use" }
      ]
    },
    {
      "prompt": "Which layout follows Fitts's law best for the primary action?",
      "answer_text": "Layout B: the large button next to the form.",
      "explanation": "Bigger and closer targets are faster to reach.",
      "options": [
        { "image_url": "https://example.com/images/layout-a.png", "image_alt": "Small primary button far in the corner" },
        { "image_url": "https://example.com/images/layout-b.png", "image_alt": "Large primary button next to the fields", "is_correct": true }
      ]
    },
    {
      "prompt": "Sketch the shape of Fitts's law (movement time vs. index of difficulty).",
      "answer_text": "A straight line.",
      "answer_image_key": "tmp/9d1e0c55-6a2b-4f3c-8e7d-1a2b3c4d5e6f.png",
      "answer_image_alt": "Line chart where movement time rises linearly",
      "options": [
        { "text": "A straight line", "is_correct": true },
        { "text": "An exponential curve" }
      ]
    }
  ]
}
```

(The `tmp/…` keys above are placeholders; use the keys returned by your own uploads.)

---

## 6. Uploading images

`POST /admin/uploads/image`, `multipart/form-data`:

| Field | Required | Meaning |
|---|---|---|
| `file` | yes | the image (PNG, JPEG, WebP, GIF). The real file bytes are checked, not just the name. |
| `quiz_id` | no | the quiz it belongs to (if it already exists) |

```bash
curl -H "Authorization: Bearer $GIC_KEY" -F "file=@figure-3.png" $GIC_API/admin/uploads/image
# 201 → { "key": "tmp/2f1c7a8e-….png", "url": "https://files.example.com/tmp/2f1c7a8e-….png" }
```

Put the `key` into `image_key`, `answer_image_key`, `explanation_image_key`, an option's
`image_key` or `cover_image_key`. The `url` is the public address (handy to check the result).
An uploaded image that you end up not using can be removed with
`DELETE /admin/uploads/image/{key}` (only works while nothing uses it).

From a PDF: export the figure as a PNG (crop it to the relevant part), upload it, and use the key.

---

## 7. Endpoint reference

All endpoints need `Authorization: Bearer <key>` with the `quizzes:write` permission, except the
public ones. Request and response bodies are JSON unless noted.

| Method & path | What it does |
|---|---|
| `GET /auth/me` | Who you are acting as, and the key's permissions. |
| `GET /subjects` | Public. All subjects (`id`, `slug`, `name`, …). |
| `GET /admin/quizzes[?subject_id=1]` | Quizzes you can manage (yours + shared with you), drafts included. |
| `GET /admin/quizzes/{id}` | One quiz with all questions and options. |
| `POST /admin/quizzes/import` | Create a complete quiz (section 5). → `201` + the quiz. |
| `POST /admin/quizzes` | Create an empty quiz: `{ "subject_id", "title", "description", "cover_image_key" }`. |
| `PUT /admin/quizzes/{id}` | Update title/description/subject/cover (same body as create). |
| `PATCH /admin/quizzes/{id}/publish` | `{ "is_published": true \| false }`. Refused while no mode is available. |
| `DELETE /admin/quizzes/{id}` | Delete the quiz and its images. **Owner only.** Ask the human first. |
| `POST /admin/quizzes/{id}/questions` | Add one question (same object as in `questions[]`, `*_key` image fields only). |
| `PUT /admin/quizzes/{id}/questions/{question_id}` | Replace a question and all its options. |
| `DELETE /admin/quizzes/{id}/questions/{question_id}` | Delete a question (and its images). |
| `PUT /admin/quizzes/{id}/questions/reorder` | `{ "question_ids": [ … every id, in the new order ] }` |
| `POST /admin/uploads/image` | Upload an image (section 6). |
| `DELETE /admin/uploads/image/{key}` | Remove an unused upload. |

**The quiz object** returned by these endpoints:

```jsonc
{
  "id": 12, "subject_id": 1, "subject_slug": "hci", "subject_name": "Human Computer Interaction",
  "title": "…", "description": "…",
  "cover_image_key": "…", "cover_image_url": "https://…",
  "is_published": false,
  "question_count": 6,
  "available_modes": ["qcm", "flashcard"],   // computed by the server
  "access": "owner",                          // or "shared"
  "owner_id": 4, "owner_name": "Dara", "share_count": 0,
  "questions": [
    { "id": 51, "position": 0, "prompt": "…", "image_key": null, "image_url": null, "image_alt": null,
      "answer_text": "…", "answer_image_key": null, "answer_image_url": null, "answer_image_alt": null,
      "explanation": "…", "explanation_image_key": null, "explanation_image_url": null,
      "multi_select": false,
      "options": [ { "id": 201, "position": 0, "text": "…", "image_key": null, "image_url": null,
                     "image_alt": null, "is_correct": true } ] }
  ]
}
```

Editing a question: send the complete question (prompt, images as `*_key`, answer,
explanation, `multi_select`, all `options`). Options are replaced as a whole, and images no
longer used are deleted automatically.

---

## 8. Errors

Every error looks like `{ "error": { "code": "…", "message": "…" } }`. Read `message`: it
names the field and the problem.

| Status | `code` | Meaning / what to do |
|---|---|---|
| 401 | `unauthorized` | Missing, wrong, expired or revoked key. Ask the human for a new key. |
| 403 | `forbidden` | The key's account isn't a manager. |
| 403 | `missing_scope` | The key lacks `quizzes:write`. Ask for a key with that permission. |
| 403 | `owner_only` | Only the quiz's owner may do this (e.g. delete a shared quiz). |
| 403 | `api_key_not_allowed` | Dashboard-only endpoint (sharing, users, subjects, keys). |
| 404 | `not_found` | Doesn't exist, or you have no access to it. |
| 409 | `conflict` | Duplicate or still in use (e.g. deleting an image a question uses). |
| 413 | `payload_too_large` | File or request too big. |
| 415 | `unsupported_media_type` | Not a PNG/JPEG/WebP/GIF (or extension and content disagree). |
| 422 | `validation_error` | Invalid field; the message says which. Fix and retry. |
| 429 | `too_many_requests` | Slow down and retry a bit later. |

---

## 9. Checklist before you report back

- [ ] Every question has a prompt or an image, ≥ 2 options with ≥ 1 correct, an
      `answer_text` and an `explanation`.
- [ ] Images have `*_alt` text.
- [ ] The response shows `available_modes: ["qcm", "flashcard"]`.
- [ ] The quiz is still a draft unless the human asked you to publish it.
- [ ] You gave the human the quiz `id`, `title` and question count. Never include the API key.
