//! Who may manage which content.
//!
//! - The **owner** of a quiz or interactive review is the manager who created it (`created_by`).
//! - The owner can **share** it with other managers by email. Shared managers can view, edit
//!   and publish it (in the dashboard or with their API key).
//! - Only the owner can delete it or change who it is shared with.
//! - Managers without access get `404` (they don't learn that the item exists).
//!
//! Students are unaffected: published content is public.

use std::collections::HashMap;

use serde::Serialize;
use sqlx::MySqlPool;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Content {
    Quiz,
    Review,
}

impl Content {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quiz => "quiz",
            Self::Review => "review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Owner,
    Shared,
}

/// Ownership details returned with every item in the manager API.
#[derive(Debug, Clone, Serialize)]
pub struct AccessInfo {
    /// `owner` or `shared` (shared with you by the owner).
    pub access: Access,
    pub owner_id: Option<i64>,
    pub owner_name: Option<String>,
    /// How many other managers it is shared with.
    pub share_count: i64,
}

impl AccessInfo {
    pub fn is_owner(&self) -> bool {
        self.access == Access::Owner
    }

    /// Only the owner may do this (delete, manage sharing).
    pub fn require_owner(&self, what: &str) -> AppResult<()> {
        if self.is_owner() {
            Ok(())
        } else {
            Err(AppError::OwnerOnly(what.to_string()))
        }
    }
}

type AccessRow = (Option<i64>, Option<String>, i64, i64);

/// Access of `user_id` to one item, or `NotFound` if it doesn't exist or isn't accessible.
pub async fn require(
    db: &MySqlPool,
    kind: Content,
    id: i64,
    user_id: i64,
) -> AppResult<AccessInfo> {
    let row: Option<AccessRow> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT z.created_by, u.display_name, \
                 CAST(EXISTS(SELECT 1 FROM quiz_shares s WHERE s.quiz_id = z.id AND s.user_id = ?) AS SIGNED), \
                 (SELECT COUNT(*) FROM quiz_shares s WHERE s.quiz_id = z.id) \
                 FROM quizzes z LEFT JOIN users u ON u.id = z.created_by WHERE z.id = ?",
            )
            .bind(user_id)
            .bind(id)
            .fetch_optional(db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT r.created_by, u.display_name, \
                 CAST(EXISTS(SELECT 1 FROM review_shares s WHERE s.review_id = r.id AND s.user_id = ?) AS SIGNED), \
                 (SELECT COUNT(*) FROM review_shares s WHERE s.review_id = r.id) \
                 FROM interactive_reviews r LEFT JOIN users u ON u.id = r.created_by WHERE r.id = ?",
            )
            .bind(user_id)
            .bind(id)
            .fetch_optional(db)
            .await?
        }
    };
    let not_found = AppError::NotFound(kind.label());
    let (owner_id, owner_name, shared, share_count) = row.ok_or(not_found)?;
    let access = if owner_id == Some(user_id) {
        Access::Owner
    } else if shared > 0 {
        Access::Shared
    } else {
        return Err(AppError::NotFound(kind.label()));
    };
    Ok(AccessInfo {
        access,
        owner_id,
        owner_name,
        share_count,
    })
}

/// Access info for every item `user_id` can manage, keyed by item id.
pub async fn accessible(
    db: &MySqlPool,
    kind: Content,
    user_id: i64,
) -> AppResult<HashMap<i64, AccessInfo>> {
    let rows: Vec<(i64, Option<i64>, Option<String>, i64)> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT z.id, z.created_by, u.display_name, \
                 (SELECT COUNT(*) FROM quiz_shares s WHERE s.quiz_id = z.id) \
                 FROM quizzes z LEFT JOIN users u ON u.id = z.created_by \
                 WHERE z.created_by = ? \
                    OR EXISTS(SELECT 1 FROM quiz_shares s WHERE s.quiz_id = z.id AND s.user_id = ?)",
            )
            .bind(user_id)
            .bind(user_id)
            .fetch_all(db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT r.id, r.created_by, u.display_name, \
                 (SELECT COUNT(*) FROM review_shares s WHERE s.review_id = r.id) \
                 FROM interactive_reviews r LEFT JOIN users u ON u.id = r.created_by \
                 WHERE r.created_by = ? \
                    OR EXISTS(SELECT 1 FROM review_shares s WHERE s.review_id = r.id AND s.user_id = ?)",
            )
            .bind(user_id)
            .bind(user_id)
            .fetch_all(db)
            .await?
        }
    };
    Ok(rows
        .into_iter()
        .map(|(id, owner_id, owner_name, share_count)| {
            let access = if owner_id == Some(user_id) {
                Access::Owner
            } else {
                Access::Shared
            };
            (
                id,
                AccessInfo {
                    access,
                    owner_id,
                    owner_name,
                    share_count,
                },
            )
        })
        .collect())
}
