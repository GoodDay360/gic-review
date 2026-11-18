# Build "GIC Review" — Exam Review Website

You are working inside an **existing Git repository** (`gic-review`). Before writing code, inspect the repo (README, existing folders, `.gitignore`) and build on what is there. Do not delete or overwrite existing files unless they conflict with this spec; if they do, explain what you changed.

Work in the phases listed at the bottom. At the end of every phase the project must build and run.

---

## 1. Product overview

**GIC Review** is a study website where students first choose their **year** (e.g. `I3`, `I4`, or any custom name like "Very Cool Year"), then a **subject** of that year, and then either:
- open an **Interactive Review**: a static HTML review page made for that subject, or
- take a **Quiz**: pick a quiz made by an admin/manager, then choose how to study it (**QCM** or **Flashcard**).

Anyone can browse and do quizzes **without logging in**. Only staff accounts can log in: **managers** create content (in the dashboard, or through the API with their personal **API key**, which is how AI assistants add quizzes and reviews); **admins** only manage manager accounts, managers' API keys and subjects.

---

## 2. Tech stack (required)

| Layer | Technology |
|---|---|
| Frontend | Vue 3 (Composition API, `<script setup>`), TypeScript, Vite, Tailwind CSS, Vue Router, Pinia. **Package manager and runtime: Bun** (not npm, yarn or pnpm) |
| Backend | Rust (stable), **Axum**, **Tokio**, **SQLx** (MySQL, compile-time-checked queries where practical), `serde`, `tower-http` (CORS, tracing, limits), `argon2` (password hashing), `jsonwebtoken` (JWT), `validator`, `thiserror`/`anyhow`, `tracing` |
| Database | MySQL 8, with SQLx migrations (files in `backend/migrations/`), applied automatically on server start or with `cargo run -- migrate` |
| File storage | **Cloudflare R2** (S3-compatible) for Interactive Review HTML files and images, used through `aws-sdk-s3` with a custom endpoint |
| Config | All secrets and credentials come from each project's own `.env` (`frontend/.env`, `backend/.env`); never hard-coded, never committed |

> **Backend framework rule:** the Rust backend MUST use **Axum** (latest stable) on **Tokio**. Do not use Actix-web, Rocket, Warp, Poem or any other web framework. Build everything with Axum's own tools: `Router` for routes, `State` for shared state (DB pool, R2 client, config), extractors (`Json`, `Path`, `Query`, `Multipart`) for request input, Tower layers (`tower-http`) for CORS, tracing, body limits and timeouts, and custom extractors for auth (`RequireStaff`, `RequireAdmin`).

### Repository layout

One Git repo holds two **fully separate, self-contained projects**. Each one installs, builds, runs and deploys from inside its own folder, with no imports or shared code between them. They talk to each other only over HTTP.

```
gic-review/                    # single Git repo
├── frontend/                  # Vue app (own package.json, own config, own .env)
│   ├── src/
│   ├── public/
│   ├── index.html
│   ├── package.json
│   ├── bun.lock               # committed (Bun text lockfile); no package-lock.json / yarn.lock / pnpm-lock.yaml
│   ├── vite.config.ts
│   ├── tailwind config / CSS entry
│   ├── tsconfig.json
│   ├── .env.example           # frontend-only vars (VITE_*), committed
│   └── .gitignore             # frontend-specific ignores
├── backend/                   # Rust Axum API (own Cargo project, own .env)
│   ├── src/
│   ├── migrations/
│   ├── .sqlx/                 # sqlx offline query data, committed (`cargo sqlx prepare`)
│   ├── Cargo.toml
│   ├── Cargo.lock             # committed (it's a binary)
│   ├── .env.example           # backend-only vars (DB, JWT, R2, admin), committed
│   └── .gitignore             # backend-specific ignores
├── docs/
│   ├── AI-SPEC.md
│   ├── AI-HOW-ADD-QUIZ.md
│   ├── AI-HOW-ADD-INTERACTIVE-REVIEW.md
│   └── DEV-ADD-QUIZ-MODE.md
├── prompt.md                  # THIS file: build brief (feature list kept in sync with docs/AI-SPEC.md)
├── .gitignore                 # repo-wide ignores (OS/editor files, any .env anywhere)
└── README.md                  # how to set up and run each part
```

Rules:
- Run frontend commands from `frontend/` with **Bun** (`bun install`, `bun run dev`, `bun run build`, `bun run lint`, `bun add <pkg>`, `bunx <tool>`) and backend commands from `backend/` (`cargo run`, `cargo run -- migrate`). There is no root `package.json` and no root Cargo workspace.
- **Frontend uses Bun only.** Scaffold with `bun create vite frontend --template vue-ts`, install with `bun install`, and add packages with `bun add` / `bun add -d`. Never run `npm`, `yarn` or `pnpm`, and never commit their lockfiles. Add `"packageManager": "bun@<current version>"` to `frontend/package.json`, and write all README and docs instructions with `bun` commands. Vite is still the dev server and bundler; Bun runs it through the `package.json` scripts (`"dev": "vite"`, `"build": "vue-tsc -b && vite build"`, `"preview": "vite preview"`).
- **Backend secrets must never reach the frontend.** The frontend has its own `.env` with only `VITE_*` values; DB, JWT and R2 credentials live only in `backend/.env`.
- Both `.env` files are git-ignored. Only the `.env.example` files are committed.

### `.gitignore` files (create all three exactly like this, then add anything else the tooling needs)

