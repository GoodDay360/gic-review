# GIC Review: AI-SPEC (project status sheet)

## Keep features in sync (rules for every AI or developer)

1. **The two files are different documents with different jobs. Do not copy one into the other.**
   - `prompt.md` (repo root) is the **build brief**: full instructions on how to build the project
     (tech stack, folder layout, `.gitignore`, `.env` templates, UI/theme rules, security rules,
     schema design, API design, build phases).
   - `docs/AI-SPEC.md` (this file) is the **project status sheet**: what has actually been built so
     far, the current schema and endpoints, how to run it, known limitations, and the changelog.
2. **Only the features/requirements list is kept in sync.** Every user-facing feature or requirement
   must appear in both files:
   - in `prompt.md`, described as a requirement in its matching section (what it should do);
   - here, as one line in the **"Features"** checklist with its status (✅ built / 🚧 in progress / ⏳ planned).

   Everything else stays in its own file and is **not** duplicated: build instructions only in
   `prompt.md`; schema, endpoints, run steps and changelog only here.
3. **Adding or changing a feature:** add or update the requirement in `prompt.md` → add or update its
   line in the Features list below (⏳ or 🚧) → implement it → mark it ✅. Do all of this in the
   **same commit/PR** as the code.
4. **Removing a feature:** remove it from both files and say so in the commit message. Never delete a
   requirement silently.
5. **Sync check:** every feature line here must correspond to a requirement in `prompt.md`, and every
   requirement in `prompt.md` must have a line here (⏳ if not built yet). If one is missing, add it.
6. If a change affects how quizzes or interactive reviews are added (schema, JSON import format,
   image fields, quiz modes, HTML rules, API keys), also update `docs/AI-HOW-ADD-QUIZ.md` and/or
   `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`.
7. Add a dated entry to the **Changelog** at the bottom of this file.

---

## Summary

GIC Review is an exam-review website. Students pick a subject, then open its **Interactive Review**
(a static HTML page in a sandboxed iframe) or practise with **Quizzes** in **QCM** or **Flashcard**
mode, without logging in. **Managers** create the content in the dashboard or through the API with
their personal **API key** (how AI assistants add quizzes and reviews). **Admins** only manage manager
accounts, managers' API keys, subjects and image cleanup.

Stack: Vue 3 + TypeScript + Vite + Tailwind CSS 4 + Pinia + Vue Router (**Bun**) in `frontend/`;
Rust + Axum + Tokio + SQLx (MySQL 8) + Cloudflare R2 (`aws-sdk-s3`) in `backend/`.

## How to run

```bash
# Backend
cd backend
cp .env.example .env               # fill in DB, JWT, admin and R2 values
cargo run -- migrate               # optional: the server also runs migrations on start
cargo run                          # http://127.0.0.1:8080/api/health

# Frontend (Bun only)
cd frontend
cp .env.example .env
bun install
bun run dev                        # http://localhost:5173
```

Checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
(DB tests use `DB_*` from `backend/.env` with the `DB_TEST_NAME` database; skipped when empty), `bun run lint`, `bun run format:check`,
`bun run build`.

---

## Features

Legend: ✅ built · 🚧 in progress / partial · ⏳ planned

### Public site
- ✅ Header with the "GIC Review" name and a small "Staff login" link (Dashboard + logout when logged in)
- ✅ Footer credit "Made by: @GoodDay360" linking to https://github.com/GoodDay360 (new tab)
- ✅ Home page: subject cards from `GET /api/subjects`, loading skeletons, empty and error states
- ✅ Only one seeded subject: Human Computer Interaction (`hci`)
- ✅ Activity modal per subject (Interactive Review / Quiz); review shown disabled with "Coming soon" when there is none
- ✅ Interactive review page: list when there are several, sandboxed iframe (`sandbox="allow-scripts"`, no `allow-same-origin`) loaded from the R2 domain, back button
- ✅ Review theme sync via `postMessage({ type: 'gic-theme', flavor, accent })`
- ✅ Quiz list per subject: published quizzes with title, description, question count, available modes
- ✅ Quiz mode modal built from the mode registry (only modes the quiz supports)
- ✅ Image lightbox (full screen, zoom, Esc closes), `loading="lazy"`, next question's images preloaded
- ✅ Anonymous progress kept in the browser only (Pinia + `localStorage`)

