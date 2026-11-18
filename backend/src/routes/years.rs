//! Years ("I3", "I4", or any custom name) group subjects. Students pick a year first.
//!
//! Public:  `GET /years`, `GET /years/{slug}` (the year and its subjects)
//! Admin:   `POST /admin/years`, `PUT/DELETE /admin/years/{id}` (dashboard session only)

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};

use crate::{
    auth::RequireAdmin,
    error::{AppError, AppResult},
    extract::AppJson,
    models::{SubjectDto, YearDto, YearRow, year_columns},
    routes::public::{all_subjects, all_years},
    state::AppState,
    validation::{check_len, clean, is_slug, slugify},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/years", get(list_years))
        .route("/years/{slug}", get(get_year))
        .route("/admin/years", post(create_year))
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

async fn list_years(State(state): State<AppState>) -> AppResult<Json<Vec<YearDto>>> {
    Ok(Json(years_with_counts(&state).await?))
}

#[derive(Debug, Serialize)]
struct YearWithSubjects {
    #[serde(flatten)]
    year: YearDto,
    subjects: Vec<SubjectDto>,
}

async fn get_year(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<YearWithSubjects>> {
    let year = years_with_counts(&state)
        .await?
        .into_iter()
        .find(|y| y.year.slug == slug)
        .ok_or(AppError::NotFound("year"))?;
    let subjects = all_subjects(&state)
        .await?
        .into_iter()
        .filter(|s| s.year_id == year.year.id)
        .collect();
    Ok(Json(YearWithSubjects { year, subjects }))
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

async fn create_year(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    AppJson(body): AppJson<YearInput>,
) -> AppResult<(StatusCode, Json<YearDto>)> {
    let (name, slug, description, sort_order) = body.normalize()?;
    let res =
        sqlx::query("INSERT INTO years (slug, name, description, sort_order) VALUES (?, ?, ?, ?)")
            .bind(&slug)
            .bind(&name)
            .bind(&description)
            .bind(sort_order)
            .execute(&state.db)
            .await?;
    Ok((
        StatusCode::CREATED,
        Json(load(&state, res.last_insert_id() as i64).await?),
    ))
}

async fn update_year(
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    AppJson(body): AppJson<YearInput>,
) -> AppResult<Json<YearDto>> {
    load(&state, id).await?;
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
    _admin: RequireAdmin,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
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