**Root `/.gitignore`**
```gitignore
# Secrets: any .env anywhere in the repo, but keep the templates
.env
.env.*
**/.env
**/.env.*
!**/.env.example

# OS / editors
.DS_Store
Thumbs.db
desktop.ini
.idea/
.vscode/*
!.vscode/extensions.json
*.swp
*.swo

# Logs
*.log
logs/
```

**`frontend/.gitignore`**
```gitignore
node_modules/
dist/
dist-ssr/
*.local
.vite/
coverage/
*.log
# only Bun is used, so ignore other package managers' lockfiles if one gets created by accident
package-lock.json
yarn.lock
pnpm-lock.yaml
.env
.env.*
!.env.example
```

**`backend/.gitignore`**
```gitignore
/target/
**/*.rs.bk
*.pdb
.env
.env.*
!.env.example
# keep these committed: Cargo.lock, .sqlx/, migrations/
```

Before the first commit, run `git status` and confirm that no `.env`, `node_modules/` or `target/` files are staged. If any were committed earlier, remove them from the index with `git rm --cached` (the files stay on disk).

---

## 3. User flow and pages

### 3.1 Home page `/`: select a year
- Header shows the site name **"GIC Review"** (logo text) and a small "Staff login" link.
- Footer (every page) shows the credit **"Made by: @GoodDay360"**, linking to https://github.com/GoodDay360 (opens in a new tab).
- Shows a grid of **year cards** loaded from `GET /api/years` (name, optional description, number of subjects), ordered by the admin's sort order. A year's name is free text: `I3`, `I4`, `Very Cool Year`, …
- Clicking a year navigates to `/years/:yearSlug`. The last year this browser opened is highlighted ("Last opened", stored in `localStorage`).
- **Seed only ONE subject:** `Human Computer Interaction` (slug `hci`). Do NOT create other subjects (Internet Programming, etc.); they will be added later through the admin panel. The years migration creates one default year, **"General"** (slug `general`), and puts existing subjects in it; admins rename it (e.g. to `I4`) or add more years.

### 3.1b Year page `/years/:yearSlug`: select a subject
- Shows the year's name/description, an "All years" back link, and a grid of that year's **subject cards** (`GET /api/years/:slug` returns the year with its subjects).
- Clicking a subject opens a **modal popup** (no navigation) titled with the subject name, with two choices:
  1. **Interactive Review**: navigates to `/subjects/:subjectSlug/review`
  2. **Quiz**: navigates to `/subjects/:subjectSlug/quizzes`
- If the subject has no interactive review yet, show that option as disabled with the text "Coming soon".

### 3.2 Interactive Review page `/subjects/:subjectSlug/review`
- Shows the subject's interactive review (a static HTML file stored in R2).
- If a subject has several reviews, show a simple list first and open the selected one.
- Render the HTML inside a **sandboxed `<iframe>`** (`sandbox="allow-scripts"`, do NOT add `allow-same-origin`) loaded from the **R2 public domain**, never from the main site origin. This keeps scripts in the uploaded HTML from reaching the main app's cookies or DOM.
- Include a back button to the home page.

### 3.3 Quiz list page `/subjects/:subjectSlug/quizzes`
- Lists all **published** quizzes for the subject (title, description, number of questions, available modes).
- Clicking a quiz opens a **modal popup** (no navigation) where the user picks a quiz mode:
  - **QCM** (multiple choice)
  - **Flashcard**
- Only show modes the quiz supports (see 4.3). Choosing a mode navigates to `/quizzes/:quizId/play?mode=qcm` or `?mode=flashcard`.
- The modal must be built from a **mode registry** so new modes can be added later without editing the modal (see 6).

### 3.4 Quiz play page `/quizzes/:quizId/play?mode=...`
**Images in both modes:** clicking any image opens it full-screen in a lightbox (zoomable, closes with Esc). Images use `loading="lazy"`, and the next question's images are preloaded so they appear without delay.

**QCM mode**
- One question at a time, with an optional question image.
- Options can be text, image, or both. When options have images, show them as a responsive grid of image cards instead of a vertical list.
- After answering, show the explanation text and the explanation image, if there is one.
- Options can be single-answer or multi-answer (the question has a flag).
- The user submits an answer, sees right or wrong plus the explanation, then clicks Next.
- Shows a progress bar and score. End screen: score, percentage, list of missed questions, "Retry" and "Retry missed only" buttons.
- Option order is shuffled each attempt; question order can be shuffled with a toggle.

**Flashcard mode**
- **Card front** = question text and/or the question image (`image_key`). Image-only fronts are allowed, for example a UI screenshot with no text where the student must name the design principle.
- **Card back** = answer text and/or the answer image (`answer_image_key`), plus the explanation. Image-only backs are allowed too.
- So a card can be text→text, image→text, text→image or image→image. Images on cards scale to fit without cropping (`object-contain`) and can be tapped to zoom without flipping the card. Click or press Space to flip.
- Buttons: "I knew it" / "Didn't know". Cards marked "Didn't know" come back at the end of the round.
- Keyboard shortcuts: Space = flip, ← / → = previous/next, 1 / 2 = knew / didn't know.
- End screen with counts and "Study missed cards again".

Progress for anonymous users is kept only in the browser (Pinia + `localStorage`). No account is needed.

