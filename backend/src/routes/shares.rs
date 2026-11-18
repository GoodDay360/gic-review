//! Sharing quizzes and interactive reviews with other managers (dashboard session only).
//!
//! - `GET    /admin/{quizzes|reviews}/{id}/shares`            owner + shared managers can view
//! - `POST   /admin/{quizzes|reviews}/{id}/shares` `{email}`  owner only
//! - `DELETE /admin/{quizzes|reviews}/{id}/shares/{user_id}`  owner, or a manager removing
//!   themselves ("leave")
//!
//! API keys cannot change sharing, so a leaked key can't grant anyone access.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    access::{self, Actor, Content},
    auth::RequireEditorSession,
    error::{AppError, AppResult},
    extract::AppJson,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/quizzes/{id}/shares",
            get(list_quiz_shares).post(add_quiz_share),
        )
        .route(
            "/admin/quizzes/{id}/shares/{user_id}",
            delete(remove_quiz_share),
        )
        .route(
            "/admin/reviews/{id}/shares",
            get(list_review_shares).post(add_review_share),
        )
        .route(
            "/admin/reviews/{id}/shares/{user_id}",
            delete(remove_review_share),
        )
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Person {
    user_id: i64,
    email: String,
    display_name: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SharedWith {
    user_id: i64,
    email: String,
    display_name: String,
    shared_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct Shares {
    owner: Option<Person>,
    shared_with: Vec<SharedWith>,
    /// Whether the caller is the owner.
    you_are_owner: bool,
    /// Whether the caller may add/remove people (owner or admin).
    can_manage: bool,
}

#[derive(Debug, Deserialize)]
struct AddShare {
    email: String,
}

async fn list(state: &AppState, kind: Content, id: i64, actor: Actor) -> AppResult<Shares> {
    let access = access::require(&state.db, kind, id, actor).await?;
    let owner: Option<Person> = match access.owner_id {
        Some(owner_id) => {
            sqlx::query_as("SELECT id AS user_id, email, display_name FROM users WHERE id = ?")
                .bind(owner_id)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };
    let shared_with: Vec<SharedWith> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT u.id AS user_id, u.email, u.display_name, s.created_at AS shared_at \
                 FROM quiz_shares s JOIN users u ON u.id = s.user_id \
                 WHERE s.quiz_id = ? ORDER BY s.created_at, u.display_name",
            )
            .bind(id)
            .fetch_all(&state.db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT u.id AS user_id, u.email, u.display_name, s.created_at AS shared_at \
                 FROM review_shares s JOIN users u ON u.id = s.user_id \
                 WHERE s.review_id = ? ORDER BY s.created_at, u.display_name",
            )
            .bind(id)
            .fetch_all(&state.db)
            .await?
        }
    };
    Ok(Shares {
        owner,
        shared_with,
        you_are_owner: access.is_owner(),
        can_manage: access.can_manage(),
    })
}

async fn add(
    state: &AppState,
    kind: Content,
    id: i64,
    actor: Actor,
    email: &str,
) -> AppResult<Shares> {
    let user_id = actor.id;
    let access = access::require(&state.db, kind, id, actor).await?;
    access.require_owner("share this")?;
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Err(AppError::Validation("email: is required".into()));
    }
    // Only active staff accounts (managers or admins) can receive content.
    let target: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = ? AND is_active = TRUE")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    let (target_id,) = target.ok_or_else(|| {
        AppError::Validation(format!(
            "no active manager or admin account uses the email {email}"
        ))
    })?;
    if Some(target_id) == access.owner_id {
        return Err(AppError::Validation(
            "that account already owns this".into(),
        ));
    }
    if target_id == user_id {
        return Err(AppError::Validation(
            "you already own this; share it with someone else".into(),
        ));
    }
    let res = match kind {
        Content::Quiz => {
            sqlx::query(
                "INSERT IGNORE INTO quiz_shares (quiz_id, user_id, granted_by) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(target_id)
            .bind(user_id)
            .execute(&state.db)
            .await?
        }
        Content::Review => sqlx::query(
            "INSERT IGNORE INTO review_shares (review_id, user_id, granted_by) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(target_id)
        .bind(user_id)
        .execute(&state.db)
        .await?,
    };
    if res.rows_affected() == 0 {
        return Err(AppError::Conflict(format!("already shared with {email}")));
    }
    tracing::info!(
        kind = kind.label(),
        id,
        by = user_id,
        to = target_id,
        "content shared"
    );
    list(state, kind, id, actor).await
}

async fn remove(
    state: &AppState,
    kind: Content,
    id: i64,
    actor: Actor,
    target_id: i64,
) -> AppResult<Shares> {
    let user_id = actor.id;
    let access = access::require(&state.db, kind, id, actor).await?;
    let leaving = target_id == user_id;
    if !leaving {
        access.require_owner("remove other people")?;
    }
    match kind {
        Content::Quiz => {
            sqlx::query("DELETE FROM quiz_shares WHERE quiz_id = ? AND user_id = ?")
                .bind(id)
                .bind(target_id)
                .execute(&state.db)
                .await?
        }
        Content::Review => {
            sqlx::query("DELETE FROM review_shares WHERE review_id = ? AND user_id = ?")
                .bind(id)
                .bind(target_id)
                .execute(&state.db)
                .await?
        }
    };
    if leaving && !access.can_manage() {
        // The caller just lost access; there is nothing more they may see.
        return Ok(Shares {
            owner: None,
            shared_with: Vec::new(),
            you_are_owner: false,
            can_manage: false,
        });
    }
    list(state, kind, id, actor).await
}

async fn list_quiz_shares(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        list(&state, Content::Quiz, id, Actor::of(&user)).await?,
    ))
}

async fn add_quiz_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<AddShare>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        add(&state, Content::Quiz, id, Actor::of(&user), &body.email).await?,
    ))
}

async fn remove_quiz_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path((id, target)): Path<(i64, i64)>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        remove(&state, Content::Quiz, id, Actor::of(&user), target).await?,
    ))
}

async fn list_review_shares(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        list(&state, Content::Review, id, Actor::of(&user)).await?,
    ))
}

async fn add_review_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<AddShare>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        add(&state, Content::Review, id, Actor::of(&user), &body.email).await?,
    ))
}

async fn remove_review_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path((id, target)): Path<(i64, i64)>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        remove(&state, Content::Review, id, Actor::of(&user), target).await?,
    ))
}
