//! Password hashing (Argon2id), JWT session cookies and the auth extractors
//! `RequireStaff`, `RequireAdmin` and `MaybeStaff`.
//!
//! Every staff request re-reads the user's role and `is_active` flag from the database,
//! so deactivating a manager takes effect immediately even if their JWT is still valid.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    state::AppState,
};

pub const SESSION_COOKIE: &str = "gic_session";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Manager,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "manager" => Some(Self::Manager),
            _ => None,
        }
    }
}

/// A user row as stored in the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// The public view of a user (never includes the password hash).
#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub is_active: bool,
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<UserRow> for UserDto {
    fn from(u: UserRow) -> Self {
        Self {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            role: Role::parse(&u.role).unwrap_or(Role::Manager),
            is_active: u.is_active,
            created_by: u.created_by,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! user_columns {
    () => {
        "id, email, password_hash, display_name, role, is_active, created_by, created_at, updated_at"
    };
}
pub(crate) use user_columns;

// ---------- Passwords ----------

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    // Argon2::default() is Argon2id v19 with the recommended parameters.
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("argon2 hash failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// A fixed, valid Argon2id hash used to spend the same time verifying a password when
/// the email does not exist, so login timing does not reveal which emails are registered.
pub fn dummy_hash() -> &'static str {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("not-a-real-password").unwrap_or_default())
}

// ---------- JWT ----------

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: Role,
    pub iat: i64,
    pub exp: i64,
}

pub fn create_token(config: &Config, user_id: i64, role: Role) -> anyhow::Result<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        role,
        iat: now,
        exp: now + config.jwt_expires_in_hours * 3600,
    };
    let key = EncodingKey::from_secret(config.jwt_secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid JWT secret: {e}"))?;
    encode(&Header::new(Algorithm::HS256), &claims, &key)
        .map_err(|e| anyhow::anyhow!("jwt encode failed: {e}"))
}

pub fn decode_token(config: &Config, token: &str) -> Option<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 30;
    let key = DecodingKey::from_secret(config.jwt_secret.as_bytes()).ok()?;
    decode::<Claims>(token, &key, &validation)
        .ok()
        .map(|d| d.claims)
}

pub fn session_cookie(config: &Config, token: String) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(config.is_production())
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(time::Duration::hours(config.jwt_expires_in_hours))
        .build()
}

pub fn clear_session_cookie(config: &Config) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .secure(config.is_production())
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
}

// ---------- Extractors ----------

/// An authenticated, active staff member, freshly loaded from the database.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub role: Role,
    pub row: UserRow,
}

async fn load_user_from_parts(parts: &Parts, state: &AppState) -> AppResult<Option<AuthUser>> {
    let jar = CookieJar::from_headers(&parts.headers);
    let Some(token) = jar.get(SESSION_COOKIE).map(|c| c.value().to_string()) else {
        return Ok(None);
    };
    let Some(claims) = decode_token(&state.config, &token) else {
        return Ok(None);
    };
    let Ok(user_id) = claims.sub.parse::<i64>() else {
        return Ok(None);
    };

    let row: Option<UserRow> = sqlx::query_as(concat!(
        "SELECT ",
        user_columns!(),
        " FROM users WHERE id = ?"
    ))
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(row.and_then(|row| {
        if !row.is_active {
            return None;
        }
        let role = Role::parse(&row.role)?;
        Some(AuthUser {
            id: row.id,
            role,
            row,
        })
    }))
}

/// Any active admin or manager.
pub struct RequireStaff(pub AuthUser);

impl FromRequestParts<AppState> for RequireStaff {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match load_user_from_parts(parts, state).await? {
            Some(user) => Ok(Self(user)),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// An active admin only.
pub struct RequireAdmin(pub AuthUser);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match load_user_from_parts(parts, state).await? {
            Some(user) if user.role == Role::Admin => Ok(Self(user)),
            Some(_) => Err(AppError::Forbidden),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// Staff if a valid session is present, otherwise a guest. Never rejects.
pub struct MaybeStaff(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeStaff {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Avoid a DB round-trip for guests without a cookie.
        let jar = CookieJar::from_headers(&parts.headers);
        if jar.get(SESSION_COOKIE).is_none() {
            return Ok(Self(None));
        }
        Ok(Self(load_user_from_parts(parts, state).await?))
    }
}

/// Client IP for rate limiting, from the socket address when available.
pub struct ClientIp(pub String);

impl<S: Send + Sync> FromRequestParts<S> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip = parts
            .extensions
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(Self(ip))
    }
}
