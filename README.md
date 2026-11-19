# GIC Review

> **AI assistants:** read `docs/AI-SPEC.md` (current status) first and keep it up to date when you add or change a feature. `prompt.md` is the frozen original build brief: don't read or update it for routine work.

A study website for exam review. Students pick their category (e.g. I3, I4), then a subject, then read its **Interactive
Review** or practise with **Quizzes** in **QCM** or **Flashcard** mode, with no account
needed. **Managers** create quizzes and interactive reviews in the dashboard, or through the
API with their personal **API key** (how AI assistants add content). **Admins** can do everything
managers can (on all content) and also manage accounts, API keys, categories and subjects.

| Part | Stack | Folder |
|---|---|---|
| Frontend | Vue 3 + TypeScript + Vite + Vuetify 4 (Material Design Icons) + Tailwind CSS 4 + Pinia + Vue Router, AOS + animate.css, Catppuccin theme | `frontend/` |
| Backend | Rust + Axum + Tokio + SQLx (MySQL 8), Cloudflare R2 via `aws-sdk-s3` | `backend/` |

The two projects are fully separate (own dependencies, own `.env`) and talk only over HTTP.

## Prerequisites

- Rust (stable). `sqlx-cli` is optional, only for creating new migration files (`sqlx migrate add <name>`)
- [Bun](https://bun.sh) 1.3+ (the frontend uses Bun only; don't use npm, yarn or pnpm)
- MySQL 8 (an empty database, e.g. `CREATE DATABASE gic_review CHARACTER SET utf8mb4;`)
- A Cloudflare R2 bucket with **public access** enabled (custom domain or `r2.dev` URL) and
  an R2 API token (Object Read & Write)

## Backend

```bash
cd backend
cp .env.example .env        # then fill it in (see below)
cargo run -- migrate        # optional: migrations also run automatically at startup
cargo run                   # API on http://127.0.0.1:8080/api (try /api/health)
```

On every start the admin account from `ADMIN_EMAIL` / `ADMIN_PASSWORD` is kept in sync: it is
created if missing, and if you change `ADMIN_PASSWORD` in `.env` the new password is applied on the
next restart (a password changed in the dashboard is kept as long as `.env` stays the same).
Log in and change that password. The server refuses to start, with a list of what is
wrong, if a required variable is missing.

Checks: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
The database tests use the same `DB_*` connection but the database named in `DB_TEST_NAME`
(`gic_review_test`), which they wipe before each test; your real database is never touched.
Create it once: `CREATE DATABASE gic_review_test CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;`
`GRANT ALL PRIVILEGES ON gic_review_test.* TO 'gicreview'@'%';`. With `DB_*` empty they are skipped.

## Frontend

```bash
cd frontend
cp .env.example .env
bun install
bun run dev                 # http://localhost:5173
bun run build               # type-check + production build into dist/
bun run lint && bun run format:check
```

## Configuration

`frontend/.env` holds only public `VITE_*` values (they end up in the JS bundle):
`VITE_APP_NAME`, `VITE_API_BASE_URL`, and `VITE_SITE_URL` (the site's public address, e.g.
`https://gic.example.com`: link previews in Telegram/Discord/WhatsApp need absolute URLs, so set it
before building for production).

`backend/.env` holds all secrets: `DB_*` (the only database settings; SSL is required by default, set
`DB_SSL_REQUIRED=false` for a local MySQL without SSL), `JWT_SECRET`
(`openssl rand -hex 64`), `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `R2_*`, upload limits, and the cache size (`CACHE_MAX_MB`, default 32; `CACHE_TTL_SECONDS`). Both
`.env` files are git-ignored; only the `.env.example` templates are committed.

Deployment notes: set `APP_ENV=production` so the session cookie is `Secure`, and set
`FRONTEND_URL` to the exact frontend origin (CORS and cookie-authenticated write protection).
HTTPS frontend cookies use `SameSite=None; Secure; Partitioned` to support a frontend and API on
separate sites, such as Cloudflare Pages and Render. The browser partitions the session by
top-level site; API requests also enforce the configured frontend origin for cookie-authenticated
writes. This relies on browser support for partitioned cookies. Serve R2 from its own domain.

## Deploying the backend (Docker: Render, Railway, …)

`backend/Dockerfile` builds a small production image (multi-stage, non-root, migrations
embedded and applied on start). Local check:

```bash
docker build -t gic-review-api backend/
docker run --env-file backend/.env -e BACKEND_HOST=0.0.0.0 -p 8080:8080 gic-review-api
```

**Render** → New → Web Service → your repo → Language **Docker**, Root Directory `backend`
(Dockerfile path `./Dockerfile`), Health Check Path `/api/health`.
**Railway** → New → GitHub repo → service Settings → Root Directory `backend` (it picks up the
Dockerfile automatically), Healthcheck Path `/api/health`, then Networking → Generate Domain.

Set the environment variables from `backend/.env.example` in the platform's dashboard:
`APP_ENV=production`, `FRONTEND_URL` (your deployed frontend origin), all `DB_*`, `JWT_SECRET`,
`ADMIN_EMAIL`, `ADMIN_PASSWORD`, all `R2_*`, upload limits. Leave out `PORT`, `BACKEND_HOST`,
`BACKEND_PORT` (the image and platform set them) and `DB_TEST_NAME` (tests only).

Things to know:
- **Database:** the cloud service must reach your MySQL, so use a hosted MySQL (Railway MySQL,
  Aiven, PlanetScale-compatible, a VPS…), not one on your own machine. Keep `DB_SSL_REQUIRED=true`.
- **Cookie compatibility:** cross-site sessions use a `Partitioned` cookie. Set `FRONTEND_URL` to
  the exact dashboard origin. Browsers without partitioned-cookie support need a shared custom
  domain or same-origin API proxy instead. API keys work from anywhere regardless.
- Free instances sleep when idle; the first request after a pause can take a while.

## API keys (for scripts and AI assistants)

A manager opens **Dashboard → API key**, creates a key (one per manager, permissions
`quizzes:write` / `reviews:write`, optional expiry) and copies it; it is shown once. Then:

```bash
curl -H "Authorization: Bearer gic_…" http://localhost:8080/api/auth/me
curl -H "Authorization: Bearer gic_…" -H 'Content-Type: application/json' \
  --data @docs/examples/hci-sample-quiz.json http://localhost:8080/api/admin/quizzes/import
```

A key reaches the content its owner created, everything in their categories, and everything in categories or
subjects shared with them.
To let an AI build content from your course material, give it the key, the API base URL and
`docs/AI-HOW-ADD-QUIZ.md` / `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`.
Admins can see and revoke every manager's key under **Dashboard → API keys**. Keys never work
for user, key or subject management. Details: `docs/AI-HOW-ADD-QUIZ.md`.

## Documentation

- `prompt.md`: original build brief (frozen, reference only; not kept in sync)
- `docs/AI-SPEC.md`: status sheet (features checklist, schema, endpoints, limitations, changelog)
- `docs/README.md`: **start here to let an AI create content**: what you need, a ready-to-use prompt, and keeping your API key safe
- `docs/AI-HOW-ADD-QUIZ.md`: **give this to an AI** (with an API key) to create quizzes through the API, e.g. from a PDF; self-contained, no code access needed
- `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`: **give this to an AI** to build and upload interactive review pages; includes the HTML template
- `docs/DEV-ADD-QUIZ-MODE.md`: developer guide for adding a new quiz mode
- `docs/examples/`: a text-only sample quiz JSON and an interactive review starter template
