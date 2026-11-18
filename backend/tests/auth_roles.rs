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
    ("POST", "/api/admin/years"),
    ("PUT", "/api/admin/years/1"),
    ("DELETE", "/api/admin/years/1"),
    ("DELETE", "/api/admin/subjects/1"),
    ("GET", "/api/auth/me"),
    ("POST", "/api/auth/change-password"),
    ("GET", "/api/admin/api-keys"),
    ("POST", "/api/admin/api-keys"),
    ("GET", "/api/admin/api-keys/mine"),
    ("DELETE", "/api/admin/api-keys/1"),
];

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

/// Create a year + subject owned by the logged-in editor (dashboard session); returns the subject id.
async fn own_subject(app: &axum::Router, cookie: &str, tag: &str) -> i64 {
    let y = send(
        app,
        "POST",
        "/api/admin/years",
        Some(cookie),
        Some(json!({ "name": format!("Year {tag}") })),
    )
    .await;
    assert_eq!(y.status, StatusCode::CREATED, "create year: {:?}", y.json);
    let s = send(app, "POST", "/api/admin/subjects", Some(cookie),
        Some(json!({ "year_id": y.json["id"], "name": format!("Subject {tag}"), "slug": format!("subject-{tag}") }))).await;
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

    // The key works for quiz content in the manager's own year…
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
async fn db_sharing_a_year_or_subject_gives_access_to_its_content() {
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

    // A owns year "Year a" with subjects s1 and s2, and a quiz in each.
    let s1 = own_subject(&app, &a, "a").await;
    let year: i64 = send(&app, "GET", "/api/subjects/subject-a", None, None)
        .await
        .json["year_id"]
        .as_i64()
        .unwrap();
    let s2 = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&a),
        Some(json!({ "year_id": year, "name": "Second", "slug": "second-a" })),
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
    assert_eq!(r.json["error"]["code"], "not_your_year");

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
        "not_your_year"
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

    // Shared users can't delete others' content, manage the year/subjects, or re-share.
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
            Some(json!({ "year_id": year, "name": "x", "slug": "x" }))
        )
        .await
        .json["error"]["code"],
        "owner_only"
    );
    // The year's owner has full rights on content others add to their year.
    let r = send(
        &app,
        "GET",
        &format!("/api/admin/quizzes/{b_quiz}"),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.json["access"], "year_owner");

    // GET /admin/years tells B where they can add content.
    let years = send(&app, "GET", "/api/admin/years", Some(&b), None).await;
    let y = years
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == year)
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

    // Sharing the whole year opens every subject.
    let year_shares = format!("/api/admin/years/{year}/shares");
    assert_eq!(
        send(
            &app,
            "POST",
            &year_shares,
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

    // B leaves the year and the subject: access is gone.
    assert_eq!(
        send(
            &app,
            "DELETE",
            &format!("{year_shares}/{b_id}"),
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

    // Admins see everything and can manage any year's sharing; an API key can't share.
    let r = send(
        &app,
        "GET",
        &format!("/api/admin/quizzes/{q1}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.json["access"], "admin");
    let r = send(&app, "GET", &year_shares, Some(&admin), None).await;
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
        &year_shares,
        &key,
        Some(json!({ "email": "editor-b@example.com" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");

    // Deleting A hands A's year to the person it was first shared with.
    send(
        &app,
        "POST",
        &year_shares,
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
    let years = send(&app, "GET", "/api/admin/years", Some(&b), None).await;
    let y = years
        .json
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == year)
        .unwrap();
    assert_eq!(y["owner_id"], b_id);
    assert_eq!(y["can_manage"], true);
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_years_group_subjects() {
    let Some(tdb) = common::test_db().await else {
        return;
    };
    let db = tdb.pool.clone();
    bootstrap::ensure_admin(&db, &test_config()).await.unwrap();
    insert_manager(&db, "m-years@example.com", "manager-pass-y", true).await;
    let app = app_with_pool(db);

    // The migration put the seeded HCI subject into a default "General" year.
    let years = send(&app, "GET", "/api/years", None, None).await;
    assert_eq!(years.status, StatusCode::OK);
    assert_eq!(years.json[0]["slug"], "general");
    assert_eq!(years.json[0]["subject_count"], 1);
    let general = send(&app, "GET", "/api/years/general", None, None).await;
    assert_eq!(general.json["subjects"][0]["slug"], "hci");
    assert_eq!(general.json["subjects"][0]["year_name"], "General");

    // Admin creates a custom-named year and a subject in it.
    let admin = cookie_from(
        &login(&app, "admin@example.com", "admin-password-123")
            .await
            .set_cookie
            .unwrap(),
    );
    let y = send(
        &app,
        "POST",
        "/api/admin/years",
        Some(&admin),
        Some(json!({ "name": "Very Cool Year", "sort_order": 1 })),
    )
    .await;
    assert_eq!(y.status, StatusCode::CREATED);
    assert_eq!(y.json["slug"], "very-cool-year");
    let year_id = y.json["id"].as_i64().unwrap();
    let subj = send(&app, "POST", "/api/admin/subjects", Some(&admin),
        Some(json!({ "year_id": year_id, "name": "Internet Programming", "slug": "ip", "color": "blue" }))).await;
    assert_eq!(subj.status, StatusCode::CREATED);
    assert_eq!(subj.json["year_slug"], "very-cool-year");

    let mine = send(&app, "GET", "/api/years/very-cool-year", None, None).await;
    assert_eq!(mine.json["subject_count"], 1);
    assert_eq!(mine.json["subjects"][0]["slug"], "ip");
    let filtered = send(&app, "GET", "/api/subjects?year=very-cool-year", None, None).await;
    assert_eq!(filtered.json.as_array().unwrap().len(), 1);

    // A subject needs a year; a year with subjects can't be deleted.
    let r = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&admin),
        Some(json!({ "name": "No year", "slug": "no-year" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["error"]["code"], "validation_error");
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/years/{year_id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // Rename keeps working (e.g. "General" -> "I4").
    let r = send(
        &app,
        "PUT",
        "/api/admin/years/1",
        Some(&admin),
        Some(json!({ "name": "I4", "slug": "i4", "sort_order": 0 })),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        send(&app, "GET", "/api/years/i4", None, None).await.json["subjects"][0]["slug"],
        "hci"
    );

    // Managers can create their own years, but not change someone else's.
    let m = cookie_from(
        &login(&app, "m-years@example.com", "manager-pass-y")
            .await
            .set_cookie
            .unwrap(),
    );
    let r = send(
        &app,
        "POST",
        "/api/admin/years",
        Some(&m),
        Some(json!({ "name": "X" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = send(
        &app,
        "PUT",
        "/api/admin/years/1",
        Some(&m),
        Some(json!({ "name": "Hijack" })),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"]["code"], "owner_only");
    tdb.cleanup().await;
}

#[tokio::test]
async fn db_years_belong_to_their_creator() {
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

    // A creates year "I4" with subject "a-hci" in the dashboard.
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

    // The key sees which years it may use…
    let years = send_key(&app, "GET", "/api/admin/years", &a_key, None).await;
    assert_eq!(years.status, StatusCode::OK);
    let mine: Vec<&serde_json::Value> = years
        .json
        .as_array()
        .unwrap()
        .iter()
        .filter(|y| y["can_manage"] == true)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["subjects"][0]["id"], a_sid);

    // …adds content to A's own year…
    let r = send_key(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        &a_key,
        Some(quiz("subject-a")),
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    // …but not to a year A doesn't own (the admin-owned "General" with hci).
    let r = send_key(
        &app,
        "POST",
        "/api/admin/quizzes/import",
        &a_key,
        Some(quiz("hci")),
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.json["error"]["code"], "not_your_year");
    assert!(
        r.json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Year a")
    );

    // The AI can't create years or subjects, only a human in the dashboard.
    let r = send_key(
        &app,
        "POST",
        "/api/admin/years",
        &a_key,
        Some(json!({ "name": "AI year" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");
    let r = send_key(
        &app,
        "POST",
        "/api/admin/subjects",
        &a_key,
        Some(json!({ "year_id": 1, "name": "x", "slug": "x" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "api_key_not_allowed");

    // B can't add content or subjects to A's year, nor change it.
    let a_year = mine[0]["id"].as_i64().unwrap();
    let r = send(&app, "POST", "/api/admin/quizzes", Some(&b),
        Some(json!({ "subject_id": a_sid, "title": "B in A's year", "description": null, "cover_image_key": null }))).await;
    assert_eq!(r.json["error"]["code"], "not_your_year");
    let r = send(
        &app,
        "POST",
        "/api/admin/subjects",
        Some(&b),
        Some(json!({ "year_id": a_year, "name": "B subject", "slug": "b-subject" })),
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");
    let r = send(
        &app,
        "DELETE",
        &format!("/api/admin/years/{a_year}"),
        Some(&b),
        None,
    )
    .await;
    assert_eq!(r.json["error"]["code"], "owner_only");

    // Admins may use and manage every year.
    let r = send(&app, "POST", "/api/admin/quizzes", Some(&admin),
        Some(json!({ "subject_id": a_sid, "title": "Admin in A's year", "description": null, "cover_image_key": null }))).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = send(
        &app,
        "PUT",
        &format!("/api/admin/years/{a_year}"),
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
