//! API key management (dashboard session only; an API key can never manage keys).
//!
//! - Managers create, regenerate and revoke **their own** key (max 1 active key each).
//! - Admins list and revoke **every** manager's keys (they cannot create keys).

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{
    api_keys::{
        self, ApiScope, DISPLAY_PREFIX_LEN, MAX_ACTIVE_KEYS_PER_MANAGER, join_scopes, parse_scopes,
    },
    auth::{RequireAdmin, RequireManagerSession, RequireSession, Role},
    error::{AppError, AppResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/api-keys", get(list_all).post(create_key))
        .route("/admin/api-keys/mine", get(list_mine))
        .route("/admin/api-keys/{id}", delete(revoke_key))
        .route("/admin/api-keys/{id}/revoke", post(revoke_key))
}

#[derive(Debug, sqlx::FromRow)]
struct KeyRow {
    id: i64,
    user_id: i64,
    owner_email: String,
    owner_name: String,
    name: String,
    key_prefix: String,
    scopes: String,
    expires_at: Option<NaiveDateTime>,
    last_used_at: Option<NaiveDateTime>,
    revoked_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    status: String,
}

macro_rules! key_select {
    () => {
        "SELECT k.id, k.user_id, u.email AS owner_email, u.display_name AS owner_name, k.name, \
         k.key_prefix, k.scopes, k.expires_at, k.last_used_at, k.revoked_at, k.created_at, \
         CASE WHEN k.revoked_at IS NOT NULL THEN 'revoked' \
              WHEN k.expires_at IS NOT NULL AND k.expires_at <= NOW() THEN 'expired' \
              ELSE 'active' END AS status \
         FROM api_keys k JOIN users u ON u.id = k.user_id "
    };
}

#[derive(Debug, Serialize)]
pub struct ApiKeyDto {
    id: i64,
    user_id: i64,
    owner_email: String,
    owner_name: String,
    name: String,
    /// The first characters of the key, e.g. `gic_3f9a1c2b…`, to recognise it.
    key_prefix: String,
    scopes: Vec<ApiScope>,
    /// `active`, `revoked` or `expired`.
    status: String,
    expires_at: Option<NaiveDateTime>,
    last_used_at: Option<NaiveDateTime>,
    revoked_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
}

impl From<KeyRow> for ApiKeyDto {
    fn from(r: KeyRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            owner_email: r.owner_email,
            owner_name: r.owner_name,
            name: r.name,
            key_prefix: r.key_prefix,
            scopes: parse_scopes(&r.scopes),
            status: r.status,
            expires_at: r.expires_at,
            last_used_at: r.last_used_at,
            revoked_at: r.revoked_at,
            created_at: r.created_at,
        }
    }
}

async fn load(state: &AppState, id: i64) -> AppResult<KeyRow> {
    sqlx::query_as(concat!(key_select!(), "WHERE k.id = ?"))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("API key"))
}

/// Manager: their own keys (newest first, revoked/expired included for history).
async fn list_mine(
    RequireManagerSession(user): RequireManagerSession,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ApiKeyDto>>> {
    let rows: Vec<KeyRow> = sqlx::query_as(concat!(
        key_select!(),
        "WHERE k.user_id = ? ORDER BY k.created_at DESC, k.id DESC LIMIT 50"
    ))
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    user_id: Option<i64>,
}

/// Admin: every manager's keys, optionally for one user.
async fn list_all(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<ApiKeyDto>>> {
    let rows: Vec<KeyRow> = sqlx::query_as(concat!(
        key_select!(),
        "WHERE (? IS NULL OR k.user_id = ?) ORDER BY u.display_name, k.created_at DESC LIMIT 500"
    ))
    .bind(q.user_id)
    .bind(q.user_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateKey {
    name: String,
    scopes: Vec<ApiScope>,
    /// 1–365 days, or `null` for a key that never expires.
    expires_in_days: Option<u32>,
    /// Revoke the current active key and create a new one ("regenerate").
    #[serde(default)]
    replace_existing: bool,
}

#[derive(Debug, Serialize)]
struct CreatedKey {
    /// The full key. It is shown only in this response and cannot be retrieved later.
    key: String,
    api_key: ApiKeyDto,
}

/// Manager: create (or regenerate) their API key. Max one active key per manager.
async fn create_key(
    RequireManagerSession(user): RequireManagerSession,
    State(state): State<AppState>,
    Json(body): Json<CreateKey>,
) -> AppResult<(StatusCode, Json<CreatedKey>)> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::Validation(
            "name: is required (max 100 characters)".into(),
        ));
    }
    let mut scopes: Vec<ApiScope> = Vec::new();
    for s in body.scopes {
        if !scopes.contains(&s) {
            scopes.push(s);
        }
    }
    if scopes.is_empty() {
        return Err(AppError::Validation(
            "scopes: choose at least one of quizzes:write, reviews:write".into(),
        ));
    }
    if let Some(days) = body.expires_in_days
        && !(1..=365).contains(&days)
    {
        return Err(AppError::Validation(
            "expires_in_days: must be between 1 and 365, or null for no expiry".into(),
        ));
    }

    let key = api_keys::generate();
    let mut tx = state.db.begin().await?;
    // Lock the user row so two parallel requests cannot both pass the limit check.
    sqlx::query("SELECT id FROM users WHERE id = ? FOR UPDATE")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    if body.replace_existing {
        sqlx::query(
            "UPDATE api_keys SET revoked_at = NOW() WHERE user_id = ? AND revoked_at IS NULL",
        )
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    }
    let (active,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM api_keys WHERE user_id = ? AND revoked_at IS NULL \
         AND (expires_at IS NULL OR expires_at > NOW())",
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;
    if active >= MAX_ACTIVE_KEYS_PER_MANAGER {
        return Err(AppError::Conflict(
            "you already have an active API key; regenerate it or revoke it first".into(),
        ));
    }
    let res = sqlx::query(
        "INSERT INTO api_keys (user_id, name, key_prefix, key_hash, scopes, expires_at) \
         VALUES (?, ?, ?, ?, ?, IF(? IS NULL, NULL, NOW() + INTERVAL ? DAY))",
    )
    .bind(user.id)
    .bind(&name)
    .bind(&key[..DISPLAY_PREFIX_LEN])
    .bind(api_keys::hash(&key))
    .bind(join_scopes(&scopes))
    .bind(body.expires_in_days)
    .bind(body.expires_in_days)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let row = load(&state, res.last_insert_id() as i64).await?;
    tracing::info!(user_id = user.id, key_id = row.id, "API key created");
    Ok((
        StatusCode::CREATED,
        Json(CreatedKey {
            key,
            api_key: row.into(),
        }),
    ))
}

/// Revoke a key. Managers: only their own. Admins: any manager's key.
async fn revoke_key(
    RequireSession(user): RequireSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiKeyDto>> {
    let row = load(&state, id).await?;
    if user.role != Role::Admin && row.user_id != user.id {
        // Don't reveal that another user's key exists.
        return Err(AppError::NotFound("API key"));
    }
    sqlx::query("UPDATE api_keys SET revoked_at = NOW() WHERE id = ? AND revoked_at IS NULL")
        .bind(id)
        .execute(&state.db)
        .await?;
    tracing::info!(by_user = user.id, key_id = id, "API key revoked");
    Ok(Json(load(&state, id).await?.into()))
}