### Quiz modes
- ✅ Mode registry `frontend/src/quiz-modes/registry.ts` + backend `QuizMode` enum, `available_modes` computed by the backend
- ✅ QCM: one question at a time, optional question image, text/image options (image grid), single or multi answer, explanation text + image, progress bar and score, shuffled options, question-shuffle toggle, end screen with score, percentage, missed list, Retry and Retry missed only
- ✅ Flashcard: text/image front and back (falls back to the correct options), explanation, `object-contain` images with tap-to-zoom that doesn't flip, click/Space to flip, "I knew it" / "Didn't know" with missed cards returning at the end of the round, Space/←/→/1/2 shortcuts, end screen with "Study missed cards again"
- ⏳ More modes (True/False, Matching, Fill-in-the-blank), see `docs/AI-HOW-ADD-QUIZ.md`

### Staff / Admin
- ✅ `/login` with email + password, no public sign-up
- ✅ Role-based dashboard: managers get Quizzes, Bulk import, Interactive reviews, API key; admins get Managers, API keys, Subjects
- ✅ Admins cannot create or edit content (backend returns 403; dashboard hides it)
- ✅ Content ownership: quizzes and reviews are private to the manager who created them (others get 404)
- ✅ Sharing by email: owner shares a quiz/review with other managers (Share dialog), shared managers can edit and publish, only the owner deletes or manages access, shared managers can leave; badges "Shared by …" / "Shared with N"
- ✅ Deleting a manager hands each of their items to the manager it was shared with first
- ✅ Quizzes (manager): create, edit, delete, publish/unpublish (refused while no mode is available)
- ✅ Questions (manager): add, edit, delete, reorder (drag & drop and ↑/↓), options add/remove/reorder
- ✅ `ImageUploader` for every image field (cover, question, options, flashcard back, explanation): browse, drag & drop, Ctrl+V paste, progress bar, preview, alt text, Replace/Remove
- ✅ Images uploaded in an editor session that end up unused are deleted again
- ✅ Preview of a question exactly as students see it, in QCM and Flashcard mode
- ✅ Interactive reviews (manager): upload `.html`, set title/slug/order/published, replace (same URL), delete
- ✅ Bulk import (manager): paste or upload JSON, "Load HCI sample"
- ✅ External API guides for AI clients without code access (`AI-HOW-ADD-QUIZ.md`, `AI-HOW-ADD-INTERACTIVE-REVIEW.md`: self-contained, PDF → content workflow, full schema, template, errors) and a developer guide `DEV-ADD-QUIZ-MODE.md`
- ✅ Manager API key page: create one key (shown once with copy button and curl examples), choose permissions and expiry, last-used time, regenerate, revoke
- ✅ Managers (admin only): create, reset password, activate/deactivate, delete
- ✅ API keys overview (admin only): every manager's keys with owner, prefix, scopes, dates and status; filter; revoke
- ✅ Subjects (admin only): create, edit (name, slug, description, color, order), delete when empty
- ✅ "Clean up orphaned images" (admin only, on the Subjects page)
- ✅ Frontend route guards per role, mirrored by the backend

### Security
- ✅ Argon2id password hashing
- ✅ JWT session in an httpOnly, SameSite=Strict cookie, Secure in production, expiry from `JWT_EXPIRES_IN_HOURS`
- ✅ Role and `is_active` re-checked from the DB on every authenticated request (deactivation is immediate, for sessions and API keys)
- ✅ API keys: `gic_` + 64 hex chars, only SHA-256 hash stored, shown once, Bearer auth, max 1 active per manager, scopes `quizzes:write` / `reviews:write`, optional expiry, revocable by owner or admin
- ✅ API keys refused on session-only endpoints (users, API key management, sharing, subjects, cleanup), so a leaked key cannot escalate
- ✅ Access checks on every content endpoint (owner or shared, else 404; owner-only delete → 403 `owner_only`), also for drafts on `GET /quizzes/:id`
- ✅ Login rate limit (5 attempts / minute / IP)
- ✅ Input validation and parameterized SQL only
- ✅ Uploads: MIME + extension + magic-number checks, size limits from `.env`, random UUID keys, re-encoding strips EXIF
- ✅ CORS limited to `FRONTEND_URL` (credentials + `Authorization` header)
- ✅ JSON errors `{ "error": { "code", "message" } }` without stack traces or SQL
- ✅ First admin bootstrapped from `.env`; no API path creates admins; created users are always managers

