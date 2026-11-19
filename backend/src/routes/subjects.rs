//! Subject management (dashboard only): editors manage subjects in categories they own;
//! admins manage every subject.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
};
use serde::Deserialize;

use crate::{
    access::{self, Actor},
    auth::{ContentManager, RequireEditor, RequireEditorSession},
    error::{AppError, AppResult},
    extract::AppJson,
    models::{AdminListQuery, SubjectDto},
    routes::{
        categories::{ManagedSubject, contains_ci, ensure_category, managed_categories, paginate},
        public::all_subjects,
    },
    state::AppState,
    validation::{ACCENTS, check_len, clean, is_slug},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/subjects", get(list_managed_subjects).post(create_subject))
        .route(
            "/admin/subjects/{id}",
            put(update_subject).delete(delete_subject),
        )
}

/// Every subject with `can_add_content` / `shared_with_you`, in category then subject order.
/// Filter with `category_id`, `subject_id`; with any of `category_id`, `search` (name, slug,
/// description or category name), `offset`, `limit` it returns one `Page` (max 20).
async fn list_managed_subjects(
    RequireEditor(user, _): ContentManager,
    State(state): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> AppResult<Response> {
    let page = q.page();
    let search = page.search().map(|s| s.to_lowercase());
    let subjects = managed_categories(&state, Actor::of(&user))
        .await?
        .into_iter()
        .flat_map(|c| c.subjects)
        .filter(move |m: &ManagedSubject| {
            let s = &m.subject;
            q.subject_id.is_none_or(|id| s.id == id)
                && q.category_id.is_none_or(|id| s.category_id == id)
                && search.as_deref().is_none_or(|needle| {
                    contains_ci(&s.name, needle)
                        || contains_ci(&s.slug, needle)
                        || contains_ci(&s.category_name, needle)
                        || s.description.as_deref().is_some_and(|d| contains_ci(d, needle))
                })
        });
    Ok(if q.is_paged() {
        Json(paginate(subjects, &page)).into_response()
    } else {
        Json(subjects.collect::<Vec<_>>()).into_response()
    })
}

#[derive(Debug, Deserialize)]
struct SubjectInput {
    /// The category this subject belongs to (required).
    category_id: i64,
    name: String,
    slug: String,
    description: Option<String>,
    color: Option<String>,
    #[serde(default)]
    sort_order: i32,
}

impl SubjectInput {
    fn normalize(mut self) -> AppResult<Self> {
        self.name = self.name.trim().to_string();
        self.slug = self.slug.trim().to_string();
        self.description = clean(self.description);
        self.color = clean(self.color);
        if self.name.is_empty() || self.name.chars().count() > 150 {
            return Err(AppError::Validation(
                "name: is required (max 150 characters)".into(),
            ));
        }
        if !is_slug(&self.slug) {
            return Err(AppError::Validation(
                "slug: use lowercase letters, digits and dashes (e.g. internet-programming)".into(),
            ));
        }
        check_len("description", &self.description, 2000)?;
        if let Some(c) = &self.color
            && !ACCENTS.contains(&c.as_str())
        {
            return Err(AppError::Validation(format!(
                "color: must be one of {}",
                ACCENTS.join(", ")
            )));
        }
        Ok(self)
    }
}

async fn find(state: &AppState, id: i64) -> AppResult<SubjectDto> {
    all_subjects(state)
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or(AppError::NotFound("subject"))
}

/// Editors create subjects in categories they own or that are shared with them (admins: any
/// category). Dashboard only.
async fn create_subject(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    AppJson(body): AppJson<SubjectInput>,
) -> AppResult<(StatusCode, Json<SubjectDto>)> {
    let body = body.normalize()?;
    ensure_category(&state, body.category_id).await?;
    access::require_category_contributor(
        &state.db,
        Actor::of(&user),
        body.category_id,
        "add subjects to this category",
    )
    .await?;
    let res = sqlx::query(
        "INSERT INTO subjects (category_id, slug, name, description, color, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(body.category_id)
    .bind(&body.slug)
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.color.as_deref().unwrap_or("mauve"))
    .bind(body.sort_order)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(find(&state, res.last_insert_id() as i64).await?),
    ))
}

async fn update_subject(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<SubjectInput>,
) -> AppResult<Json<SubjectDto>> {
    let body = body.normalize()?;
    let current = find(&state, id).await?;
    ensure_category(&state, body.category_id).await?;
    let actor = Actor::of(&user);
    // Owner rights on the subject's current category, and on the new one if it moves.
    access::require_category(&state.db, actor, current.category_id, "change this subject").await?;
    access::require_category(
        &state.db,
        actor,
        body.category_id,
        "move subjects into this category",
    )
    .await?;
    sqlx::query(
        "UPDATE subjects SET category_id = ?, slug = ?, name = ?, description = ?, color = ?, sort_order = ? WHERE id = ?",
    )
    .bind(body.category_id)
    .bind(&body.slug)
    .bind(&body.name)
    .bind(&body.description)
    .bind(body.color.as_deref().unwrap_or("mauve"))
    .bind(body.sort_order)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(Json(find(&state, id).await?))
}

async fn delete_subject(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let current = find(&state, id).await?;
    access::require_category(
        &state.db,
        Actor::of(&user),
        current.category_id,
        "delete this subject",
    )
    .await?;
    // FK RESTRICT: fails with 409 if quizzes or reviews still belong to the subject.
    sqlx::query("DELETE FROM subjects WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| match AppError::from(e) {
            AppError::Conflict(_) => {
                AppError::Conflict("delete or move this subject's quizzes and reviews first".into())
            }
            other => other,
        })?;
    Ok(StatusCode::NO_CONTENT)
}
