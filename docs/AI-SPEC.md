# GIC Review: AI-SPEC (current state)

## Keep docs in sync (rules for every AI or developer)

1. **Every new feature or changed requirement** must be recorded in **both**
   `docs/AI-SPEC.md` (as built: ✅/🚧/⏳ status, schema, endpoints) **and** `prompt.md`
   at the repo root (as a requirement, in the matching section).
2. `prompt.md` is the **source of truth for requirements**: what the project should be.
   `AI-SPEC.md` is the **source of truth for current state**: what is actually built.
   If they disagree, `prompt.md` wins for intent; fix the code or update `AI-SPEC.md`,
   then note the mismatch in the commit message.
3. When the user asks for a new feature in chat: first add it to `prompt.md`, then
   implement it, then mark it ✅ here, all **in the same commit/PR** as the code.
4. If a change affects how quizzes or interactive reviews are added (schema, JSON import
   format, image fields, quiz modes, HTML rules), also update `docs/AI-HOW-ADD-QUIZ.md`
   and/or `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`.
5. Never delete requirements from `prompt.md` silently. When one is removed or replaced,
   say so in the commit message.
6. Add a dated entry to the **Changelog** at the bottom of this file.

---

## Summary

GIC Review is an exam-review website. Students pick a subject, then open its
**Interactive Review** (a static HTML page shown in a sandboxed iframe) or practise
with **Quizzes** in **QCM** or **Flashcard** mode. No login is needed to study. Staff
(admins and managers) log in to create quizzes, upload images and reviews; admins also
manage subjects and manager accounts.

## Tech stack

| Layer | Technology |
|---|---|
| Frontend | Vue 3 (`<script setup>`, TS), Vite 8, Tailwind CSS 4 (`@tailwindcss/vite`), Vue Router 5, Pinia 4, `@catppuccin/palette` |
| Backend | Rust 2024, Axum 0.8 on Tokio, SQLx 0.9 (MySQL), tower-http 0.7 (CORS, trace, body limit, timeout), argon2 0.6 (Argon2id), jsonwebtoken 12, validator, thiserror/anyhow, tracing, `image`, `aws-sdk-s3`, reqwest |
| Database | MySQL 8, migrations in `backend/migrations/` (run by `sqlx migrate run` or automatically at startup) |
| Storage | Cloudflare R2 through the S3 API (`aws-sdk-s3`, custom endpoint, path-style) |

## Folder structure

```
gic-review/
├── frontend/                       # Vue app (own package.json, .env, .gitignore)
│   ├── index.html                  # inline script applies the saved flavor before mount
│   ├── vite.config.ts
│   ├── vite/catppuccin.ts          # Vite plugin: generates --ctp-* CSS vars from @catppuccin/palette
│   └── src/
│       ├── api/                    # client.ts (fetch + XHR upload), types.ts, admin.ts
│       ├── components/             # AppHeader, ThemePicker, BaseModal, SubjectCard, ActivityModal,
│       │   │                       # QuizCard, QuizModeModal, ResultScreen, ImageLightbox, ZoomableImage, …
│       │   └── admin/              # ImageUploader, QuestionEditor, QuestionPreview
│       ├── quiz-modes/             # registry.ts, QcmPlayer, FlashcardPlayer, QcmQuestionCard, FlashcardCard
│       ├── router/index.ts         # routes + staff/admin guards
│       ├── stores/                 # theme, auth, progress (localStorage), toast, lightbox
│       ├── styles/main.css         # Tailwind + Catppuccin theme mapping + .btn/.card/.input utilities
│       └── views/                  # Home, Review, QuizList, QuizPlay, Login, NotFound, admin/*
├── backend/                        # Axum API (own Cargo project, .env, .gitignore)
│   ├── migrations/                 # 0001 schema, 0002 seed "Human Computer Interaction"
│   ├── src/
│   │   ├── main.rs                 # config → DB pool → migrations → admin bootstrap → serve
│   │   ├── lib.rs                  # build_router(): /api + CORS, trace, body limit, timeout
│   │   ├── config.rs               # .env loading, fail fast with a list of problems
│   │   ├── auth.rs                 # Argon2id, JWT cookie, RequireStaff/RequireAdmin/MaybeStaff
│   │   ├── quiz_modes.rs           # QuizMode enum + availability rules (single source)
│   │   ├── images.rs               # magic-number check, resize/re-encode, safe URL download
│   │   ├── storage.rs              # R2 client (put/delete/list, public URLs)
│   │   ├── repo.rs, models.rs      # shared queries and DTOs
│   │   └── routes/                 # public, auth, admin_quizzes, import, uploads, reviews, users, subjects
│   └── tests/auth_roles.rs         # integration tests (auth + role checks)
├── docs/                           # this file, AI-HOW-ADD-QUIZ.md, AI-HOW-ADD-INTERACTIVE-REVIEW.md, examples/
├── prompt.md                       # requirements (source of truth for intent)
└── README.md
```

