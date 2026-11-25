//! Interactive reviews: self-contained `.html` files stored in R2 and shown in a sandboxed
//! iframe. Managers upload, replace and delete them; a manager only manages reviews they own
//! or that were shared with them (see `access`). Only the owner can delete a review.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use std::collections::BTreeSet;

use serde::Serialize;
use sqlx::{MySql, QueryBuilder};

use crate::{
    access::{self, AccessInfo, Actor, Content},
    auth::{RequireEditor, ReviewManager},
    config::Config,
    error::{AppError, AppResult},
    html_minify::minify_review_html_async,
    models::{AdminListQuery, Page, ReviewDto, ReviewRow, review_columns},
    state::AppState,
    validation::{find_image_keys, is_slug, slugify},
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

async fn managed(state: &AppState, id: i64, actor: Actor) -> AppResult<ManagedReview> {
    let access = access::require(&state.db, Content::Review, id, actor).await?;
    let row = load(state, id).await?;
    Ok(ManagedReview {
        review: ReviewDto::from_row(row, &state.storage),
        access,
    })
}

/// Reviews the manager owns or that were shared with them (unpublished included).
/// Filtered by `subject_id` / `category_id` / `search` (title or slug) in SQL; paged
/// (`Page`, max 20) when any of `category_id`, `search`, `offset`, `limit` is given.
async fn list_reviews(
    RequireEditor(user, _): ReviewManager,
    State(state): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> AppResult<Response> {
    let actor = Actor::of(&user);
    let page = q.page();
    let limit = q.is_paged().then(|| page.limit());
    let mut query = QueryBuilder::<MySql>::new(concat!(
        "SELECT ",
        review_columns!(),
        " FROM interactive_reviews WHERE TRUE"
    ));
    if let Some(subject_id) = q.subject_id {
        query
            .push(" AND interactive_reviews.subject_id = ")
            .push_bind(subject_id);
    }
    if let Some(category_id) = q.category_id {
        query
            .push(" AND interactive_reviews.subject_id IN (SELECT id FROM subjects WHERE category_id = ")
            .push_bind(category_id)
            .push(")");
    }
    if let Some(search) = page.search() {
        let pattern = format!("%{search}%");
        query
            .push(" AND (interactive_reviews.title LIKE ")
            .push_bind(pattern.clone())
            .push(" OR interactive_reviews.slug LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    access::push_manageable_filter(&mut query, "interactive_reviews", actor);
    query.push(" ORDER BY subject_id, sort_order, title, id");
    if let Some(limit) = limit {
        query
            .push(" LIMIT ")
            .push_bind(limit + 1)
            .push(" OFFSET ")
            .push_bind(page.offset());
    }
    let mut rows: Vec<ReviewRow> = query.build_query_as().fetch_all(&state.db).await?;
    let has_more = limit.is_some_and(|limit| rows.len() > limit as usize);
    if let Some(limit) = limit {
        rows.truncate(limit as usize);
    }
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    let mut access = access::accessible(&state.db, Content::Review, actor, &ids).await?;
    let items: Vec<ManagedReview> = rows
        .into_iter()
        .filter_map(|r| {
            access.remove(&r.id).map(|access| ManagedReview {
                review: ReviewDto::from_row(r, &state.storage),
                access,
            })
        })
        .collect();
    Ok(if limit.is_some() {
        Json(Page { items, has_more }).into_response()
    } else {
        Json(items).into_response()
    })
}

async fn get_review(
    RequireEditor(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ManagedReview>> {
    Ok(Json(managed(&state, id, Actor::of(&user)).await?))
}

/// multipart: `file` (.html), `subject_id`, `title`, optional `slug`, `sort_order`, `is_published`.
async fn create_review(
    RequireEditor(user, _): ReviewManager,
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
    access::check_can_add_to_subject(&state.db, Actor::of(&user), subject_id).await?;

    let html = minify_review_html_async(html).await;
    let image_keys = find_image_keys(&String::from_utf8_lossy(&html));
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
    let id = res.last_insert_id() as i64;
    set_review_images(&state.db, id, &image_keys).await?;
    Ok((
        StatusCode::CREATED,
        Json(managed(&state, id, Actor::of(&user)).await?),
    ))
}

/// Same fields as create, all optional. A new `file` overwrites the SAME R2 key, so the
/// review's public URL does not change.
async fn update_review(
    RequireEditor(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    multipart: Multipart,
) -> AppResult<Json<ManagedReview>> {
    access::require(&state.db, Content::Review, id, Actor::of(&user)).await?;
    let existing = load(&state, id).await?;
    let form = parse_form(&state, multipart).await?;
    if let Some(subject_id) = form.subject_id
        && subject_id != existing.subject_id
    {
        access::check_can_add_to_subject(&state.db, Actor::of(&user), subject_id).await?;
    }

    if let Some(html) = form.html {
        let html = minify_review_html_async(html).await;
        let image_keys = find_image_keys(&String::from_utf8_lossy(&html));
        // Record the new image references before the new HTML goes live, so the orphan
        // cleanup never sees an image the live HTML uses as unreferenced.
        add_review_images(&state.db, id, &image_keys).await?;
        state
            .storage
            .put(&existing.r2_key, html, HTML_CONTENT_TYPE, HTML_CACHE)
            .await?;
        set_review_images(&state.db, id, &image_keys).await?;
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
    Ok(Json(managed(&state, id, Actor::of(&user)).await?))
}

async fn delete_review(
    RequireEditor(user, _): ReviewManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    access::require(&state.db, Content::Review, id, Actor::of(&user))
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

/// Record that review `review_id` uses `keys`, keeping any rows it already has.
async fn add_review_images<'e, E>(db: E, review_id: i64, keys: &BTreeSet<String>) -> AppResult<()>
where
    E: sqlx::Executor<'e, Database = MySql>,
{
    if keys.is_empty() {
        return Ok(());
    }
    let mut qb =
        QueryBuilder::<MySql>::new("INSERT IGNORE INTO review_images (review_id, image_key) ");
    qb.push_values(keys, |mut row, key| {
        row.push_bind(review_id).push_bind(key);
    });
    qb.build().execute(db).await?;
    Ok(())
}

/// Replace the images review `review_id` uses with exactly `keys` (found in its stored
/// HTML), and mark the review as indexed. The orphan cleanup treats these keys as in use.
pub async fn set_review_images(
    db: &sqlx::MySqlPool,
    review_id: i64,
    keys: &BTreeSet<String>,
) -> AppResult<()> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM review_images WHERE review_id = ?")
        .bind(review_id)
        .execute(&mut *tx)
        .await?;
    add_review_images(&mut *tx, review_id, keys).await?;
    sqlx::query("UPDATE interactive_reviews SET images_indexed = TRUE WHERE id = ?")
        .bind(review_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Scan every review whose HTML has not been indexed yet (reviews created before
/// `review_images` existed) and record the images it embeds. Fails if a review's HTML
/// cannot be read, so the cleanup never deletes images that an unscanned review may use.
pub async fn index_unindexed_reviews(state: &AppState) -> AppResult<usize> {
    let pending: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, r2_key FROM interactive_reviews WHERE images_indexed = FALSE")
            .fetch_all(&state.db)
            .await?;
    for (id, key) in &pending {
        let keys = match state.storage.get(key).await? {
            Some(html) => find_image_keys(&String::from_utf8_lossy(&html)),
            None => {
                tracing::warn!(
                    review_id = id,
                    key,
                    "review HTML missing in R2; indexing as no images"
                );
                BTreeSet::new()
            }
        };
        set_review_images(&state.db, *id, &keys).await?;
        tracing::info!(review_id = id, images = keys.len(), "indexed review images");
    }
    Ok(pending.len())
}
