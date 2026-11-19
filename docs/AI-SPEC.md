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
their personal **API key** (how AI assistants add quizzes and reviews). **Admins** have every manager
permission on all content, plus account, API key, category and subject management and image cleanup.

Stack: Vue 3 + TypeScript + Vite + Vuetify 4 (components, `@mdi/font` icons) + Tailwind CSS 4 (layout/utilities) + AOS + animate.css + Pinia + Vue Router (**Bun**) in `frontend/`;
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
- ✅ Header with the "GIC Review" name, a small "Staff login" link (Dashboard + logout when logged in), a flavor picker, and an icon-only link to the documentation
- ✅ Default font Plus Jakarta Sans, self-hosted at build time by `vite-plugin-webfont-dl` (zero config) and set via Tailwind `--font-sans`; Vuetify reads the same font from `--v-font-body` / `--v-font-heading` (`src/styles/main.css`)
- ✅ Link previews: meta description, Open Graph + Twitter card tags with a 1200×630 preview image, absolute URLs from `VITE_SITE_URL`; favicon.ico/PNG/Apple/Android icons and web manifest
- ✅ Footer credit "Made by: @GoodDay360" linking to https://github.com/GoodDay360 (new tab)
- ✅ Home page: subject cards from `GET /api/subjects`, loading skeletons, empty and error states
- ✅ Category picker first: Home shows category cards (`I3`, `I4`, any custom name) from `GET /api/categories`, last opened category highlighted
- ✅ Category page `/categories/:categorySlug` lists that category's subjects; subject pages link back to their category
- ✅ Only one seeded subject: Human Computer Interaction (`hci`), in the default category "General" (renamable)
- ✅ Activity modal per subject (Interactive Review / Quiz); review shown disabled with "Coming soon" when there is none
- ✅ Interactive review page: list when there are several, sandboxed iframe (`sandbox="allow-scripts"`, no `allow-same-origin`) loaded from the R2 domain, back button
- ✅ Review theme sync via `postMessage({ type: 'gic-theme', flavor, accent })`
- ✅ Per-review scroll position saved in browser `localStorage` and restored through the sandbox-safe `postMessage` bridge; review HTML must implement the scroll protocol
- ✅ Quiz list per subject: published quizzes with title, description, question count, available modes
- ✅ Quiz mode modal built from the mode registry (only modes the quiz supports)
- ✅ Image lightbox (full screen, zoom, Esc closes), `loading="lazy"`, next question's images preloaded
- ✅ Anonymous quiz progress and results kept in the browser only (Pinia + `localStorage`): resume one unfinished QCM/Flashcard attempt, confirm before replacing it, retain completed results, redo completed quizzes, and cap saved results with oldest-first eviction (`VITE_MAX_SAVED_RESULTS`)

### Quiz modes
- ✅ Mode registry `frontend/src/quiz-modes/registry.ts` + backend `QuizMode` enum, `available_modes` computed by the backend
- ✅ QCM: one question at a time, optional question image, text/image options (image grid), single or multi answer, explanation text + image, progress bar and score, shuffled options, question-shuffle toggle, resumable in-progress attempts, end screen with score, percentage, missed list, Retry and Retry missed only
- ✅ Flashcard: text/image front and back (falls back to the correct options), explanation, `object-contain` images with tap-to-zoom that doesn't flip, click/Space to flip, "I knew it" / "Didn't know" with missed cards returning once at the end of the round, resumable in-progress attempts, Space/←/→/1/2 shortcuts, end screen with "Study missed cards again"
- ⏳ More modes (True/False, Matching, Fill-in-the-blank), see `docs/AI-HOW-ADD-QUIZ.md`

