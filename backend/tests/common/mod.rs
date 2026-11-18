#![allow(dead_code)]

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use gic_review_api::{
    build_router,
    config::{AppEnv, Config},
    rate_limit::RateLimiter,
    state::AppState,
    storage::Storage,
};
use http_body_util::BodyExt;
use sqlx::{
    MySqlPool,
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
};
use tower::ServiceExt;

pub fn test_config() -> Config {
    Config {
        app_env: AppEnv::Development,
        backend_host: "127.0.0.1".into(),
        backend_port: 0,
        frontend_url: "http://localhost:5173".into(),
        db_host: "127.0.0.1".into(),
        db_port: 1,
        db_name: "unused".into(),
        db_user: "unused".into(),
        db_password: "unused".into(),
        jwt_secret: "test-secret-test-secret-test-secret-0123456789".into(),
        jwt_expires_in_hours: 1,
        admin_email: "admin@example.com".into(),
        admin_password: "admin-password-123".into(),
        r2_access_key_id: "test".into(),
        r2_secret_access_key: "test".into(),
        r2_bucket: "test".into(),
        // Nothing listens here; tests never reach R2.
        r2_endpoint: "http://127.0.0.1:9".into(),
        r2_public_base_url: "https://files.example.com".into(),
        max_image_size_bytes: 5 * 1024 * 1024,
        max_review_html_size_bytes: 10 * 1024 * 1024,
    }
}

pub fn app_with_pool(db: MySqlPool) -> Router {
    let config = test_config();
    build_router(AppState {
        db,
        storage: Storage::new(&config),
        config: Arc::new(config),
        login_limiter: RateLimiter::new(5, Duration::from_secs(60)),
    })
}

/// An app whose database pool points nowhere. Requests that are rejected before any
/// query (auth checks, rate limiting) behave exactly as in production.
pub fn app_without_db() -> Router {
    let opts = MySqlConnectOptions::new().host("127.0.0.1").port(1);
    let pool = MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy_with(opts);
    app_with_pool(pool)
}

pub struct TestResponse {
    pub status: StatusCode,
    pub set_cookie: Option<String>,
    pub json: serde_json::Value,
}

pub async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    TestResponse {
        status,
        set_cookie,
        json,
    }
}
