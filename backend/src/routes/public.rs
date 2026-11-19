//! Public, no-login endpoints.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::QueryBuilder;

use crate::{
    access::{self, Actor, Content},
    auth::MaybeEditor,
    error::{AppError, AppResult},
    models::{
        CategoryRow, Page, PageQuery, ReviewDto, ReviewRow, SubjectDto, SubjectRow,
        category_columns, review_columns, subject_columns,
    },
    repo,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/subjects", get(list_subjects))
        // Subject slugs are unique per category, so subjects are addressed by both slugs.
        .route("/categories/{slug}/subjects/{subject}", get(get_subject))
        .route(
            "/categories/{slug}/subjects/{subject}/reviews",
            get(list_reviews),
        )
        .route(
            "/categories/{slug}/subjects/{subject}/quizzes",
            get(list_quizzes),
        )
        // Legacy (pre per-category slugs): only works while the slug is used by one category.
        .route("/subjects/{slug}", get(get_subject_legacy))
        .route("/subjects/{slug}/reviews", get(list_reviews_legacy))
        .route("/subjects/{slug}/quizzes", get(list_quizzes_legacy))
        .route("/quizzes/{id}", get(get_quiz))
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if db_ok { "ok" } else { "degraded" },
            "database": if db_ok { "ok" } else { "unavailable" },
        })),
    )
}

#[derive(sqlx::FromRow)]
struct SubjectStats {
    id: i64,
    // MySQL returns EXISTS/COUNT as BIGINT, so decode as integers.
    has_review: i64,
    quiz_count: i64,
}

async fn subject_stats(state: &AppState) -> AppResult<Vec<SubjectStats>> {
    Ok(sqlx::query_as(
        "SELECT s.id, \
           EXISTS(SELECT 1 FROM interactive_reviews r WHERE r.subject_id = s.id AND r.is_published = TRUE) AS has_review, \
           (SELECT COUNT(*) FROM quizzes z WHERE z.subject_id = s.id AND z.is_published = TRUE) AS quiz_count \
         FROM subjects s",
    )
    .fetch_all(&state.db)
    .await?)
}

pub async fn all_categories(state: &AppState) -> AppResult<Vec<CategoryRow>> {
    Ok(sqlx::query_as(concat!(
        "SELECT ",
        category_columns!(),
        " FROM categories ORDER BY sort_order, name"
    ))
    .fetch_all(&state.db)
    .await?)
}

/// All subjects (ordered by category, then subject order), with their category and stats.
pub async fn all_subjects(state: &AppState) -> AppResult<Vec<SubjectDto>> {
    let rows: Vec<SubjectRow> = sqlx::query_as(concat!(
        "SELECT ",
        subject_columns!(),
        " FROM subjects ORDER BY sort_order, name"
    ))
    .fetch_all(&state.db)
    .await?;
    subjects_from_rows(state, rows).await
}

async fn subjects_from_rows(state: &AppState, rows: Vec<SubjectRow>) -> AppResult<Vec<SubjectDto>> {
    let categories = all_categories(state).await?;
    let stats = subject_stats(state).await?;
    let mut subjects: Vec<(usize, SubjectDto)> = rows
        .into_iter()
        .filter_map(|r| {
            let (pos, category) = categories
                .iter()
                .enumerate()
                .find(|(_, y)| y.id == r.category_id)?;
            let s = stats.iter().find(|s| s.id == r.id);
            Some((
                pos,
                SubjectDto::from_row(
                    r,
                    category,
                    s.is_some_and(|s| s.has_review > 0),
                    s.map_or(0, |s| s.quiz_count),
                ),
            ))
        })
        .collect();
    subjects.sort_by_key(|(pos, _)| *pos); // stable: keeps subject order inside a category
    Ok(subjects.into_iter().map(|(_, s)| s).collect())
}

