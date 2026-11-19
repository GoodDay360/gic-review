//! Image uploads to R2 and orphan cleanup.

use std::collections::HashSet;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    routing::{delete, post},
};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use crate::{
    access::{self, Actor, Content},
    auth::{ContentManager, RequireAdmin, RequireEditor},
    config::Config,
    error::{AppError, AppResult},
    images,
    state::AppState,
    validation::is_image_key,
};

pub fn router(config: &Config) -> Router<AppState> {
    Router::new()
        .route(
            "/admin/uploads/image",
            post(upload_image).layer(DefaultBodyLimit::max(
                config.max_image_size_bytes + 64 * 1024,
            )),
        )
        .route("/admin/uploads/image/{*key}", delete(delete_image))
        .route("/admin/uploads/cleanup-orphans", post(cleanup_orphans))
}

#[derive(Debug, Serialize)]
pub struct UploadedImage {
    pub key: String,
    pub url: String,
}

/// Validate, process and store an image. Returns its new key.
pub async fn store_image(
    state: &AppState,
    bytes: Vec<u8>,
    filename: Option<String>,
    quiz_id: Option<i64>,
) -> AppResult<UploadedImage> {
    if bytes.is_empty() {
        return Err(AppError::BadRequest("the uploaded file is empty".into()));
    }
    if bytes.len() > state.config.max_image_size_bytes {
        return Err(AppError::PayloadTooLarge(format!(
            "images must be at most {} MB",
            state.config.max_image_size_bytes / 1024 / 1024
        )));
    }
    // Encoding is CPU- and memory-heavy: process at most N uploads at once (all clients
    // together); the others wait here in FIFO order.
    let _slot = state
        .image_upload_slots
        .acquire()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let processed =
        tokio::task::spawn_blocking(move || images::process(bytes, filename.as_deref()))
            .await
            .map_err(|e| AppError::Internal(e.into()))??;

    // Random key; the original file name is never used.
    let id = uuid::Uuid::new_v4();
    let ext = processed.kind.ext();
    let key = match quiz_id {
        Some(qid) => format!("quizzes/{qid}/{id}.{ext}"),
        None => format!("images/{id}.{ext}"),
    };
    state
        .storage
        .put(
            &key,
            processed.bytes,
            processed.kind.content_type(),
            "public, max-age=31536000, immutable",
        )
        .await?;
    Ok(UploadedImage {
        url: state.storage.public_url(&key),
        key,
    })
}

/// `multipart/form-data` with a `file` field and an optional `quiz_id` field.
async fn upload_image(
    RequireEditor(user, _): ContentManager,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<UploadedImage>)> {
    let mut file: Option<(Vec<u8>, Option<String>, Option<String>)> = None;
    let mut quiz_id: Option<i64> = None;

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("file") => {
                let filename = field.file_name().map(str::to_string);
                let content_type = field.content_type().map(str::to_string);
                let bytes = field.bytes().await?.to_vec();
                file = Some((bytes, filename, content_type));
            }
            Some("quiz_id") => {
                let text = field.text().await?;
                let text = text.trim();
                if !text.is_empty() {
                    quiz_id =
                        Some(text.parse().map_err(|_| {
                            AppError::Validation("quiz_id: must be a number".into())
                        })?);
                }
            }
            _ => {}
        }
    }

    let (bytes, filename, content_type) =
        file.ok_or_else(|| AppError::BadRequest("missing 'file' field".into()))?;
    if let Some(ct) = &content_type
        && !matches!(
            ct.as_str(),
            "image/png" | "image/jpeg" | "image/jpg" | "image/webp" | "image/gif"
        )
    {
        return Err(AppError::UnsupportedMediaType(
            "only PNG, JPEG, WebP and GIF images are accepted".into(),
        ));
    }

    // Only use the quiz prefix for quizzes this manager can manage.
    if let Some(qid) = quiz_id
        && access::require(&state.db, Content::Quiz, qid, Actor::of(&user))
            .await
            .is_err()
    {
        quiz_id = None;
    }

    let uploaded = store_image(&state, bytes, filename, quiz_id).await?;
    Ok((StatusCode::CREATED, Json(uploaded)))
}

