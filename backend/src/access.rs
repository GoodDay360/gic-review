//! Who may manage which content.
//!
//! - The **owner** of a quiz or interactive review is the user who created it (`created_by`).
//! - The owner can **share** it with other managers by email. Shared users can view, edit
//!   and publish it (in the dashboard or with their API key).
//! - Only the owner may delete it or change who it is shared with.
//! - **Admins** have the owner's rights on every item (`access: "admin"`).
//! - Managers without access get `404` (they don't learn that the item exists).
//!
//! Students are unaffected: published content is public.

use std::collections::HashMap;

use serde::Serialize;
use sqlx::MySqlPool;

use crate::{
    auth::{AuthUser, Role},
    error::{AppError, AppResult},
};

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

/// Who is asking: their user id and whether they are an admin.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub id: i64,
    pub is_admin: bool,
}

impl Actor {
    pub fn of(user: &AuthUser) -> Self {
        Self {
            id: user.id,
            is_admin: user.role == Role::Admin,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// You created it.
    Owner,
    /// You are an admin: owner rights on someone else's item.
    Admin,
    /// Its owner shared it with you: edit and publish, but not delete or re-share.
    Shared,
}

/// Ownership details returned with every item in the editor API.
#[derive(Debug, Clone, Serialize)]
pub struct AccessInfo {
    /// `owner`, `admin` or `shared`.
    pub access: Access,
    pub owner_id: Option<i64>,
    pub owner_name: Option<String>,
    /// How many users it is shared with.
    pub share_count: i64,
}

impl AccessInfo {
    pub fn is_owner(&self) -> bool {
        self.access == Access::Owner
    }

    /// Owner rights: delete the item and manage who it is shared with (owner or admin).
    pub fn can_manage(&self) -> bool {
        matches!(self.access, Access::Owner | Access::Admin)
    }

    /// Only the owner (or an admin) may do this (delete, manage sharing).
    pub fn require_owner(&self, what: &str) -> AppResult<()> {
        if self.can_manage() {
            Ok(())
        } else {
            Err(AppError::OwnerOnly(what.to_string()))
        }
    }
}

fn access_for(actor: Actor, owner_id: Option<i64>, shared_with_actor: bool) -> Option<Access> {
    if owner_id == Some(actor.id) {
        Some(Access::Owner)
    } else if actor.is_admin {
        Some(Access::Admin)
    } else if shared_with_actor {
        Some(Access::Shared)
    } else {
        None
    }
}

type AccessRow = (Option<i64>, Option<String>, i64, i64);
/// (id, owner_id, owner_name, share_count, shared_with_actor)
type AccessListRow = (i64, Option<i64>, Option<String>, i64, i64);

/// Access of `actor` to one item, or `NotFound` if it doesn't exist or isn't accessible.
pub async fn require(
    db: &MySqlPool,
    kind: Content,
    id: i64,
    actor: Actor,
) -> AppResult<AccessInfo> {
    let row: Option<AccessRow> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT z.created_by, u.display_name, \
                 CAST(EXISTS(SELECT 1 FROM quiz_shares s WHERE s.quiz_id = z.id AND s.user_id = ?) AS SIGNED), \
                 (SELECT COUNT(*) FROM quiz_shares s WHERE s.quiz_id = z.id) \
                 FROM quizzes z LEFT JOIN users u ON u.id = z.created_by WHERE z.id = ?",
            )
            .bind(actor.id)
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
            .bind(actor.id)
            .bind(id)
            .fetch_optional(db)
            .await?
        }
    };
    let (owner_id, owner_name, shared, share_count) =
        row.ok_or(AppError::NotFound(kind.label()))?;
    let access = access_for(actor, owner_id, shared > 0).ok_or(AppError::NotFound(kind.label()))?;
    Ok(AccessInfo {
        access,
        owner_id,
        owner_name,
        share_count,
    })
}

/// Access info for every item `actor` can manage (admins: every item), keyed by item id.
pub async fn accessible(
    db: &MySqlPool,
    kind: Content,
    actor: Actor,
) -> AppResult<HashMap<i64, AccessInfo>> {
    let rows: Vec<AccessListRow> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT z.id, z.created_by, u.display_name, \
                 (SELECT COUNT(*) FROM quiz_shares s WHERE s.quiz_id = z.id), \
                 CAST(EXISTS(SELECT 1 FROM quiz_shares s WHERE s.quiz_id = z.id AND s.user_id = ?) AS SIGNED) \
                 FROM quizzes z LEFT JOIN users u ON u.id = z.created_by",
            )
            .bind(actor.id)
            .fetch_all(db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT r.id, r.created_by, u.display_name, \
                 (SELECT COUNT(*) FROM review_shares s WHERE s.review_id = r.id), \
                 CAST(EXISTS(SELECT 1 FROM review_shares s WHERE s.review_id = r.id AND s.user_id = ?) AS SIGNED) \
                 FROM interactive_reviews r LEFT JOIN users u ON u.id = r.created_by",
            )
            .bind(actor.id)
            .fetch_all(db)
            .await?
        }
    };
    Ok(rows
        .into_iter()
        .filter_map(|(id, owner_id, owner_name, share_count, shared)| {
            let access = access_for(actor, owner_id, shared > 0)?;
            Some((
                id,
                AccessInfo {
                    access,
                    owner_id,
                    owner_name,
                    share_count,
                },
            ))
        })
        .collect())
}
