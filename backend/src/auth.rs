//! Password hashing (Argon2id), JWT session cookies and the auth extractors
//! `RequireEditor<Scope>`, `RequireEditorSession`, `RequireAdmin`, `RequireAuth`, `MaybeEditor`.
//!
//! Roles: managers create content (dashboard or API key); admins can do everything managers
//! can, plus manage accounts, API keys, years, subjects and orphan cleanup (dashboard only).
//!
//! Every authenticated request re-reads the user's role and `is_active` flag from the database,
//! so deactivating a manager takes effect immediately even if their JWT is still valid.

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use std::marker::PhantomData;

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{NaiveDateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    api_keys::{self, ApiScope},
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
    /// Bumped on every password change; session tokens with an older version are rejected.
    pub session_version: i32,
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
        "id, email, password_hash, session_version, display_name, role, is_active, created_by, created_at, updated_at"
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
    /// The user's `session_version` when the token was issued.
    #[serde(default)]
    pub sv: i32,
}

pub fn create_token(
    config: &Config,
    user_id: i64,
    role: Role,
    session_version: i32,
) -> anyhow::Result<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        role,
        iat: now,
        exp: now + config.jwt_expires_in_hours * 3600,
        sv: session_version,
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
        .secure(config.uses_secure_cookies())
        .same_site(session_cookie_same_site(config))
        .path("/")
        .max_age(time::Duration::hours(config.jwt_expires_in_hours))
        .build()
}

pub fn clear_session_cookie(config: &Config) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .secure(config.uses_secure_cookies())
        .same_site(session_cookie_same_site(config))
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
}

fn session_cookie_same_site(config: &Config) -> SameSite {
    if config.uses_secure_cookies() {
        SameSite::None
    } else {
        SameSite::Strict
    }
}

// ---------- Extractors ----------

/// An authenticated, active staff member, freshly loaded from the database.
/// How the request was authenticated.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Browser session cookie (dashboard).
    Session,
    /// `Authorization: Bearer gic_...` API key (scripts / AI assistants).
    ApiKey { key_id: i64, scopes: Vec<ApiScope> },
}

/// An authenticated, active user, freshly loaded from the database.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub role: Role,
    pub row: UserRow,
    pub method: AuthMethod,
}

impl AuthUser {
    pub fn is_api_key(&self) -> bool {
        matches!(self.method, AuthMethod::ApiKey { .. })
    }
}

/// The raw value of an `Authorization: Bearer ...` header, if any.
fn bearer(parts: &Parts) -> Option<Option<String>> {
    let value = parts.headers.get(header::AUTHORIZATION)?;
    let value = value.to_str().ok().map(str::trim).unwrap_or_default();
    let token = value
        .split_once(' ')
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, t)| t.trim().to_string());
    Some(token)
}

async fn load_active_user(state: &AppState, user_id: i64) -> AppResult<Option<(UserRow, Role)>> {
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
        Some((row, role))
    }))
}

#[derive(sqlx::FromRow)]
struct KeyLookup {
    id: i64,
    user_id: i64,
    scopes: String,
}

async fn authenticate_api_key(state: &AppState, key: &str) -> AppResult<Option<AuthUser>> {
    if !api_keys::looks_like_key(key) {
        return Ok(None);
    }
    let found: Option<KeyLookup> = sqlx::query_as(
        "SELECT id, user_id, scopes FROM api_keys \
         WHERE key_hash = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > NOW())",
    )
    .bind(api_keys::hash(key))
    .fetch_optional(&state.db)
    .await?;
    let Some(found) = found else {
        return Ok(None);
    };
    // The key's owner must still be active: deactivating them disables their keys.
    let Some((row, role)) = load_active_user(state, found.user_id).await? else {
        return Ok(None);
    };
    sqlx::query(
        "UPDATE api_keys SET last_used_at = NOW() \
         WHERE id = ? AND (last_used_at IS NULL OR last_used_at < NOW() - INTERVAL 1 MINUTE)",
    )
    .bind(found.id)
    .execute(&state.db)
    .await?;
    Ok(Some(AuthUser {
        id: row.id,
        role,
        row,
        method: AuthMethod::ApiKey {
            key_id: found.id,
            scopes: api_keys::parse_scopes(&found.scopes),
        },
    }))
}

async fn authenticate_session(parts: &Parts, state: &AppState) -> AppResult<Option<AuthUser>> {
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
    Ok(load_active_user(state, user_id)
        .await?
        // A password change bumps session_version and signs out every older session.
        .filter(|(row, _)| claims.sv == row.session_version)
        .map(|(row, role)| AuthUser {
            id: row.id,
            role,
            row,
            method: AuthMethod::Session,
        }))
}

