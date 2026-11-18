//! Sharing a year, or one subject of a year, with other users (dashboard session only).
//!
//! - `GET    /admin/{years|subjects}/{id}/shares`            year owner, admins, and the people it
//!   is shared with can view
//! - `POST   /admin/{years|subjects}/{id}/shares` `{email}`  year owner or admin
//! - `DELETE /admin/{years|subjects}/{id}/shares/{user_id}`  year owner or admin, or a recipient
//!   removing themselves ("leave")
//!
//! A shared year gives access to every subject in it; a shared subject to that subject only.
//! Either way the recipient can add content there and view, edit and publish all content in it.
//! API keys cannot change sharing, so a leaked key can't grant anyone access.

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    access::{self, Actor},
    auth::RequireEditorSession,
    error::{AppError, AppResult},
    extract::AppJson,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/years/{id}/shares",
            get(list_year_shares).post(add_year_share),
        )
        .route(
            "/admin/years/{id}/shares/{user_id}",
            delete(remove_year_share),
        )
        .route(
            "/admin/subjects/{id}/shares",
            get(list_subject_shares).post(add_subject_share),
        )
        .route(
            "/admin/subjects/{id}/shares/{user_id}",
            delete(remove_subject_share),
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Year,
    Subject,
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
    /// The year's owner (for a subject: the owner of its year).
    owner: Option<Person>,
    shared_with: Vec<SharedWith>,
    /// Whether the caller owns the year.
    you_are_owner: bool,
    /// Whether the caller may add/remove people (year owner or admin).
    can_manage: bool,
}

#[derive(Debug, Deserialize)]
struct AddShare {
    email: String,
}

/// (year id, year owner) of the target; `404` if it doesn't exist.
async fn year_of(state: &AppState, target: Target, id: i64) -> AppResult<(i64, Option<i64>)> {
    let row: Option<(i64, Option<i64>)> = match target {
        Target::Year => {
            sqlx::query_as("SELECT id, created_by FROM years WHERE id = ?")
                .bind(id)
                .fetch_optional(&state.db)
                .await?
        }
        Target::Subject => {
            sqlx::query_as(
                "SELECT y.id, y.created_by FROM subjects s JOIN years y ON y.id = s.year_id WHERE s.id = ?",
            )
            .bind(id)
            .fetch_optional(&state.db)
            .await?
        }
    };
    row.ok_or(AppError::NotFound(match target {
        Target::Year => "year",
        Target::Subject => "subject",
    }))
}

