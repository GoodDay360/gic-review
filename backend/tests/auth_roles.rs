//! Auth and role checks.
//!
//! The first group runs without a database (requests are rejected before any query).
//! The `db_*` tests need MySQL and are ignored by default; run them with
//!   DATABASE_URL=mysql://user:pass@host:3306/gic_review cargo test -- --ignored
//! (`#[sqlx::test]` creates and drops a throwaway database per test, so the user needs
//! CREATE/DROP DATABASE privileges.)

mod common;

use axum::http::StatusCode;
use common::{app_with_pool, app_without_db, send, test_config};
use gic_review_api::{
    auth::{Role, create_token, hash_password},
    bootstrap,
};
use serde_json::json;
use sqlx::MySqlPool;

const STAFF_ENDPOINTS: &[(&str, &str)] = &[
    ("GET", "/api/admin/quizzes"),
    ("POST", "/api/admin/quizzes"),
    ("PUT", "/api/admin/quizzes/1"),
    ("DELETE", "/api/admin/quizzes/1"),
    ("PATCH", "/api/admin/quizzes/1/publish"),
    ("POST", "/api/admin/quizzes/1/questions"),
    ("PUT", "/api/admin/quizzes/1/questions/reorder"),
    ("POST", "/api/admin/quizzes/import"),
    ("POST", "/api/admin/uploads/image"),
    ("POST", "/api/admin/uploads/cleanup-orphans"),
    ("POST", "/api/admin/reviews"),
    ("GET", "/api/admin/users"),
    ("POST", "/api/admin/users"),
    ("PATCH", "/api/admin/users/2"),
    ("POST", "/api/admin/users/2/reset-password"),
    ("DELETE", "/api/admin/users/2"),
    ("POST", "/api/admin/subjects"),
    ("PUT", "/api/admin/subjects/1"),
    ("DELETE", "/api/admin/subjects/1"),
    ("GET", "/api/auth/me"),
];

#[tokio::test]
async fn guest_gets_401_on_every_staff_endpoint() {
    let app = app_without_db();
    for (method, uri) in STAFF_ENDPOINTS {
        let body = (*method != "GET" && *method != "DELETE").then(|| json!({}));
        let res = send(&app, method, uri, None, body).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        assert_eq!(res.json["error"]["code"], "unauthorized", "{method} {uri}");
    }
}