/// Authenticate with an API key if an `Authorization` header is present (the cookie is
/// then ignored), otherwise with the session cookie.
async fn authenticate(parts: &Parts, state: &AppState) -> AppResult<Option<AuthUser>> {
    match bearer(parts) {
        Some(Some(key)) => authenticate_api_key(state, &key).await,
        Some(None) => Ok(None), // malformed Authorization header
        None => authenticate_session(parts, state).await,
    }
}

/// Session-only endpoints reject API keys before touching the database.
fn reject_api_key(parts: &Parts) -> AppResult<()> {
    if bearer(parts).is_some() {
        Err(AppError::ApiKeyNotAllowed)
    } else {
        Ok(())
    }
}

// ----- Scope requirements for endpoints that accept API keys -----

/// Which API key scopes satisfy an endpoint (any one of them is enough).
pub trait ScopeRequirement {
    const ANY_OF: &'static [ApiScope];
}

/// Quiz content (quizzes, questions, import).
pub struct QuizzesScope;
impl ScopeRequirement for QuizzesScope {
    const ANY_OF: &'static [ApiScope] = &[ApiScope::QuizzesWrite];
}

/// Interactive reviews.
pub struct ReviewsScope;
impl ScopeRequirement for ReviewsScope {
    const ANY_OF: &'static [ApiScope] = &[ApiScope::ReviewsWrite];
}

/// Image uploads (used by both quizzes and reviews).
pub struct ContentScope;
impl ScopeRequirement for ContentScope {
    const ANY_OF: &'static [ApiScope] = &[ApiScope::QuizzesWrite, ApiScope::ReviewsWrite];
}

/// An active **editor** (manager or admin), via session cookie or an API key holding scope `S`.
/// Admins are rejected: they only manage accounts and subjects.
pub struct RequireEditor<S: ScopeRequirement>(pub AuthUser, pub PhantomData<S>);

impl<S: ScopeRequirement> FromRequestParts<AppState> for RequireEditor<S> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = authenticate(parts, state)
            .await?
            .ok_or(AppError::Unauthorized)?;
        if let AuthMethod::ApiKey { scopes, .. } = &user.method
            && !S::ANY_OF.iter().any(|s| scopes.contains(s))
        {
            return Err(AppError::MissingScope(
                S::ANY_OF
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" or "),
            ));
        }
        Ok(Self(user, PhantomData))
    }
}

/// An active editor (manager or admin) logged in through the dashboard (no API keys).
pub struct RequireEditorSession(pub AuthUser);

impl FromRequestParts<AppState> for RequireEditorSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        reject_api_key(parts)?;
        match authenticate_session(parts, state).await? {
            Some(user) => Ok(Self(user)),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// An active admin logged in through the dashboard (no API keys).
pub struct RequireAdmin(pub AuthUser);

impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        reject_api_key(parts)?;
        match authenticate_session(parts, state).await? {
            Some(user) if user.role == Role::Admin => Ok(Self(user)),
            Some(_) => Err(AppError::Forbidden),
            None => Err(AppError::Unauthorized),
        }
    }
}

/// Any authenticated user (session or API key). Used by `GET /auth/me`.
pub struct RequireAuth(pub AuthUser);

impl FromRequestParts<AppState> for RequireAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        authenticate(parts, state)
            .await?
            .map(Self)
            .ok_or(AppError::Unauthorized)
    }
}

/// An editor (manager or admin; session or any API key) if present, otherwise a guest.
/// Never rejects. Used to let editors preview unpublished quizzes they can manage.
pub struct MaybeEditor(pub Option<AuthUser>);

impl FromRequestParts<AppState> for MaybeEditor {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Avoid a DB round-trip for guests.
        let jar = CookieJar::from_headers(&parts.headers);
        if jar.get(SESSION_COOKIE).is_none() && bearer(parts).is_none() {
            return Ok(Self(None));
        }
        Ok(Self(authenticate(parts, state).await?))
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

/// Editor allowed to change quizzes (session, or API key with `quizzes:write`).
pub type QuizManager = RequireEditor<QuizzesScope>;
/// Editor allowed to change interactive reviews (session, or API key with `reviews:write`).
pub type ReviewManager = RequireEditor<ReviewsScope>;
/// Editor allowed to upload images (session, or API key with either write scope).
pub type ContentManager = RequireEditor<ContentScope>;

/// Any active user (admin or manager) logged in through the dashboard (no API keys).
pub struct RequireSession(pub AuthUser);

impl FromRequestParts<AppState> for RequireSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        reject_api_key(parts)?;
        authenticate_session(parts, state)
            .await?
            .map(Self)
            .ok_or(AppError::Unauthorized)
    }
}