### 3.5 Staff area
- `/login`: email + password login for admin/manager only. There is no public sign-up.
- `/admin` is the dashboard. Its menu depends on the role (managers: Quizzes, Bulk import, Interactive reviews, API key; admins: Managers, API keys, Years, Subjects).
- Manager dashboard (**manager only**; admins cannot create or edit content):
  - **Quizzes**: create, edit, delete, publish/unpublish quizzes; add, edit and reorder questions and options.
  - **Image inputs**: every image field (quiz cover, question, each option, flashcard back, explanation) uses a reusable `ImageUploader` component. It supports click-to-browse, **drag & drop**, and **paste from clipboard (Ctrl+V)**, which is handy for screenshots. It shows an upload progress bar, a preview thumbnail, an alt-text field, and Replace/Remove buttons.
  - **Preview**: a "Preview" button shows the question exactly as students will see it in QCM and Flashcard mode, images included.
  - **Interactive Reviews**: upload an `.html` file for a subject, set title and order, replace or delete it.
  - **Bulk import**: paste or upload JSON in the format defined in `docs/AI-HOW-ADD-QUIZ.md` to create a whole quiz at once.
  - **Sharing**: every quiz and interactive review has a **Share** button. The owner types another manager's email to give them access, sees who has access, and can remove people; a manager something was shared with sees "Shared by <owner>", can edit and publish it, and can **leave**. Lists show "Shared by …" / "Shared with N" badges, and the delete button only appears for the owner.
  - **API key** (`/admin/api-key`): a manager can create **one** personal API key (shown once, with a copy button and curl examples), choose its permissions (`quizzes:write`, `reviews:write`) and expiry (30/90/365 days or never), see when it was last used, **regenerate** it (the old key stops working immediately) or revoke it. Scripts and AI assistants send it as `Authorization: Bearer gic_…` to add quizzes and interactive reviews through the API.
- `/admin/users` (**admin only**): create manager accounts, reset a manager's password, deactivate or delete managers.
- `/admin/api-keys` (**admin only**): see every manager's API keys (owner, name, prefix, permissions, created/last used/expiry, status), filter by manager or status, and revoke any key. Admins cannot create API keys.
- `/admin/years` (**admin only**): create, rename, reorder and delete years (name, link name/slug, description, order); a year can only be deleted when it has no subjects.
- `/admin/subjects` (**admin only**): create and edit subjects (**year**, name, slug, description, icon/color, order), plus the "clean up orphaned images" maintenance action.
- Route guards hide staff pages from users without the role, AND the backend enforces the same rules (never trust the frontend).

---

## 4. Roles and permissions

| Action | Guest (no login) | Manager | Admin |
|---|---|---|---|
| View subjects, reviews, quizzes; play quizzes | ✅ | ✅ | ✅ |
| Create quizzes and interactive reviews (dashboard or API key) | ❌ | ✅ | ❌ |
| Edit/publish **own** content and content **shared with them** | ❌ | ✅ | ❌ |
| Delete content and manage who it is shared with | ❌ | owner only | ❌ |
| See other managers' unshared content | ❌ | ❌ | ❌ |
| Preview unpublished quizzes they can manage | ❌ | ✅ | ❌ |
| Create/regenerate/revoke **own** API key (max 1 active) | ❌ | ✅ | ❌ |
| See and revoke **any manager's** API keys | ❌ | ❌ | ✅ |
| Create/edit years and subjects, clean up orphaned images | ❌ | ❌ | ✅ |
| Create/manage manager accounts | ❌ | ❌ | ✅ |

- **Content ownership and sharing:** the owner of a quiz or interactive review is the manager who created it (`created_by`). Content is private to its owner in the manager area (others get `404`). The owner shares it with other active managers **by email**; shared managers can view, edit (questions, images, file) and publish it, in the dashboard or with their API key, but only the owner can delete it or add/remove people; a shared manager can remove themselves. Sharing is dashboard-only (API keys get `403`). When an owner account is deleted, each of their items goes to the manager it was shared with first; never-shared items stay online (if published) without an owner. Students are unaffected: published content is public.
- **Admins cannot do anything else:** they only manage manager accounts, managers' API keys and subjects (+ orphan cleanup). The backend returns `403` when an admin calls a content endpoint.

- **Initial admin bootstrap:** when the backend starts, if no admin exists, create one from `ADMIN_EMAIL` / `ADMIN_PASSWORD` in `backend/.env`. Log a warning telling the operator to change the password. Never log the password itself.
- Only an admin can create users, and created users are always `manager`. There is no API path to create another admin; that is done manually in the database.

### 4.1 Auth and security requirements
- Hash passwords with **Argon2id**.
- Login returns a JWT stored in an **httpOnly, Secure (in production), SameSite=Strict cookie**. Expiry comes from `JWT_EXPIRES_IN_HOURS`.
- Add Axum extractors for the roles: `RequireManager<Scope>` (manager via session **or** API key with the needed scope), `RequireManagerSession`, `RequireAdmin` and `RequireSession` (dashboard session only). Check the user's role and `is_active` from the database on each request, so a deactivated manager (and their API key) loses access right away.
- **API keys:**
  - Format `gic_` + 64 hex characters from a CSPRNG. Store only the SHA-256 hash plus a short display prefix; show the full key once, at creation.
  - Sent as `Authorization: Bearer <key>`. If the header is present, the cookie is ignored. CORS allows the `Authorization` header.
  - Only managers own keys, at most **1 active key per manager** (enforced in a transaction). "Regenerate" revokes the active key and creates a new one atomically.
  - Scopes: `quizzes:write` (quizzes, questions, reorder, publish, import, image uploads) and `reviews:write` (interactive reviews, image uploads). A key without the needed scope gets `403 missing_scope`.
  - Keys never work on session-only endpoints (users, API key management, subjects, orphan cleanup): `403 api_key_not_allowed`. So a leaked key cannot create more keys or escalate.
  - Optional expiry (1–365 days), `last_used_at` tracking, revocation by the owner or any admin. Deleting a manager deletes their keys; deactivating them disables the keys.
