//! `AppJson<T>`: like `axum::Json<T>` for request bodies, but a malformed or incomplete body
//! becomes our standard JSON error (`422 validation_error` / `400 bad_request`) instead of
//! Axum's plain-text rejection.

use axum::extract::FromRequest;

use crate::error::AppError;

#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);
