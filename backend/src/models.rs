//! Database rows and API response types.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::{quiz_modes::QuizMode, storage::Storage};

// ---------- Categories ----------

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct CategoryRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    /// Owner (creator) of the category; not exposed publicly.
    #[serde(skip)]
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! category_columns {
    () => {
        "id, slug, name, description, sort_order, created_by, created_at, updated_at"
    };
}
pub(crate) use category_columns;

#[derive(Debug, Serialize)]
pub struct CategoryDto {
    #[serde(flatten)]
    pub category: CategoryRow,
    /// Number of subjects in this category.
    pub subject_count: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct PageQuery {
    pub search: Option<String>,
    pub slug: Option<String>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

impl PageQuery {
    pub fn search(&self) -> Option<String> {
        self.search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 20)
    }
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

// ---------- Subjects ----------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubjectRow {
    pub id: i64,
    pub category_id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub sort_order: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! subject_columns {
    () => {
        "id, category_id, slug, name, description, color, sort_order, created_at, updated_at"
    };
}
pub(crate) use subject_columns;

#[derive(Debug, Clone, Serialize)]
pub struct SubjectDto {
    pub id: i64,
    pub category_id: i64,
    pub category_slug: String,
    pub category_name: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub sort_order: i32,
    pub has_review: bool,
    pub quiz_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl SubjectDto {
    pub fn from_row(
        r: SubjectRow,
        category: &CategoryRow,
        has_review: bool,
        quiz_count: i64,
    ) -> Self {
        Self {
            id: r.id,
            category_id: r.category_id,
            category_slug: category.slug.clone(),
            category_name: category.name.clone(),
            slug: r.slug,
            name: r.name,
            description: r.description,
            color: r.color,
            sort_order: r.sort_order,
            has_review,
            quiz_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------- Interactive reviews ----------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReviewRow {
    pub id: i64,
    pub subject_id: i64,
    pub title: String,
    pub slug: String,
    pub r2_key: String,
    pub sort_order: i32,
    pub is_published: bool,
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! review_columns {
    () => {
        "id, subject_id, title, slug, r2_key, sort_order, is_published, created_by, created_at, updated_at"
    };
}
pub(crate) use review_columns;

#[derive(Debug, Serialize)]
pub struct ReviewDto {
    pub id: i64,
    pub subject_id: i64,
    pub title: String,
    pub slug: String,
    pub r2_key: String,
    pub url: String,
    pub sort_order: i32,
    pub is_published: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ReviewDto {
    pub fn from_row(r: ReviewRow, storage: &Storage) -> Self {
        // `?v=` busts CDN/browser caches when a review is replaced under the same key.
        let url = format!(
            "{}?v={}",
            storage.public_url(&r.r2_key),
            r.updated_at.and_utc().timestamp()
        );
        Self {
            id: r.id,
            subject_id: r.subject_id,
            title: r.title,
            slug: r.slug,
            r2_key: r.r2_key,
            url,
            sort_order: r.sort_order,
            is_published: r.is_published,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------- Quizzes ----------

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuizRow {
    pub id: i64,
    pub subject_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_key: Option<String>,
    pub is_published: bool,
    pub created_by: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! quiz_columns {
    () => {
        "z.id, z.subject_id, z.title, z.description, z.cover_image_key, \
     z.is_published, z.created_by, z.created_at, z.updated_at"
    };
}
pub(crate) use quiz_columns;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuestionRow {
    pub id: i64,
    pub quiz_id: i64,
    pub position: i32,
    pub prompt: Option<String>,
    pub image_key: Option<String>,
    pub image_alt: Option<String>,
    pub answer_text: Option<String>,
    pub answer_image_key: Option<String>,
    pub answer_image_alt: Option<String>,
    pub explanation: Option<String>,
    pub explanation_image_key: Option<String>,
    pub multi_select: bool,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! question_columns {
    () => {
        "q.id, q.quiz_id, q.position, q.prompt, q.image_key, \
     q.image_alt, q.answer_text, q.answer_image_key, q.answer_image_alt, q.explanation, \
     q.explanation_image_key, q.multi_select"
    };
}
pub(crate) use question_columns;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OptionRow {
    pub id: i64,
    pub question_id: i64,
    pub position: i32,
    pub text: Option<String>,
    pub image_key: Option<String>,
    pub image_alt: Option<String>,
    pub is_correct: bool,
}

/// Column list for SELECTs (a macro so queries stay `&'static str` literals).
macro_rules! option_columns {
    () => {
        "o.id, o.question_id, o.position, o.text, o.image_key, o.image_alt, o.is_correct"
    };
}
pub(crate) use option_columns;

#[derive(Debug, Clone, Serialize)]
pub struct OptionDto {
    pub id: i64,
    pub position: i32,
    pub text: Option<String>,
    pub image_key: Option<String>,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuestionDto {
    pub id: i64,
    pub position: i32,
    pub prompt: Option<String>,
    pub image_key: Option<String>,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    pub answer_text: Option<String>,
    pub answer_image_key: Option<String>,
    pub answer_image_url: Option<String>,
    pub answer_image_alt: Option<String>,
    pub explanation: Option<String>,
    pub explanation_image_key: Option<String>,
    pub explanation_image_url: Option<String>,
    pub multi_select: bool,
    pub options: Vec<OptionDto>,
}

impl QuestionDto {
    pub fn build(q: QuestionRow, options: Vec<OptionRow>, storage: &Storage) -> Self {
        let options = options
            .into_iter()
            .map(|o| OptionDto {
                id: o.id,
                position: o.position,
                text: o.text,
                image_url: storage.public_url_opt(o.image_key.as_deref()),
                image_key: o.image_key,
                image_alt: o.image_alt,
                is_correct: o.is_correct,
            })
            .collect();
        Self {
            id: q.id,
            position: q.position,
            prompt: q.prompt,
            image_url: storage.public_url_opt(q.image_key.as_deref()),
            image_key: q.image_key,
            image_alt: q.image_alt,
            answer_text: q.answer_text,
            answer_image_url: storage.public_url_opt(q.answer_image_key.as_deref()),
            answer_image_key: q.answer_image_key,
            answer_image_alt: q.answer_image_alt,
            explanation: q.explanation,
            explanation_image_url: storage.public_url_opt(q.explanation_image_key.as_deref()),
            explanation_image_key: q.explanation_image_key,
            multi_select: q.multi_select,
            options,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QuizSummary {
    pub id: i64,
    pub subject_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_key: Option<String>,
    pub cover_image_url: Option<String>,
    pub is_published: bool,
    pub question_count: usize,
    pub available_modes: Vec<QuizMode>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct QuizDetail {
    #[serde(flatten)]
    pub summary: QuizSummary,
    pub subject_slug: String,
    pub subject_name: String,
    /// Subject slugs are unique per category: links need both.
    pub category_slug: String,
    pub questions: Vec<QuestionDto>,
}

impl QuizSummary {
    pub fn build(z: QuizRow, questions: &[QuestionDto], storage: &Storage) -> Self {
        Self {
            id: z.id,
            subject_id: z.subject_id,
            title: z.title,
            description: z.description,
            cover_image_url: storage.public_url_opt(z.cover_image_key.as_deref()),
            cover_image_key: z.cover_image_key,
            is_published: z.is_published,
            question_count: questions.len(),
            available_modes: crate::quiz_modes::available_modes(questions),
            created_at: z.created_at,
            updated_at: z.updated_at,
        }
    }
}

/// All R2 keys referenced by a question and its options.
pub fn question_image_keys(q: &QuestionDto) -> Vec<String> {
    let mut keys: Vec<String> = [&q.image_key, &q.answer_image_key, &q.explanation_image_key]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    keys.extend(q.options.iter().filter_map(|o| o.image_key.clone()));
    keys
}
