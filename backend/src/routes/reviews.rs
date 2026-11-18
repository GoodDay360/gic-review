//! Interactive reviews: self-contained `.html` files stored in R2 and shown in a sandboxed
//! iframe. Managers upload, replace and delete them; a manager only manages reviews they own
//! or that were shared with them (see `access`). Only the owner can delete a review.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    access::{self, AccessInfo, Content},
    auth::{RequireManager, ReviewManager},
    config::Config,
    error::{AppError, AppResult},
    models::{ReviewDto, ReviewRow, review_columns},
    state::AppState,
    validation::{is_slug, slugify},
};

pub fn router(config: &Config) -> Router<AppState> {
    let limit = DefaultBodyLimit::max(config.max_review_html_size_bytes + 64 * 1024);
    Router::new()
        .route(
            "/admin/reviews",
            get(list_reviews).post(create_review).layer(limit),
        )
        .route(
            "/admin/reviews/{id}",
            get(get_review)
                .put(update_review)
                .delete(delete_review)
                .layer(limit),
        )
}

const HTML_CACHE: &str = "public, max-age=60";
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";

#[derive(Default)]
struct ReviewForm {
    subject_id: Option<i64>,
    title: Option<String>,
    slug: Option<String>,
    sort_order: Option<i32>,
    is_published: Option<bool>,
    html: Option<Vec<u8>>,
}

async fn parse_form(state: &AppState, mut multipart: Multipart) -> AppResult<ReviewForm> {
    let mut form = ReviewForm::default();
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or_default().to_ascii_lowercase();
                let content_type = field.content_type().unwrap_or_default().to_string();
                if !filename.ends_with(".html") {
                    return Err(AppError::UnsupportedMediaType(
                        "only .html files are accepted".into(),
                    ));
                }
                if !content_type.is_empty()
                    && !content_type.starts_with("text/html")
                    && content_type != "application/octet-stream"
                {
                    return Err(AppError::UnsupportedMediaType(format!(
                        "expected text/html, got {content_type}"
                    )));
                }
                let bytes = field.bytes().await?.to_vec();
                if bytes.len() > state.config.max_review_html_size_bytes {
                    return Err(AppError::PayloadTooLarge(format!(
                        "review HTML must be at most {} MB",
                        state.config.max_review_html_size_bytes / 1024 / 1024
                    )));
                }
                let text = std::str::from_utf8(&bytes).map_err(|_| {
                    AppError::Validation("file: the HTML must be UTF-8 encoded".into())
                })?;
                if !text.to_ascii_lowercase().contains("<html") {
                    return Err(AppError::Validation(
                        "file: this does not look like an HTML document".into(),
                    ));
                }
                form.html = Some(bytes);
            }
            "subject_id" => {
                form.subject_id =
                    Some(field.text().await?.trim().parse().map_err(|_| {
                        AppError::Validation("subject_id: must be a number".into())
                    })?);
            }
            "title" => form.title = Some(field.text().await?.trim().to_string()),
            "slug" => form.slug = Some(field.text().await?.trim().to_string()),
            "sort_order" => {
                let t = field.text().await?;
                if !t.trim().is_empty() {
                    form.sort_order = Some(t.trim().parse().map_err(|_| {
                        AppError::Validation("sort_order: must be a number".into())
                    })?);
                }
            }
            "is_published" => {
                let t = field.text().await?;
                form.is_published = Some(matches!(t.trim(), "true" | "1" | "on"));
            }
            _ => {}
        }
    }
    if let Some(t) = &form.title
        && (t.is_empty() || t.chars().count() > 200)
    {
        return Err(AppError::Validation(
            "title: is required (max 200 characters)".into(),
        ));
    }
    if let Some(s) = &form.slug
        && !s.is_empty()
        && !is_slug(s)
    {
        return Err(AppError::Validation(
            "slug: use lowercase letters, digits and dashes".into(),
        ));
    }
    Ok(form)
}