- Rate-limit `POST /api/auth/login` (for example 5 attempts per minute per IP).
- Validate every input. Use parameterized SQLx queries only, never string-concatenated SQL.
- Uploads: check MIME type and extension (images: png/jpg/webp/gif; reviews: `.html` only), enforce size limits from `backend/.env`, and generate random object keys (UUID). Never use the original filename as the key.
- Set CORS to `FRONTEND_URL` only.
- Return errors as JSON `{ "error": { "code": "...", "message": "..." } }` with no stack traces or SQL errors in responses.

### 4.2 Database schema (MySQL, via migrations)

```
users              (id, email UNIQUE, password_hash, display_name, role ENUM('admin','manager'),
                    is_active BOOL, created_by NULL FK users, created_at, updated_at)
years              (id, slug UNIQUE, name, description NULL, sort_order, created_at, updated_at)
subjects           (id, year_id FK years ON DELETE RESTRICT, slug UNIQUE, name, description, color, sort_order,
                    created_at, updated_at)
interactive_reviews(id, subject_id FK, title, slug, r2_key, sort_order, is_published,
                    created_by FK users, created_at, updated_at)
quizzes            (id, subject_id FK, title, description,
                    cover_image_key VARCHAR NULL,        -- R2 key, shown on the quiz card
                    is_published, created_by FK users, created_at, updated_at)
questions          (id, quiz_id FK ON DELETE CASCADE, position,
                    prompt TEXT NULL,                    -- text and/or image required
                    image_key VARCHAR NULL, image_alt VARCHAR NULL,               -- question image
                    answer_text TEXT NULL,               -- flashcard back / short answer
                    answer_image_key VARCHAR NULL, answer_image_alt VARCHAR NULL, -- flashcard back image
                    explanation TEXT NULL,
                    explanation_image_key VARCHAR NULL,  -- e.g. a diagram shown after answering
                    multi_select BOOL DEFAULT false,
                    created_at, updated_at)
quiz_shares        (quiz_id FK ON DELETE CASCADE, user_id FK users ON DELETE CASCADE, granted_by NULL FK users,
                    created_at, PRIMARY KEY (quiz_id, user_id))
review_shares      (review_id FK ON DELETE CASCADE, user_id FK users ON DELETE CASCADE, granted_by NULL FK users,
                    created_at, PRIMARY KEY (review_id, user_id))
api_keys           (id, user_id FK users ON DELETE CASCADE, name, key_prefix, key_hash UNIQUE (sha-256 hex),
                    scopes, expires_at NULL, last_used_at NULL, revoked_at NULL, created_at)
question_options   (id, question_id FK ON DELETE CASCADE, position,
                    text TEXT NULL,                      -- text and/or image required
                    image_key VARCHAR NULL, image_alt VARCHAR NULL,               -- image answer choice
                    is_correct BOOL)
```

The database stores only R2 object keys, never full URLs. The API builds full URLs as `R2_PUBLIC_BASE_URL + "/" + key`, so the storage domain can change later without editing data. Validation rule: a question needs `prompt` or `image_key` (or both), and an option needs `text` or `image_key` (or both).

### 4.2.1 Images in quizzes
Managers can add images to:
- the **quiz** (cover image on the quiz card),
- the **question** (for example a screenshot of a UI to evaluate),
- each **answer option** (image-based choices, such as "Which of these layouts follows Fitts's law?"),
- the **flashcard front** (this is the question image, so the same field serves the QCM question and the flashcard front),
- the **flashcard back** (answer image),
- the **explanation** (a diagram shown after answering).

