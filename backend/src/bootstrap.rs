//! Keeps the admin account from `ADMIN_EMAIL` / `ADMIN_PASSWORD` (backend/.env) in sync on
//! every start:
//!
//! - No account with `ADMIN_EMAIL` yet → create it as an admin.
//! - `ADMIN_PASSWORD` changed in `.env` since it was last applied → update the admin's password
//!   and sign out its sessions.
//! - `ADMIN_PASSWORD` unchanged → leave the account alone, so a password changed in the
//!   dashboard (My account) is kept across restarts.
//!
//! "Last applied" is tracked with an Argon2 hash in `users.env_password_hash`.
//! The password itself is never logged.

use sqlx::MySqlPool;

use crate::{
    auth::{hash_password, verify_password},
    config::Config,
};

async fn hash(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || hash_password(&password)).await?
}

async fn verify(password: String, hash: String) -> anyhow::Result<bool> {
    Ok(tokio::task::spawn_blocking(move || verify_password(&password, &hash)).await?)
}

pub async fn ensure_admin(db: &MySqlPool, config: &Config) -> anyhow::Result<()> {
    let row: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, role, password_hash, env_password_hash FROM users WHERE email = ?",
    )
    .bind(&config.admin_email)
    .fetch_optional(db)
    .await?;

    let Some((id, role, password_hash, env_hash)) = row else {
        let new_hash = hash(config.admin_password.clone()).await?;
        sqlx::query(
            "INSERT INTO users (email, password_hash, env_password_hash, display_name, role, is_active) \
             VALUES (?, ?, ?, 'Administrator', 'admin', TRUE)",
        )
        .bind(&config.admin_email)
        .bind(&new_hash)
        .bind(&new_hash)
        .execute(db)
        .await?;
        tracing::warn!(
            email = %config.admin_email,
            "created the admin account from ADMIN_EMAIL / ADMIN_PASSWORD in backend/.env"
        );
        return Ok(());
    };

    if role != "admin" {
        tracing::warn!(
            email = %config.admin_email,
            "ADMIN_EMAIL belongs to a manager account; it was not changed. Use another email for the admin."
        );
        return Ok(());
    }

    // Same ADMIN_PASSWORD as last time: keep whatever password the admin has now.
    if let Some(applied) = env_hash.clone()
        && verify(config.admin_password.clone(), applied).await?
    {
        return Ok(());
    }

    // First start since env tracking was added, and the password already matches .env:
    // just remember it.
    if env_hash.is_none() && verify(config.admin_password.clone(), password_hash).await? {
        let new_hash = hash(config.admin_password.clone()).await?;
        sqlx::query("UPDATE users SET env_password_hash = ? WHERE id = ?")
            .bind(new_hash)
            .bind(id)
            .execute(db)
            .await?;
        return Ok(());
    }

    // ADMIN_PASSWORD changed in .env: apply it and sign out the admin's sessions.
    let new_hash = hash(config.admin_password.clone()).await?;
    sqlx::query(
        "UPDATE users SET password_hash = ?, env_password_hash = ?, \
         session_version = session_version + 1, is_active = TRUE WHERE id = ?",
    )
    .bind(&new_hash)
    .bind(&new_hash)
    .bind(id)
    .execute(db)
    .await?;
    tracing::warn!(
        email = %config.admin_email,
        "admin password updated from ADMIN_PASSWORD in backend/.env; existing admin sessions were signed out"
    );
    Ok(())
}
