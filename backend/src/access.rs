//! Who may manage which content.
//!
//! **Years** belong to their creator. The year owner (or an admin) manages the year, its
//! subjects, and who it is shared with. A year, or one subject of a year, can be **shared** with
//! other users by email (dashboard only). There is no per-item sharing: access to a year or
//! subject covers every quiz and interactive review inside it.
//!
//! Access to a quiz/review, from strongest to weakest:
//! - `owner`: you created the item;
//! - `admin`: you are an admin (every item);
//! - `year_owner`: the item is in a year you own;
//! - `shared`: its year, or its subject, is shared with you.
//!
//! The first three have full rights (edit, publish, delete). `shared` can view, edit and publish
//! but not delete. Without any of them the item is invisible (`404`).
//!
//! **Adding content** (or moving it into a subject) requires owning the subject's year, having
//! the year or that subject shared with you, or being an admin (`403 not_your_year` otherwise).
//!
//! Students are unaffected: published content is public.

use std::collections::{HashMap, HashSet};

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
#[serde(rename_all = "snake_case")]
pub enum Access {
    /// You created it.
    Owner,
    /// You are an admin.
    Admin,
    /// It is in a year you own.
    YearOwner,
    /// Its year or subject is shared with you: view, edit and publish, but not delete.
    Shared,
}

/// Access details returned with every item in the editor API.
#[derive(Debug, Clone, Serialize)]
pub struct AccessInfo {
    /// `owner`, `admin`, `year_owner` or `shared`.
    pub access: Access,
    /// Who created the item.
    pub owner_id: Option<i64>,
    pub owner_name: Option<String>,
}

impl AccessInfo {
    /// Full rights: delete the item (owner, admin or the year's owner).
    pub fn can_manage(&self) -> bool {
        self.access != Access::Shared
    }

    pub fn require_owner(&self, what: &str) -> AppResult<()> {
        if self.can_manage() {
            Ok(())
        } else {
            Err(AppError::OwnerOnly(what.to_string()))
        }
    }
}

fn access_for(
    actor: Actor,
    owner_id: Option<i64>,
    year_owner: Option<i64>,
    shared: bool,
) -> Option<Access> {
    if owner_id == Some(actor.id) {
        Some(Access::Owner)
    } else if actor.is_admin {
        Some(Access::Admin)
    } else if year_owner == Some(actor.id) {
        Some(Access::YearOwner)
    } else if shared {
        Some(Access::Shared)
    } else {
        None
    }
}

/// (item owner, owner name, year owner, shared with actor via year or subject)
type AccessRow = (Option<i64>, Option<String>, Option<i64>, i64);
/// (id, item owner, owner name, year owner, shared with actor via year or subject)
type AccessListRow = (i64, Option<i64>, Option<String>, Option<i64>, i64);

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
                "SELECT z.created_by, u.display_name, y.created_by, \
                 CAST(EXISTS(SELECT 1 FROM year_shares ys WHERE ys.year_id = y.id AND ys.user_id = ?) \
                   OR EXISTS(SELECT 1 FROM subject_shares ss WHERE ss.subject_id = sj.id AND ss.user_id = ?) AS SIGNED) \
                 FROM quizzes z JOIN subjects sj ON sj.id = z.subject_id JOIN years y ON y.id = sj.year_id \
                 LEFT JOIN users u ON u.id = z.created_by WHERE z.id = ?",
            )
            .bind(actor.id)
            .bind(actor.id)
            .bind(id)
            .fetch_optional(db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT r.created_by, u.display_name, y.created_by, \
                 CAST(EXISTS(SELECT 1 FROM year_shares ys WHERE ys.year_id = y.id AND ys.user_id = ?) \
                   OR EXISTS(SELECT 1 FROM subject_shares ss WHERE ss.subject_id = sj.id AND ss.user_id = ?) AS SIGNED) \
                 FROM interactive_reviews r JOIN subjects sj ON sj.id = r.subject_id JOIN years y ON y.id = sj.year_id \
                 LEFT JOIN users u ON u.id = r.created_by WHERE r.id = ?",
            )
            .bind(actor.id)
            .bind(actor.id)
            .bind(id)
            .fetch_optional(db)
            .await?
        }
    };
    let (owner_id, owner_name, year_owner, shared) = row.ok_or(AppError::NotFound(kind.label()))?;
    let access = access_for(actor, owner_id, year_owner, shared > 0)
        .ok_or(AppError::NotFound(kind.label()))?;
    Ok(AccessInfo {
        access,
        owner_id,
        owner_name,
    })
}