Requirements:
- **Upload flow:** the image goes to `POST /api/admin/uploads/image` first, which returns `{ key, url }`; the key is then saved on the quiz, question or option.
- **Validation:** only png, jpg/jpeg, webp and gif are accepted. The backend checks real file bytes (magic numbers), not just the extension, and enforces `MAX_IMAGE_SIZE_MB`.
- **Re-encoding:** images are optionally re-encoded and resized server-side (for example max 1600px wide, using the `image` crate) to strip EXIF metadata and keep pages fast.
- **Object keys:** use the form `quizzes/{quiz_id}/{uuid}.{ext}` for quiz images (use a `tmp/` prefix if the quiz doesn't exist yet).
- **Cleanup:** when an image is replaced or removed, or its question, option or quiz is deleted, delete the old object from R2 as well. Add an admin-only "clean up orphaned images" action that removes R2 objects no row references.
- **Alt text:** encourage alt text with an optional field in the editor, used for accessibility.

Seed migration: insert only the subject `Human Computer Interaction` (slug `hci`).

### 4.3 How quiz modes use the data
- Questions are **mode-agnostic**. One quiz can be played in several modes.
- **QCM** is available when every question has at least 2 options (each with text or an image) and at least 1 correct option.
- **Flashcard** is available when every question has `answer_text` or an `answer_image_key`, or has at least one correct option. In the last case, the back of the card shows the correct option text(s) and image(s).
- The backend computes `available_modes` for each quiz and returns it in the API, so the frontend never guesses.

---

## 5. API (REST, JSON, prefix `/api`)

Public:
- `GET  /years` (with `subject_count`), `GET /years/:slug` (the year plus its `subjects`)
- `GET  /subjects[?year=<year slug>]`: every subject includes `year_id`, `year_slug`, `year_name`
- `GET  /subjects/:slug`, which includes a `has_review` flag
- `GET  /subjects/:slug/reviews`, which returns published reviews with their `url` (public R2 URL)
- `GET  /subjects/:slug/quizzes`, which returns published quizzes with `question_count` and `available_modes`
- `GET  /quizzes/:id`, which returns the quiz plus questions and options, with full image URLs for every image field (`cover_image_url`, `image_url`, `answer_image_url`, `explanation_image_url`, and each option's `image_url`). Published quizzes only, unless the caller is staff.
- `GET  /health`

Auth:
- `POST /auth/login`, `POST /auth/logout`, `GET /auth/me` (also works with an API key and then returns `auth_method: "api_key"` and the key's scopes, handy to test a key)

Manager only, via session cookie **or** API key (`Authorization: Bearer gic_…`) with the scope in brackets:
- `POST/PUT/DELETE /admin/quizzes[/:id]`, `PATCH /admin/quizzes/:id/publish` (`quizzes:write`)
- `POST/PUT/DELETE /admin/quizzes/:id/questions[/:qid]`, `PUT /admin/quizzes/:id/questions/reorder` (`quizzes:write`)
- `POST /admin/quizzes/import`, which takes bulk JSON (schema in `AI-HOW-ADD-QUIZ.md`) and runs in a single DB transaction (`quizzes:write`)
- `POST /admin/uploads/image` (multipart, optional `quiz_id`), which returns `{ key, url }` (`quizzes:write` or `reviews:write`)
- `DELETE /admin/uploads/image/:key` (`quizzes:write` or `reviews:write`)
- `GET /admin/reviews[?subject_id=]`, `GET /admin/reviews/:id`, `POST/PUT/DELETE /admin/reviews[/:id]` (multipart `.html` upload) (`reviews:write`)
- All of the above only for content the manager owns or that was shared with them (`404` otherwise); `DELETE` of a quiz or review is owner-only (`403 owner_only`). List endpoints return `access` (`owner`/`shared`), `owner_id`, `owner_name`, `share_count`.

Manager only, dashboard session only (never with an API key):
- `GET /admin/quizzes/:id/shares`, `POST /admin/quizzes/:id/shares` (`{ email }`, owner only), `DELETE /admin/quizzes/:id/shares/:user_id` (owner, or yourself to leave); the same three under `/admin/reviews/:id/shares`
- `GET /admin/api-keys/mine`, `POST /admin/api-keys` (`{ name, scopes, expires_in_days | null, replace_existing }` → `{ key, api_key }`, the key is shown only in this response), `DELETE /admin/api-keys/:id` (own key)

Admin only, dashboard session only:
- `GET/POST /admin/users`, `PATCH /admin/users/:id` (active flag, display name), `POST /admin/users/:id/reset-password`, `DELETE /admin/users/:id`
- `GET /admin/api-keys[?user_id=]` (all managers' keys), `DELETE /admin/api-keys/:id` (revoke any key)
- `POST/PUT/DELETE /admin/subjects[/:id]` (subjects require `year_id`)
- `POST/PUT/DELETE /admin/years[/:id]` (`{ name, slug?, description?, sort_order }`; slug derived from the name when empty; a year with subjects can't be deleted → 409)
- `POST /admin/uploads/cleanup-orphans`

---

## 6. Extensibility (important)

More quiz modes will be added later (for example True/False, Matching, Fill-in-the-blank). Design for it:
- **Frontend:** create `frontend/src/quiz-modes/registry.ts`. Each mode is an object `{ id, label, description, icon, component, isAvailable(quiz) }`. The mode-select modal and the play page both read from this registry.
- **Backend:** put mode availability rules in one place (for example `backend/src/quiz_modes.rs`), using an enum `QuizMode` that serializes as lowercase strings.
- Adding a mode should only require: a new component, one registry entry, one enum variant plus availability rule, and (only if needed) a migration. Document this in `docs/AI-HOW-ADD-QUIZ.md`.

---

## 7. UI / design

### 7.1 Theme: Catppuccin (all 4 flavors selectable)
The whole UI uses the **[Catppuccin](https://catppuccin.com/palette)** color palette. Users can switch between all four official flavors:

| Flavor | Type | `base` | `text` | Default accent (`mauve`) |
|---|---|---|---|---|
| **Latte** | Light | `#eff1f5` | `#4c4f69` | `#8839ef` |
| **Frappé** | Dark | `#303446` | `#c6d0f5` | `#ca9ee6` |
| **Macchiato** | Dark | `#24273a` | `#cad3f5` | `#c6a0f6` |
| **Mocha** | Dark (darkest) | `#1e1e2e` | `#cdd6f4` | `#cba6f7` |

Implementation requirements:
- Do NOT hand-copy hex values. Get the full official palette (all 26 colors per flavor: rosewater … crust) from the **`@catppuccin/palette`** package (`bun add @catppuccin/palette`), or use the official **`@catppuccin/tailwindcss`** plugin if it supports runtime flavor switching with your Tailwind version.
- Expose each color as a CSS variable (for example `--ctp-base`, `--ctp-text`, `--ctp-mauve`, …) defined per flavor under a class or attribute on `<html>`, such as `<html data-flavor="mocha">`. Map those variables into Tailwind's theme so components use semantic classes like `bg-ctp-base`, `bg-ctp-mantle`, `text-ctp-text`, `text-ctp-subtext0`, `border-ctp-surface1`, `bg-ctp-mauve`. Components must never hard-code hex colors, so switching flavor recolors everything instantly without a reload.
- Follow Catppuccin's style guide for color roles: `base` for the page background, `mantle`/`crust` for header, sidebar and footer, `surface0–2` for cards, inputs and borders, `text` and `subtext0/1` for text, `overlay` for muted or disabled elements, the **accent** for buttons, links, focus rings and progress bars, `green` for correct answers, `red` for wrong answers, `yellow` for warnings, and `blue` for info.
- Configure Tailwind's `dark:` variant to apply when the flavor is Frappé, Macchiato or Mocha. Latte counts as light.
- **Theme picker** in the header: a dropdown showing the 4 flavors, each with a small color swatch preview and name, plus a **"System"** option. System uses Latte when the OS is in light mode and Mocha when it is in dark mode, and follows `prefers-color-scheme` changes live.
- **Accent picker** (nice to have): let users choose one of the 14 Catppuccin accent colors (rosewater, flamingo, pink, mauve, red, maroon, peach, yellow, green, teal, sky, sapphire, blue, lavender). Default is mauve. Implement it as a `--ctp-accent` variable.
- Save the chosen flavor and accent in `localStorage`, managed by a Pinia `useThemeStore`. Add a tiny inline script in `index.html` that applies the saved flavor before Vue mounts, so the page never flashes the wrong theme on load.
- Check that text has readable contrast in all 4 flavors, especially Latte.

### 7.2 General UI
- Use Tailwind with a clean, modern, mobile-first design.
- Popups are accessible modals: focus trap, close on Esc or backdrop click, and `aria` labels.
- Show loading skeletons, empty states ("No quizzes yet for this subject") and error toasts.
- Keep components small: `SubjectCard`, `ActivityModal`, `QuizCard`, `QuizModeModal`, `QcmPlayer`, `FlashcardPlayer`, `ResultScreen`, plus admin forms.

---

## 8. `.env.example` templates (one per project)

Generate **two** templates, one for each project, with empty or safe placeholder values and a short comment on each key. I will copy them to `.env` and fill them in: `cp frontend/.env.example frontend/.env` and `cp backend/.env.example backend/.env`.

### `frontend/.env.example`
Vite only exposes `VITE_`-prefixed values to the browser. Never put secrets here, because everything in this file ends up in the public JavaScript bundle.

```dotenv
VITE_APP_NAME=GIC Review
VITE_API_BASE_URL=http://localhost:8080/api
```

### `backend/.env.example`
The backend loads this with `dotenvy`. The connection is built **only from the `DB_*` values**; there is no `DATABASE_URL` (so credentials are never written twice). The MySQL TLS connection uses **OpenSSL** (sqlx `tls-native-tls`), not rustls, because managed MySQL services (e.g. Oracle Cloud MySQL HeatWave) present self-signed **X.509 v1** certificates that rustls cannot parse (`UnsupportedCertVersion`); the Docker image therefore installs `libssl-dev` (build) and `libssl3` (runtime). `DB_SSL_REQUIRED=true` (the default, also when the line is missing) forces an SSL connection to MySQL (connecting fails without it); `false` does not require it but still uses SSL when the server offers it (e.g. a local MySQL without SSL). Any value other than true/false stops startup with a clear error. The server, `cargo run -- migrate` and the database integration tests all use `DB_*`.

```dotenv
# ---------- App ----------
APP_ENV=development              # development | production
BACKEND_HOST=127.0.0.1
BACKEND_PORT=8080
FRONTEND_URL=http://localhost:5173   # used for CORS

# ---------- MySQL ----------
DB_HOST=
DB_PORT=3306
DB_NAME=gic_review
DB_TEST_NAME=gic_review_test    # database used by `cargo test` (wiped on every run!) - must differ from DB_NAME
DB_USER=
DB_PASSWORD=
DB_SSL_REQUIRED=true            # true (default) = MySQL connection must use SSL; false = only if the server offers it

# ---------- Auth ----------
JWT_SECRET=                      # long random string (e.g. openssl rand -hex 64)
JWT_EXPIRES_IN_HOURS=12
ADMIN_EMAIL=                     # first admin, created on first start if none exists
ADMIN_PASSWORD=                  # change after first login

# ---------- Cloudflare R2 ----------
R2_ACCOUNT_ID=
R2_ACCESS_KEY_ID=
R2_SECRET_ACCESS_KEY=
R2_BUCKET=gic-review
R2_ENDPOINT=https://<R2_ACCOUNT_ID>.r2.cloudflarestorage.com
R2_PUBLIC_BASE_URL=              # public bucket domain, e.g. https://files.example.com

# ---------- Upload limits ----------
MAX_IMAGE_SIZE_MB=5
MAX_REVIEW_HTML_SIZE_MB=10
```

The backend must fail fast at startup, with a clear message, if a required variable is missing.

---

## 9. Documentation to write (in `docs/`)

Two audiences:

- **External API clients** (`AI-HOW-ADD-QUIZ.md`, `AI-HOW-ADD-INTERACTIVE-REVIEW.md`): usually an AI assistant that a manager gave an API key, e.g. "read this PDF and create a quiz / interactive review". They only use the REST API and **have no access to the code base**, so these two documents must be fully **self-contained**: no source paths, no SQL, no repo commands; every schema, example, template and error code inline; placeholders `$BASE` (API base URL) and `$KEY` (API key). Both use the same easy-to-follow structure: **1. Quick start** (the two values, the `Authorization` header, one test call) → **2. Endpoint list**: a numbered table of every endpoint the client may call (method, path, whether the key is needed, "use it to…"), which ones most jobs need, and what not to call → **3. Recipe**: the exact sequence of calls for the main job (PDF → quiz / review page) with curl and expected responses → **4. Endpoint details** (body, response, status codes) and the response object → format/HTML rules and template → error table → final checklist. Every documented call must be verified against the running API.
- **Developers working in this repo** (`AI-SPEC.md`, `DEV-ADD-QUIZ-MODE.md`, `prompt.md`).

### `docs/AI-HOW-ADD-QUIZ.md` (external API guide)
- What the client needs (base URL, a manager's API key with `quizzes:write`), `Authorization: Bearer` usage, keeping the key secret.
- What the key can and cannot do: acts as its manager; only quizzes that manager owns or that were shared with them; shared ones can be edited and published but not deleted; sharing/users/subjects/keys are dashboard-only; ask the human when a subject is missing.
- A recommended workflow from source material (e.g. a PDF): check the key → pick the subject → plan questions → upload images → one import call → check `available_modes` → report back and publish only when asked → fix with question endpoints, avoid duplicates.
- Question-writing guidance (facts only from the source, same language, one concept per question, plausible distractors, explanations, `answer_text`, sensible quiz size) and the format rules (`is_correct`, `multi_select`, what makes QCM and Flashcard available, limits, image types/sizes, alt text).
- The full import JSON schema plus a text-only example and an example using every image field (cover, question image, image options, flashcard answer image, explanation image), with both ways to supply images (upload and use the returned `key`, or a public `*_url`).
- Image upload, a full endpoint reference with the quiz response object, the error code table, and a final checklist.

### `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md` (external API guide)
- What an interactive review is: a single self-contained `.html` file (inline CSS/JS; images as data URIs/inline SVG or uploaded with `POST /admin/uploads/image` and referenced by their absolute public URL).
- What the client needs and what the key can do (same rules as for quizzes, with `reviews:write`).
- A recommended workflow (e.g. PDF → review page), including checking existing reviews to avoid duplicates and uploading unpublished first.
- Rules for the HTML: works in a sandboxed iframe with no same-origin access (no storage/cookies, no calls to the API, no external trackers/assets), responsive, accessible, and styled only with the Catppuccin `--ctp-*` variables.
- Theme protocol: the parent sends `postMessage({ type: "gic-theme", flavor, accent })`; without a message the page follows `prefers-color-scheme` (Latte light / Mocha dark).
- The **complete starter template inline** (all 4 flavors' variables, accent overrides, theme listener, example content).
- Create/replace (same URL)/read/delete endpoints with curl examples and field tables, the review response object, the error code table, and a final checklist.

### `docs/DEV-ADD-QUIZ-MODE.md` (developers)
- **Adding a new quiz mode/type.** Give a step-by-step checklist: backend enum + availability rule → API `available_modes` → frontend component in `src/quiz-modes/` → registry entry → any schema change as a new migration (never edit old migrations) → add the mode as a requirement in `prompt.md`, as a Features line in `docs/AI-SPEC.md`, and to the external quiz guide. Include a worked example (for example "True/False").

### `docs/AI-SPEC.md`
- A short project summary, the tech stack, the folder structure and how to run it.
- A **"Features"** checklist listing every feature built so far and planned (✅ built / 🚧 in progress / ⏳ planned), grouped by Public site, Quiz modes, Staff/Admin, Security, and Infrastructure.
- The current DB schema summary and API endpoint list.
- Known limitations and next steps.
- A **"Keep features in sync"** section at the top of `AI-SPEC.md`, written as rules for any AI or developer working on this repo:
  1. **The two files are different documents with different jobs. Do not copy one into the other.**
     - `prompt.md` (repo root) is the **build brief**: full instructions on how to build the project (tech stack, folder layout, `.gitignore`, `.env` templates, UI/theme rules, security rules, schema design, API design, build phases).
     - `docs/AI-SPEC.md` is the **project status sheet**: what has actually been built so far, the current schema and endpoints, how to run it, known limitations, and the changelog.
  2. **Only the features/requirements list is kept in sync.** Every user-facing feature or requirement must appear in both files:
     - in `prompt.md`, described as a requirement in its matching section (what it should do);
     - in `AI-SPEC.md`, as one line in the **"Features"** checklist with its status (✅ built / 🚧 in progress / ⏳ planned).
     Everything else stays in its own file and is **not** duplicated: build instructions only in `prompt.md`; schema, endpoints, run steps and changelog only in `AI-SPEC.md`.
  3. **Adding or changing a feature:** add or update the requirement in `prompt.md` → add or update its line in the `AI-SPEC.md` Features list (⏳ or 🚧) → implement it → mark it ✅. Do all of this in the **same commit/PR** as the code.
  4. **Removing a feature:** remove it from both files and say so in the commit message. Never delete a requirement silently.
  5. **Sync check:** every feature line in `AI-SPEC.md` must correspond to a requirement in `prompt.md`, and every requirement in `prompt.md` must have a line in `AI-SPEC.md` (⏳ if not built yet). If one is missing, add it.
  6. If a change affects how quizzes or interactive reviews are added (schema, JSON import format, image fields, quiz modes, HTML rules), also update `docs/AI-HOW-ADD-QUIZ.md` and/or `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`.
  7. Add a dated entry to the **Changelog** at the bottom of `AI-SPEC.md` (for example `2026-11-18: added image support for flashcard front/back`).
- Also add a short pointer at the top of `README.md` that says: "AI assistants: read `prompt.md` (build brief) and `docs/AI-SPEC.md` (current status) first. When you add or change a feature, update the feature/requirement list in both."

---

## 10. Build phases and acceptance criteria

1. **Scaffold:** separate `frontend/` and `backend/` projects, both `.env.example` files, all three `.gitignore` files, README, Rust Axum server with `/api/health`, Vue + Tailwind app with the "GIC Review" header, and the Catppuccin theme system with the flavor picker. ✔ Both run locally, and switching between Latte, Frappé, Macchiato and Mocha recolors the page instantly and is still applied after a reload. `git status` shows no `.env`, `node_modules/` or `target/` files.
2. **Database:** SQLx migrations for all tables, seed `Human Computer Interaction`, admin bootstrap from `backend/.env`. ✔ `cargo run -- migrate` (and server start) applies the migrations from `DB_*` alone, and an admin is created on first start.
3. **Public browsing:** home subjects grid → activity modal → quiz list → mode modal. ✔ Works with no login.
4. **Quiz players:** QCM and Flashcard players with result screens and the mode registry. ✔ A sample quiz is playable in both modes.
5. **Auth and staff area:** login, role guards (frontend and backend), quiz CRUD, image upload to R2 (drag & drop, paste, preview) for every image field, bulk import. ✔ A manager can create a quiz with images on the cover, questions, options and flashcard backs, and all of them display in both play modes. Deleting a question also deletes its images from R2. A guest gets `401`/`403` on admin endpoints.
6. **Admin features:** manager management, API key oversight and subject management (admin only). ✔ A manager cannot reach `/admin/users` (UI and API), and an admin gets `403` on quiz/review endpoints.
7. **Interactive reviews:** HTML upload to R2 and the sandboxed iframe viewer. ✔ The uploaded HTML renders, and scripts inside it cannot read the main site's cookies.
8. **Docs:** write all three `docs/` files. Commit this `prompt.md` to the repo root if it isn't already there. ✔ `AI-SPEC.md` matches what was actually built, includes the "Keep features in sync" rules and a Changelog, and its Features checklist matches the requirements in `prompt.md` one-to-one (no other content duplicated between the two files).
9. **API keys:** manager API key page and admin API keys page, Bearer authentication with scopes. ✔ A manager creates one key (a second one is refused), uses it with curl to import a quiz and upload a review, regenerating makes the old key fail, an admin can revoke it, and the key gets `403` on user/key/subject endpoints.

General quality bar:
- Rust passes `cargo fmt` and `cargo clippy` with no warnings. The frontend passes `bun run build` and ESLint/Prettier (`bun run lint`).
- Add a few backend integration tests (auth and role checks at minimum). Database tests use one separate database named by `DB_TEST_NAME` in `backend/.env` (e.g. `gic_review_test`, must differ from `DB_NAME`), with the same `DB_*` connection values; it is wiped and re-migrated before each test, tests run one at a time, the real `DB_NAME` database is never touched, and the tests are skipped with a message when `DB_*` or `DB_TEST_NAME` is empty.
- Do not commit `.env` or any real credentials.
- When done, print a short summary of what was built, how to run it, and which values I still need to fill in for `frontend/.env` and `backend/.env`.

---

## 11. Additions and decisions recorded during the build (2026-11-18)

These refine the requirements above; they do not remove any of them.

- **Manager read endpoints:** `GET /admin/quizzes[?subject_id=]`, `GET /admin/quizzes/:id` (`quizzes:write`) and `GET /admin/reviews[?subject_id=]` (`reviews:write`) list drafts/unpublished items for the dashboard and for API clients.
- **Subjects API** also returns `quiz_count` (published quizzes). Subject `color` is one of the 14 Catppuccin accent names.
- **Publishing** a quiz is refused while it has no available mode.
- **Deleting** a subject is refused (409) while it still has quizzes or reviews. Deleting a manager keeps their content (`created_by` becomes NULL, so `created_by` columns are nullable with `ON DELETE SET NULL`).
- **Roles changed (2026-11-18):** admins no longer create or edit quizzes, reviews or images; that is manager-only. Admins manage manager accounts, managers' API keys, subjects and orphan cleanup.
- **Uploads:** `DELETE /admin/uploads/image/:key` only deletes images no row references (409 otherwise). The question editor deletes images uploaded during an edit session that end up unused (cancel/close/replace). Orphan cleanup skips files younger than one hour.
- **Bulk import:** unknown JSON fields are rejected; `*_url` images must be public http(s) URLs (no private IPs, no redirects); the admin import page has a "Load HCI sample" button (`docs/examples/hci-sample-quiz.json`, text-only, playable in QCM and Flashcard).
- **Interactive reviews:** replacing the file overwrites the same R2 key; the API returns the URL with a `?v=<updated_at>` cache-buster. The viewer refuses to render a review URL on the site's own origin.
- **Theme:** text on accent buttons uses `base`, or `text` where that gives better contrast (e.g. Latte yellow), computed at build time from `@catppuccin/palette`.
- **No `DATABASE_URL` (2026-11-18):** replaced by `cargo run -- migrate` and `DB_*`-based test databases; the `.env` template no longer has it.
- **Docker deployment:** `backend/Dockerfile` (multi-stage build, slim non-root runtime, migrations embedded) and `backend/.dockerignore` (never ship `.env` or `target/`) so the API can be deployed to Render, Railway or any Docker host. The server listens on `PORT` when the platform sets it (else `BACKEND_PORT`), and the image sets `BACKEND_HOST=0.0.0.0`, `APP_ENV=production`.
- **JSON errors for bad request bodies:** a body that isn't valid JSON or misses/mistypes a field returns the standard `{ "error": { "code": "validation_error" | "bad_request", "message" } }` (never Axum's plain-text rejection).
- **Dashboard subject pickers** show `Year · Subject`, since two years can have subjects with similar names. Back links on subject pages lead to the subject's year page.
- **SQL:** queries are parameterized `&'static str` SQL (sqlx 0.9 rejects dynamic SQL strings). Compile-time `query!` macros and a committed `.sqlx/` directory are planned once a development database is available.
