//! In-memory response cache for the public read API, to spare the database (and CPU).
//!
//! - Caches successful `GET` responses of `/api/years…`, `/api/subjects…` and `/api/quizzes/…`
//!   for **anonymous** requests only (no session cookie, no `Authorization` header): logged-in
//!   editors and API keys can see drafts, so they always get fresh data.
//! - Any successful write through the editor API (`POST/PUT/PATCH/DELETE /api/admin/…`) clears
//!   the whole cache. Entries are also stamped with a "generation" that every write bumps, so a
//!   response computed before a write can never be served after it.
//! - Size and lifetime come from `CACHE_MAX_MB` (0 = disabled) and `CACHE_TTL_SECONDS`.
//! - Responses carry `x-cache: hit` or `x-cache: miss`.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::body::HttpBody as _;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use moka::future::Cache;

use crate::{auth::SESSION_COOKIE, state::AppState};

/// Paths whose anonymous GET responses may be cached.
const CACHEABLE_PREFIXES: &[&str] = &["/api/years", "/api/subjects", "/api/quizzes/"];

#[derive(Clone)]
struct Entry {
    generation: u64,
    content_type: HeaderValue,
    body: Bytes,
}

#[derive(Clone)]
pub struct ResponseCache {
    inner: Option<Cache<String, Entry>>,
    generation: Arc<AtomicU64>,
    /// Responses larger than this are not cached (keeps one big quiz from evicting everything).
    max_entry_bytes: usize,
}

impl ResponseCache {
    /// `max_bytes = 0` disables the cache.
    pub fn new(max_bytes: u64, ttl: Duration) -> Self {
        let inner = (max_bytes > 0).then(|| {
            Cache::builder()
                .max_capacity(max_bytes)
                .weigher(|key: &String, entry: &Entry| {
                    (key.len() + entry.body.len() + 64)
                        .try_into()
                        .unwrap_or(u32::MAX)
                })
                .time_to_live(ttl)
                .build()
        });
        Self {
            inner,
            generation: Arc::new(AtomicU64::new(0)),
            max_entry_bytes: (max_bytes / 8).clamp(64 * 1024, 4 * 1024 * 1024) as usize,
        }
    }

    pub fn disabled() -> Self {
        Self::new(0, Duration::from_secs(1))
    }

    /// Drop everything (called after any successful write).
    pub fn invalidate_all(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(cache) = &self.inner {
            cache.invalidate_all();
        }
    }

    fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
}

fn is_anonymous(req: &Request) -> bool {
    if req.headers().contains_key(header::AUTHORIZATION) {
        return false;
    }
    !req.headers()
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .any(|c| c.contains(SESSION_COOKIE))
}

fn with_cache_header(mut response: Response, value: &'static str) -> Response {
    let headers = response.headers_mut();
    headers.insert("x-cache", HeaderValue::from_static(value));
    // Browsers must always ask the server, so they never keep showing an old quiz.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

/// Axum middleware: serves/stores cacheable responses and clears the cache after writes.
pub async fn layer(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let cache = state.cache.clone();
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Writes through the editor API clear the cache once they succeed.
    if !matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        let response = next.run(req).await;
        if path.starts_with("/api/admin/") && response.status().is_success() {
            cache.invalidate_all();
        }
        return response;
    }

    let Some(store) = cache.inner.clone() else {
        return next.run(req).await;
    };
    let cacheable = method == Method::GET
        && CACHEABLE_PREFIXES.iter().any(|p| path.starts_with(p))
        && is_anonymous(&req);
    if !cacheable {
        return next.run(req).await;
    }

    let key = req
        .uri()
        .path_and_query()
        .map_or_else(|| path.clone(), |pq| pq.as_str().to_string());
    let generation = cache.current_generation();
    if let Some(entry) = store.get(&key).await
        && entry.generation == generation
    {
        let mut response = (StatusCode::OK, entry.body).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, entry.content_type);
        return with_cache_header(response, "hit");
    }

    let response = next.run(req).await;
    if response.status() != StatusCode::OK {
        return with_cache_header(response, "miss");
    }
    if response
        .body()
        .size_hint()
        .upper()
        .is_none_or(|size| size > cache.max_entry_bytes as u64)
    {
        return with_cache_header(response, "miss");
    }
    let (parts, body) = response.into_parts();
    // Keep an explicit per-response bound even when a body reports an incorrect size hint.
    let bytes = match axum::body::to_bytes(body, cache.max_entry_bytes).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(error = %err, "could not read response body for caching");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    if bytes.len() <= cache.max_entry_bytes
        && let Some(content_type) = parts.headers.get(header::CONTENT_TYPE).cloned()
    {
        store
            .insert(
                key,
                Entry {
                    generation,
                    content_type,
                    body: bytes.clone(),
                },
            )
            .await;
    }
    with_cache_header(Response::from_parts(parts, Body::from(bytes)), "miss")
}
