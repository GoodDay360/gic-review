//! Application error type. Every error is returned as
//! `{ "error": { "code": "...", "message": "..." } }` with no internal details.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Validation(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("you do not have permission to do this")]
    Forbidden,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    PayloadTooLarge(String),
    #[error("{0}")]
    UnsupportedMediaType(String),
    #[error("too many attempts, please wait a minute and try again")]
    TooManyRequests,
    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, "invalid_credentials"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::PayloadTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            Self::UnsupportedMediaType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_media_type")
            }
            Self::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        if let Self::Internal(err) = &self {
            // Full detail goes to the server log only, never to the client.
            tracing::error!(error = ?err, "internal error");
        }
        let body = json!({ "error": { "code": code, "message": self.to_string() } });
        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            // MySQL duplicate key (1062) and FK violations (1451/1452).
            match db_err.kind() {
                sqlx::error::ErrorKind::UniqueViolation => {
                    return Self::Conflict("a record with this value already exists".into());
                }
                sqlx::error::ErrorKind::ForeignKeyViolation => {
                    return Self::Conflict(
                        "this record is still referenced by other records".into(),
                    );
                }
                _ => {}
            }
        }
        if matches!(err, sqlx::Error::RowNotFound) {
            return Self::NotFound("record");
        }
        Self::Internal(err.into())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errs: validator::ValidationErrors) -> Self {
        let mut parts: Vec<String> = errs
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let msg = errors
                    .iter()
                    .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                    .next()
                    .unwrap_or_else(|| "is invalid".to_string());
                format!("{field}: {msg}")
            })
            .collect();
        parts.sort();
        Self::Validation(parts.join("; "))
    }
}

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(err: axum::extract::multipart::MultipartError) -> Self {
        let status = err.status();
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            Self::PayloadTooLarge("upload is too large".into())
        } else {
            Self::BadRequest(format!("invalid multipart body: {}", err.body_text()))
        }
    }
}
