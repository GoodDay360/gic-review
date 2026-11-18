//! Years ("I3", "I4", or any custom name) group subjects. Students pick a year first.
//!
//! - Public: `GET /years`, `GET /years/{slug}` (the year and its subjects)
//! - Editors (session or API key): `GET /admin/years`: every year with its owner, its subjects,
//!   `can_manage` (owner/admin), `shared_with_you` and `can_add_content` (per year and subject)
//! - Dashboard only (never with an API key): `POST /admin/years` (the creator becomes the
//!   owner), `PUT/DELETE /admin/years/{id}` (owner or admin)

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    access::{self, Actor},
    auth::{ContentManager, RequireEditor, RequireEditorSession},
    error::{AppError, AppResult},
    extract::AppJson,
    models::{Page, PageQuery, SubjectDto, YearDto, YearRow, year_columns},
    routes::public::{all_subjects, all_years},
    state::AppState,
    validation::{check_len, clean, is_slug, slugify},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/years", get(list_years))
        .route("/years/{slug}", get(get_year))
        .route("/admin/years", get(list_managed_years).post(create_year))
        .route("/admin/years/{id}", put(update_year).delete(delete_year))
}

async fn years_with_counts(state: &AppState) -> AppResult<Vec<YearDto>> {
    let years = all_years(state).await?;
    let counts: Vec<(i64, i64)> =
        sqlx::query_as("SELECT year_id, COUNT(*) FROM subjects GROUP BY year_id")
            .fetch_all(&state.db)
            .await?;
    Ok(years
        .into_iter()
        .map(|year| {
            let subject_count = counts
                .iter()
                .find(|(id, _)| *id == year.id)
                .map_or(0, |(_, c)| *c);
            YearDto {
                year,
                subject_count,
            }
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct YearListRow {
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

async fn list_years(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<PageQuery>,
) -> AppResult<axum::response::Response> {
    if q.search.is_none() && q.offset.is_none() && q.limit.is_none() {
        return Ok(Json(years_with_counts(&state).await?).into_response());
    }
    let search = q.search();
    let pattern = search.as_ref().map(|s| format!("%{s}%"));
    let limit = q.limit();
    let rows: Vec<YearListRow> = sqlx::query_as(
        "SELECT y.id, y.slug, y.name, y.description, y.sort_order, y.created_by, \
         y.created_at, y.updated_at, \
         (SELECT COUNT(*) FROM subjects s WHERE s.year_id = y.id) AS subject_count \
         FROM years y \
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
        .map(|r| YearDto {
            year: YearRow {
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
struct YearWithSubjects {
    #[serde(flatten)]
    year: YearDto,
    subjects: Vec<SubjectDto>,
    subjects_has_more: bool,
}

async fn get_year(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    axum::extract::Query(q): axum::extract::Query<PageQuery>,
) -> AppResult<Json<YearWithSubjects>> {
    let year = years_with_counts(&state)
        .await?
        .into_iter()
        .find(|y| y.year.slug == slug)
        .ok_or(AppError::NotFound("year"))?;
    let paged = q.limit.is_some() || q.offset.is_some() || q.search.is_some();
    let (subjects, subjects_has_more) = crate::routes::public::subjects_for_year(
        &state,
        &slug,
        q.search(),
        paged.then(|| q.limit()),
        q.offset(),
    )
    .await?;
    Ok(Json(YearWithSubjects {
        year,
        subjects,
        subjects_has_more,
    }))
}

/// A subject as seen in the dashboard and by API clients.
#[derive(Debug, Serialize)]
struct ManagedSubject {
    #[serde(flatten)]
    subject: SubjectDto,
    /// You may add quizzes/reviews to this subject (your year, shared year/subject, or admin).
    can_add_content: bool,
    /// This subject was shared with you on its own.
    shared_with_you: bool,
}

/// A year as seen in the dashboard and by API clients.
#[derive(Debug, Serialize)]
struct ManagedYear {
    #[serde(flatten)]
    year: YearDto,
    owner_id: Option<i64>,
    owner_name: Option<String>,
    /// You own this year (or are an admin): edit it, manage its subjects and sharing.
    can_manage: bool,
    /// The whole year was shared with you.
    shared_with_you: bool,
    /// You may add content to at least one of its subjects.
    can_add_content: bool,
    subjects: Vec<ManagedSubject>,
}

async fn list_managed_years(
    RequireEditor(user, _): ContentManager,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<ManagedYear>>> {
    let actor = Actor::of(&user);
    let shared = access::shared_with(&state.db, actor.id).await?;
    let owners: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT y.id, u.display_name FROM years y LEFT JOIN users u ON u.id = y.created_by",
    )
    .fetch_all(&state.db)
    .await?;
    let subjects = all_subjects(&state).await?;
    let years = years_with_counts(&state).await?;
    Ok(Json(
        years
            .into_iter()
            .map(|year| {
                let owner_id = year.year.created_by;
                let owner_name = owners
                    .iter()
                    .find(|(id, _)| *id == year.year.id)
                    .and_then(|(_, n)| n.clone());
                let can_manage = access::can_manage_year(actor, owner_id);
                let year_shared = shared.years.contains(&year.year.id);
                let subjects: Vec<ManagedSubject> = subjects
                    .iter()
                    .filter(|s| s.year_id == year.year.id)
                    .map(|s| {
                        let subject_shared = shared.subjects.contains(&s.id);
                        ManagedSubject {
                            can_add_content: can_manage || year_shared || subject_shared,
                            shared_with_you: subject_shared,
                            subject: s.clone(),
                        }
                    })
                    .collect();
                ManagedYear {
                    can_add_content: can_manage
                        || year_shared
                        || subjects.iter().any(|s| s.can_add_content),
                    can_manage,
                    shared_with_you: year_shared,
                    owner_id,
                    owner_name,
                    subjects,
                    year,
                }
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
struct YearInput {
    name: String,
    /// Optional; derived from the name when empty.
    slug: Option<String>,
    description: Option<String>,
    #[serde(default)]
    sort_order: i32,
}

impl YearInput {
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

async fn load(state: &AppState, id: i64) -> AppResult<YearDto> {
    years_with_counts(state)
        .await?
        .into_iter()
        .find(|y| y.year.id == id)
        .ok_or(AppError::NotFound("year"))
}

/// Any editor (manager or admin) can create a year; they become its owner.
async fn create_year(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    AppJson(body): AppJson<YearInput>,
) -> AppResult<(StatusCode, Json<YearDto>)> {
    let (name, slug, description, sort_order) = body.normalize()?;
    let res = sqlx::query(
        "INSERT INTO years (slug, name, description, sort_order, created_by) VALUES (?, ?, ?, ?, ?)",
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

async fn update_year(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<YearInput>,
) -> AppResult<Json<YearDto>> {
    access::require_year(&state.db, Actor::of(&user), id, "change this year").await?;
    let (name, slug, description, sort_order) = body.normalize()?;
    sqlx::query(
        "UPDATE years SET slug = ?, name = ?, description = ?, sort_order = ? WHERE id = ?",
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

async fn delete_year(
    RequireEditorSession(user): RequireEditorSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    access::require_year(&state.db, Actor::of(&user), id, "delete this year").await?;
    let year = load(&state, id).await?;
    if year.subject_count > 0 {
        return Err(AppError::Conflict(
            "move or delete this year's subjects first".into(),
        ));
    }
    sqlx::query("DELETE FROM years WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Used by subject create/update to validate `year_id`.
pub async fn ensure_year(state: &AppState, year_id: i64) -> AppResult<()> {
    let found: Option<YearRow> = sqlx::query_as(concat!(
        "SELECT ",
        year_columns!(),
        " FROM years WHERE id = ?"
    ))
    .bind(year_id)
    .fetch_optional(&state.db)
    .await?;
    found
        .map(|_| ())
        .ok_or_else(|| AppError::Validation("year_id: year not found".into()))
}