### Staff / Admin
- ✅ `/login` with email + password, no public sign-up
- ✅ My account page: change your own password (current password verified first, rate-limited); signs out every other session
- ✅ Role-based dashboard: managers get Quizzes, Bulk import, Interactive reviews, API key; admins get Managers, API keys, Subjects
- ✅ Admins have every manager permission: create content, own API key, and owner rights on **all** quizzes/reviews (`access: "admin"`, "Owned by …" badge)
- ✅ Content ownership: quizzes and reviews are private to the manager who created them (others get 404)
- ✅ Sharing categories and subjects by email (Share buttons on the Categories and Subjects pages): recipients can add content there and view, edit and publish everything in it, but not delete others' items; the category owner/admin manages access, recipients can leave. No per-item sharing (removed).
- ✅ Deleting a manager hands each of their categories to the person it was first shared with
- ✅ Quizzes (manager): create, edit, delete, publish/unpublish (refused while no mode is available)
- ✅ Questions (manager): add, edit, delete, reorder (drag & drop and ↑/↓), options add/remove/reorder
- ✅ `ImageUploader` for every image field (cover, question, options, flashcard back, explanation): browse, drag & drop, Ctrl+V paste, progress bar, preview, alt text, Replace/Remove
- ✅ Images uploaded in an editor session that end up unused are deleted again
- ✅ Preview of a question exactly as students see it, in QCM and Flashcard mode
- ✅ Interactive reviews (manager): upload `.html`, set title/slug/order/published, replace (same URL), delete
- ✅ Bulk import (manager): paste or upload JSON, "Load HCI sample"
- ✅ External API guides for AI clients without code access (`AI-HOW-ADD-QUIZ.md`, `AI-HOW-ADD-INTERACTIVE-REVIEW.md`: self-contained; quick start, numbered endpoint list, step-by-step PDF recipe with curl, per-endpoint details, full schema/template, errors; every documented call verified against the live API) and a developer guide `DEV-ADD-QUIZ-MODE.md`
- ✅ Manager API key page: create one key (shown once with copy button and curl examples), choose permissions and expiry, last-used time, regenerate, revoke
- ✅ Managers (admin only): create, reset password, activate/deactivate, delete
- ✅ API keys overview (admin only): every manager's keys with owner, prefix, scopes, dates and status; filter; revoke
- ✅ Categories belong to their creator: any editor creates categories in the dashboard ("Yours" / "Owned by …"), only the owner or an admin edits/deletes them; seeded "General" has no owner (admins only)
- ✅ Categories and subjects can only be created in the dashboard (API keys get `403 api_key_not_allowed`)
- ✅ New quizzes/reviews only in subjects of categories you own or that are shared with you (category or subject); `403 not_your_category` lists where you can add; admins: all; `GET /admin/categories` (works with API keys) shows `can_manage`, `shared_with_you` and `can_add_content` per category and subject
- ✅ Subjects: managers create/edit/delete them in their own categories, admins in all (category, name, slug, description, color, order; delete when empty); subject pickers show `Category · Subject` and only offer your own categories for new content
- ✅ "Clean up orphaned images" (admin only, on the Subjects page)
- ✅ Frontend route guards per role, mirrored by the backend

### Security
- ✅ Argon2id password hashing
- ✅ JWT session in an httpOnly, SameSite=Strict cookie, Secure in production, expiry from `JWT_EXPIRES_IN_HOURS`
- ✅ Role and `is_active` re-checked from the DB on every authenticated request (deactivation is immediate, for sessions and API keys)
- ✅ API keys: `gic_` + 64 hex chars, only SHA-256 hash stored, shown once, Bearer auth, max 1 active per manager, scopes `quizzes:write` / `reviews:write`, optional expiry, revocable by owner or admin
- ✅ API keys refused on session-only endpoints (users, API key management, sharing, subjects, cleanup), so a leaked key cannot escalate
- ✅ Access checks on every content endpoint (creator, category owner, admin, or shared category/subject; else 404; delete needs full rights → 403 `owner_only`), also for drafts on `GET /quizzes/:id`
- ✅ Login rate limit (5 attempts / minute / IP)
- ✅ In-memory response cache for anonymous public reads (`CACHE_MAX_MB`, `CACHE_TTL_SECONDS`; `x-cache` header); any successful `/api/admin/` write clears it at once (generation-stamped entries), logged-in requests bypass it
- ✅ Password change / admin reset bumps `session_version`; session JWTs with an older `sv` are rejected (other devices signed out)
- ✅ Input validation and parameterized SQL only
- ✅ Uploads: MIME + extension + magic-number checks, size limits from `.env`, random UUID keys, re-encoding strips EXIF
- ✅ CORS limited to `FRONTEND_URL` (credentials + `Authorization` header)
- ✅ JSON errors `{ "error": { "code", "message" } }` without stack traces or SQL, including malformed/incomplete JSON bodies (`AppJson` extractor)
- ✅ First admin bootstrapped from `.env`; no API path creates admins; created users are always managers
- ✅ `ADMIN_PASSWORD` changes in `backend/.env` are applied on restart (sessions signed out); unchanged `.env` keeps a password changed in the dashboard (`users.env_password_hash`)

