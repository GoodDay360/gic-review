//! `POST /auth/login`, `POST /auth/logout`, `GET /auth/me`.

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    api_keys::ApiScope,
    auth::{
        AuthMethod, ClientIp, RequireAuth, RequireSession, Role, UserDto, UserRow,
        clear_session_cookie, create_token, dummy_hash, hash_password, session_cookie,
        user_columns, verify_password,
    },
    error::{AppError, AppResult},
    extract::AppJson,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/change-password", post(change_password))
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"), length(max = 255))]
    pub email: String,
    #[validate(length(min = 1, max = 256, message = "is required"))]
    pub password: String,
}

async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    AppJson(body): AppJson<LoginRequest>,
) -> AppResult<(CookieJar, Json<UserDto>)> {
    // Rate limit first, before touching the database or validating input.
    if !state.login_limiter.check(&ip) {
        return Err(AppError::TooManyRequests);
    }
    body.validate()?;

    let email = body.email.trim().to_lowercase();
    let user: Option<UserRow> = sqlx::query_as(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE email = ?"
    ))
    .bind(&email)
    .fetch_optional(&state.db)
    .await?;

    let password = body.password;
    let hash = user
        .as_ref()
        .map(|u| u.password_hash.clone())
        .unwrap_or_else(|| dummy_hash().to_string());
    let valid = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let user = match user {
        Some(u) if valid && u.is_active => u,
        _ => return Err(AppError::InvalidCredentials),
    };
    let role = Role::parse(&user.role).ok_or(AppError::InvalidCredentials)?;
    let token = create_token(&state.config, user.id, role, user.session_version)?;
    tracing::info!(user_id = user.id, "staff login");
    Ok((
        jar.add(session_cookie(&state.config, token)),
        Json(user.into()),
    ))
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, Json<serde_json::Value>) {
    (
        jar.add(clear_session_cookie(&state.config)),
        Json(serde_json::json!({ "ok": true })),
    )
}

#[derive(Debug, Serialize)]
struct MeResponse {
    #[serde(flatten)]
    user: UserDto,
    /// `"session"` (dashboard) or `"api_key"`.
    auth_method: &'static str,
    /// Scopes of the API key used for this request (`null` for sessions).
    api_key_scopes: Option<Vec<ApiScope>>,
}

/// Who am I? Works with the session cookie and with API keys (handy to test a key).
async fn me(RequireAuth(user): RequireAuth) -> Json<MeResponse> {
    let (auth_method, api_key_scopes) = match user.method {
        AuthMethod::Session => ("session", None),
        AuthMethod::ApiKey { scopes, .. } => ("api_key", Some(scopes)),
    };
    Json(MeResponse {
        user: user.row.into(),
        auth_method,
        api_key_scopes,
    })
}

#[derive(Debug, Deserialize)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

/// Change your own password (dashboard session only). The current password must be given and
/// is checked first; attempts are rate-limited per user. On success every other session is
/// signed out and this browser gets a fresh session cookie.
async fn change_password(
    RequireSession(user): RequireSession,
    State(state): State<AppState>,
    jar: CookieJar,
    AppJson(body): AppJson<ChangePassword>,
) -> AppResult<(CookieJar, Json<serde_json::Value>)> {
    if !state
        .login_limiter
        .check(&format!("change-password:{}", user.id))
    {
        return Err(AppError::TooManyRequests);
    }
    let new_len = body.new_password.chars().count();
    if !(8..=256).contains(&new_len) {
        return Err(AppError::Validation(
            "new_password: must be 8 to 256 characters".into(),
        ));
    }
    if body.new_password == body.current_password {
        return Err(AppError::Validation(
            "new_password: must be different from the current password".into(),
        ));
    }

    // Verify the current password before anything else.
    let hash = user.row.password_hash.clone();
    let current = body.current_password;
    let ok = tokio::task::spawn_blocking(move || verify_password(&current, &hash))
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    if !ok {
        return Err(AppError::Validation(
            "current_password: is incorrect".into(),
        ));
    }

    let new_password = body.new_password;
    let new_hash = tokio::task::spawn_blocking(move || hash_password(&new_password))
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .map_err(AppError::Internal)?;
    let new_version = user.row.session_version + 1;
    sqlx::query("UPDATE users SET password_hash = ?, session_version = ? WHERE id = ?")
        .bind(new_hash)
        .bind(new_version)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    // Keep this browser signed in with a token for the new session version.
    let token = create_token(&state.config, user.id, user.role, new_version)?;
    tracing::info!(user_id = user.id, "password changed");
    Ok((
        jar.add(session_cookie(&state.config, token)),
        Json(serde_json::json!({ "ok": true })),
    ))
}
