//! Auth and role checks.
//!
//! The first group runs without a database (requests are rejected before any query).
//! The `db_*` tests use ONE test database, `DB_TEST_NAME` (e.g. `gic_review_test`), on the
//! MySQL server from `DB_*` in `backend/.env` (no DATABASE_URL). Create it once and grant
//! `DB_USER` all privileges on it. Each test wipes it and re-runs the migrations; the tests
//! run one at a time, and the real database is never touched. When `DB_*` is not filled in
//! or `DB_TEST_NAME` is empty, they are skipped with a message and plain `cargo test` still passes.

mod common;

use axum::http::StatusCode;
use common::{app_with_pool, app_without_db, send, send_key, test_config};
use gic_review_api::{
    auth::{Role, clear_session_cookie, create_token, hash_password, session_cookie},
    bootstrap,
    config::AppEnv,
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
    ("POST", "/api/admin/categories"),
    ("PUT", "/api/admin/categories/1"),
    ("DELETE", "/api/admin/categories/1"),
    ("DELETE", "/api/admin/subjects/1"),
    ("GET", "/api/auth/me"),
    ("POST", "/api/auth/change-password"),
    ("GET", "/api/admin/api-keys"),
    ("POST", "/api/admin/api-keys"),
    ("GET", "/api/admin/api-keys/mine"),
    ("DELETE", "/api/admin/api-keys/1"),
];