#[tokio::test]
async fn garbage_or_forged_tokens_are_rejected() {
    let app = app_without_db();
    let res = send(
        &app,
        "GET",
        "/api/admin/users",
        Some("gic_session=not-a-jwt"),
        None,
    )
    .await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    // Signed with a different secret.
    let mut other = test_config();
    other.jwt_secret = "another-secret-another-secret-another-secret".into();
    let forged = create_token(&other, 1, Role::Admin).unwrap();
    let cookie = format!("gic_session={forged}");
    let res = send(&app, "GET", "/api/admin/users", Some(&cookie), None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_is_rate_limited_per_ip() {
    let app = app_without_db();
    let body = json!({ "email": "someone@example.com", "password": "whatever" });
    for _ in 0..5 {
        let res = send(&app, "POST", "/api/auth/login", None, Some(body.clone())).await;
        assert_ne!(res.status, StatusCode::TOO_MANY_REQUESTS);
    }
    let res = send(&app, "POST", "/api/auth/login", None, Some(body)).await;
    assert_eq!(res.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(res.json["error"]["code"], "too_many_requests");
}

#[tokio::test]
async fn errors_are_json_without_internal_details() {
    let app = app_without_db();
    let res = send(&app, "GET", "/api/does-not-exist", None, None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.json["error"]["code"], "not_found");

    // DB is unreachable: the client must get a generic message, not the driver error.
    let res = send(&app, "GET", "/api/subjects", None, None).await;
    assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(res.json["error"]["code"], "internal_error");
    assert_eq!(res.json["error"]["message"], "internal server error");
}

// ---------------- Database-backed tests ----------------

async fn insert_manager(db: &MySqlPool, email: &str, password: &str, active: bool) -> i64 {
    let hash = hash_password(password).unwrap();
    sqlx::query(
        "INSERT INTO users (email, password_hash, display_name, role, is_active) \
         VALUES (?, ?, 'Manager', 'manager', ?)",
    )
    .bind(email)
    .bind(hash)
    .bind(active)
    .execute(db)
    .await
    .unwrap()
    .last_insert_id() as i64
}

fn cookie_from(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_string()
}

async fn login(app: &axum::Router, email: &str, password: &str) -> common::TestResponse {
    send(
        app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await
}

#[ignore = "needs MySQL (DATABASE_URL)"]
#[sqlx::test(migrations = "./migrations")]
async fn db_seed_and_admin_bootstrap(db: MySqlPool) {
    let config = test_config();
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    bootstrap::ensure_admin(&db, &config).await.unwrap(); // idempotent

    let (admins,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(admins, 1);

    let app = app_with_pool(db);
    let subjects = send(&app, "GET", "/api/subjects", None, None).await;
    assert_eq!(subjects.status, StatusCode::OK);
    assert_eq!(subjects.json.as_array().unwrap().len(), 1);
    assert_eq!(subjects.json[0]["slug"], "hci");
}

#[ignore = "needs MySQL (DATABASE_URL)"]
#[sqlx::test(migrations = "./migrations")]
async fn db_login_sets_secure_cookie_and_me_works(db: MySqlPool) {
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let app = app_with_pool(db);

    let bad = login(&app, "admin@example.com", "wrong-password").await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);

    let ok = login(&app, "admin@example.com", "admin-password-123").await;
    assert_eq!(ok.status, StatusCode::OK);
    let set_cookie = ok.set_cookie.expect("login sets a cookie");
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Strict"));
    assert!(ok.json.get("password_hash").is_none());

    let me = send(
        &app,
        "GET",
        "/api/auth/me",
        Some(&cookie_from(&set_cookie)),
        None,
    )
    .await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["role"], "admin");
}

#[ignore = "needs MySQL (DATABASE_URL)"]
#[sqlx::test(migrations = "./migrations")]
async fn db_manager_cannot_reach_admin_only_endpoints(db: MySqlPool) {
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "manager@example.com", "manager-pass-1", true).await;
    let app = app_with_pool(db);

    let res = login(&app, "manager@example.com", "manager-pass-1").await;
    let manager = cookie_from(&res.set_cookie.unwrap());

    // Admin-only: 403 for a manager.
    for (method, uri, body) in [
        ("GET", "/api/admin/users", None),
        ("POST", "/api/admin/users", Some(json!({}))),
        ("POST", "/api/admin/subjects", Some(json!({}))),
        ("POST", "/api/admin/uploads/cleanup-orphans", None),
    ] {
        let r = send(&app, method, uri, Some(&manager), body).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{method} {uri}");
    }

    // Staff endpoints work for a manager.
    let r = send(&app, "GET", "/api/admin/quizzes", Some(&manager), None).await;
    assert_eq!(r.status, StatusCode::OK);

    // Admin can list users.
    let res = login(&app, "admin@example.com", "admin-password-123").await;
    let admin = cookie_from(&res.set_cookie.unwrap());
    let r = send(&app, "GET", "/api/admin/users", Some(&admin), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json.as_array().unwrap().len(), 2);
}

#[ignore = "needs MySQL (DATABASE_URL)"]
#[sqlx::test(migrations = "./migrations")]
async fn db_deactivated_manager_loses_access_immediately(db: MySqlPool) {
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let id = insert_manager(&db, "m2@example.com", "manager-pass-2", true).await;
    let app = app_with_pool(db.clone());

    let res = login(&app, "m2@example.com", "manager-pass-2").await;
    let manager = cookie_from(&res.set_cookie.unwrap());
    let r = send(&app, "GET", "/api/admin/quizzes", Some(&manager), None).await;
    assert_eq!(r.status, StatusCode::OK);

    let res = login(&app, "admin@example.com", "admin-password-123").await;
    let admin = cookie_from(&res.set_cookie.unwrap());
    let r = send(
        &app,
        "PATCH",
        &format!("/api/admin/users/{id}"),
        Some(&admin),
        Some(json!({ "is_active": false })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);

    // Same, still-unexpired JWT is now rejected.
    let r = send(&app, "GET", "/api/admin/quizzes", Some(&manager), None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[ignore = "needs MySQL (DATABASE_URL)"]
#[sqlx::test(migrations = "./migrations")]
async fn db_admin_accounts_cannot_be_changed_through_the_api(db: MySqlPool) {
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let app = app_with_pool(db);
    let res = login(&app, "admin@example.com", "admin-password-123").await;
    let admin = cookie_from(&res.set_cookie.unwrap());
    let me = send(&app, "GET", "/api/auth/me", Some(&admin), None).await;
    let admin_id = me.json["id"].as_i64().unwrap();

    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/users/{admin_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Created users are always managers, even if "role" is sent.
    let r = send(
        &app,
        "POST",
        "/api/admin/users",
        Some(&admin),
        Some(
            json!({ "email": "new@example.com", "password": "password-123",
                     "display_name": "New", "role": "admin" }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.json["role"], "manager");
}
