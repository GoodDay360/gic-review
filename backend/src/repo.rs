//! Shared read queries for quizzes (used by public and admin routes).
//! All queries are parameterized; nothing is built by string concatenation of user input.

use std::collections::HashMap;

use sqlx::MySqlPool;

use crate::{
    error::AppResult,
    models::{
        OptionRow, QuestionDto, QuestionRow, QuizDetail, QuizRow, QuizSummary, SubjectRow,
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
) -> AppResult<HashMap<i64, Vec<QuestionDto>>> {
    let questions: Vec<QuestionRow> = sqlx::query_as(concat!(
        "SELECT ",
        question_columns!(),
        " FROM questions q JOIN quizzes z ON z.id = q.quiz_id \
         WHERE (? IS NULL OR z.subject_id = ?) AND (? IS NULL OR z.id = ?) \
         ORDER BY q.quiz_id, q.position, q.id"
    ))
    .bind(subject_id)
    .bind(subject_id)
    .bind(quiz_id)
    .bind(quiz_id)
    .fetch_all(db)
    .await?;

    let options: Vec<OptionRow> = sqlx::query_as(concat!(
        "SELECT ",
        option_columns!(),
        " FROM question_options o \
         JOIN questions q ON q.id = o.question_id JOIN quizzes z ON z.id = q.quiz_id \
         WHERE (? IS NULL OR z.subject_id = ?) AND (? IS NULL OR z.id = ?) \
         ORDER BY o.question_id, o.position, o.id"
    ))
    .bind(subject_id)
    .bind(subject_id)
    .bind(quiz_id)
    .bind(quiz_id)
    .fetch_all(db)
    .await?;

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

    let mut questions = load_questions(db, storage, subject_id, None).await?;
    Ok(quizzes
        .into_iter()
        .map(|z| {
            let qs = questions.remove(&z.id).unwrap_or_default();
            QuizSummary::build(z, &qs, storage)
        })
        .collect())
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

    let questions = load_questions(db, storage, None, Some(quiz_id))
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
