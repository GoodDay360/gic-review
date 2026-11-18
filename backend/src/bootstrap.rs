//! Creates the first admin from `ADMIN_EMAIL` / `ADMIN_PASSWORD` when no admin exists.

use sqlx::MySqlPool;

use crate::{auth::hash_password, config::Config};

pub async fn ensure_admin(db: &MySqlPool, config: &Config) -> anyhow::Result<()> {
    let (admins,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(db)
        .await?;
    if admins > 0 {
        return Ok(());
    }

    let password = config.admin_password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash_password(&password)).await??;
    sqlx::query(
        "INSERT INTO users (email, password_hash, display_name, role, is_active) \
         VALUES (?, ?, 'Administrator', 'admin', TRUE) \
         ON DUPLICATE KEY UPDATE role = 'admin', is_active = TRUE, password_hash = VALUES(password_hash)",
    )
    .bind(&config.admin_email)
    .bind(password_hash)
    .execute(db)
    .await?;

    // Never log the password itself.
    tracing::warn!(
        email = %config.admin_email,
        "no admin existed, so one was created from ADMIN_EMAIL / ADMIN_PASSWORD. \
         Log in and change this password as soon as possible."
    );
    Ok(())
}