### Infrastructure
- ✅ Separate `frontend/` and `backend/` projects with their own `.env.example` and `.gitignore`, plus the root `.gitignore`
- ✅ Frontend uses Bun only (`bun.lock`, `packageManager` field, no npm/yarn/pnpm lockfiles)
- ✅ Catppuccin theme: Latte, Frappé, Macchiato, Mocha + System, CSS variables generated from `@catppuccin/palette`, Tailwind `ctp-*` colors, `dark:` for dark flavors, saved in `localStorage`, applied before mount; one Vuetify theme per flavor built from the same palette in `src/plugins/vuetify.ts` (`primary` = chosen accent, `ctp-<color>` theme colors), switched by the theme store
- ✅ Accent picker with the 14 Catppuccin accents (`--ctp-accent`), readable text on every accent
- ✅ Accessible modals (focus trap, Esc, backdrop click, aria labels) and error toasts
- ✅ SQLx migrations for all tables (`users`, `subjects`, `interactive_reviews`, `quizzes`, `questions`, `question_options`, `api_keys`)
- ✅ R2 storage stores keys only; URLs built from `R2_PUBLIC_BASE_URL`; images deleted from R2 when replaced or when their row is deleted
- ✅ Backend fails fast at startup with a clear list of missing/invalid variables
- ✅ Integration tests for auth, roles and API keys (database tests use one test database from `DB_TEST_NAME`, wiped before each test, run one at a time; skipped when `DB_*` is empty)
- ✅ No `DATABASE_URL`: the server, `cargo run -- migrate` and the tests build the connection from `DB_*`
- ✅ `DB_SSL_REQUIRED=true|false` (default `true`) to require an SSL connection to MySQL (validated at startup)
- ✅ MySQL TLS via OpenSSL (native-tls) so managed MySQL with X.509 v1 certificates works (verified with Oracle Cloud MySQL 26.10 HeatWave)
- 🚧 Compile-time checked queries (`sqlx::query!`) and committed `.sqlx/`: not used yet, needs a dev database
- ✅ Backend Dockerfile (multi-stage, non-root, `PORT` support) for Render/Railway, with deployment notes in the README
- ⏳ CI pipeline, frontend deployment docs

---

## Database schema (current)

```
users               id, email UNIQUE, password_hash, session_version, env_password_hash NULL, display_name, role ENUM('admin','manager'),
                    is_active, created_by NULL → users (SET NULL), created_at, updated_at
categories               id, slug UNIQUE, name, description NULL, sort_order, created_by NULL → users (SET NULL)
                    (owner; NULL = admins only), created_at, updated_at
subjects            id, category_id → categories (RESTRICT), slug UNIQUE, name, description NULL, color (accent name), sort_order,
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
category_shares         category_id → categories (CASCADE), user_id → users (CASCADE), granted_by NULL → users,
                    created_at, PK (category_id, user_id)
subject_shares      subject_id → subjects (CASCADE), user_id → users (CASCADE),
                    granted_by NULL → users, created_at, PK (subject_id, user_id)
api_keys            id, user_id → users (CASCADE), name, key_prefix, key_hash UNIQUE (sha-256 hex),
                    scopes ('quizzes:write,reviews:write'), expires_at NULL, last_used_at NULL,
                    revoked_at NULL, created_at
```

Migrations: `20261118000001_init_schema.sql`, `20261118000002_seed_hci_subject.sql`,
`20261118000003_api_keys.sql`, `20261118000004_content_shares.sql`, `20261118000005_years.sql` (creates the default category "General"), `20261118000006_year_owners.sql`, `20261118000007_year_subject_shares.sql` (adds category/subject shares, drops the item share tables),
`20261118000008_session_version.sql`, `20261118000009_admin_env_password.sql`, `20261119000001_rename_years_to_categories.sql` (renames years → categories: tables `categories`, `category_shares`, column `subjects.category_id`). R2 key formats: `quizzes/{quiz_id}/{uuid}.{ext}`, `tmp/{uuid}.{ext}`,
`reviews/{subject_id}/{uuid}.html`.

## API endpoints (current, prefix `/api`)

Auth column: **public**, **M** = manager or admin via session or API key (scope shown), **M-s** = manager
or admin session only, **A-s** = admin session only, **any** = any session or API key.