/// Every image/review key currently referenced by a database row.
/// Which of `keys` some row still references. Each key column is indexed, so this stays
/// cheap however many rows there are.
async fn referenced_among(state: &AppState, keys: &[String]) -> AppResult<HashSet<String>> {
    if keys.is_empty() {
        return Ok(HashSet::new());
    }
    let sources = [
        ("quizzes", "cover_image_key"),
        ("questions", "image_key"),
        ("questions", "answer_image_key"),
        ("questions", "explanation_image_key"),
        ("question_options", "image_key"),
        ("interactive_reviews", "r2_key"),
    ];
    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new("");
    for (i, (table, column)) in sources.iter().enumerate() {
        if i > 0 {
            qb.push(" UNION ");
        }
        qb.push(format!("SELECT {column} FROM {table} WHERE {column} IN ("));
        let mut list = qb.separated(", ");
        for key in keys {
            list.push_bind(key);
        }
        list.push_unseparated(")");
    }
    let rows: Vec<(String,)> = qb.build_query_as().fetch_all(&state.db).await?;
    Ok(rows.into_iter().map(|(k,)| k).collect())
}

/// Delete an uploaded image that is not (or no longer) used by any row, e.g. when an
/// editor discards an image before saving.
async fn delete_image(
    _manager: ContentManager,
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> AppResult<StatusCode> {
    if !is_image_key(&key) {
        return Err(AppError::Validation("not a valid image key".into()));
    }
    if !referenced_among(&state, std::slice::from_ref(&key)).await?.is_empty() {
        return Err(AppError::Conflict(
            "this image is still used; remove it from the quiz/question and save instead".into(),
        ));
    }
    state.storage.delete(&key).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Admin only, production only: run the orphan cleanup now (see [`cleanup_orphans_now`]).
async fn cleanup_orphans(
    _admin: RequireAdmin,
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    // A dev server has its own database but may share the R2 bucket with production, so a
    // cleanup there would delete images that live quizzes still use.
    if !state.config.is_production() {
        return Err(AppError::BadRequest(
            "image cleanup is only available in production (APP_ENV=production)".into(),
        ));
    }
    let report = cleanup_orphans_now(&state).await?;
    Ok(Json(json!({
        "scanned": report.scanned,
        "deleted": report.deleted,
    })))
}

pub struct CleanupReport {
    pub scanned: usize,
    pub deleted: usize,
}

/// Serializes cleanup runs, so the admin button and the daily worker never scan/delete at
/// the same time. A caller that arrives mid-run waits, then does its own (usually quick) run.
static CLEANUP_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// How many R2 objects the cleanup lists, checks and deletes per step.
const CLEANUP_BATCH: i32 = 50;

/// Remove R2 objects that no row references. Objects younger than one hour are kept,
/// because an editor may have just uploaded them and not saved yet.
///
/// Works in steps of [`CLEANUP_BATCH`] objects: list one page from R2, ask the database
/// which of those keys are still used, delete the rest, then move to the next page. Memory
/// and query size stay constant however many objects the bucket holds.
pub async fn cleanup_orphans_now(state: &AppState) -> AppResult<CleanupReport> {
    let _guard = CLEANUP_LOCK.lock().await;
    let cutoff = Utc::now() - chrono::Duration::hours(1);

    let mut scanned = 0usize;
    let mut deleted = 0usize;
    for prefix in ["quizzes/", "images/", "reviews/"] {
        let mut token = None;
        loop {
            let (objects, next) = state
                .storage
                .list_page(prefix, token, CLEANUP_BATCH)
                .await?;
            scanned += objects.len();
            let candidates: Vec<String> = objects
                .into_iter()
                .filter(|obj| obj.last_modified.is_none_or(|t| t < cutoff))
                .map(|obj| obj.key)
                .collect();
            let referenced = referenced_among(state, &candidates).await?;
            for key in candidates.iter().filter(|k| !referenced.contains(*k)) {
                state.storage.delete(key).await?;
                tracing::info!(key, "deleted orphaned R2 object");
                deleted += 1;
            }
            match next {
                Some(next) => token = Some(next),
                None => break,
            }
        }
    }
    tracing::info!(scanned, deleted, "orphan cleanup finished");
    Ok(CleanupReport { scanned, deleted })
}

/// Background worker: runs the orphan cleanup once right away (at startup), then every 24 hours.
/// Failures are logged and retried on the next tick; they never stop the server.
pub fn spawn_orphan_cleanup_worker(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            // The first tick completes immediately, so this also covers the startup run.
            interval.tick().await;
            if let Err(err) = cleanup_orphans_now(&state).await {
                tracing::warn!(error = ?err, "scheduled orphan cleanup failed");
            }
        }
    })
}
