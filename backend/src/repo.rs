//! Shared read queries for quizzes (used by public and admin routes).
//! All queries are parameterized; nothing is built by string concatenation of user input.

use std::collections::HashMap;

use sqlx::{MySqlPool, QueryBuilder};

use crate::{
    error::AppResult,
    models::{
        OptionRow, Page, QuestionDto, QuestionRow, QuizDetail, QuizRow, QuizSummary, SubjectRow,
        option_columns, question_columns, quiz_columns, subject_columns,
    },
    storage::Storage,
};

/// Load questions (with options) for every quiz matching the filter, grouped by quiz id.
/// `subject_id = None` means all subjects, `quiz_id = Some` narrows to one quiz.
async fn load_questions(
    db: &MySqlPool,
    storage: &Storage,
    subject_id: Option<i64>,
    quiz_id: Option<i64>,
    quiz_ids: Option<&[i64]>,
) -> AppResult<HashMap<i64, Vec<QuestionDto>>> {
    if quiz_ids.is_some_and(|ids| ids.is_empty()) {
        return Ok(HashMap::new());
    }

    let mut question_query = QueryBuilder::<sqlx::MySql>::new(concat!(
        "SELECT ",
        question_columns!(),
        " FROM questions q JOIN quizzes z ON z.id = q.quiz_id WHERE 1 = 1"
    ));
    if let Some(subject_id) = subject_id {
        question_query
            .push(" AND z.subject_id = ")
            .push_bind(subject_id);
    }
    if let Some(quiz_id) = quiz_id {
        question_query.push(" AND z.id = ").push_bind(quiz_id);
    }
    if let Some(ids) = quiz_ids {
        question_query.push(" AND z.id IN (");
        {
            let mut separated = question_query.separated(", ");
            for id in ids {
                separated.push_bind(*id);
            }
        }
        question_query.push(")");
    }
    question_query.push(" ORDER BY q.quiz_id, q.position, q.id");
    let questions: Vec<QuestionRow> = question_query.build_query_as().fetch_all(db).await?;

    let mut option_query = QueryBuilder::<sqlx::MySql>::new(concat!(
        "SELECT ",
        option_columns!(),
        " FROM question_options o \
         JOIN questions q ON q.id = o.question_id JOIN quizzes z ON z.id = q.quiz_id \
         WHERE 1 = 1"
    ));
    if let Some(subject_id) = subject_id {
        option_query
            .push(" AND z.subject_id = ")
            .push_bind(subject_id);
    }
    if let Some(quiz_id) = quiz_id {
        option_query.push(" AND z.id = ").push_bind(quiz_id);
    }
    if let Some(ids) = quiz_ids {
        option_query.push(" AND z.id IN (");
        {
            let mut separated = option_query.separated(", ");
            for id in ids {
                separated.push_bind(*id);
            }
        }
        option_query.push(")");
    }
    option_query.push(" ORDER BY o.question_id, o.position, o.id");
    let options: Vec<OptionRow> = option_query.build_query_as().fetch_all(db).await?;

    let mut options_by_question: HashMap<i64, Vec<OptionRow>> = HashMap::new();
    for o in options {
        options_by_question
            .entry(o.question_id)
            .or_default()
            .push(o);
    }

    let mut by_quiz: HashMap<i64, Vec<QuestionDto>> = HashMap::new();
    for q in questions {
        let opts = options_by_question.remove(&q.id).unwrap_or_default();
        let quiz_id = q.quiz_id;
        by_quiz
            .entry(quiz_id)
            .or_default()
            .push(QuestionDto::build(q, opts, storage));
    }
    Ok(by_quiz)
}

