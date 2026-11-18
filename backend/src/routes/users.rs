//! Admin-only manager account management. Created users are always `manager`;
//! there is deliberately no API path that creates or modifies an admin.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    auth::{RequireAdmin, UserDto, UserRow, hash_password, user_columns},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/users/{id}", patch(update_user).delete(delete_user))
        .route("/admin/users/{id}/reset-password", post(reset_password))
}

#[derive(Debug, Deserialize, Validate)]
struct CreateUser {
    #[validate(email(message = "must be a valid email"), length(max = 255))]
    email: String,
    #[validate(length(min = 8, max = 256, message = "must be 8 to 256 characters"))]
    password: String,
    #[validate(length(min = 1, max = 100, message = "is required (max 100 characters)"))]
    display_name: String,
}

#[derive(Debug, Deserialize, Validate)]
struct UpdateUser {
    is_active: Option<bool>,
    #[validate(length(min = 1, max = 100, message = "must be 1 to 100 characters"))]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
struct ResetPassword {
    #[validate(length(min = 8, max = 256, message = "must be 8 to 256 characters"))]
    password: String,
}

async fn load(state: &AppState, id: i64) -> AppResult<UserRow> {
    sqlx::query_as(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound("user"))
}

/// Only managers can be changed through the API.
async fn load_manager(state: &AppState, id: i64) -> AppResult<UserRow> {
    let user = load(state, id).await?;
    if user.role != "manager" {
        return Err(AppError::Forbidden);
    }
    Ok(user)
}

async fn hash(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .map_err(AppError::Internal)
}

async fn list_users(
    _admin: RequireAdmin,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<UserDto>>> {
    let rows: Vec<UserRow> = sqlx::query_as(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users ORDER BY role, display_name"
    ))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn create_user(
    RequireAdmin(admin): RequireAdmin,
    State(state): State<AppState>,
    Json(body): Json<CreateUser>,
) -> AppResult<(StatusCode, Json<UserDto>)> {
    body.validate()?;
    let password_hash = hash(body.password).await?;
    let res = sqlx::query(
        "INSERT INTO users (email, password_hash, display_name, role, is_active, created_by) \
         VALUES (?, ?, ?, 'manager', TRUE, ?)",
    )
    .bind(body.email.trim().to_lowercase())
    .bind(password_hash)
    .bind(body.display_name.trim())
    .bind(admin.id)
    .execute(&state.db)
    .await?;
    let user = load(&state, res.last_insert_id() as i64).await?;
    tracing::info!(admin_id = admin.id, user_id = user.id, "manager created");
    Ok((StatusCode::CREATED, Json(user.into())))
}

async fn update_user(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUser>,
) -> AppResult<Json<UserDto>> {
    body.validate()?;
    let user = load_manager(&state, id).await?;
    sqlx::query("UPDATE users SET is_active = ?, display_name = ? WHERE id = ?")
        .bind(body.is_active.unwrap_or(user.is_active))
        .bind(
            body.display_name
                .map(|d| d.trim().to_string())
                .unwrap_or(user.display_name),
        )
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(load(&state, id).await?.into()))
}

async fn reset_password(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ResetPassword>,
) -> AppResult<StatusCode> {
    body.validate()?;
    load_manager(&state, id).await?;
    let password_hash = hash(body.password).await?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(password_hash)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_user(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    load_manager(&state, id).await?;
    // Their content is handed to the manager it was shared with first. Content that was
    // never shared keeps existing (and stays public if published) with no owner.
    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE quizzes z SET z.created_by = \
           (SELECT s.user_id FROM quiz_shares s WHERE s.quiz_id = z.id \
            ORDER BY s.created_at, s.user_id LIMIT 1) \
         WHERE z.created_by = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE interactive_reviews r SET r.created_by = \
           (SELECT s.user_id FROM review_shares s WHERE s.review_id = r.id \
            ORDER BY s.created_at, s.user_id LIMIT 1) \
         WHERE r.created_by = ?",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    // New owners no longer need a share entry for their own content.
    sqlx::query(
        "DELETE s FROM quiz_shares s JOIN quizzes z ON z.id = s.quiz_id WHERE s.user_id = z.created_by",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "DELETE s FROM review_shares s JOIN interactive_reviews r ON r.id = s.review_id \
         WHERE s.user_id = r.created_by",
    )
    .execute(&mut *tx)
    .await?;
    // Deleting the user also removes their API keys and the shares they received.
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