/// Access info for every item `actor` can manage, keyed by item id.
pub async fn accessible(
    db: &MySqlPool,
    kind: Content,
    actor: Actor,
) -> AppResult<HashMap<i64, AccessInfo>> {
    let rows: Vec<AccessListRow> = match kind {
        Content::Quiz => {
            sqlx::query_as(
                "SELECT z.id, z.created_by, u.display_name, y.created_by, \
                 CAST(EXISTS(SELECT 1 FROM year_shares ys WHERE ys.year_id = y.id AND ys.user_id = ?) \
                   OR EXISTS(SELECT 1 FROM subject_shares ss WHERE ss.subject_id = sj.id AND ss.user_id = ?) AS SIGNED) \
                 FROM quizzes z JOIN subjects sj ON sj.id = z.subject_id JOIN years y ON y.id = sj.year_id \
                 LEFT JOIN users u ON u.id = z.created_by",
            )
            .bind(actor.id)
            .bind(actor.id)
            .fetch_all(db)
            .await?
        }
        Content::Review => {
            sqlx::query_as(
                "SELECT r.id, r.created_by, u.display_name, y.created_by, \
                 CAST(EXISTS(SELECT 1 FROM year_shares ys WHERE ys.year_id = y.id AND ys.user_id = ?) \
                   OR EXISTS(SELECT 1 FROM subject_shares ss WHERE ss.subject_id = sj.id AND ss.user_id = ?) AS SIGNED) \
                 FROM interactive_reviews r JOIN subjects sj ON sj.id = r.subject_id JOIN years y ON y.id = sj.year_id \
                 LEFT JOIN users u ON u.id = r.created_by",
            )
            .bind(actor.id)
            .bind(actor.id)
            .fetch_all(db)
            .await?
        }
    };
    Ok(rows
        .into_iter()
        .filter_map(|(id, owner_id, owner_name, year_owner, shared)| {
            let access = access_for(actor, owner_id, year_owner, shared > 0)?;
            Some((
                id,
                AccessInfo {
                    access,
                    owner_id,
                    owner_name,
                },
            ))
        })
        .collect())
}

// ---------- Years and subjects ----------

/// Owner rights on a year: edit/delete it, manage its subjects and who it is shared with.
pub fn can_manage_year(actor: Actor, year_owner: Option<i64>) -> bool {
    actor.is_admin || year_owner == Some(actor.id)
}

/// Require owner rights on a year (`404` if it doesn't exist, `403 owner_only` otherwise).
pub async fn require_year(db: &MySqlPool, actor: Actor, year_id: i64, what: &str) -> AppResult<()> {
    let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT created_by FROM years WHERE id = ?")
        .bind(year_id)
        .fetch_optional(db)
        .await?;
    let (owner,) = row.ok_or(AppError::NotFound("year"))?;
    if can_manage_year(actor, owner) {
        Ok(())
    } else {
        Err(AppError::OwnerOnly(what.to_string()))
    }
}

/// The years and subjects shared with a user.
#[derive(Debug, Default)]
pub struct SharedWithMe {
    pub years: HashSet<i64>,
    pub subjects: HashSet<i64>,
}

pub async fn shared_with(db: &MySqlPool, user_id: i64) -> AppResult<SharedWithMe> {
    let years: Vec<(i64,)> = sqlx::query_as("SELECT year_id FROM year_shares WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db)
        .await?;
    let subjects: Vec<(i64,)> =
        sqlx::query_as("SELECT subject_id FROM subject_shares WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(db)
            .await?;
    Ok(SharedWithMe {
        years: years.into_iter().map(|(y,)| y).collect(),
        subjects: subjects.into_iter().map(|(s,)| s).collect(),
    })
}

/// Content may only be added to (or moved into) a subject whose year you own, whose year or
/// subject is shared with you, or anywhere if you are an admin. The error lists where you can
/// add content, so an API client knows its options.
pub async fn check_can_add_to_subject(
    db: &MySqlPool,
    actor: Actor,
    subject_id: i64,
) -> AppResult<()> {
    if actor.is_admin {
        return Ok(());
    }
    let row: Option<(i64, Option<i64>, String, String)> = sqlx::query_as(
        "SELECT y.id, y.created_by, y.name, s.name FROM subjects s JOIN years y ON y.id = s.year_id \
         WHERE s.id = ?",
    )
    .bind(subject_id)
    .fetch_optional(db)
    .await?;
    let (year_id, owner, year_name, subject_name) = row.ok_or(AppError::NotFound("subject"))?;
    let shared = shared_with(db, actor.id).await?;
    if owner == Some(actor.id)
        || shared.years.contains(&year_id)
        || shared.subjects.contains(&subject_id)
    {
        return Ok(());
    }
    let places: Vec<(String,)> = sqlx::query_as(
        "SELECT y.name FROM years y WHERE y.created_by = ? \
           OR EXISTS(SELECT 1 FROM year_shares ys WHERE ys.year_id = y.id AND ys.user_id = ?) \
         UNION \
         SELECT CONCAT(y.name, ' · ', s.name) FROM subject_shares ss \
           JOIN subjects s ON s.id = ss.subject_id JOIN years y ON y.id = s.year_id \
         WHERE ss.user_id = ?",
    )
    .bind(actor.id)
    .bind(actor.id)
    .bind(actor.id)
    .fetch_all(db)
    .await?;
    let yours = if places.is_empty() {
        "you can't add content anywhere yet: create a year in the dashboard, or ask a year's \
         owner to share it (or one of its subjects) with you"
            .to_string()
    } else {
        format!(
            "you can add content to: {}",
            places
                .into_iter()
                .map(|(p,)| p)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Err(AppError::NotYourYear(format!(
        "\"{year_name} · {subject_name}\" isn't yours or shared with you ({yours})"
    )))
}