| Method & path | Auth | Notes |
|---|---|---|
| `GET /health` | public | 503 when the DB is down |
| `GET /categories`, `GET /categories/:slug` | public | categories with `subject_count`; one category with its `subjects` |
| `GET /subjects[?category=slug]`, `GET /subjects/:slug` | public | `category_id`, `category_slug`, `category_name`, `has_review`, `quiz_count` |
| `GET /subjects/:slug/reviews` | public | published reviews with `url` (`?v=` cache-buster) |
| `GET /subjects/:slug/quizzes` | public | published, with `question_count`, `available_modes` |
| `GET /quizzes/:id` | public / M | unpublished only for managers |
| `POST /auth/login`, `POST /auth/logout` | public | login is rate-limited |
| `GET /auth/me` | any | adds `auth_method` and `api_key_scopes` |
| `POST /auth/change-password` | session | `{current_password, new_password}`; signs out other sessions |
| `GET/POST /admin/quizzes`, `GET/PUT/DELETE /admin/quizzes/:id` | M `quizzes:write` | |
| `PATCH /admin/quizzes/:id/publish` | M `quizzes:write` | |
| `POST /admin/quizzes/:id/questions`, `PUT/DELETE …/questions/:qid`, `PUT …/questions/reorder` | M `quizzes:write` | |
| `POST /admin/quizzes/import` | M `quizzes:write` | one transaction |
| `POST /admin/uploads/image`, `DELETE /admin/uploads/image/*key` | M `quizzes:write` or `reviews:write` | |
| `GET/POST /admin/reviews`, `GET/PUT/DELETE /admin/reviews/:id` | M `reviews:write` | multipart `.html`; delete owner-only |
| `GET/POST /admin/categories/:id/shares`, `DELETE …/shares/:user_id` | M-s | view: category owner/admin/recipients; add (`{email}`): category owner or admin; delete: category owner/admin, or yourself to leave |
| `GET/POST /admin/subjects/:id/shares`, `DELETE …/shares/:user_id` | M-s | same rules (managed by the subject's category owner or an admin) |
| `GET /admin/api-keys/mine`, `POST /admin/api-keys` | M-s | 409 when an active key exists (unless `replace_existing`) |
| `DELETE /admin/api-keys/:id` (also `POST …/:id/revoke`) | M-s own / A-s any | |
| `GET /admin/api-keys[?user_id=]` | A-s | all managers' keys |
| `GET/POST /admin/users`, `PATCH/DELETE /admin/users/:id`, `POST …/:id/reset-password` | A-s | managers only |
| `POST /admin/subjects`, `PUT/DELETE /admin/subjects/:id` | M-s | `category_id` required; only in categories you own (admins: any) |
| `GET /admin/categories` | M (any scope) | every category with `owner_id`, `owner_name`, `can_manage`, `subjects` |
| `POST /admin/categories`, `PUT/DELETE /admin/categories/:id` | M-s | create: you become owner; edit/delete: owner or admin; delete refused (409) while it has subjects |
| `POST /admin/uploads/cleanup-orphans` | A-s | |

All editor content endpoints only reach items the caller created, items in their categories, or items in a
category/subject shared with them (admins: all; 404 otherwise), and return `access`
(`owner`/`admin`/`category_owner`/`shared`), `owner_id`, `owner_name` (creator).

Error codes worth knowing: `unauthorized` (401), `forbidden` (403, wrong role), `owner_only` (403),
`api_key_not_allowed` (403), `missing_scope` (403), `conflict` (409), `validation_error` (422),
`too_many_requests` (429).

---

## Known limitations and next steps

- Queries are parameterized `&'static str` SQL, not `query!` macros; no `.sqlx/` yet (no DB was
  available while building). Next: convert hot queries and run `cargo sqlx prepare`.
- The database integration tests need `DB_*` filled in and `DB_TEST_NAME` set to an existing database
  (e.g. `gic_review_test`, never the same as `DB_NAME`) the user has all privileges on; without `DB_*` they are skipped.
- Login rate limiting and the response cache are in memory (per process): with several backend
  instances, another instance may serve a cached response for up to `CACHE_TTL_SECONDS` after an edit.
- Login rate limiting: behind a proxy the client IP is the proxy's.
- Cookie auth needs frontend and API on the same site (`SameSite=Strict`). API keys work from anywhere.
- API key timestamps use the MySQL server clock (`NOW()`); the dashboard shows them in the browser's
  local time, so keep the DB server on UTC or the same timezone.
- No per-key rate limit yet; keys are long random secrets, but a busy AI client is only limited by
  the server.
- Serve R2 from its own domain (not the API host).
- With `DB_SSL_REQUIRED=true` the connection is encrypted but the server certificate is not verified
  (managed MySQL services use self-signed certificates); there is no CA-pinning option yet.
- Import URL downloads check for private IPs before connecting (DNS rebinding could race it).
- `tmp/` image keys are not moved under `quizzes/{id}/` once the quiz exists.

---

## Changelog

- 2026-11-19: **Vuetify 4 UI.** All buttons, forms, inputs, selects, switches, dialogs, menus, alerts, chips,
  progress bars and toasts (`v-snackbar-queue`) are Vuetify components; icons are Material Design Icons
  (`@mdi/font`, `mdi-*` names, `AppIcon.vue` removed). CSS layer order in `main.css` keeps Tailwind utilities
  above Vuetify's layers. AOS (`data-aos`, init in `main.ts`, off for reduced motion) and animate.css
  (`animate__*` page/alert/result entrances). Login shows specific messages: wrong email/password (error),
  too many attempts (warning), server/network errors, empty fields and Caps Lock (warning), "sign in to
  continue" (info) and a welcome toast (success). Toasts take an optional title.
- 2026-11-19: **Years renamed to categories** everywhere (UI, API, database). Endpoints `/categories`, `/admin/categories`, `?category=`; fields `category_id`, `category_slug`, `category_name`; access value `category_owner`; error `not_your_category`; route `/categories/:categorySlug`. Migration `20261119000001_rename_years_to_categories`.
- 2026-11-18: Configurable local quiz-result cap (`VITE_MAX_SAVED_RESULTS`, default 1,000,000) with oldest-first eviction.
- 2026-11-18: QCM and Flashcard progress: resume one unfinished client-side attempt, confirm before replacing it, retain completed results, and show saved results with a redo action.
- 2026-11-18: Interactive review progress: save each review's scroll position in browser storage and restore it through the sandbox-safe iframe message protocol; updated the starter HTML and AI review guide.
- 2026-11-18: Response cache (moka) for anonymous public reads, sized by `CACHE_MAX_MB` / `CACHE_TTL_SECONDS`;
  cleared on every successful editor write; `Cache-Control: no-cache` so browsers never keep old data.
- 2026-11-18: Admin from `.env` synced on every start: a changed `ADMIN_PASSWORD` is applied (sessions
  signed out), an unchanged one keeps dashboard password changes; a new `ADMIN_EMAIL` creates that admin.
- 2026-11-18: My account page with password change (current password required, rate-limited);
  `users.session_version` + JWT `sv` claim so password changes/resets sign out other sessions.
- 2026-11-18: Default font Plus Jakarta Sans, self-hosted with `vite-plugin-webfont-dl`.
- 2026-11-18: Link previews (Open Graph/Twitter tags, `og-image.png`), full icon set (`favicon.ico`, PNGs,
  `apple-touch-icon.png`, `site.webmanifest`) and `VITE_SITE_URL`; fixed Vite's native-config-loader warning.
- 2026-11-18: AI quiz guide: "How to create a new quiz" box; `POST /admin/quizzes` (create an empty quiz)
  added to the endpoint list as #17 (it was missing). Both creation paths verified with an API key.
- 2026-11-18: Sharing moved to categories and subjects (`category_shares`, `subject_shares`, Share buttons on the
  Categories/Subjects pages); per-item quiz/review sharing **removed** (tables dropped). Access values now
  `owner`/`admin`/`category_owner`/`shared`; `share_count` removed. Category owners get full rights on content in their categories.
- 2026-11-18: Categories belong to their creator; managers create categories and subjects in the dashboard (never via
  API keys) and can only add content to subjects of their own categories (`403 not_your_category`). New
  `GET /admin/categories` (works with API keys) shows `can_manage`. Migration `0006_year_owners`.
- 2026-11-18: Admins get every manager permission back: content creation, own API key, and owner
  rights on all content (`access: "admin"`). Extractors renamed `RequireEditor*` / `MaybeEditor`.
- 2026-11-18: Categories: students pick a category (I3, I4, custom names) before a subject. `categories` table,
  `subjects.category_id`, default category "General", public `/categories` endpoints, admin Categories page, category in the
  subject form. Bad JSON bodies now return the standard JSON error.
- 2026-11-18: MySQL TLS switched from rustls to OpenSSL (native-tls): the cloud MySQL's X.509 v1
  certificate failed with `UnsupportedCertVersion`. Dockerfile installs libssl. All tests pass on the cloud DB.
- 2026-11-18: External AI guides restructured around a numbered endpoint list and a step-by-step recipe;
  all 25 documented calls verified against the live API.
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
