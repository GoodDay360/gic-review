#![allow(dead_code)]

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use gic_review_api::{
    build_router,
    cache::ResponseCache,
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
use tokio::sync::Semaphore;
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
        db_ssl_required: false,
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
        max_concurrent_image_uploads: 5,
        cache_max_bytes: 4 * 1024 * 1024,
        cache_ttl_seconds: 300,
    }
}

pub fn app_with_pool(db: MySqlPool) -> Router {
    let config = test_config();
    build_router(AppState {
        db,
        storage: Storage::new(&config),
        config: Arc::new(config),
        login_limiter: RateLimiter::new(5, Duration::from_secs(60)),
        image_upload_slots: Arc::new(Semaphore::new(5)),
        // Same cache as production, so tests exercise caching and invalidation.
        cache: ResponseCache::new(4 * 1024 * 1024, Duration::from_secs(300)),
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
    /// `x-cache` response header (`hit` / `miss`), if any.
    pub x_cache: Option<String>,
    pub cache_control: Option<String>,
    pub json: serde_json::Value,
}

pub async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<serde_json::Value>,
) -> TestResponse {
    send_auth(app, method, uri, cookie, None, body).await
}

/// Like `send`, with an `Authorization: Bearer <api_key>` header.
pub async fn send_key(
    app: &Router,
    method: &str,
    uri: &str,
    api_key: &str,
    body: Option<serde_json::Value>,
) -> TestResponse {
    send_auth(app, method, uri, None, Some(api_key), body).await
}

pub async fn send_auth(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    api_key: Option<&str>,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    if let Some(k) = api_key {
        req = req.header(header::AUTHORIZATION, format!("Bearer {k}"));
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
    let x_cache = resp
        .headers()
        .get("x-cache")
        .map(|v| v.to_str().unwrap().to_string());
    let cache_control = resp
        .headers()
        .get(header::CACHE_CONTROL)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    TestResponse {
        status,
        set_cookie,
        x_cache,
        cache_control,
        json,
    }
}

/// Database tests share ONE database, `DB_TEST_NAME` from `backend/.env` (e.g. `gic_review_test`),
/// on the server from the other `DB_*` values (no `DATABASE_URL`). It must already exist and `DB_USER`
/// needs all privileges on it. Each test takes a lock (so they run one at a time), wipes the
/// tables, and re-applies the migrations. The real `DB_NAME` database is never touched.
pub struct TestDb {
    pub pool: MySqlPool,
    _lock: tokio::sync::MutexGuard<'static, ()>,
}

static TEST_DB_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// `None` (test skipped) when `DB_*` is not filled in. Panics with a clear message when it
/// is filled in but the test database cannot be used, so real problems are not hidden.
pub async fn test_db() -> Option<TestDb> {
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    let options = match gic_review_api::config::db_options_from_env() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("skipping database test: {msg}");
            return None;
        }
    };
    let real_name = options.get_database().expect("DB_NAME is set").to_string();
    let test_name = std::env::var("DB_TEST_NAME")
        .ok()
        .map(|v| v.split(" #").next().unwrap_or_default().trim().to_string())
        .filter(|v| !v.is_empty());
    let Some(test_name) = test_name else {
        eprintln!("skipping database test: DB_TEST_NAME is empty in backend/.env");
        return None;
    };
    assert_ne!(
        test_name, real_name,
        "DB_TEST_NAME must be a different database than DB_NAME (tests wipe it)"
    );

    let lock = TEST_DB_LOCK.lock().await;

    // One connection, so FOREIGN_KEY_CHECKS=0 applies to every DROP below.
    let pool = MySqlPoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone().database(&test_name))
        .await
        .unwrap_or_else(|e| {
            panic!(
                "cannot use the test database `{test_name}`. Create it and grant access:\n  \
                 CREATE DATABASE {test_name} CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;\n  \
                 GRANT ALL PRIVILEGES ON {test_name}.* TO '<DB_USER>'@'%';\n({e})"
            )
        });

    // Wipe every table (including sqlx's migration history) so each test starts empty.
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT CAST(table_name AS CHAR) FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'",
    )
    .fetch_all(&pool)
    .await
    .expect("cannot list tables in the test database");
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&pool)
        .await
        .unwrap();
    for (table,) in tables {
        // Table names come from information_schema of the test database itself.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP TABLE IF EXISTS `{}`",
            table.replace('`', "``")
        )))
        .execute(&pool)
        .await
        .expect("cannot drop a table in the test database");
    }
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_with(options.database(&test_name))
        .await
        .expect("cannot reconnect to the test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed on the test database");
    Some(TestDb { pool, _lock: lock })
}

impl TestDb {
    /// Release the connections (the data stays until the next test wipes it).
    pub async fn cleanup(self) {
        self.pool.close().await;
    }
}
