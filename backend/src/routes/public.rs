//! Public, no-login endpoints.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde_json::json;

use crate::{
    access::{self, Content},
    auth::MaybeManager,
    error::{AppError, AppResult},
    models::{ReviewDto, ReviewRow, SubjectDto, SubjectRow, review_columns, subject_columns},
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

pub async fn all_subjects(state: &AppState) -> AppResult<Vec<SubjectDto>> {
    let rows: Vec<SubjectRow> = sqlx::query_as(concat!(
        "SELECT ",
        subject_columns!(),
        " FROM subjects ORDER BY sort_order, name"
    ))
    .fetch_all(&state.db)
    .await?;
    let stats = subject_stats(state).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let s = stats.iter().find(|s| s.id == r.id);
            SubjectDto::from_row(
                r,
                s.is_some_and(|s| s.has_review > 0),
                s.map_or(0, |s| s.quiz_count),
            )
        })
        .collect())
}

async fn list_subjects(State(state): State<AppState>) -> AppResult<Json<Vec<SubjectDto>>> {
    Ok(Json(all_subjects(&state).await?))
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
) -> AppResult<Json<Vec<ReviewDto>>> {
    let subject = repo::subject_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("subject"))?;
    let rows: Vec<ReviewRow> = sqlx::query_as(concat!(
        "SELECT ",
        review_columns!(),
        " FROM interactive_reviews \
         WHERE subject_id = ? AND is_published = TRUE ORDER BY sort_order, title"
    ))
    .bind(subject.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ReviewDto::from_row(r, &state.storage))
            .collect(),
    ))
}

async fn list_quizzes(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<impl IntoResponse> {
    let subject = repo::subject_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound("subject"))?;
    let quizzes = repo::quiz_summaries(&state.db, &state.storage, Some(subject.id), true).await?;
    Ok(Json(quizzes))
}

async fn get_quiz(
    State(state): State<AppState>,
    MaybeManager(manager): MaybeManager,
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
        access::require(&state.db, Content::Quiz, id, manager.id).await?;
    }
    Ok(Json(quiz))
}