### Infrastructure
- ✅ Separate `frontend/` and `backend/` projects with their own `.env.example` and `.gitignore`, plus the root `.gitignore`
- ✅ Frontend uses Bun only (`bun.lock`, `packageManager` field, no npm/yarn/pnpm lockfiles)
- ✅ Catppuccin theme: Latte, Frappé, Macchiato, Mocha + System, CSS variables generated from `@catppuccin/palette`, Tailwind `ctp-*` colors, `dark:` for dark flavors, saved in `localStorage`, applied before mount
- ✅ Accent picker with the 14 Catppuccin accents (`--ctp-accent`), readable text on every accent
- ✅ Accessible modals (focus trap, Esc, backdrop click, aria labels) and error toasts
- ✅ SQLx migrations for all tables (`users`, `subjects`, `interactive_reviews`, `quizzes`, `questions`, `question_options`, `api_keys`)
- ✅ R2 storage stores keys only; URLs built from `R2_PUBLIC_BASE_URL`; images deleted from R2 when replaced or when their row is deleted
- ✅ Backend fails fast at startup with a clear list of missing/invalid variables
- ✅ Integration tests for auth, roles and API keys (database tests use one test database from `DB_TEST_NAME`, wiped before each test, run one at a time; skipped when `DB_*` is empty)
- ✅ No `DATABASE_URL`: the server, `cargo run -- migrate` and the tests build the connection from `DB_*`
- ✅ `DB_SSL_REQUIRED=true|false` (default `true`) to require an SSL connection to MySQL (validated at startup)
- 🚧 Compile-time checked queries (`sqlx::query!`) and committed `.sqlx/`: not used yet, needs a dev database
- ✅ Backend Dockerfile (multi-stage, non-root, `PORT` support) for Render/Railway, with deployment notes in the README
- ⏳ CI pipeline, frontend deployment docs

---

## Database schema (current)

```
users               id, email UNIQUE, password_hash, display_name, role ENUM('admin','manager'),
                    is_active, created_by NULL → users (SET NULL), created_at, updated_at
subjects            id, slug UNIQUE, name, description NULL, color (accent name), sort_order,
                    created_at, updated_at
interactive_reviews id, subject_id → subjects (RESTRICT), title, slug (UNIQUE per subject), r2_key,
                    sort_order, is_published, created_by NULL → users (SET NULL), created_at, updated_at
quizzes             id, subject_id → subjects (RESTRICT), title, description NULL, cover_image_key NULL,
                    is_published, created_by NULL → users (SET NULL), created_at, updated_at
questions           id, quiz_id → quizzes (CASCADE), position, prompt NULL, image_key NULL,
                    image_alt NULL, answer_text NULL, answer_image_key NULL, answer_image_alt NULL,
                    explanation NULL, explanation_image_key NULL, multi_select, created_at, updated_at
question_options    id, question_id → questions (CASCADE), position, text NULL, image_key NULL,
                    image_alt NULL, is_correct
quiz_shares         quiz_id → quizzes (CASCADE), user_id → users (CASCADE), granted_by NULL → users,
                    created_at, PK (quiz_id, user_id)
review_shares       review_id → interactive_reviews (CASCADE), user_id → users (CASCADE),
                    granted_by NULL → users, created_at, PK (review_id, user_id)
api_keys            id, user_id → users (CASCADE), name, key_prefix, key_hash UNIQUE (sha-256 hex),
                    scopes ('quizzes:write,reviews:write'), expires_at NULL, last_used_at NULL,
                    revoked_at NULL, created_at
```

Migrations: `20261118000001_init_schema.sql`, `20261118000002_seed_hci_subject.sql`,
`20261118000003_api_keys.sql`, `20261118000004_content_shares.sql`. R2 key formats: `quizzes/{quiz_id}/{uuid}.{ext}`, `tmp/{uuid}.{ext}`,
`reviews/{subject_id}/{uuid}.html`.

## API endpoints (current, prefix `/api`)

Auth column: **public**, **M** = manager via session or API key (scope shown), **M-s** = manager
session only, **A-s** = admin session only, **any** = any session or API key.

| Method & path | Auth | Notes |
|---|---|---|
| `GET /health` | public | 503 when the DB is down |
| `GET /subjects`, `GET /subjects/:slug` | public | `has_review`, `quiz_count` |
| `GET /subjects/:slug/reviews` | public | published reviews with `url` (`?v=` cache-buster) |
| `GET /subjects/:slug/quizzes` | public | published, with `question_count`, `available_modes` |
| `GET /quizzes/:id` | public / M | unpublished only for managers |
| `POST /auth/login`, `POST /auth/logout` | public | login is rate-limited |
| `GET /auth/me` | any | adds `auth_method` and `api_key_scopes` |
| `GET/POST /admin/quizzes`, `GET/PUT/DELETE /admin/quizzes/:id` | M `quizzes:write` | |
| `PATCH /admin/quizzes/:id/publish` | M `quizzes:write` | |
| `POST /admin/quizzes/:id/questions`, `PUT/DELETE …/questions/:qid`, `PUT …/questions/reorder` | M `quizzes:write` | |
| `POST /admin/quizzes/import` | M `quizzes:write` | one transaction |
| `POST /admin/uploads/image`, `DELETE /admin/uploads/image/*key` | M `quizzes:write` or `reviews:write` | |
| `GET/POST /admin/reviews`, `GET/PUT/DELETE /admin/reviews/:id` | M `reviews:write` | multipart `.html`; delete owner-only |
| `GET/POST /admin/quizzes/:id/shares`, `DELETE …/shares/:user_id` | M-s | add = owner only (`{email}`); delete = owner, or yourself to leave |
| `GET/POST /admin/reviews/:id/shares`, `DELETE …/shares/:user_id` | M-s | same rules as quizzes |
| `GET /admin/api-keys/mine`, `POST /admin/api-keys` | M-s | 409 when an active key exists (unless `replace_existing`) |
| `DELETE /admin/api-keys/:id` (also `POST …/:id/revoke`) | M-s own / A-s any | |
| `GET /admin/api-keys[?user_id=]` | A-s | all managers' keys |
| `GET/POST /admin/users`, `PATCH/DELETE /admin/users/:id`, `POST …/:id/reset-password` | A-s | managers only |
| `POST /admin/subjects`, `PUT/DELETE /admin/subjects/:id` | A-s | |
| `POST /admin/uploads/cleanup-orphans` | A-s | |

