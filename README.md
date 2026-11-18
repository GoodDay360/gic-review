# GIC Review

> **AI assistants:** read `prompt.md` (build brief) and `docs/AI-SPEC.md` (current status) first. When you add or change a feature, update the feature/requirement list in both.

A study website for exam review. Students pick a subject, then read its **Interactive
Review** or practise with **Quizzes** in **QCM** or **Flashcard** mode, with no account
needed. **Managers** create quizzes and interactive reviews in the dashboard, or through the
API with their personal **API key** (how AI assistants add content). **Admins** only manage
manager accounts, managers' API keys and subjects.

| Part | Stack | Folder |
|---|---|---|
| Frontend | Vue 3 + TypeScript + Vite + Tailwind CSS 4 + Pinia + Vue Router, Catppuccin theme | `frontend/` |
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

On first start, if no admin exists, one is created from `ADMIN_EMAIL` / `ADMIN_PASSWORD`.
Log in and change that password. The server refuses to start, with a list of what is
wrong, if a required variable is missing.

Checks: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
The database tests use the same `DB_*` values: each creates a temporary `gic_test_*` database
and drops it afterwards (so `DB_USER` needs CREATE/DROP rights). With `DB_*` empty they are skipped.

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
`VITE_APP_NAME`, `VITE_API_BASE_URL`.

`backend/.env` holds all secrets: `DB_*` (the only database settings; SSL is required by default, set
`DB_SSL_REQUIRED=false` for a local MySQL without SSL), `JWT_SECRET`
(`openssl rand -hex 64`), `ADMIN_EMAIL`, `ADMIN_PASSWORD`, `R2_*`, upload limits. Both
`.env` files are git-ignored; only the `.env.example` templates are committed.

Deployment notes: serve the frontend and the API on the same site (e.g. `example.com` and
`api.example.com`), because the session cookie is `SameSite=Strict`; set
`APP_ENV=production` so the cookie is `Secure`; set `FRONTEND_URL` to the frontend origin
(CORS); serve R2 from its own domain.

## API keys (for scripts and AI assistants)

A manager opens **Dashboard → API key**, creates a key (one per manager, permissions
`quizzes:write` / `reviews:write`, optional expiry) and copies it; it is shown once. Then:

```bash
curl -H "Authorization: Bearer gic_…" http://localhost:8080/api/auth/me
curl -H "Authorization: Bearer gic_…" -H 'Content-Type: application/json' \
  --data @docs/examples/hci-sample-quiz.json http://localhost:8080/api/admin/quizzes/import
```

Admins can see and revoke every manager's key under **Dashboard → API keys**. Keys never work
for user, key or subject management. Details: `docs/AI-HOW-ADD-QUIZ.md`.

## Documentation

- `prompt.md`: build brief (requirements and how to build the project)
- `docs/AI-SPEC.md`: status sheet (features checklist, schema, endpoints, limitations, changelog)
- `docs/AI-HOW-ADD-QUIZ.md`: adding quizzes (admin panel, JSON import, SQL) and new quiz modes
- `docs/AI-HOW-ADD-INTERACTIVE-REVIEW.md`: writing and uploading interactive reviews
- `docs/examples/`: a text-only sample quiz JSON and an interactive review starter template
