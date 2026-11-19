# GIC Review API: how to add quizzes

**For:** an AI assistant (or script) that creates quizzes in GIC Review **through the REST API
only**. You have no access to the source code and don't need it. Typical job:
*"Read this source file (PDF, DOC/DOCX, CSV, or any other readable format) and create a quiz for the HCI subject."*

The source may be a PDF, Word document, CSV, spreadsheet, text file, Markdown file, or another
readable file format. Do not limit the workflow to PDFs.

Everything you need is in this file. (Interactive review pages: see `AI-HOW-ADD-INTERACTIVE-REVIEW.md`.)

---

## 1. Quick start

You need two values from the human:

| Value | Example | Notes |
|---|---|---|
| `BASE` (API base URL) | `https://api.example.com/api` | Every endpoint below is `BASE` + path. |
| `KEY` (API key) | `gic_3f9a…` (68 chars) | Made by a manager or admin in **Dashboard → API key**, permission **`quizzes:write`**. Secret: never show it. |

Every request that needs the key sends this header:

```
Authorization: Bearer KEY
```

Test it:

```bash
curl -H "Authorization: Bearer $KEY" $BASE/auth/me
# 200 → { "id": 4, "email": "dara@school.edu", "role": "manager",
#         "auth_method": "api_key", "api_key_scopes": ["quizzes:write"], … }
```

`401` → the key is wrong, expired or revoked. Stop and ask the human for a new key.

---

## 2. Endpoint list (everything you may call)