/// Quiz cards for a subject (or all subjects), optionally only published ones.
pub async fn quiz_summaries(
    db: &MySqlPool,
    storage: &Storage,
    subject_id: Option<i64>,
    published_only: bool,
) -> AppResult<Vec<QuizSummary>> {
    let quizzes: Vec<QuizRow> = sqlx::query_as(concat!(
        "SELECT ",
        quiz_columns!(),
        " FROM quizzes z \
         WHERE (? IS NULL OR z.subject_id = ?) AND (? = FALSE OR z.is_published = TRUE) \
         ORDER BY z.created_at DESC, z.id DESC"
    ))
    .bind(subject_id)
    .bind(subject_id)
    .bind(published_only)
    .fetch_all(db)
    .await?;

    let mut questions = load_questions(db, storage, subject_id, None, None).await?;
    Ok(quizzes
        .into_iter()
        .map(|z| {
            let qs = questions.remove(&z.id).unwrap_or_default();
            QuizSummary::build(z, &qs, storage)
        })
        .collect())
}

pub async fn quiz_summaries_page(
    db: &MySqlPool,
    storage: &Storage,
    subject_id: i64,
    search: Option<String>,
    offset: i64,
    limit: i64,
) -> AppResult<Page<QuizSummary>> {
    let mut query = QueryBuilder::<sqlx::MySql>::new(concat!(
        "SELECT ",
        quiz_columns!(),
        " FROM quizzes z WHERE z.subject_id = "
    ));
    query
        .push_bind(subject_id)
        .push(" AND z.is_published = TRUE");
    if let Some(search) = search {
        let pattern = format!("%{search}%");
        query
            .push(" AND (z.title LIKE ")
            .push_bind(pattern.clone())
            .push(" OR COALESCE(z.description, '') LIKE ")
            .push_bind(pattern)
            .push(")");
    }
    query
        .push(" ORDER BY z.created_at DESC, z.id DESC LIMIT ")
        .push_bind(limit + 1)
        .push(" OFFSET ")
        .push_bind(offset);
    let mut quizzes: Vec<QuizRow> = query.build_query_as().fetch_all(db).await?;
    let has_more = quizzes.len() > limit as usize;
    quizzes.truncate(limit as usize);
    let quiz_ids: Vec<i64> = quizzes.iter().map(|quiz| quiz.id).collect();
    let mut questions = load_questions(db, storage, None, None, Some(&quiz_ids)).await?;
    let items = quizzes
        .into_iter()
        .map(|quiz| {
            let qs = questions.remove(&quiz.id).unwrap_or_default();
            QuizSummary::build(quiz, &qs, storage)
        })
        .collect();
    Ok(Page { items, has_more })
}

/// One quiz with all questions and options, or `None` if it does not exist.
pub async fn quiz_detail(
    db: &MySqlPool,
    storage: &Storage,
    quiz_id: i64,
) -> AppResult<Option<QuizDetail>> {
    let quiz: Option<QuizRow> = sqlx::query_as(concat!(
        "SELECT ",
        quiz_columns!(),
        " FROM quizzes z WHERE z.id = ?"
    ))
    .bind(quiz_id)
    .fetch_optional(db)
    .await?;
    let Some(quiz) = quiz else {
        return Ok(None);
    };
    let subject: SubjectRow = sqlx::query_as(concat!(
        "SELECT ",
        subject_columns!(),
        " FROM subjects WHERE id = ?"
    ))
    .bind(quiz.subject_id)
    .fetch_one(db)
    .await?;

    let questions = load_questions(db, storage, None, Some(quiz_id), None)
        .await?
        .remove(&quiz_id)
        .unwrap_or_default();
    let summary = QuizSummary::build(quiz, &questions, storage);
    Ok(Some(QuizDetail {
        summary,
        subject_slug: subject.slug,
        subject_name: subject.name,
        questions,
    }))
}

pub async fn subject_by_slug(db: &MySqlPool, slug: &str) -> AppResult<Option<SubjectRow>> {
    Ok(sqlx::query_as(concat!(
        "SELECT ",
        subject_columns!(),
        " FROM subjects WHERE slug = ?"
    ))
    .bind(slug)
    .fetch_optional(db)
    .await?)
}
