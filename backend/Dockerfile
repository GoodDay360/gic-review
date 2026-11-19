# syntax=docker/dockerfile:1
# GIC Review API: production image for Render, Railway, Fly.io or any Docker host.
#
#   docker build -t gic-review-api backend/
#   docker run --env-file backend/.env -e BACKEND_HOST=0.0.0.0 -p 8080:8080 gic-review-api
#
# Configuration comes from environment variables (the same names as backend/.env.example);
# no .env file is copied into the image. Migrations are embedded and run on startup.

ARG RUST_VERSION=1.96

# ---------- build ----------
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /app

# C toolchain for aws-lc-sys / ring / libwebp, nasm for the AV1 (AVIF) encoder, and OpenSSL headers for the MySQL TLS connection
# (native-tls/OpenSSL accepts the X.509 v1 certificates some managed MySQL servers use).
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential cmake nasm pkg-config libssl-dev \
 && rm -rf /var/lib/apt/lists/*

# 1) Build dependencies only, so they are cached until Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
 && echo "fn main() {}" > src/main.rs \
 && touch src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src

# 2) Build the real app (migrations are embedded at compile time by sqlx::migrate!).
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs \
 && cargo build --release --locked \
 && strip target/release/gic-review-api

# ---------- runtime ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin app

COPY --from=build /app/target/release/gic-review-api /usr/local/bin/gic-review-api

# Listen on all interfaces inside the container. Render/Railway set PORT themselves,
# which the app uses instead of BACKEND_PORT.
ENV BACKEND_HOST=0.0.0.0 \
    BACKEND_PORT=8080 \
    APP_ENV=production \
    RUST_LOG=gic_review_api=info,tower_http=info,sqlx=warn

USER app
EXPOSE 8080

# Health check endpoint for the platform: GET /api/health (200 when the DB is reachable).
CMD ["gic-review-api"]