async fn list(state: &AppState, target: Target, id: i64, actor: Actor) -> AppResult<Shares> {
    let (year_id, owner_id) = year_of(state, target, id).await?;
    let can_manage = access::can_manage_year(actor, owner_id);
    if !can_manage {
        // Recipients may see who else has access (a subject's list is visible to people the
        // subject or its whole year is shared with).
        let mine = access::shared_with(&state.db, actor.id).await?;
        let allowed = match target {
            Target::Year => mine.years.contains(&id),
            Target::Subject => mine.subjects.contains(&id) || mine.years.contains(&year_id),
        };
        if !allowed {
            return Err(AppError::OwnerOnly("see who this is shared with".into()));
        }
    }
    let owner: Option<Person> = match owner_id {
        Some(owner_id) => {
            sqlx::query_as("SELECT id AS user_id, email, display_name FROM users WHERE id = ?")
                .bind(owner_id)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };
    let shared_with: Vec<SharedWith> = match target {
        Target::Year => {
            sqlx::query_as(
                "SELECT u.id AS user_id, u.email, u.display_name, s.created_at AS shared_at \
                 FROM year_shares s JOIN users u ON u.id = s.user_id \
                 WHERE s.year_id = ? ORDER BY s.created_at, u.display_name",
            )
            .bind(id)
            .fetch_all(&state.db)
            .await?
        }
        Target::Subject => {
            sqlx::query_as(
                "SELECT u.id AS user_id, u.email, u.display_name, s.created_at AS shared_at \
                 FROM subject_shares s JOIN users u ON u.id = s.user_id \
                 WHERE s.subject_id = ? ORDER BY s.created_at, u.display_name",
            )
            .bind(id)
            .fetch_all(&state.db)
            .await?
        }
    };
    Ok(Shares {
        owner,
        shared_with,
        you_are_owner: owner_id == Some(actor.id),
        can_manage,
    })
}

async fn add(
    state: &AppState,
    target: Target,
    id: i64,
    actor: Actor,
    email: &str,
) -> AppResult<Shares> {
    let (_, owner_id) = year_of(state, target, id).await?;
    if !access::can_manage_year(actor, owner_id) {
        return Err(AppError::OwnerOnly("share this".into()));
    }
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Err(AppError::Validation("email: is required".into()));
    }
    // Only active staff accounts (managers or admins) can receive access.
    let found: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = ? AND is_active = TRUE")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    let (target_id,) = found.ok_or_else(|| {
        AppError::Validation(format!(
            "no active manager or admin account uses the email {email}"
        ))
    })?;
    if Some(target_id) == owner_id {
        return Err(AppError::Validation(
            "that account already owns this year".into(),
        ));
    }
    if target_id == actor.id {
        return Err(AppError::Validation(
            "share it with someone else, not yourself".into(),
        ));
    }
    let res = match target {
        Target::Year => {
            sqlx::query(
                "INSERT IGNORE INTO year_shares (year_id, user_id, granted_by) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(target_id)
            .bind(actor.id)
            .execute(&state.db)
            .await?
        }
        Target::Subject => sqlx::query(
            "INSERT IGNORE INTO subject_shares (subject_id, user_id, granted_by) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(target_id)
        .bind(actor.id)
        .execute(&state.db)
        .await?,
    };
    if res.rows_affected() == 0 {
        return Err(AppError::Conflict(format!("already shared with {email}")));
    }
    tracing::info!(?target, id, by = actor.id, to = target_id, "access shared");
    list(state, target, id, actor).await
}

async fn remove(
    state: &AppState,
    target: Target,
    id: i64,
    actor: Actor,
    target_id: i64,
) -> AppResult<Shares> {
    let (_, owner_id) = year_of(state, target, id).await?;
    let can_manage = access::can_manage_year(actor, owner_id);
    let leaving = target_id == actor.id;
    if !leaving && !can_manage {
        return Err(AppError::OwnerOnly("remove other people".into()));
    }
    match target {
        Target::Year => {
            sqlx::query("DELETE FROM year_shares WHERE year_id = ? AND user_id = ?")
                .bind(id)
                .bind(target_id)
                .execute(&state.db)
                .await?
        }
        Target::Subject => {
            sqlx::query("DELETE FROM subject_shares WHERE subject_id = ? AND user_id = ?")
                .bind(id)
                .bind(target_id)
                .execute(&state.db)
                .await?
        }
    };
    if leaving && !can_manage {
        // The caller just gave up their access; nothing more to show them.
        return Ok(Shares {
            owner: None,
            shared_with: Vec::new(),
            you_are_owner: false,
            can_manage: false,
        });
    }
    list(state, target, id, actor).await
}

async fn list_year_shares(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        list(&state, Target::Year, id, Actor::of(&user)).await?,
    ))
}

async fn add_year_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<AddShare>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        add(&state, Target::Year, id, Actor::of(&user), &body.email).await?,
    ))
}

async fn remove_year_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path((id, target)): Path<(i64, i64)>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        remove(&state, Target::Year, id, Actor::of(&user), target).await?,
    ))
}

async fn list_subject_shares(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        list(&state, Target::Subject, id, Actor::of(&user)).await?,
    ))
}

async fn add_subject_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<AddShare>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        add(&state, Target::Subject, id, Actor::of(&user), &body.email).await?,
    ))
}

async fn remove_subject_share(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path((id, target)): Path<(i64, i64)>,
) -> AppResult<Json<Shares>> {
    Ok(Json(
        remove(&state, Target::Subject, id, Actor::of(&user), target).await?,
    ))
}