> **How to create a new quiz** (yes, your key can):
> - **Recommended: one call.** `POST /admin/quizzes/import` (#6) creates a **new quiz together with
>   all its questions**. Despite the name "import", this is the normal way to create a quiz.
> - **Or step by step.** `POST /admin/quizzes` (#17) creates a **new, empty quiz**, then add questions
>   one at a time with `POST /admin/quizzes/{id}/questions` (#9).
>
> Either way the quiz must go into a subject where you have `"can_add_content": true` (#16), and it
> starts as an unpublished draft.

Full URL = `BASE` + path. `{id}` = a quiz id, `{qid}` = a question id.

| # | Method | Path | Key? | Use it to… |
|---|---|---|---|---|
| 1 | `GET` | `/auth/me` | yes | check the key works and has `quizzes:write` |
| 2 | `GET` | `/subjects` or `/subjects?category={category_slug}` | no | list subjects (each with its category) → get the subject's `slug` / `id` |
| 3 | `GET` | `/admin/quizzes?subject_id={n}` | yes | list quizzes you can manage (to avoid duplicates, find ids) |
| 4 | `GET` | `/admin/quizzes/{id}` | yes | read one quiz with all its questions and options |
| 5 | `POST` | `/admin/uploads/image` | yes | upload an image file → get a `key` to use in the quiz |
| 6 | `POST` | `/admin/quizzes/import` | yes | **create a NEW quiz with all its questions in one call** ← main endpoint |
| 7 | `PATCH` | `/admin/quizzes/{id}/publish` | yes | publish / unpublish (only when the human asks) |
| 8 | `PUT` | `/admin/quizzes/{id}` | yes | change title, description, subject or cover image |
| 9 | `POST` | `/admin/quizzes/{id}/questions` | yes | add one question to an existing quiz |
| 10 | `PUT` | `/admin/quizzes/{id}/questions/{qid}` | yes | fix / replace one question |
| 11 | `DELETE` | `/admin/quizzes/{id}/questions/{qid}` | yes | remove one question |
| 12 | `PUT` | `/admin/quizzes/{id}/questions/reorder` | yes | change the question order |
| 13 | `DELETE` | `/admin/quizzes/{id}` | yes | delete a whole quiz (owner only; **ask the human first**) |
| 14 | `DELETE` | `/admin/uploads/image/{key}` | yes | delete an uploaded image you didn't end up using |
| 15 | `GET` | `/categories/{category_slug}/subjects/{subject_slug}/quizzes` | no | see what students see (published quizzes only) |
| 16 | `GET` | `/admin/categories` | yes | **list categories and subjects with `can_add_content`** (where you may add content) |
| 17 | `POST` | `/admin/quizzes` | yes | create a NEW **empty** quiz (title, description, cover), then add questions with #9 |

**Most jobs only need #1, #16, #3, #5 (if there are images) and #6.**

Don't call anything else under `/admin/…` (users, subjects, API keys, sharing). Those are for
humans in the dashboard and refuse API keys (`403 api_key_not_allowed`). You can't create
categories or subjects either: a human does that in the dashboard. If what you need is missing, ask.

**What your key can reach:** it acts as the person who made it. You see and edit quizzes
in **their categories** and in **categories or subjects shared with them** (plus anything they created);
others return `404`. Items you reach through a share (`"access": "shared"`) can be edited and
published but not deleted (`403 owner_only`). An **admin's** key reaches everything.

---

## 3. Recipe: create a quiz from source files

**Step 1. Check the key** (#1)

```bash
curl -H "Authorization: Bearer $KEY" $BASE/auth/me
```
Expect `"role"` `"manager"` or `"admin"`, and `"quizzes:write"` in `"api_key_scopes"`.

**Step 2. Find a subject you may add to** (#16)

```bash
curl -H "Authorization: Bearer $KEY" $BASE/admin/categories
# [ { "id": 3, "name": "I4", "slug": "i4", "can_manage": true, "can_add_content": true,
#     "subjects": [ { "id": 7, "slug": "hci-i4", "name": "Human Computer Interaction",
#                     "can_add_content": true, … } ] },
#   { "id": 5, "name": "I3", "can_manage": false, "can_add_content": true,
#     "subjects": [ { "id": 9, "name": "Networks", "can_add_content": true, "shared_with_you": true },
#                   { "id": 10, "name": "Databases", "can_add_content": false } ] } ]
```
Only use **subjects with `"can_add_content": true`**, and take their `slug`.

**Categories and subjects: the rules**
- Subjects are grouped by **category** (e.g. `I3`, `I4`, or custom names like "Very Cool Category").
- Each category **belongs to a person**, who can share the whole category, or single subjects of it, with
  others. You can add content to a subject when the key's owner owns its category or the category/subject
  was shared with them: that's `"can_add_content": true`. Anything else is refused with
  `403 not_your_category` (the message lists where you can add). Admin keys may use every category.
- **You cannot create, rename or delete categories or subjects, or share them.** A human does that in
  the dashboard (Categories / Subjects pages). If the category or subject you need doesn't exist or isn't
  available to you, **stop and ask the human** to create or share it (or to say which one to use).
- When the human says e.g. *"I4 HCI"*, pick the subject named like "HCI" inside your category named
  "I4". Two categories can have similarly named subjects, so always check the category.

**Step 3. Check what already exists** (#3)

```bash
curl -H "Authorization: Bearer $KEY" "$BASE/admin/quizzes?subject_id=1"
# [ { "id": 12, "title": "HCI Fundamentals", "question_count": 6, "is_published": true, … } ]
```
If a quiz on the same topic exists, ask whether to add to it (#9) instead of creating a new one.

**Step 4. Write the questions** from the source files, following section 6 (writing rules).

**Step 5. Upload images, if any** (#5). One call per image:

```bash
curl -H "Authorization: Bearer $KEY" -F "file=@figure-3.png" $BASE/admin/uploads/image
# 201 → { "key": "tmp/2f1c7a8e-….png", "url": "https://files.example.com/tmp/2f1c7a8e-….png" }
```
Keep the returned `key`; you put it in the quiz JSON (e.g. `"image_key": "tmp/2f1c…png"`).

**Step 6. Create the quiz in one call** (#6). JSON format in section 5.

```bash
curl -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
     --data @quiz.json $BASE/admin/quizzes/import
# 201 → { "id": 15, "title": "…", "question_count": 12, "is_published": false,
#         "available_modes": ["qcm", "flashcard"], "questions": [ … ], … }
```
- `422` → read `error.message`; it names the bad field (e.g. `questions[3].options[1]: …`). Fix
  the JSON and send it again (nothing was created).
- `available_modes` should be `["qcm", "flashcard"]`. If one is missing, see section 6 "Both modes".

**Step 7. Report back** to the human: quiz `id`, `title`, number of questions. The quiz is a
**draft** (students can't see it yet).

**Step 8. Publish only if the human says so** (#7)

```bash
curl -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
     -X PATCH -d '{"is_published": true}' $BASE/admin/quizzes/15/publish
```

---

## 4. Endpoint details

### #1 `GET /auth/me`
Response `200`: `{ "id", "email", "display_name", "role": "manager" | "admin", "is_active", "auth_method": "api_key", "api_key_scopes": [...] }`.

### #2 `GET /subjects` (no key)
Optional query `?category={category_slug}` to get one category's subjects only. Response `200`: array of
`{ "id", "category_id", "category_slug", "category_name", "slug", "name", "description", "color", "sort_order", "has_review", "quiz_count" }`,
ordered by category, then subject.

### #3 `GET /admin/quizzes?subject_id={n}`
`subject_id` is optional. Response `200`: array of quiz summaries (no questions):
`{ "id", "subject_id", "title", "description", "cover_image_key", "cover_image_url", "is_published", "question_count", "available_modes", "access", "owner_id", "owner_name", "created_at", "updated_at" }`.
Only quizzes the key's owner owns (`"access": "owner"`) or that were shared with them (`"access": "shared"`); an admin's key lists all quizzes (`"access": "admin"`).

### #4 `GET /admin/quizzes/{id}`
Response `200`: the **quiz object** (section 4, at the end). `404` if it doesn't exist or isn't yours.

### #5 `POST /admin/uploads/image`
`multipart/form-data` with field `file` (PNG, JPEG, WebP or GIF; max 5 MB by default) and,
optionally, `quiz_id`. Response `201`: `{ "key": "…", "url": "https://…" }`. Images are
resized to max 1600 px wide. `415` = not a supported image.

### #6 `POST /admin/quizzes/import` (create a new quiz with all its questions)
Body: the import JSON (section 5). Max 4 MB, 500 questions. Response `201`: the **quiz object**.
All-or-nothing: on any error, nothing is created.

### #7 `PATCH /admin/quizzes/{id}/publish`
Body `{ "is_published": true }` or `false`. Response `200`: the quiz object. `422` if the quiz
has no playable mode yet.

### #17 `POST /admin/quizzes` (create an empty quiz)
Body: `{ "subject_id": 7, "title": "…", "description": "…" | null, "cover_image_key": "…" | null }`
(`subject_id` from #16; the subject must have `"can_add_content": true`). Response `201`: the
**quiz object** with `"questions": []` and `"is_published": false`. Then add each question with #9
(same question object as in section 5, images as `*_key`).

```bash
curl -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
     -d '{"subject_id": 7, "title": "Fitts law drill", "description": null, "cover_image_key": null}' \
     $BASE/admin/quizzes
# 201 → { "id": 18, "title": "Fitts law drill", "question_count": 0, "is_published": false, … }

curl -H "Authorization: Bearer $KEY" -H "Content-Type: application/json" \
     -d '{"prompt": "…", "answer_text": "…", "explanation": "…",
          "options": [{"text": "…", "is_correct": true}, {"text": "…"}]}' \
     $BASE/admin/quizzes/18/questions
# 201 → the whole quiz, now with 1 question
```

Use #6 instead when you already have all the questions: it's one call and all-or-nothing.

### #8 `PUT /admin/quizzes/{id}`
Body: `{ "subject_id": 1, "title": "…", "description": "…", "cover_image_key": "…" }`.
`subject_id` and `title` are required. **Always send `description` and `cover_image_key` too**:
leaving one out (or `null`) clears it. Copy the current values from #4 and change only what you
need. Response `200`: the quiz object.

### #9 `POST /admin/quizzes/{id}/questions`
Body: **one question object** (same as an item of `questions[]` in section 5, but images only as
`*_key` fields, not `*_url`). It is added at the end. Response `201`: the whole quiz object.

### #10 `PUT /admin/quizzes/{id}/questions/{qid}`
Body: the **complete** question (prompt, images, answer, explanation, `multi_select`, all
`options`). It replaces the old one entirely. Images no longer used are deleted. Response `200`:
the quiz object. Tip: get the current question with #4, change it, send it back.

### #11 `DELETE /admin/quizzes/{id}/questions/{qid}`
Response `200`: the quiz object without that question.

### #12 `PUT /admin/quizzes/{id}/questions/reorder`
Body `{ "question_ids": [53, 51, 52] }`: **every** question id of the quiz, once, in the new
order. Response `200`: the quiz object.

### #13 `DELETE /admin/quizzes/{id}`
Deletes the quiz, its questions and images. Response `204`. **Owner only** (`403 owner_only` on
shared quizzes). Never do this without the human's explicit request.

### #14 `DELETE /admin/uploads/image/{key}`
`{key}` is the full key, e.g. `/admin/uploads/image/tmp/2f1c7a8e-….png`. Response `204`.
`409` if a quiz still uses it.

### #15 `GET /categories/{category_slug}/subjects/{subject_slug}/quizzes` (no key)
What students see: published quizzes of the subject.

### #16 `GET /admin/categories`
Response `200`: array of categories, each with `{ "id", "slug", "name", "description", "sort_order", "subject_count", "owner_id", "owner_name", "can_manage", "shared_with_you", "can_add_content", "subjects": [ { …subject, "can_add_content", "shared_with_you" } ] }`.
`can_manage: true` = the category is yours (or you're an admin). `can_add_content` (on the category and on each
subject) = you may add quizzes/reviews there, because the category is yours or the category/subject is shared with you
(`shared_with_you`).
The public `GET /categories` and `GET /categories/{category_slug}` (no key) show the same categories without ownership.
`GET /categories/{category_slug}` returns one category plus its `subjects` array.

### The quiz object (returned by #4, #6–#12)

```jsonc
{
  "id": 15,
  "subject_id": 1, "subject_slug": "hci", "subject_name": "Human Computer Interaction",
  "category_slug": "i4",              // subject slugs are unique per category, links need both
  "title": "…", "description": "…",
  "cover_image_key": null, "cover_image_url": null,
  "is_published": false,
  "question_count": 2,
  "available_modes": ["qcm", "flashcard"],      // computed by the server
  "access": "owner", "owner_id": 4, "owner_name": "Dara",
  "created_at": "2026-11-18T10:00:00", "updated_at": "2026-11-18T10:00:00",
  "questions": [
    {
      "id": 51, "position": 0,
      "prompt": "…", "image_key": null, "image_url": null, "image_alt": null,
      "answer_text": "…", "answer_image_key": null, "answer_image_url": null, "answer_image_alt": null,
      "explanation": "…", "explanation_image_key": null, "explanation_image_url": null,
      "multi_select": false,
      "options": [
        { "id": 201, "position": 0, "text": "…", "image_key": null, "image_url": null,
          "image_alt": null, "is_correct": true }
      ]
    }
  ]
}
```

---

## 5. Import JSON format (for #6)

```jsonc
{
  "subject_slug": "hci",              // required (or "subject_id": 1)
  "category_slug": "i4",              // needed when several categories have a subject with this slug
  "title": "string",                  // required, 1–200 chars
  "description": "string | null",     // optional
  "cover_image_key": "…",             // optional cover image: EITHER an uploaded key (#5)…
  "cover_image_url": "https://…",     // …OR a public URL the server downloads (never both)
  "questions": [                      // required, 1–500, kept in this order
    {
      "prompt": "string | null",                      // prompt and/or image required
      "image_key": "…", "image_url": "…",             // question image (= flashcard front)
      "image_alt": "string | null",
      "answer_text": "string | null",                 // short answer (flashcard back)
      "answer_image_key": "…", "answer_image_url": "…",
      "answer_image_alt": "string | null",
      "explanation": "string | null",                 // why the answer is right
      "explanation_image_key": "…", "explanation_image_url": "…",
      "multi_select": false,                          // true = several correct options
      "options": [
        {
          "text": "string | null",                    // text and/or image required
          "image_key": "…", "image_url": "…", "image_alt": "string | null",
          "is_correct": false
        }
      ]
    }
  ]
}
```

- Unknown fields are **rejected** (catches typos like `"is_corect"`).
- Each image is a `*_key` / `*_url` pair: give at most one, or neither for no image.
- `*_url` must be a public `http(s)` URL (no private addresses, no redirects).

### Example A: text-only (most common)

```json
{
  "subject_slug": "hci",
  "title": "Nielsen's Heuristics: basics",
  "description": "Usability heuristics and Fitts's law.",
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

### Example B: with images (every image field)

```json
{
  "subject_slug": "hci",
  "title": "Visual Design Principles",
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
      "prompt": "What shape does Fitts's law have (movement time vs. index of difficulty)?",
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
(The `tmp/…` keys are placeholders; use the keys returned by your own uploads in step 5.)

---

## 6. Writing rules

**Content:**
- Use only facts from the source material; don't invent. Write in the source's language unless told otherwise.
- One concept per question; short, clear prompts.
- 3–5 options, exactly one correct (unless `multi_select: true`). Wrong options must be plausible and similar in length.
- No "all/none of the above" (options are shuffled for students).
- Always add `explanation` (1–3 sentences; you may cite e.g. "(slide 12)") and a short `answer_text`.
- 10–30 questions per quiz; split long documents into several quizzes (one per chapter).
- Images: export figures from source files as PNG, crop them, upload (step 5), and always set `*_alt`.

**Format:**

| Rule | Details |
|---|---|
| Question | needs `prompt` or an image (or both) |
| Option | needs `text` or an image (or both) |
| `is_correct` | `true` on each correct option; omitted = `false` |
| `multi_select` | `false` → at most one correct option (more is rejected). `true` → students must pick all correct ones |
| Both modes | QCM needs **every** question to have ≥ 2 options and ≥ 1 correct. Flashcard needs **every** question to have `answer_text`, an answer image, or ≥ 1 correct option. ⇒ Always give ≥ 2 options with ≥ 1 correct **and** an `answer_text`. |
| Limits | title ≤ 200, description ≤ 5000, prompt/answer ≤ 5000, explanation ≤ 10000, option text ≤ 2000, alt ≤ 255 chars; ≤ 12 options per question; 1–500 questions per import |

---

## 7. Errors

Errors look like `{ "error": { "code": "…", "message": "…" } }`. The `message` says what to fix.

| HTTP | `code` | What to do |
|---|---|---|
| 401 | `unauthorized` | Key missing/wrong/expired/revoked → ask the human for a new key. |
| 403 | `missing_scope` | Key lacks `quizzes:write` → ask for a key with that permission. |
| 403 | `owner_only` | Only the quiz's owner can do that (e.g. delete a quiz shared with you). |
| 403 | `not_your_category` | You can't add to that subject → use a subject with `can_add_content: true` (#16), or ask the human. |
| 403 | `api_key_not_allowed` | You called a dashboard-only endpoint; don't. |
| 404 | `not_found` | Wrong id, or not your quiz. |
| 409 | `conflict` | Something is still in use / duplicate. |
| 413 | `payload_too_large` | File or request too big. |
| 415 | `unsupported_media_type` | Not PNG/JPEG/WebP/GIF. |
| 422 | `validation_error` | Bad field → fix what `message` says and retry. |
| 429 | `too_many_requests` | Wait a bit and retry. |

---

## 8. Final checklist

- [ ] Every question: prompt or image, ≥ 2 options, ≥ 1 correct, `answer_text`, `explanation`.
- [ ] Every image has `*_alt`.
- [ ] Response shows `"available_modes": ["qcm", "flashcard"]`.
- [ ] Still a draft unless the human asked to publish.
- [ ] Told the human the quiz `id`, `title` and question count. Never reveal the key.
