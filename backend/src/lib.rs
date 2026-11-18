//! GIC Review API: Axum + Tokio + SQLx (MySQL) + Cloudflare R2.

pub mod access;
pub mod api_keys;
pub mod auth;
pub mod bootstrap;
pub mod config;
pub mod error;
pub mod extract;
pub mod images;
pub mod models;
pub mod quiz_modes;
pub mod rate_limit;
pub mod repo;
pub mod routes;
pub mod state;
pub mod storage;
pub mod validation;

use std::time::Duration;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, StatusCode, header},
};
use serde_json::json;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};

use crate::state::AppState;

/// Build the full application router. Used by `main` and by the integration tests.
pub fn build_router(state: AppState) -> Router {
    let config = state.config.clone();

    let cors = CorsLayer::new()
        .allow_origin(
            HeaderValue::from_str(&config.frontend_url).expect("FRONTEND_URL is a valid header"),
        )
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION])
        .max_age(Duration::from_secs(3600));

    Router::new()
        .nest("/api", routes::api(&config))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": { "code": "not_found", "message": "route not found" } })),
            )
        })
        // Default JSON body limit; upload routes set their own larger limits.
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(120),
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