pub async fn subjects_for_category(
    state: &AppState,
    category_slug: &str,
    search: Option<String>,
    limit: Option<i64>,
    offset: i64,
) -> AppResult<(Vec<SubjectDto>, bool)> {
    let mut query = QueryBuilder::<sqlx::MySql>::new(concat!(
        "SELECT s.id, s.category_id, s.slug, s.name, s.description, s.color, s.sort_order, \
         s.created_at, s.updated_at FROM subjects s JOIN categories y ON y.id = s.category_id \
         WHERE y.slug = "
    ));
    query.push_bind(category_slug);
    if let Some(search) = search {
        let pattern = format!("%{search}%");
        query
            .push(" AND (s.name LIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(s.description, '') LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    query.push(" ORDER BY y.sort_order, y.name, s.sort_order, s.name");
    if let Some(limit) = limit {
        query
            .push(" LIMIT ")
            .push_bind(limit + 1)
            .push(" OFFSET ")
            .push_bind(offset);
    }
    let rows: Vec<SubjectRow> = query.build_query_as().fetch_all(&state.db).await?;
    let has_more = limit.is_some_and(|limit| rows.len() > limit as usize);
    let rows = if let Some(limit) = limit {
        rows.into_iter().take(limit as usize).collect()
    } else {
        rows
    };
    Ok((subjects_from_rows(state, rows).await?, has_more))
}

#[derive(Debug, Deserialize)]
struct SubjectsQuery {
    /// Only subjects of this category (category slug).
    category: Option<String>,
}

async fn list_subjects(
    State(state): State<AppState>,
    Query(q): Query<SubjectsQuery>,
) -> AppResult<Json<Vec<SubjectDto>>> {
    let subjects = all_subjects(&state).await?;
    Ok(Json(match q.category {
        Some(category) => subjects
            .into_iter()
            .filter(|s| s.category_slug == category)
            .collect(),
        None => subjects,
    }))
}

/// `/categories/{slug}/subjects/{subject}` path params (`slug` is the category's slug).
#[derive(Debug, Deserialize)]
struct SubjectPath {
    slug: String,
    subject: String,
}

async fn find_subject(
    state: &AppState,
    category: Option<&str>,
    slug: &str,
) -> AppResult<SubjectRow> {
    repo::subject_by_slug(&state.db, category, slug)
        .await?
        .ok_or(AppError::NotFound("subject"))
}

async fn subject_dto(state: &AppState, subject: SubjectRow) -> AppResult<Json<SubjectDto>> {
    all_subjects(state)
        .await?
        .into_iter()
        .find(|s| s.id == subject.id)
        .map(Json)
        .ok_or(AppError::NotFound("subject"))
}

async fn get_subject(
    State(state): State<AppState>,
    Path(p): Path<SubjectPath>,
) -> AppResult<Json<SubjectDto>> {
    let subject = find_subject(&state, Some(&p.slug), &p.subject).await?;
    subject_dto(&state, subject).await
}

async fn get_subject_legacy(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<SubjectDto>> {
    let subject = find_subject(&state, None, &slug).await?;
    subject_dto(&state, subject).await
}

async fn list_reviews(
    State(state): State<AppState>,
    Path(p): Path<SubjectPath>,
    query: Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = find_subject(&state, Some(&p.slug), &p.subject).await?;
    reviews_of(&state, subject, query.0).await
}

async fn list_reviews_legacy(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    query: Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = find_subject(&state, None, &slug).await?;
    reviews_of(&state, subject, query.0).await
}

async fn list_quizzes(
    State(state): State<AppState>,
    Path(p): Path<SubjectPath>,
    query: Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = find_subject(&state, Some(&p.slug), &p.subject).await?;
    quizzes_of(&state, subject, query.0).await
}

async fn list_quizzes_legacy(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    query: Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = find_subject(&state, None, &slug).await?;
    quizzes_of(&state, subject, query.0).await
}

async fn reviews_of(
    state: &AppState,
    subject: SubjectRow,
    q: PageQuery,
) -> AppResult<axum::response::Response> {
    if q.search.is_none() && q.slug.is_none() && q.offset.is_none() && q.limit.is_none() {
        let rows: Vec<ReviewRow> = sqlx::query_as(concat!(
            "SELECT ",
            review_columns!(),
            " FROM interactive_reviews \
             WHERE subject_id = ? AND is_published = TRUE ORDER BY sort_order, title"
        ))
        .bind(subject.id)
        .fetch_all(&state.db)
        .await?;
        let items = rows
            .into_iter()
            .map(|r| ReviewDto::from_row(r, &state.storage))
            .collect::<Vec<_>>();
        return Ok(Json(items).into_response());
    }
    let search = q.search();
    let pattern = search.as_ref().map(|s| format!("%{s}%"));
    let limit = q.limit();
    let mut rows: Vec<ReviewRow> = sqlx::query_as(concat!(
        "SELECT ",
        review_columns!(),
        " FROM interactive_reviews \
         WHERE subject_id = ? AND is_published = TRUE \
        AND (? IS NULL OR slug = ?) \
         AND (? IS NULL OR title LIKE ? OR slug LIKE ?) \
         ORDER BY sort_order, title LIMIT ? OFFSET ?"
    ))
    .bind(subject.id)
    .bind(q.slug.as_deref())
    .bind(q.slug.as_deref())
    .bind(pattern.as_deref())
    .bind(pattern.as_deref())
    .bind(pattern.as_deref())
    .bind(limit + 1)
    .bind(q.offset())
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let items = rows
        .into_iter()
        .map(|r| ReviewDto::from_row(r, &state.storage))
        .collect();
    Ok(Json(Page { items, has_more }).into_response())
}

async fn quizzes_of(
    state: &AppState,
    subject: SubjectRow,
    q: PageQuery,
) -> AppResult<axum::response::Response> {
    if q.search.is_none() && q.offset.is_none() && q.limit.is_none() {
        let quizzes =
            repo::quiz_summaries(&state.db, &state.storage, Some(subject.id), true).await?;
        return Ok(Json(quizzes).into_response());
    }
    let quizzes = repo::quiz_summaries_page(
        &state.db,
        &state.storage,
        subject.id,
        q.search(),
        q.offset(),
        q.limit(),
    )
    .await?;
    Ok(Json(quizzes).into_response())
}

async fn get_quiz(
    State(state): State<AppState>,
    MaybeEditor(manager): MaybeEditor,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let quiz = repo::quiz_detail(&state.db, &state.storage, id)
        .await?
        .ok_or(AppError::NotFound("quiz"))?;
    // Unpublished quizzes are only visible to managers who can manage them (preview).
    if !quiz.summary.is_published {
        let Some(manager) = manager else {
            return Err(AppError::NotFound("quiz"));
        };
        access::require(&state.db, Content::Quiz, id, Actor::of(&manager)).await?;
    }
    Ok(Json(quiz))
}
