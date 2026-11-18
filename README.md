# GIC Review

> **AI assistants:** read `prompt.md` and `docs/AI-SPEC.md` first, and keep both in sync.

A study website for exam review. Students pick a subject, then read its **Interactive
Review** or practise with **Quizzes** in **QCM** or **Flashcard** mode, with no account
needed. Staff (admins and managers) log in to create quizzes, upload images and reviews.

| Part | Stack | Folder |
|---|---|---|
| Frontend | Vue 3 + TypeScript + Vite + Tailwind CSS 4 + Pinia + Vue Router, Catppuccin theme | `frontend/` |
| Backend | Rust + Axum + Tokio + SQLx (MySQL 8), Cloudflare R2 via `aws-sdk-s3` | `backend/` |

The two projects are fully separate (own dependencies, own `.env`) and talk only over HTTP.

## Prerequisites

- Rust (stable) and `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features mysql,rustls`
- Node.js 20+ and npm
- MySQL 8 (an empty database, e.g. `CREATE DATABASE gic_review CHARACTER SET utf8mb4;`)
- A Cloudflare R2 bucket with **public access** enabled (custom domain or `r2.dev` URL) and
  an R2 API token (Object Read & Write)

## Backend

```bash
cd backend
cp .env.example .env        # then fill it in (see below)
cargo sqlx migrate run      # optional: migrations also run automatically at startup
cargo run                   # API on http://127.0.0.1:8080/api (try /api/health)
```

On first start, if no admin exists, one is created from `ADMIN_EMAIL` / `ADMIN_PASSWORD`.
Log in and change that password. The server refuses to start, with a list of what is
wrong, if a required variable is missing.

Checks: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
DB-backed tests: `DATABASE_URL=mysql://user:pass@host:3306/gic_review cargo test -- --ignored`
(the user needs permission to create test databases).

## Frontend

```bash
cd frontend
cp .env.example .env
npm install
npm run dev                 # http://localhost:5173
npm run build               # type-check + production build into dist/
npm run lint:check && npm run format:check
```

## Configuration

`frontend/.env` holds only public `VITE_*` values (they end up in the JS bundle):
`VITE_APP_NAME`, `VITE_API_BASE_URL`.

`backend/.env` holds all secrets: `DB_*` / `DATABASE_URL`, `JWT_SECRET`
(`openssl rand -hex 64`), `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `R2_*`, upload limits. Both
`.env` files are git-ignored; only the `.env.example` templates are committed.

Deployment notes: serve the frontend and the API on the same site (e.g. `example.com` and
`api.example.com`), because the session cookie is `SameSite=Strict`; set
`APP_ENV=production` so the cookie is `Secure`; set `FRONTEND_URL` to the frontend origin
(CORS); serve R2 from its own domain.

## Documentation

- `prompt.md`: requirements (source of truth for intent)
- `docs/AI-SPEC.md`: what is built, schema, endpoints, limitations, changelog
- `docs/AI-HOW-ADD-QUIZ.md`: adding quizzes (admin panel, JSON import, SQL) and new quiz modes
- `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`: writing and uploading interactive reviews
- `docs/examples/`: a text-only sample quiz JSON and an interactive review starter template
