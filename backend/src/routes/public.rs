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
        Page, PageQuery, ReviewDto, ReviewRow, SubjectDto, SubjectRow, YearRow, review_columns,
        subject_columns, year_columns,
    },
    repo,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/subjects", get(list_subjects))
        .route("/subjects/{slug}", get(get_subject))
        .route("/subjects/{slug}/reviews", get(list_reviews))
        .route("/subjects/{slug}/quizzes", get(list_quizzes))
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

pub async fn all_years(state: &AppState) -> AppResult<Vec<YearRow>> {
    Ok(sqlx::query_as(concat!(
        "SELECT ",
        year_columns!(),
        " FROM years ORDER BY sort_order, name"
    ))
    .fetch_all(&state.db)
    .await?)
}

/// All subjects (ordered by year, then subject order), with their year and stats.
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
    let years = all_years(state).await?;
    let stats = subject_stats(state).await?;
    let mut subjects: Vec<(usize, SubjectDto)> = rows
        .into_iter()
        .filter_map(|r| {
            let (pos, year) = years.iter().enumerate().find(|(_, y)| y.id == r.year_id)?;
            let s = stats.iter().find(|s| s.id == r.id);
            Some((
                pos,
                SubjectDto::from_row(
                    r,
                    year,
                    s.is_some_and(|s| s.has_review > 0),
                    s.map_or(0, |s| s.quiz_count),
                ),
            ))
        })
        .collect();
    subjects.sort_by_key(|(pos, _)| *pos); // stable: keeps subject order inside a year
    Ok(subjects.into_iter().map(|(_, s)| s).collect())
}

pub async fn subjects_for_year(
    state: &AppState,
    year_slug: &str,
    search: Option<String>,
    limit: Option<i64>,
    offset: i64,
) -> AppResult<(Vec<SubjectDto>, bool)> {
    let mut query = QueryBuilder::<sqlx::MySql>::new(concat!(
        "SELECT s.id, s.year_id, s.slug, s.name, s.description, s.color, s.sort_order, \
         s.created_at, s.updated_at FROM subjects s JOIN years y ON y.id = s.year_id \
         WHERE y.slug = "
    ));
    query.push_bind(year_slug);
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
    /// Only subjects of this year (year slug).
    year: Option<String>,
}

async fn list_subjects(
    State(state): State<AppState>,
    Query(q): Query<SubjectsQuery>,
) -> AppResult<Json<Vec<SubjectDto>>> {
    let subjects = all_subjects(&state).await?;
    Ok(Json(match q.year {
        Some(year) => subjects
            .into_iter()
            .filter(|s| s.year_slug == year)
            .collect(),
        None => subjects,
    }))
}

async fn get_subject(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<SubjectDto>> {
    all_subjects(&state)
        .await?
        .into_iter()
        .find(|s| s.slug == slug)
        .map(Json)
        .ok_or(AppError::NotFound("subject"))
}

async fn list_reviews(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = repo::subject_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("subject"))?;
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

async fn list_quizzes(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    let subject = repo::subject_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("subject"))?;
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