## How to run

```bash
# Backend
cd backend
cp .env.example .env               # fill in DB, JWT, admin and R2 values
cargo sqlx migrate run             # optional: the server also runs migrations on start
cargo run                          # http://127.0.0.1:8080/api/health

# Frontend
cd frontend
cp .env.example .env
npm install
npm run dev                        # http://localhost:5173
```

Quality checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test` (DB tests: `DATABASE_URL=… cargo test -- --ignored`),
`npm run lint:check`, `npm run format:check`, `npm run build`.

---

## Features built so far

Legend: ✅ done · 🚧 in progress / partial · ⏳ planned

### Public site
- ✅ Home page with subject cards from `GET /api/subjects`, loading skeletons, empty/error states
- ✅ Activity modal (Interactive Review / Quiz); review option disabled with "Coming soon" when `has_review` is false
- ✅ Interactive review page: list when several, sandboxed iframe (`sandbox="allow-scripts"`) from the R2 domain, refuses same-origin URLs, back button
- ✅ Theme sync to reviews via `postMessage({ type: 'gic-theme', flavor, accent })`
- ✅ Quiz list per subject (published only) with question count, mode badges, best score
- ✅ Quiz mode modal built from the mode registry
- ✅ Image lightbox (zoom with buttons/wheel/drag, Esc to close), lazy images, next-question image preloading
- ✅ Catppuccin theme: Latte/Frappé/Macchiato/Mocha + System, 14 accents, `localStorage`, no flash on load, `dark:` variant for dark flavors
- ✅ Anonymous progress in `localStorage` (last/best score, attempts, missed questions, shuffle setting)
- ✅ Error toasts, accessible modals (focus trap, Esc, backdrop click, aria labels, nested-modal stack)

### Quiz modes
- ✅ Mode registry `frontend/src/quiz-modes/registry.ts` + backend `QuizMode` enum, `available_modes` computed server-side
- ✅ QCM: one question at a time, single/multi answer, image options as a card grid, explanation text + image, progress bar, score, shuffled options every attempt, question-shuffle toggle, keyboard (1–9, Enter), result screen with missed list, Retry and Retry missed only
- ✅ Flashcard: text/image front and back (falls back to correct options), explanation, flip animation, tap-to-zoom without flipping, "I knew it" / "Didn't know" (missed cards return at the end of the round), Space/←/→/1/2 shortcuts, result screen with "Study missed cards again"
- ⏳ More modes (True/False, Matching, Fill-in-the-blank), see `docs/AI-HOW-ADD-QUIZ.md`

### Staff / Admin
- ✅ `/login` (email + password), no public sign-up
- ✅ Quizzes: create, edit details, cover image, delete, publish/unpublish (refused while no mode is available)
- ✅ Questions: add, edit, delete, reorder (drag & drop and ↑/↓), options add/remove/reorder, single/multi answer
- ✅ `ImageUploader` for every image field: browse, drag & drop, Ctrl+V paste, progress bar, preview (opens lightbox), alt text, Replace/Remove
- ✅ Unsaved uploads are deleted again when the editor is closed without saving
- ✅ Student preview of a question in QCM and Flashcard mode (same components as the players)
- ✅ Bulk import page (paste or upload JSON, "Load HCI sample")
- ✅ Interactive reviews: upload `.html`, edit title/slug/order/published, replace file (same URL), delete
- ✅ Managers (admin only): create, reset password, activate/deactivate, delete
- ✅ Subjects (admin only): create, edit (name, slug, description, color, order), delete when empty
- ✅ "Clean up orphaned images" (admin only, keeps files younger than 1 hour)

### Security
- ✅ Argon2id password hashing; constant-time-ish login (dummy hash for unknown emails)
- ✅ JWT in `gic_session` cookie: httpOnly, SameSite=Strict, Secure when `APP_ENV=production`, expiry from `JWT_EXPIRES_IN_HOURS`
- ✅ `RequireStaff` / `RequireAdmin` extractors re-check role and `is_active` in the DB on every request
- ✅ Login rate limit: 5 attempts / minute / IP (in-memory)
- ✅ CORS restricted to `FRONTEND_URL` with credentials
- ✅ Parameterized SQL only (sqlx 0.9 also refuses non-literal SQL strings at compile time)
- ✅ Uploads: MIME + extension + magic-number checks, size limits from `.env`, UUID keys, EXIF stripped by re-encoding, decompression-bomb limits
- ✅ Import `*_url` downloads: http(s) only, no private/loopback IPs, no redirects, size-capped, 20 s timeout
- ✅ JSON errors `{ "error": { "code", "message" } }`, internal details only in server logs
- ✅ No API path creates or edits admins; created users are always managers
- ✅ Frontend route guards (UI only) mirror backend checks

### Infrastructure
- ✅ Separate `frontend/` and `backend/` projects, each with its own `.env.example` and `.gitignore`; root `.gitignore`
- ✅ SQLx migrations: schema + seed of the single subject `Human Computer Interaction` (`hci`)
- ✅ Admin bootstrap from `ADMIN_EMAIL` / `ADMIN_PASSWORD` when no admin exists (warning logged, password never logged)
- ✅ Startup fails fast listing every missing/invalid variable
- ✅ Integration tests: guest 401 on every staff endpoint, forged/garbage tokens, rate limit, error format (no DB needed); DB-backed tests for login cookie flags, manager 403 on admin endpoints, instant deactivation, admin immutability (`--ignored`)
- 🚧 Compile-time checked queries (`sqlx::query!`) + committed `.sqlx/`: not used yet (see limitations)
- ⏳ CI pipeline, Docker images, deployment docs

---

## Database schema (MySQL 8)

```
users               id, email UNIQUE, password_hash, display_name, role ENUM('admin','manager'),
                    is_active, created_by NULL → users ON DELETE SET NULL, created_at, updated_at