async fn load(state: &AppState, id: i64) -> AppResult<ReviewRow> {
    sqlx::query_as(concat!(
        "SELECT ",
        review_columns!(),
        " FROM interactive_reviews WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("review"))
}

/// A review as seen by a manager: the review plus who owns it and whether it is shared.
#[derive(Debug, Serialize)]
pub struct ManagedReview {
    #[serde(flatten)]
    pub review: ReviewDto,
    #[serde(flatten)]
    pub access: AccessInfo,
}

async fn managed(state: &AppState, id: i64, user_id: i64) -> AppResult<ManagedReview> {
    let access = access::require(&state.db, Content::Review, id, user_id).await?;
    let row = load(state, id).await?;
    Ok(ManagedReview {
        review: ReviewDto::from_row(row, &state.storage),
        access,
    })
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    subject_id: Option<i64>,
}

/// Reviews the manager owns or that were shared with them (unpublished included).
async fn list_reviews(
    RequireManager(user, _): ReviewManager,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<ManagedReview>>> {
    let mut access = access::accessible(&state.db, Content::Review, user.id).await?;
    let rows: Vec<ReviewRow> = sqlx::query_as(concat!(
        "SELECT ",
        review_columns!(),
        " FROM interactive_reviews \
         WHERE (? IS NULL OR subject_id = ?) ORDER BY subject_id, sort_order, title"
    ))
    .bind(q.subject_id)
    .bind(q.subject_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .filter_map(|r| {
                access.remove(&r.id).map(|access| ManagedReview {
                    review: ReviewDto::from_row(r, &state.storage),
                    access,
                })
            })
            .collect(),
    ))
}

async fn get_review(
    RequireManager(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ManagedReview>> {
    Ok(Json(managed(&state, id, user.id).await?))
}

/// multipart: `file` (.html), `subject_id`, `title`, optional `slug`, `sort_order`, `is_published`.
async fn create_review(
    RequireManager(user, _): ReviewManager,
    State(state): State<AppState>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<ManagedReview>)> {
    let form = parse_form(&state, multipart).await?;
    let subject_id = form
        .subject_id
        .ok_or_else(|| AppError::Validation("subject_id: is required".into()))?;
    let title = form
        .title
        .ok_or_else(|| AppError::Validation("title: is required".into()))?;
    let html = form
        .html
        .ok_or_else(|| AppError::Validation("file: an .html file is required".into()))?;
    let slug = match form.slug.filter(|s| !s.is_empty()) {
        Some(s) => s,
        None => slugify(&title),
    };
    if !is_slug(&slug) {
        return Err(AppError::Validation(
            "slug: could not derive a slug from the title; please provide one".into(),
        ));
    }
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM subjects WHERE id = ?")
        .bind(subject_id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::Validation("subject_id: subject not found".into()));
    }

    let key = format!("reviews/{subject_id}/{}.html", uuid::Uuid::new_v4());
    state
        .storage
        .put(&key, html, HTML_CONTENT_TYPE, HTML_CACHE)
        .await?;

    let res = sqlx::query(
        "INSERT INTO interactive_reviews (subject_id, title, slug, r2_key, sort_order, is_published, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(subject_id)
    .bind(&title)
    .bind(&slug)
    .bind(&key)
    .bind(form.sort_order.unwrap_or(0))
    .bind(form.is_published.unwrap_or(false))
    .bind(user.id)
    .execute(&state.db)
    .await;
    let res = match res {
        Ok(r) => r,
        Err(e) => {
            state.storage.delete_many_best_effort([&key]).await;
            return Err(e.into());
        }
    };
    Ok((
        StatusCode::CREATED,
        Json(managed(&state, res.last_insert_id() as i64, user.id).await?),
    ))
}

/// Same fields as create, all optional. A new `file` overwrites the SAME R2 key, so the
/// review's public URL does not change.
async fn update_review(
    RequireManager(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> AppResult<Json<ManagedReview>> {
    access::require(&state.db, Content::Review, id, user.id).await?;
    let existing = load(&state, id).await?;
    let form = parse_form(&state, multipart).await?;

    if let Some(html) = form.html {
        state
            .storage
            .put(&existing.r2_key, html, HTML_CONTENT_TYPE, HTML_CACHE)
            .await?;
    }
    let slug = form.slug.filter(|s| !s.is_empty()).unwrap_or(existing.slug);
    // Note: the R2 key keeps its original subject prefix even if the subject changes.
    sqlx::query(
        "UPDATE interactive_reviews SET subject_id = ?, title = ?, slug = ?, sort_order = ?, \
         is_published = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(form.subject_id.unwrap_or(existing.subject_id))
    .bind(form.title.unwrap_or(existing.title))
    .bind(slug)
    .bind(form.sort_order.unwrap_or(existing.sort_order))
    .bind(form.is_published.unwrap_or(existing.is_published))
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(Json(managed(&state, id, user.id).await?))
}

async fn delete_review(
    RequireManager(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    access::require(&state.db, Content::Review, id, user.id)
        .await?
        .require_owner("delete this review")?;
    let existing = load(&state, id).await?;
    sqlx::query("DELETE FROM interactive_reviews WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    state
        .storage
        .delete_many_best_effort([existing.r2_key])
        .await;
    Ok(StatusCode::NO_CONTENT)
}
