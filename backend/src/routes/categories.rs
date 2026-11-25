//! Categories ("I3", "I4", or any custom name) group subjects. Students pick a category first.
//!
//! - Public: `GET /categories`, `GET /categories/{slug}` (the category and its subjects)
//! - Editors (session or API key): `GET /admin/categories`: every category with its owner, its subjects,
//!   `can_manage` (owner/admin), `shared_with_you` and `can_add_content` (per category and subject)
//! - Dashboard only (never with an API key): `POST /admin/categories` (the creator becomes the
//!   owner), `PUT/DELETE /admin/categories/{id}` (owner or admin)

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    access::{self, Actor},
    auth::{ContentManager, RequireEditor, RequireEditorSession},
    error::{AppError, AppResult},
    extract::AppJson,
    models::{
        AdminListQuery, CategoryDto, CategoryRow, Page, PageQuery, SubjectDto, category_columns,
    },
    routes::public::{all_categories, all_subjects},
    state::AppState,
    validation::{check_len, clean, is_slug, slugify},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories))
        .route("/categories/{slug}", get(get_category))
        .route(
            "/admin/categories",
            get(list_managed_categories).post(create_category),
        )
        .route(
            "/admin/categories/{id}",
            put(update_category).delete(delete_category),
        )
}

async fn categories_with_counts(state: &AppState) -> AppResult<Vec<CategoryDto>> {
    let categories = all_categories(state).await?;
    let counts: Vec<(i64, i64)> =
        sqlx::query_as("SELECT category_id, COUNT(*) FROM subjects GROUP BY category_id")
            .fetch_all(&state.db)
            .await?;
    Ok(categories
        .into_iter()
        .map(|category| {
            let subject_count = counts
                .iter()
                .find(|(id, _)| *id == category.id)
                .map_or(0, |(_, c)| *c);
            CategoryDto {
                category,
                subject_count,
            }
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct CategoryListRow {
    id: i64,
    slug: String,
    name: String,
    description: Option<String>,
    sort_order: i32,
    created_by: Option<i64>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    subject_count: i64,
}

async fn list_categories(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    if q.search.is_none() && q.offset.is_none() && q.limit.is_none() {
        return Ok(Json(categories_with_counts(&state).await?).into_response());
    }
    let search = q.search();
    let pattern = search.as_ref().map(|s| format!("%{s}%"));
    let limit = q.limit();
    let rows: Vec<CategoryListRow> = sqlx::query_as(
        "SELECT y.id, y.slug, y.name, y.description, y.sort_order, y.created_by, \
         y.created_at, y.updated_at, \
         (SELECT COUNT(*) FROM subjects s WHERE s.category_id = y.id) AS subject_count \
         FROM categories y \
         WHERE (? IS NULL OR y.name LIKE ? OR COALESCE(y.description, '') LIKE ?) \
         ORDER BY y.sort_order, y.name LIMIT ? OFFSET ?",
    )
    .bind(pattern.as_deref())
    .bind(pattern.as_deref())
    .bind(pattern.as_deref())
    .bind(limit + 1)
    .bind(q.offset())
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() > limit as usize;
    let items = rows
        .into_iter()
        .take(limit as usize)
        .map(|r| CategoryDto {
            category: CategoryRow {
                id: r.id,
                slug: r.slug,
                name: r.name,
                description: r.description,
                sort_order: r.sort_order,
                created_by: r.created_by,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
            subject_count: r.subject_count,
        })
        .collect();
    Ok(Json(Page { items, has_more }).into_response())
}

#[derive(Debug, Serialize)]
struct CategoryWithSubjects {
    #[serde(flatten)]
    category: CategoryDto,
    subjects: Vec<SubjectDto>,
    subjects_has_more: bool,
}

async fn get_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(q): axum::extract::Query<PageQuery>,
) -> AppResult<Json<CategoryWithSubjects>> {
    let category = categories_with_counts(&state)
        .await?
        .into_iter()
        .find(|y| y.category.slug == slug)
        .ok_or(AppError::NotFound("category"))?;
    let paged = q.limit.is_some() || q.offset.is_some() || q.search.is_some();
    let (subjects, subjects_has_more) = crate::routes::public::subjects_for_category(
        &state,
        &slug,
        q.search(),
        paged.then(|| q.limit()),
        q.offset(),
    )
    .await?;
    Ok(Json(CategoryWithSubjects {
        category,
        subjects,
        subjects_has_more,
    }))
}

/// A subject as seen in the dashboard and by API clients.
#[derive(Debug, Serialize)]
pub(crate) struct ManagedSubject {
    #[serde(flatten)]
    pub(crate) subject: SubjectDto,
    /// You may add quizzes/reviews to this subject (your category, shared category/subject, or admin).
    can_add_content: bool,
    /// This subject was shared with you on its own.
    shared_with_you: bool,
}

/// A category as seen in the dashboard and by API clients.
#[derive(Debug, Serialize)]
pub(crate) struct ManagedCategory {
    #[serde(flatten)]
    category: CategoryDto,
    owner_id: Option<i64>,
    owner_name: Option<String>,
    /// You own this category (or are an admin): edit it, manage its subjects and sharing.
    can_manage: bool,
    /// The whole category was shared with you.
    shared_with_you: bool,
    /// You may add content to at least one of its subjects.
    can_add_content: bool,
    pub(crate) subjects: Vec<ManagedSubject>,
}

/// Every category with its subjects, as `GET /admin/categories` returns them. With any of
/// `search` (name, slug or description), `offset`, `limit` it returns one `Page` (max 20) instead.
async fn list_managed_categories(
    RequireEditor(user, _): ContentManager,
    State(state): State<AppState>,
    Query(q): Query<AdminListQuery>,
) -> AppResult<Response> {
    let categories = managed_categories(&state, Actor::of(&user)).await?;
    if !q.is_paged() {
        return Ok(Json(categories).into_response());
    }
    let page = q.page();
    let search = page.search().map(|s| s.to_lowercase());
    let matches = categories.into_iter().filter(|c| {
        search.as_deref().is_none_or(|s| {
            let c = &c.category.category;
            contains_ci(&c.name, s)
                || contains_ci(&c.slug, s)
                || c.description.as_deref().is_some_and(|d| contains_ci(d, s))
        })
    });
    Ok(Json(paginate(matches, &page)).into_response())
}

/// Case-insensitive substring match (`needle` already lowercased), like SQL `LIKE '%…%'`.
pub(crate) fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

/// One page (`offset`, `limit`) of an already filtered list.
pub(crate) fn paginate<T>(items: impl Iterator<Item = T>, page: &PageQuery) -> Page<T> {
    let limit = page.limit() as usize;
    let mut items: Vec<T> = items.skip(page.offset() as usize).take(limit + 1).collect();
    let has_more = items.len() > limit;
    items.truncate(limit);
    Page { items, has_more }
}

/// Every category (with owner, sharing and its subjects) as seen by `actor`.
pub(crate) async fn managed_categories(
    state: &AppState,
    actor: Actor,
) -> AppResult<Vec<ManagedCategory>> {
    let shared = access::shared_with(&state.db, actor.id).await?;
    let owners: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT y.id, u.display_name FROM categories y LEFT JOIN users u ON u.id = y.created_by",
    )
    .fetch_all(&state.db)
    .await?;
    let subjects = all_subjects(state).await?;
    let categories = categories_with_counts(state).await?;
    Ok(categories
        .into_iter()
        .map(|category| {
            let owner_id = category.category.created_by;
            let owner_name = owners
                .iter()
                .find(|(id, _)| *id == category.category.id)
                .and_then(|(_, n)| n.clone());
            let can_manage = access::can_manage_category(actor, owner_id);
            let category_shared = shared.categories.contains(&category.category.id);
            let subjects: Vec<ManagedSubject> = subjects
                .iter()
                .filter(|s| s.category_id == category.category.id)
                .map(|s| {
                    let subject_shared = shared.subjects.contains(&s.id);
                    ManagedSubject {
                        can_add_content: can_manage || category_shared || subject_shared,
                        shared_with_you: subject_shared,
                        subject: s.clone(),
                    }
                })
                .collect();
            ManagedCategory {
                can_add_content: can_manage
                    || category_shared
                    || subjects.iter().any(|s| s.can_add_content),
                can_manage,
                shared_with_you: category_shared,
                owner_id,
                owner_name,
                subjects,
                category,
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct CategoryInput {
    name: String,
    /// Optional; derived from the name when empty.
    slug: Option<String>,
    description: Option<String>,
    #[serde(default)]
    sort_order: i32,
}

impl CategoryInput {
    fn normalize(mut self) -> AppResult<(String, String, Option<String>, i32)> {
        self.name = self.name.trim().to_string();
        if self.name.is_empty() || self.name.chars().count() > 100 {
            return Err(AppError::Validation(
                "name: is required (max 100 characters)".into(),
            ));
        }
        let slug = clean(self.slug).unwrap_or_else(|| slugify(&self.name));
        if !is_slug(&slug) {
            return Err(AppError::Validation(
                "slug: use lowercase letters, digits and dashes (e.g. i4)".into(),
            ));
        }
        let description = clean(self.description);
        check_len("description", &description, 2000)?;
        Ok((self.name, slug, description, self.sort_order))
    }
}

async fn load(state: &AppState, id: i64) -> AppResult<CategoryDto> {
    categories_with_counts(state)
        .await?
        .into_iter()
        .find(|y| y.category.id == id)
        .ok_or(AppError::NotFound("category"))
}

/// Any editor (manager or admin) can create a category; they become its owner.
async fn create_category(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    AppJson(body): AppJson<CategoryInput>,
) -> AppResult<(StatusCode, Json<CategoryDto>)> {
    let (name, slug, description, sort_order) = body.normalize()?;
    let res = sqlx::query(
        "INSERT INTO categories (slug, name, description, sort_order, created_by) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&slug)
    .bind(&name)
    .bind(&description)
    .bind(sort_order)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(load(&state, res.last_insert_id() as i64).await?),
    ))
}

async fn update_category(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<CategoryInput>,
) -> AppResult<Json<CategoryDto>> {
    access::require_category(&state.db, Actor::of(&user), id, "change this category").await?;
    let (name, slug, description, sort_order) = body.normalize()?;
    sqlx::query(
        "UPDATE categories SET slug = ?, name = ?, description = ?, sort_order = ? WHERE id = ?",
    )
    .bind(&slug)
    .bind(&name)
    .bind(&description)
    .bind(sort_order)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(Json(load(&state, id).await?))
}

async fn delete_category(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    access::require_category(&state.db, Actor::of(&user), id, "delete this category").await?;
    let category = load(&state, id).await?;
    if category.subject_count > 0 {
        return Err(AppError::Conflict(
            "move or delete this category's subjects first".into(),
        ));
    }
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Used by subject create/update to validate `category_id`.
pub async fn ensure_category(state: &AppState, category_id: i64) -> AppResult<()> {
    let found: Option<CategoryRow> = sqlx::query_as(concat!(
        "SELECT ",
        category_columns!(),
        " FROM categories WHERE id = ?"
    ))
    .bind(category_id)
    .fetch_optional(&state.db)
    .await?;
    found
        .map(|_| ())
        .ok_or_else(|| AppError::Validation("category_id: category not found".into()))
}