All manager content endpoints only reach items the manager owns or that were shared with them
(404 otherwise) and return `access`, `owner_id`, `owner_name`, `share_count`.

Error codes worth knowing: `unauthorized` (401), `forbidden` (403, wrong role), `owner_only` (403),
`api_key_not_allowed` (403), `missing_scope` (403), `conflict` (409), `validation_error` (422),
`too_many_requests` (429).

---

## Known limitations and next steps

- Queries are parameterized `&'static str` SQL, not `query!` macros; no `.sqlx/` yet (no DB was
  available while building). Next: convert hot queries and run `cargo sqlx prepare`.
- The database integration tests need `DB_*` filled in and `DB_TEST_NAME` set to an existing database
  (e.g. `gic_review_test`, never the same as `DB_NAME`) the user has all privileges on; without `DB_*` they are skipped.
- Login rate limiting is in memory (per process); behind a proxy the client IP is the proxy's.
- Cookie auth needs frontend and API on the same site (`SameSite=Strict`). API keys work from anywhere.
- API key timestamps use the MySQL server clock (`NOW()`); the dashboard shows them in the browser's
  local time, so keep the DB server on UTC or the same timezone.
- No per-key rate limit yet; keys are long random secrets, but a busy AI client is only limited by
  the server.
- Serve R2 from its own domain (not the API host).
- Import URL downloads check for private IPs before connecting (DNS rebinding could race it).
- `tmp/` image keys are not moved under `quizzes/{id}/` once the quiz exists.

---

## Changelog

- 2026-11-18: Content ownership and sharing between managers by email (`quiz_shares`, `review_shares`,
  share endpoints, Share dialog, owner-only delete, handover on manager deletion).
- 2026-11-18: `AI-HOW-ADD-QUIZ.md` and `AI-HOW-ADD-INTERACTIVE-REVIEW.md` rewritten as self-contained
  external API guides for AI clients; quiz-mode developer guide moved to `DEV-ADD-QUIZ-MODE.md`.
- 2026-11-18: Footer credit "Made by: @GoodDay360" → https://github.com/GoodDay360.
- 2026-11-18: Added `backend/Dockerfile` + `.dockerignore`; the server uses `PORT` when the platform sets it.
- 2026-11-18: Initial build. Scaffold, Catppuccin theme system, MySQL schema + HCI seed + admin
  bootstrap, public browsing, QCM and Flashcard players with the mode registry, staff area (quiz CRUD,
  image uploads to R2, preview, bulk import), admin features, interactive reviews, docs.
- 2026-11-18: Manager API keys (one per manager, scopes, expiry, regenerate, revoke) with Bearer
  authentication for quiz and review endpoints; admin API key overview.
- 2026-11-18: Roles changed: admins now only manage manager accounts, API keys, subjects and orphan
  cleanup; quizzes, reviews and uploads are manager-only.
- 2026-11-18: Frontend switched to Bun (`bun.lock`, `packageManager`), per the updated build brief.
- 2026-11-18: Database tests use a single database named by `DB_TEST_NAME` (`gic_review_test`); all 7 pass against MySQL 8.4
- 2026-11-18: Database tests use a single `gic_review_test` database (wiped per test, run serially)
  instead of creating `gic_test_*` databases; only one extra grant is needed.
- 2026-11-18: Added `DB_SSL_REQUIRED` (true/false, default true) to require SSL for the MySQL connection.
- 2026-11-18: Removed `DATABASE_URL`; added `cargo run -- migrate`; database tests create their own
  temporary database from `DB_*` instead of using `#[sqlx::test]`.
- 2026-11-18: AI-SPEC restructured to the "Keep features in sync" rules (Features checklist only is
  shared with `prompt.md`).