subjects            id, slug UNIQUE, name, description NULL, color (Catppuccin accent name),
                    sort_order, created_at, updated_at
interactive_reviews id, subject_id → subjects (RESTRICT), title, slug (UNIQUE per subject), r2_key,
                    sort_order, is_published, created_by NULL → users (SET NULL), created_at, updated_at
quizzes             id, subject_id → subjects (RESTRICT), title, description NULL, cover_image_key NULL,
                    is_published, created_by NULL → users (SET NULL), created_at, updated_at
questions           id, quiz_id → quizzes ON DELETE CASCADE, position, prompt NULL,
                    image_key NULL, image_alt NULL, answer_text NULL, answer_image_key NULL,
                    answer_image_alt NULL, explanation NULL, explanation_image_key NULL,
                    multi_select, created_at, updated_at
question_options    id, question_id → questions ON DELETE CASCADE, position, text NULL,
                    image_key NULL, image_alt NULL, is_correct
```

Only R2 keys are stored. URLs are built as `R2_PUBLIC_BASE_URL + "/" + key`.
Key formats: `quizzes/{quiz_id}/{uuid}.{ext}`, `tmp/{uuid}.{ext}` (quiz not created yet),
`reviews/{subject_id}/{uuid}.html`.

## API endpoints (prefix `/api`)

| Method & path | Access | Notes |
|---|---|---|
| `GET /health` | public | 200 `{status, database}`, 503 if the DB is down |
| `GET /subjects` | public | includes `has_review`, `quiz_count` (published) |
| `GET /subjects/:slug` | public | |
| `GET /subjects/:slug/reviews` | public | published reviews with `url` (`?v=` cache-buster) |
| `GET /subjects/:slug/quizzes` | public | published quizzes with `question_count`, `available_modes` |
| `GET /quizzes/:id` | public / staff | full quiz with all image URLs; unpublished only for staff |
| `POST /auth/login` | public | rate-limited; sets cookie; returns user |
| `POST /auth/logout` | public | clears cookie |
| `GET /auth/me` | staff | |
| `GET /admin/quizzes[?subject_id=]` | staff | all quizzes incl. drafts (added for the admin UI) |
| `POST /admin/quizzes` | staff | always created unpublished |
| `GET/PUT/DELETE /admin/quizzes/:id` | staff | GET added for the admin UI; DELETE also removes R2 images |
| `PATCH /admin/quizzes/:id/publish` | staff | `{is_published}` |
| `POST /admin/quizzes/:id/questions` | staff | question + options |
| `PUT/DELETE /admin/quizzes/:id/questions/:qid` | staff | options replaced as a whole; unused images deleted from R2 |
| `PUT /admin/quizzes/:id/questions/reorder` | staff | `{question_ids: [...]}` (all ids, once each) |
| `POST /admin/quizzes/import` | staff | bulk JSON, one transaction |
| `POST /admin/uploads/image` | staff | multipart `file`, optional `quiz_id` → `{key, url}` |
| `DELETE /admin/uploads/image/*key` | staff | only for unreferenced images (409 otherwise) |
| `POST /admin/uploads/cleanup-orphans` | admin | |
| `GET /admin/reviews[?subject_id=]` | staff | all reviews incl. unpublished (added for the admin UI) |
| `POST /admin/reviews`, `PUT/DELETE /admin/reviews/:id` | staff | multipart `.html` |
| `GET/POST /admin/users` | admin | creates managers only |
| `PATCH /admin/users/:id` | admin | `is_active`, `display_name` (managers only) |
| `POST /admin/users/:id/reset-password` | admin | managers only |
| `DELETE /admin/users/:id` | admin | managers only |
| `POST /admin/subjects`, `PUT/DELETE /admin/subjects/:id` | admin | delete refused (409) while quizzes/reviews exist |

---

## Known limitations and next steps

- **No compile-time checked SQL yet.** All queries are parameterized `sqlx::query`/`query_as`
  with `&'static str` SQL, but not `query!` macros, because no database was available while
  building. Next: with a DB running, convert hot queries to `query!`/`query_as!`, run
  `cargo sqlx prepare`, and commit `backend/.sqlx/`.
- The DB-backed integration tests are `#[ignore]` by default; run them with a MySQL
  `DATABASE_URL` whose user may create databases.
- Login rate limiting is in memory (per process). Behind a reverse proxy, the client IP is
  the proxy's IP unless the proxy is configured to preserve it. Use a shared limiter
  (e.g. Redis) and trusted `X-Forwarded-For` handling when scaling out.
- Cookie auth needs the frontend and API on the **same site** (e.g. `example.com` and
  `api.example.com`, or both on `localhost` in development) because the cookie is
  `SameSite=Strict`.
- Serve R2 from a domain that is **not** the API's host (e.g. `files.example.com`). The
  session cookie is host-only and httpOnly, so reviews cannot read it either way.
- The import's private-IP check resolves the host before downloading; a DNS-rebinding
  attacker with staff access could still race it. Acceptable for staff-only input.
- Quiz images uploaded before a quiz exists keep their `tmp/` key (they are not moved).
- Changing a review's subject keeps its original `reviews/{subject_id}/` key prefix.
- `npm audit` reports advisories in build-time dependencies of `vue-router` 5
  (`unplugin`/`picomatch`); they are not shipped to browsers. Re-check on upgrades.
- Next: CI (fmt, clippy, tests, lint, build), Dockerfiles, more quiz modes, quiz search.

---

## Changelog

- 2026-11-18: Initial build. Scaffold (separate frontend/backend, env templates,
  gitignores), Catppuccin theme system (4 flavors + System + 14 accents), MySQL schema +
  HCI seed + admin bootstrap, public browsing, QCM and Flashcard players with the mode
  registry, staff area (auth, quiz/question CRUD, image uploads to R2 for every image
  field, preview, bulk import), admin features (managers, subjects, orphan cleanup),
  interactive reviews (upload + sandboxed viewer + theme postMessage), docs.