#[test]
fn production_session_cookie_allows_cross_site_requests_securely() {
    let mut config = test_config();
    config.app_env = AppEnv::Production;

    let cookie = session_cookie(&config, "test-token".into()).to_string();
    assert!(cookie.contains("SameSite=None"));
    assert!(cookie.contains("Partitioned"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("HttpOnly"));
    assert!(
        clear_session_cookie(&config)
            .to_string()
            .contains("Partitioned")
    );
}

#[test]
fn https_frontend_uses_secure_cross_site_cookie_without_production_flag() {
    let mut config = test_config();
    config.frontend_url = "https://gicreview.pages.dev".into();

    let cookie = session_cookie(&config, "test-token".into()).to_string();
    assert!(cookie.contains("SameSite=None"));
    assert!(cookie.contains("Partitioned"));
    assert!(cookie.contains("Secure"));
}

/// Endpoints that only accept a dashboard session: an API key must be refused
/// (403 api_key_not_allowed) before any database access.
const SESSION_ONLY_ENDPOINTS: &[(&str, &str)] = &[
    ("POST", "/api/auth/change-password"),
    ("GET", "/api/admin/users"),
    ("POST", "/api/admin/users"),
    ("PATCH", "/api/admin/users/2"),
    ("POST", "/api/admin/subjects"),
    ("POST", "/api/admin/uploads/cleanup-orphans"),
    ("GET", "/api/admin/api-keys"),
    ("POST", "/api/admin/api-keys"),
    ("GET", "/api/admin/api-keys/mine"),
    ("DELETE", "/api/admin/api-keys/1"),
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
    let forged = create_token(&other, 1, Role::Admin, 0).unwrap();
    let cookie = format!("gic_session={forged}");
    let res = send(&app, "GET", "/api/admin/users", Some(&cookie), None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_keys_are_refused_on_session_only_endpoints() {
    let app = app_without_db();
    let key = gic_review_api::api_keys::generate();
    for (method, uri) in SESSION_ONLY_ENDPOINTS {
        let body = (*method != "GET" && *method != "DELETE").then(|| json!({}));
        let res = send_key(&app, method, uri, &key, body).await;
        assert_eq!(res.status, StatusCode::FORBIDDEN, "{method} {uri}");
        assert_eq!(
            res.json["error"]["code"], "api_key_not_allowed",
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn malformed_api_keys_are_unauthorized() {
    let app = app_without_db();
    for key in ["nope", "gic_123", "Basic abc"] {
        let res = send_key(&app, "GET", "/api/admin/quizzes", key, None).await;
        assert_eq!(res.status, StatusCode::UNAUTHORIZED, "{key}");
    }
}

#[tokio::test]
async fn bad_json_bodies_get_json_errors() {
    let app = app_without_db();
    // Missing field: our JSON error format, not Axum's plain text.
    let res = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "a@b.c" })),
    )
    .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(res.json["error"]["code"], "validation_error");
    assert!(
        res.json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("password")
    );
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

/// Create a category + subject owned by the logged-in editor (dashboard session); returns the subject id.
async fn own_subject(app: &axum::Router, cookie: &str, tag: &str) -> i64 {
    let y = send(
        app,
        "POST",
        "/api/admin/categories",
        Some(cookie),
        Some(json!({ "name": format!("Category {tag}") })),
    )
    .await;
    assert_eq!(
        y.status,
        StatusCode::CREATED,
        "create category: {:?}",
        y.json
    );
    let s = send(app, "POST", "/api/admin/subjects", Some(cookie),
        Some(json!({ "category_id": y.json["id"], "name": format!("Subject {tag}"), "slug": format!("subject-{tag}") }))).await;
    assert_eq!(
        s.status,
        StatusCode::CREATED,
        "create subject: {:?}",
        s.json
    );
    s.json["id"].as_i64().unwrap()
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

#[tokio::test]
async fn db_seed_and_admin_bootstrap() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
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
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_login_sets_secure_cookie_and_me_works() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
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
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_manager_cannot_reach_admin_only_endpoints() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "manager@example.com", "manager-pass-1", true).await;
    let app = app_with_pool(db);

    let res = login(&app, "manager@example.com", "manager-pass-1").await;
    let manager = cookie_from(&res.set_cookie.unwrap());

    // Admin-only: 403 for a manager.
    for (method, uri, body) in [
        ("GET", "/api/admin/users", None),
        ("POST", "/api/admin/users", Some(json!({}))),
        ("POST", "/api/admin/uploads/cleanup-orphans", None),
    ] {
        let r = send(&app, method, uri, Some(&manager), body).await;
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{method} {uri}");
    }

    // Content endpoints work for a manager.
    let r = send(&app, "GET", "/api/admin/quizzes", Some(&manager), None).await;
    assert_eq!(r.status, StatusCode::OK);

    // Admin can list users…
    let res = login(&app, "admin@example.com", "admin-password-123").await;
    let admin = cookie_from(&res.set_cookie.unwrap());
    let r = send(&app, "GET", "/api/admin/users", Some(&admin), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json.as_array().unwrap().len(), 2);

    // …and has every manager permission too: content and an own API key.
    let r = send(&app, "GET", "/api/admin/quizzes", Some(&admin), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = send(&app, "POST", "/api/admin/quizzes", Some(&admin),
        Some(json!({ "subject_id": 1, "title": "Admin's quiz", "description": null, "cover_image_key": null }))).await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.json["access"], "owner");
    let r = send(
        &app,
        "POST",
        "/api/admin/api-keys",
        Some(&admin),
        Some(json!({ "name": "admin key", "scopes": ["quizzes:write"], "expires_in_days": null })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let key = r.json["key"].as_str().unwrap().to_string();
    let r = send_key(&app, "GET", "/api/admin/quizzes", &key, None).await;
    assert_eq!(r.status, StatusCode::OK);
    // The admin's key still can't reach admin-only endpoints.
    let r = send_key(&app, "GET", "/api/admin/users", &key, None).await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_deactivated_manager_loses_access_immediately() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
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
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_admin_accounts_cannot_be_changed_through_the_api() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
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
    tdb.cleanup().await;
}

async fn create_key(
    app: &axum::Router,
    cookie: &str,
    scopes: &[&str],
    replace: bool,
) -> common::TestResponse {
    send(
        app,
        "POST",
        "/api/admin/api-keys",
        Some(cookie),
        Some(json!({
            "name": "AI assistant",
            "scopes": scopes,
            "expires_in_days": 30,
            "replace_existing": replace
        })),
    )
    .await
}

#[tokio::test]
async fn db_manager_api_key_lifecycle() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let manager_id = insert_manager(&db, "ai-owner@example.com", "manager-pass-3", true).await;
    let app = app_with_pool(db);
    let manager = cookie_from(
        &login(&app, "ai-owner@example.com", "manager-pass-3")
            .await
            .set_cookie
            .unwrap(),
    );

    // Create: the full key is returned once; only a prefix is stored/listed.
    let created = create_key(&app, &manager, &["quizzes:write"], false).await;
    assert_eq!(created.status, StatusCode::CREATED);
    let key = created.json["key"].as_str().unwrap().to_string();
    assert!(key.starts_with("gic_"));
    let mine = send(
        &app,
        "GET",
        "/api/admin/api-keys/mine",
        Some(&manager),
        None,
    )
    .await;
    assert!(!mine.json.to_string().contains(&key));

    // One active key per manager.
    let second = create_key(&app, &manager, &["quizzes:write"], false).await;
    assert_eq!(second.status, StatusCode::CONFLICT);

    // The key works for quiz content in the manager's own category…
    let sid = own_subject(&app, &manager, "lifecycle").await;
    let me = send_key(&app, "GET", "/api/auth/me", &key, None).await;
    assert_eq!(me.json["auth_method"], "api_key");
    let quiz = send_key(
        &app,
        "POST",
        "/api/admin/quizzes",
        &key,
        Some(json!({ "subject_id": sid, "title": "From an AI", "description": null, "cover_image_key": null })),
    )
    .await;
    assert_eq!(quiz.status, StatusCode::CREATED);
    assert_eq!(quiz.json["is_published"], false);

    // …but not for reviews (missing scope) or key management (session only).
    let r = send_key(&app, "POST", "/api/admin/reviews", &key, None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"]["code"], "missing_scope");
    let r = send_key(&app, "GET", "/api/admin/api-keys/mine", &key, None).await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");

    // Regenerate: the old key stops working, the new one works.
    let regen = create_key(&app, &manager, &["quizzes:write", "reviews:write"], true).await;
    assert_eq!(regen.status, StatusCode::CREATED);
    let new_key = regen.json["key"].as_str().unwrap().to_string();
    let r = send_key(&app, "GET", "/api/admin/quizzes", &key, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = send_key(&app, "GET", "/api/admin/quizzes", &new_key, None).await;
    assert_eq!(r.status, StatusCode::OK);

    // Admin sees every manager's keys and can revoke them.
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    let all = send(&app, "GET", "/api/admin/api-keys", Some(&admin), None).await;
    let active = all
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["status"] == "active")
        .unwrap();
    assert_eq!(active["user_id"], manager_id);
    let id = active["id"].as_i64().unwrap();
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/api-keys/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["status"], "revoked");
    let r = send_key(&app, "GET", "/api/admin/quizzes", &new_key, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_deactivating_a_manager_disables_their_api_key() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let id = insert_manager(&db, "m4@example.com", "manager-pass-4", true).await;
    let app = app_with_pool(db);
    let manager = cookie_from(
        &login(&app, "m4@example.com", "manager-pass-4")
            .await
            .set_cookie
            .unwrap(),
    );
    let key = create_key(&app, &manager, &["reviews:write"], false)
        .await
        .json["key"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        send_key(&app, "GET", "/api/auth/me", &key, None)
            .await
            .status,
        StatusCode::OK
    );

    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    send(
        &app,
        "PATCH",
        &format!("/api/admin/users/{id}"),
        Some(&admin),
        Some(json!({ "is_active": false })),
    )
    .await;
    assert_eq!(
        send_key(&app, "GET", "/api/auth/me", &key, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_sharing_a_category_or_subject_gives_access_to_its_content() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let a_id = insert_manager(&db, "owner-a@example.com", "manager-pass-a", true).await;
    let b_id = insert_manager(&db, "editor-b@example.com", "manager-pass-b", true).await;
    let app = app_with_pool(db.clone());
    let a = cookie_from(
        &login(&app, "owner-a@example.com", "manager-pass-a")
            .await
            .set_cookie
            .unwrap(),
    );
    let b = cookie_from(
        &login(&app, "editor-b@example.com", "manager-pass-b")
            .await
            .set_cookie
            .unwrap(),
    );
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    let quiz = |sid: i64, title: &str| json!({ "subject_id": sid, "title": title, "description": null, "cover_image_key": null });

    // A owns category "Category a" with subjects s1 and s2, and a quiz in each.
    let s1 = own_subject(&app, &a, "a").await;
    let category: i64 = send(&app, "GET", "/api/subjects/subject-a", None, None)
        .await
        .json["category_id"]
        .as_i64()
        .unwrap();
    let s2 = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&a),
        Some(json!({ "category_id": category, "name": "Second", "slug": "second-a" })),
    )
    .await
    .json["id"]
        .as_i64()
        .unwrap();
    let q1 = send(
        &app,
        "POST",
        "/api/admin/quizzes",
        Some(&a),
        Some(quiz(s1, "A in s1")),
    )
    .await
    .json["id"]
        .as_i64()
        .unwrap();
    let q2 = send(
        &app,
        "POST",
        "/api/admin/quizzes",
        Some(&a),
        Some(quiz(s2, "A in s2")),
    )
    .await
    .json["id"]
        .as_i64()
        .unwrap();

    // Not shared: B sees nothing and can't add.
    assert_eq!(
        send(&app, "GET", "/api/admin/quizzes", Some(&b), None)
            .await
            .json
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/admin/quizzes/{q1}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let r = send(
        &app,
        "POST",
        "/api/admin/quizzes",
        Some(&b),
        Some(quiz(s1, "B in s1")),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "not_your_category");

    // A shares subject s1 with B (by email).
    let s1_shares = format!("/api/admin/subjects/{s1}/shares");
    let r = send(
        &app,
        "POST",
        &s1_shares,
        Some(&a),
        Some(json!({ "email": "Editor-B@example.com" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["shared_with"][0]["user_id"], b_id);
    assert_eq!(
        send(
            &app,
            "POST",
            &s1_shares,
            Some(&a),
            Some(json!({ "email": "editor-b@example.com" }))
        )
        .await
        .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        send(
            &app,
            "POST",
            &s1_shares,
            Some(&a),
            Some(json!({ "email": "nobody@example.com" }))
        )
        .await
        .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // B now sees and edits ALL content in s1 (no per-item sharing), but nothing in s2.
    let list = send(&app, "GET", "/api/admin/quizzes", Some(&b), None).await;
    let ids: Vec<i64> = list
        .json
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![q1]);
    assert_eq!(list.json[0]["access"], "shared");
    assert_eq!(list.json[0]["owner_id"], a_id);
    let r = send(
        &app,
        "PUT",
        &format!("/api/admin/quizzes/{q1}"),
        Some(&b),
        Some(quiz(s1, "Edited by B")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let r = send(
        &app,
        "POST",
        "/api/admin/quizzes",
        Some(&b),
        Some(quiz(s1, "B in s1")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let b_quiz = r.json["id"].as_i64().unwrap();
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/admin/quizzes",
            Some(&b),
            Some(quiz(s2, "B in s2"))
        )
        .await
        .json["error"]["code"],
        "not_your_category"
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/admin/quizzes/{q2}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Shared users can't delete others' content, manage the category/subjects, or re-share.
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/quizzes/{q1}"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");
    assert_eq!(
        send(
            &app,
            "POST",
            &s1_shares,
            Some(&b),
            Some(json!({ "email": "owner-a@example.com" }))
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        send(
            &app,
            "PUT",
            &format!("/api/admin/subjects/{s1}"),
            Some(&b),
            Some(json!({ "category_id": category, "name": "x", "slug": "x" }))
        )
        .await
        .json["error"]["code"],
        "owner_only"
    );
    // The category's owner has full rights on content others add to their category.
    let r = send(
        &app,
        "GET",
        &format!("/api/admin/quizzes/{b_quiz}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.json["access"], "category_owner");

    // GET /admin/categories tells B where they can add content.
    let categories = send(&app, "GET", "/api/admin/categories", Some(&b), None).await;
    let y = categories
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == category)
        .unwrap();
    assert_eq!(y["can_manage"], false);
    assert_eq!(y["shared_with_you"], false);
    let subj = |id: i64| {
        y["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(subj(s1)["can_add_content"], true);
    assert_eq!(subj(s1)["shared_with_you"], true);
    assert_eq!(subj(s2)["can_add_content"], false);

    // Sharing the whole category opens every subject.
    let category_shares = format!("/api/admin/categories/{category}/shares");
    assert_eq!(
        send(
            &app,
            "POST",
            &category_shares,
            Some(&a),
            Some(json!({ "email": "editor-b@example.com" }))
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/admin/quizzes/{q2}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/admin/quizzes",
            Some(&b),
            Some(quiz(s2, "B in s2"))
        )
        .await
        .status,
        StatusCode::CREATED
    );

    // A shared category also lets B add subjects to it (but not edit or delete them).
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/admin/subjects",
            Some(&b),
            Some(json!({ "category_id": category, "name": "B's subject", "slug": "b-subject" }))
        )
        .await
        .status,
        StatusCode::CREATED
    );

    // B leaves the category and the subject: access is gone.
    assert_eq!(
        send(
            &app,
            "DELETE",
            &format!("{category_shares}/{b_id}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/admin/quizzes/{q2}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &app,
            "DELETE",
            &format!("{s1_shares}/{b_id}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/admin/quizzes/{q1}"),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Sharing with all managers: only the category's owner (or an admin) can turn it on.
    let s1_all = format!("{s1_shares}/all");
    let category_all = format!("{category_shares}/all");
    let r = send(
        &app,
        "PUT",
        &s1_all,
        Some(&b),
        Some(json!({ "enabled": true })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");
    let r = send(
        &app,
        "PUT",
        &s1_all,
        Some(&a),
        Some(json!({ "enabled": true })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["shared_with_all"], true);
    let get_as_b = |path: String| {
        let app = app.clone();
        let b = b.clone();
        async move { send(&app, "GET", &path, Some(&b), None).await.status }
    };
    assert_eq!(
        get_as_b(format!("/api/admin/quizzes/{q1}")).await,
        StatusCode::OK
    );
    assert_eq!(
        get_as_b(format!("/api/admin/quizzes/{q2}")).await,
        StatusCode::NOT_FOUND
    );
    let r = send(&app, "GET", "/api/admin/categories", Some(&b), None).await;
    let y = r
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == category)
        .unwrap();
    let s1_dto = y["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == s1)
        .unwrap();
    assert_eq!(s1_dto["shared_with_you"], true);
    assert_eq!(s1_dto["can_add_content"], true);
    send(
        &app,
        "PUT",
        &s1_all,
        Some(&a),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(
        get_as_b(format!("/api/admin/quizzes/{q1}")).await,
        StatusCode::NOT_FOUND
    );
    // The whole category, for every manager: B can add content anywhere in it.
    let r = send(
        &app,
        "PUT",
        &category_all,
        Some(&a),
        Some(json!({ "enabled": true })),
    )
    .await;
    assert_eq!(r.json["shared_with_all"], true);
    assert_eq!(
        get_as_b(format!("/api/admin/quizzes/{q2}")).await,
        StatusCode::OK
    );
    let r = send(&app, "GET", &s1_shares, Some(&b), None).await;
    assert_eq!(r.json["category_shared_with_all"], true);
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/admin/quizzes",
            Some(&b),
            Some(quiz(s2, "B in s2 via all"))
        )
        .await
        .status,
        StatusCode::CREATED
    );
    send(
        &app,
        "PUT",
        &category_all,
        Some(&a),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(
        get_as_b(format!("/api/admin/quizzes/{q2}")).await,
        StatusCode::NOT_FOUND
    );

    // Admins see everything and can manage any category's sharing; an API key can't share.
    let r = send(
        &app,
        "GET",
        &format!("/api/admin/quizzes/{q1}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.json["access"], "admin");
    let r = send(&app, "GET", &category_shares, Some(&admin), None).await;
    assert_eq!(r.json["can_manage"], true);
    let key = send(
        &app,
        "POST",
        "/api/admin/api-keys",
        Some(&a),
        Some(json!({ "name": "k", "scopes": ["quizzes:write"], "expires_in_days": 1 })),
    )
    .await
    .json["key"]
        .as_str()
        .unwrap()
        .to_string();
    let r = send_key(
        &app,
        "POST",
        &category_shares,
        &key,
        Some(json!({ "email": "editor-b@example.com" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");

    // Deleting A hands A's category to the person it was first shared with.
    send(
        &app,
        "POST",
        &category_shares,
        Some(&a),
        Some(json!({ "email": "editor-b@example.com" })),
    )
    .await;
    assert_eq!(
        send(
            &app,
            "DELETE",
            &format!("/api/admin/users/{a_id}"),
            Some(&admin),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let categories = send(&app, "GET", "/api/admin/categories", Some(&b), None).await;
    let y = categories
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == category)
        .unwrap();
    assert_eq!(y["owner_id"], b_id);
    assert_eq!(y["can_manage"], true);
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_categories_group_subjects() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "m-categories@example.com", "manager-pass-y", true).await;
    let app = app_with_pool(db);

    // The migration put the seeded HCI subject into a default "General" category.
    let categories = send(&app, "GET", "/api/categories", None, None).await;
    assert_eq!(categories.status, StatusCode::OK);
    assert_eq!(categories.json[0]["slug"], "general");
    assert_eq!(categories.json[0]["subject_count"], 1);
    let category_page = send(
        &app,
        "GET",
        "/api/categories?search=General&limit=20&offset=0",
        None,
        None,
    )
    .await;
    assert_eq!(category_page.json["items"][0]["slug"], "general");
    assert_eq!(category_page.json["has_more"], false);
    let general = send(&app, "GET", "/api/categories/general", None, None).await;
    assert_eq!(general.json["subjects"][0]["slug"], "hci");
    assert_eq!(general.json["subjects"][0]["category_name"], "General");
    let subject_page = send(
        &app,
        "GET",
        "/api/categories/general?search=Human&limit=20&offset=0",
        None,
        None,
    )
    .await;
    assert_eq!(subject_page.json["subjects"][0]["slug"], "hci");
    assert_eq!(subject_page.json["subjects_has_more"], false);
    let review_page = send(
        &app,
        "GET",
        "/api/subjects/hci/reviews?slug=missing&limit=1&offset=0",
        None,
        None,
    )
    .await;
    assert_eq!(review_page.json["items"].as_array().unwrap().len(), 0);

    // Admin creates a custom-named category and a subject in it.
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    let y = send(
        &app,
        "POST",
        "/api/admin/categories",
        Some(&admin),
        Some(json!({ "name": "Very Cool Category", "sort_order": 1 })),
    )
    .await;
    assert_eq!(y.status, StatusCode::CREATED);
    assert_eq!(y.json["slug"], "very-cool-category");
    let category_id = y.json["id"].as_i64().unwrap();
    let subj = send(&app, "POST", "/api/admin/subjects", Some(&admin),
        Some(json!({ "category_id": category_id, "name": "Internet Programming", "slug": "ip", "color": "blue" }))).await;
    assert_eq!(subj.status, StatusCode::CREATED);
    assert_eq!(subj.json["category_slug"], "very-cool-category");

    let mine = send(
        &app,
        "GET",
        "/api/categories/very-cool-category",
        None,
        None,
    )
    .await;
    assert_eq!(mine.json["subject_count"], 1);
    assert_eq!(mine.json["subjects"][0]["slug"], "ip");
    let filtered = send(
        &app,
        "GET",
        "/api/subjects?category=very-cool-category",
        None,
        None,
    )
    .await;
    assert_eq!(filtered.json.as_array().unwrap().len(), 1);

    // A subject needs a category; a category with subjects can't be deleted.
    let r = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&admin),
        Some(json!({ "name": "No category", "slug": "no-category" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["error"]["code"], "validation_error");
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/categories/{category_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // Rename keeps working (e.g. "General" -> "I4").
    let r = send(
        &app,
        "PUT",
        "/api/admin/categories/1",
        Some(&admin),
        Some(json!({ "name": "I4", "slug": "i4", "sort_order": 0 })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        send(&app, "GET", "/api/categories/i4", None, None)
            .await
            .json["subjects"][0]["slug"],
        "hci"
    );

    // Managers can create their own categories, but not change someone else's.
    let m = cookie_from(
        &login(&app, "m-categories@example.com", "manager-pass-y")
            .await
            .set_cookie
            .unwrap(),
    );
    let r = send(
        &app,
        "POST",
        "/api/admin/categories",
        Some(&m),
        Some(json!({ "name": "X" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = send(
        &app,
        "PUT",
        "/api/admin/categories/1",
        Some(&m),
        Some(json!({ "name": "Hijack" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"]["code"], "owner_only");
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_categories_belong_to_their_creator() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "owner-y@example.com", "manager-pass-1", true).await;
    insert_manager(&db, "other-y@example.com", "manager-pass-2", true).await;
    let app = app_with_pool(db);
    let a = cookie_from(
        &login(&app, "owner-y@example.com", "manager-pass-1")
            .await
            .set_cookie
            .unwrap(),
    );
    let b = cookie_from(
        &login(&app, "other-y@example.com", "manager-pass-2")
            .await
            .set_cookie
            .unwrap(),
    );
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );

    // A creates category "I4" with subject "a-hci" in the dashboard.
    let a_sid = own_subject(&app, &a, "a").await;
    let a_key = send(&app, "POST", "/api/admin/api-keys", Some(&a),
        Some(json!({ "name": "A's AI", "scopes": ["quizzes:write", "reviews:write"], "expires_in_days": 1 }))).await
        .json["key"].as_str().unwrap().to_string();
    let quiz = |slug: &str| {
        json!({
            "subject_slug": slug, "title": "From the AI",
            "questions": [{ "prompt": "Q?", "answer_text": "A", "options": [{ "text": "A", "is_correct": true }, { "text": "B" }] }]
        })
    };

    // The key sees which categories it may use…
    let categories = send_key(&app, "GET", "/api/admin/categories", &a_key, None).await;
    assert_eq!(categories.status, StatusCode::OK);
    let mine: Vec<&serde_json::Value> = categories
        .json
        .as_array()
        .unwrap()
        .iter()
        .filter(|y| y["can_manage"] == true)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["subjects"][0]["id"], a_sid);

    // …adds content to A's own category…
    let r = send_key(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        &a_key,
        Some(quiz("subject-a")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    // …but not to a category A doesn't own (the admin-owned "General" with hci).
    let r = send_key(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        &a_key,
        Some(quiz("hci")),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"]["code"], "not_your_category");
    assert!(
        r.json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Category a")
    );

    // The AI can't create categories or subjects, only a human in the dashboard.
    let r = send_key(
        &app,
        "POST",
        "/api/admin/categories",
        &a_key,
        Some(json!({ "name": "AI category" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");
    let r = send_key(
        &app,
        "POST",
        "/api/admin/subjects",
        &a_key,
        Some(json!({ "category_id": 1, "name": "x", "slug": "x" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");

    // B can't add content or subjects to A's category, nor change it.
    let a_category = mine[0]["id"].as_i64().unwrap();
    let r = send(&app, "POST", "/api/admin/quizzes", Some(&b),
        Some(json!({ "subject_id": a_sid, "title": "B in A's category", "description": null, "cover_image_key": null }))).await;
    assert_eq!(r.json["error"]["code"], "not_your_category");
    let r = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&b),
        Some(json!({ "category_id": a_category, "name": "B subject", "slug": "b-subject" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/categories/{a_category}"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");

    // Admins may use and manage every category.
    let r = send(&app, "POST", "/api/admin/quizzes", Some(&admin),
        Some(json!({ "subject_id": a_sid, "title": "Admin in A's category", "description": null, "cover_image_key": null }))).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = send(
        &app,
        "PUT",
        &format!("/api/admin/categories/{a_category}"),
        Some(&admin),
        Some(json!({ "name": "I4" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_change_own_password_requires_the_current_one() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    let m_id = insert_manager(&db, "pw@example.com", "old-password-1", true).await;
    let app = app_with_pool(db.clone());
    let browser1 = cookie_from(
        &login(&app, "pw@example.com", "old-password-1")
            .await
            .set_cookie
            .unwrap(),
    );
    let browser2 = cookie_from(
        &login(&app, "pw@example.com", "old-password-1")
            .await
            .set_cookie
            .unwrap(),
    );
    let change =
        |current: &str, new: &str| json!({ "current_password": current, "new_password": new });

    // Wrong current password, too short, or unchanged: refused, nothing changes.
    let r = send(
        &app,
        "POST",
        "/api/auth/change-password",
        Some(&browser1),
        Some(change("wrong-password", "new-password-2")),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        r.json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("current_password")
    );
    let r = send(
        &app,
        "POST",
        "/api/auth/change-password",
        Some(&browser1),
        Some(change("old-password-1", "short")),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    let r = send(
        &app,
        "POST",
        "/api/auth/change-password",
        Some(&browser1),
        Some(change("old-password-1", "old-password-1")),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Correct current password: changed; this browser gets a fresh cookie and stays signed in…
    let r = send(
        &app,
        "POST",
        "/api/auth/change-password",
        Some(&browser1),
        Some(change("old-password-1", "new-password-2")),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let fresh = cookie_from(&r.set_cookie.expect("new session cookie"));
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&fresh), None)
            .await
            .status,
        StatusCode::OK
    );
    // …and every other session is signed out (including this browser's old cookie).
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&browser2), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&browser1), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    // Old password no longer works, new one does.
    assert_eq!(
        login(&app, "pw@example.com", "old-password-1").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(&app, "pw@example.com", "new-password-2").await.status,
        StatusCode::OK
    );

    // An admin reset also signs the manager out everywhere.
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    let r = send(
        &app,
        "POST",
        &format!("/api/admin/users/{m_id}/reset-password"),
        Some(&admin),
        Some(json!({ "password": "reset-password-3" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&fresh), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    // Guessing the current password is rate-limited (5 attempts / minute per user).
    // (A fresh app instance: the login limiter above has already seen 6 logins from this test.)
    let app = app_with_pool(db);
    let m = cookie_from(
        &login(&app, "pw@example.com", "reset-password-3")
            .await
            .set_cookie
            .unwrap(),
    );
    let mut last = StatusCode::OK;
    for _ in 0..6 {
        last = send(
            &app,
            "POST",
            "/api/auth/change-password",
            Some(&m),
            Some(change("guess-123", "whatever-456")),
        )
        .await
        .status;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_admin_password_follows_env_but_keeps_dashboard_changes() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    let mut config = test_config();

    // First start: admin created from .env.
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    let app = app_with_pool(db.clone());
    let session = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );

    // Restart with the SAME .env password: nothing changes, the session survives.
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&session), None)
            .await
            .status,
        StatusCode::OK
    );

    // ADMIN_PASSWORD changed in .env → applied on restart; old password and sessions stop working.
    config.admin_password = "env-password-new-1".into();
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    assert_eq!(
        send(&app, "GET", "/api/auth/me", Some(&session), None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let app = app_with_pool(db.clone()); // fresh login rate limiter
    assert_eq!(
        login(&app, "admin@example.com", "admin-password-123")
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let r = login(&app, "admin@example.com", "env-password-new-1").await;
    assert_eq!(r.status, StatusCode::OK);
    let session = cookie_from(&r.set_cookie.unwrap());

    // The admin changes the password in the dashboard; a restart with the unchanged .env keeps it.
    let r = send(
        &app,
        "POST",
        "/api/auth/change-password",
        Some(&session),
        Some(
            json!({ "current_password": "env-password-new-1", "new_password": "dashboard-pass-2" }),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    let app = app_with_pool(db.clone());
    assert_eq!(
        login(&app, "admin@example.com", "dashboard-pass-2")
            .await
            .status,
        StatusCode::OK
    );

    // Changing .env again wins over the dashboard password.
    config.admin_password = "env-password-new-3".into();
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    assert_eq!(
        login(&app, "admin@example.com", "env-password-new-3")
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        login(&app, "admin@example.com", "dashboard-pass-2")
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    // A new ADMIN_EMAIL creates that admin account too.
    config.admin_email = "new-admin@example.com".into();
    bootstrap::ensure_admin(&db, &config).await.unwrap();
    let app = app_with_pool(db);
    let r = login(&app, "new-admin@example.com", "env-password-new-3").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["role"], "admin");
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_cache_serves_students_but_never_outlives_an_update() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "cache-m@example.com", "manager-pass-c", true).await;
    let app = app_with_pool(db);
    let m = cookie_from(
        &login(&app, "cache-m@example.com", "manager-pass-c")
            .await
            .set_cookie
            .unwrap(),
    );
    own_subject(&app, &m, "cache").await;

    // A published quiz in the manager's subject.
    let quiz = send(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        Some(&m),
        Some(json!({
            "subject_slug": "subject-cache", "title": "Cached quiz",
            "questions": [{ "prompt": "Original prompt", "answer_text": "A",
                            "options": [{ "text": "A", "is_correct": true }, { "text": "B" }] }]
        })),
    )
    .await;
    assert_eq!(quiz.status, StatusCode::CREATED);
    let qid = quiz.json["id"].as_i64().unwrap();
    let question = quiz.json["questions"][0].clone();
    send(
        &app,
        "PATCH",
        &format!("/api/admin/quizzes/{qid}/publish"),
        Some(&m),
        Some(json!({ "is_published": true })),
    )
    .await;
    let quiz_page = send(
        &app,
        "GET",
        "/api/subjects/subject-cache/quizzes?search=Cached&limit=20&offset=0",
        None,
        None,
    )
    .await;
    assert_eq!(quiz_page.json["items"][0]["title"], "Cached quiz");
    assert_eq!(quiz_page.json["items"][0]["question_count"], 1);
    assert_eq!(quiz_page.json["has_more"], false);
    let public = format!("/api/quizzes/{qid}");

    // Students: first request computes, second is served from the cache.
    let first = send(&app, "GET", &public, None, None).await;
    assert_eq!(first.x_cache.as_deref(), Some("miss"));
    assert_eq!(first.cache_control.as_deref(), Some("no-cache")); // browsers must re-check
    let second = send(&app, "GET", &public, None, None).await;
    assert_eq!(second.x_cache.as_deref(), Some("hit"));
    assert_eq!(second.json, first.json);
    assert_eq!(
        send(
            &app,
            "GET",
            "/api/subjects/subject-cache/quizzes",
            None,
            None
        )
        .await
        .x_cache
        .as_deref(),
        Some("miss")
    );
    assert_eq!(
        send(
            &app,
            "GET",
            "/api/subjects/subject-cache/quizzes",
            None,
            None
        )
        .await
        .x_cache
        .as_deref(),
        Some("hit")
    );

    // Logged-in editors never get cached data.
    assert_eq!(
        send(&app, "GET", &public, Some(&m), None).await.x_cache,
        None
    );

    // The editor changes the question → the very next student request sees the new text.
    let mut edited = json!({
        "prompt": "Updated prompt", "answer_text": "A", "multi_select": false,
        "options": [{ "text": "A", "is_correct": true }, { "text": "B" }]
    });
    edited["image_key"] = question["image_key"].clone();
    let r = send(
        &app,
        "PUT",
        &format!("/api/admin/quizzes/{qid}/questions/{}", question["id"]),
        Some(&m),
        Some(edited),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let after = send(&app, "GET", &public, None, None).await;
    assert_eq!(after.x_cache.as_deref(), Some("miss"));
    assert_eq!(after.json["questions"][0]["prompt"], "Updated prompt");

    // Same through an API key: unpublishing hides the quiz immediately (no stale cached copy).
    send(&app, "GET", &public, None, None).await; // re-cache
    let key = send(
        &app,
        "POST",
        "/api/admin/api-keys",
        Some(&m),
        Some(json!({ "name": "k", "scopes": ["quizzes:write"], "expires_in_days": 1 })),
    )
    .await
    .json["key"]
        .as_str()
        .unwrap()
        .to_string();
    let r = send_key(
        &app,
        "PATCH",
        &format!("/api/admin/quizzes/{qid}/publish"),
        &key,
        Some(json!({ "is_published": false })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        send(&app, "GET", &public, None, None).await.status,
        StatusCode::NOT_FOUND
    );
    let list = send(
        &app,
        "GET",
        "/api/subjects/subject-cache/quizzes",
        None,
        None,
    )
    .await;
    assert_eq!(list.json.as_array().unwrap().len(), 0);

    // A failed write does not clear the cache (nothing changed).
    send(&app, "GET", "/api/categories", None, None).await;
    assert_eq!(
        send(&app, "GET", "/api/categories", None, None)
            .await
            .x_cache
            .as_deref(),
        Some("hit")
    );
    let r = send(
        &app,
        "PUT",
        &format!("/api/admin/quizzes/{qid}"),
        Some(&m),
        Some(json!({ "title": "" })),
    )
    .await;
    assert!(!r.status.is_success());
    assert_eq!(
        send(&app, "GET", "/api/categories", None, None)
            .await
            .x_cache
            .as_deref(),
        Some("hit")
    );
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_subject_slugs_are_unique_per_category_only() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "slug-m@example.com", "manager-pass-s", true).await;
    let app = app_with_pool(db);
    let m = cookie_from(
        &login(&app, "slug-m@example.com", "manager-pass-s")
            .await
            .set_cookie
            .unwrap(),
    );

    // Two categories, each with an "algo" subject.
    let mut categories = Vec::new();
    for slug in ["i3", "i4"] {
        let c = send(
            &app,
            "POST",
            "/api/admin/categories",
            Some(&m),
            Some(json!({ "name": slug.to_uppercase(), "slug": slug })),
        )
        .await;
        assert_eq!(
            c.status,
            StatusCode::CREATED,
            "create category: {:?}",
            c.json
        );
        let s = send(&app, "POST", "/api/admin/subjects", Some(&m),
            Some(json!({ "category_id": c.json["id"], "name": format!("Algo {slug}"), "slug": "algo" }))).await;
        assert_eq!(
            s.status,
            StatusCode::CREATED,
            "create subject: {:?}",
            s.json
        );
        categories.push((
            c.json["slug"].as_str().unwrap().to_string(),
            c.json["id"].clone(),
        ));
    }
    // ...but still only one "algo" per category.
    let dup = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&m),
        Some(json!({ "category_id": categories[0].1, "name": "Algo again", "slug": "algo" })),
    )
    .await;
    assert_eq!(dup.status, StatusCode::CONFLICT);

    // Each is reachable through its category.
    for (cat, _) in &categories {
        let r = send(
            &app,
            "GET",
            &format!("/api/categories/{cat}/subjects/algo"),
            None,
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.json["name"], format!("Algo {cat}"));
        let q = send(
            &app,
            "GET",
            &format!("/api/categories/{cat}/subjects/algo/quizzes"),
            None,
            None,
        )
        .await;
        assert_eq!(q.status, StatusCode::OK);
    }
    // The legacy slug-only route can't tell them apart.
    let legacy = send(&app, "GET", "/api/subjects/algo", None, None).await;
    assert_eq!(
        legacy.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{:?}",
        legacy.json
    );

    // Import: subject_slug alone is ambiguous; with category_slug it lands in the right one.
    let quiz = json!({ "title": "Sorting", "subject_slug": "algo",
        "questions": [{ "prompt": "Fastest?", "answer_text": "Quick",
                        "options": [{ "text": "Quick", "is_correct": true }, { "text": "Bubble" }] }] });
    let r = send(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        Some(&m),
        Some(quiz.clone()),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{:?}", r.json);
    let mut with_cat = quiz;
    with_cat["category_slug"] = json!(categories[1].0);
    let r = send(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        Some(&m),
        Some(with_cat),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{:?}", r.json);
    assert_eq!(r.json["category_slug"], categories[1].0);
    assert_eq!(r.json["category_name"], "I4");
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_question_text_format_defaults_to_plain_and_is_per_question() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "fmt-m@example.com", "manager-pass-f", true).await;
    let app = app_with_pool(db);
    let m = cookie_from(
        &login(&app, "fmt-m@example.com", "manager-pass-f")
            .await
            .set_cookie
            .unwrap(),
    );
    own_subject(&app, &m, "fmt").await;
    let opts = json!([{ "text": "A", "is_correct": true }, { "text": "B" }]);

    // Import: unspecified → plain, explicit markdown kept, per question.
    let quiz = send(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        Some(&m),
        Some(json!({
            "subject_slug": "subject-fmt", "title": "Formats",
            "questions": [
                { "prompt": "Plain __init__", "options": opts },
                { "prompt": "```py\nprint(1)\n```", "text_format": "markdown", "options": opts }
            ]
        })),
    )
    .await;
    assert_eq!(quiz.status, StatusCode::CREATED);
    let qid = quiz.json["id"].as_i64().unwrap();
    assert_eq!(quiz.json["questions"][0]["text_format"], "plain");
    assert_eq!(quiz.json["questions"][1]["text_format"], "markdown");

    // Unknown values are rejected.
    let bad = send(
        &app,
        "POST",
        &format!("/api/admin/quizzes/{qid}/questions"),
        Some(&m),
        Some(json!({ "prompt": "x", "text_format": "html", "options": opts })),
    )
    .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Updating a question can switch its format; leaving it out resets to plain.
    let first = quiz.json["questions"][0]["id"].clone();
    let path = format!("/api/admin/quizzes/{qid}/questions/{first}");
    let r = send(
        &app,
        "PUT",
        &path,
        Some(&m),
        Some(json!({ "prompt": "`x`", "text_format": "markdown", "options": opts })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["questions"][0]["text_format"], "markdown");
    assert_eq!(r.json["questions"][1]["text_format"], "markdown");
    let r = send(
        &app,
        "PUT",
        &path,
        Some(&m),
        Some(json!({ "prompt": "`x`", "options": opts })),
    )
    .await;
    assert_eq!(r.json["questions"][0]["text_format"], "plain");
}
