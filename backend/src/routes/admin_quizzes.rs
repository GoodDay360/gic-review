//! Manager quiz management: quizzes, questions, options, reorder, publish.
//! A manager can only manage quizzes they own or that were shared with them (see `access`).

use std::{collections::HashSet, ops::Deref};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Transaction};

use crate::{
    access::{self, AccessInfo, Content},
    auth::{QuizManager, RequireManager},
    error::{AppError, AppResult},
    models::{QuizDetail, QuizSummary, question_image_keys},
    repo,
    state::AppState,
    validation::{check_image_key, check_len, clean},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/quizzes", get(list_quizzes).post(create_quiz))
        .route(
            "/admin/quizzes/{id}",
            get(get_quiz).put(update_quiz).delete(delete_quiz),
        )
        .route("/admin/quizzes/{id}/publish", patch(set_published))
        .route("/admin/quizzes/{id}/questions", post(create_question))
        .route(
            "/admin/quizzes/{id}/questions/reorder",
            put(reorder_questions),
        )
        .route(
            "/admin/quizzes/{id}/questions/{qid}",
            put(update_question).delete(delete_question),
        )
}

// ---------- Inputs ----------

#[derive(Debug, Deserialize)]
pub struct QuizInput {
    pub subject_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_key: Option<String>,
}

impl QuizInput {
    fn normalize(mut self) -> AppResult<Self> {
        self.title = self.title.trim().to_string();
        self.description = clean(self.description);
        self.cover_image_key = clean(self.cover_image_key);
        if self.title.is_empty() || self.title.chars().count() > 200 {
            return Err(AppError::Validation(
                "title: is required (max 200 characters)".into(),
            ));
        }
        check_len("description", &self.description, 5000)?;
        check_image_key("cover_image_key", &self.cover_image_key)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionInput {
    pub text: Option<String>,
    pub image_key: Option<String>,
    pub image_alt: Option<String>,
    #[serde(default)]
    pub is_correct: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuestionInput {
    pub prompt: Option<String>,
    pub image_key: Option<String>,
    pub image_alt: Option<String>,
    pub answer_text: Option<String>,
    pub answer_image_key: Option<String>,
    pub answer_image_alt: Option<String>,
    pub explanation: Option<String>,
    pub explanation_image_key: Option<String>,
    #[serde(default)]
    pub multi_select: bool,
    #[serde(default)]
    pub options: Vec<OptionInput>,
}

pub const MAX_OPTIONS: usize = 12;

impl QuestionInput {
    /// Trim strings, drop empties, and enforce the content rules.
    /// `label` prefixes error messages (e.g. "questions[3]").
    pub fn normalize(mut self, label: &str) -> AppResult<Self> {
        self.prompt = clean(self.prompt);
        self.image_key = clean(self.image_key);
        self.image_alt = clean(self.image_alt);
        self.answer_text = clean(self.answer_text);
        self.answer_image_key = clean(self.answer_image_key);
        self.answer_image_alt = clean(self.answer_image_alt);
        self.explanation = clean(self.explanation);
        self.explanation_image_key = clean(self.explanation_image_key);

        let v = |e: AppError| match e {
            AppError::Validation(m) => AppError::Validation(format!("{label}.{m}")),
            other => other,
        };

        if self.prompt.is_none() && self.image_key.is_none() {
            return Err(AppError::Validation(format!(
                "{label}: a question needs a prompt or an image (or both)"
            )));
        }
        check_len("prompt", &self.prompt, 5000).map_err(v)?;
        check_len("answer_text", &self.answer_text, 5000).map_err(v)?;
        check_len("explanation", &self.explanation, 10000).map_err(v)?;
        check_len("image_alt", &self.image_alt, 255).map_err(v)?;
        check_len("answer_image_alt", &self.answer_image_alt, 255).map_err(v)?;
        check_image_key("image_key", &self.image_key).map_err(v)?;
        check_image_key("answer_image_key", &self.answer_image_key).map_err(v)?;
        check_image_key("explanation_image_key", &self.explanation_image_key).map_err(v)?;

        if self.options.len() > MAX_OPTIONS {
            return Err(AppError::Validation(format!(
                "{label}: at most {MAX_OPTIONS} options"
            )));
        }
        let mut options = Vec::with_capacity(self.options.len());
        for (i, mut o) in self.options.into_iter().enumerate() {
            o.text = clean(o.text);
            o.image_key = clean(o.image_key);
            o.image_alt = clean(o.image_alt);
            let ol = format!("{label}.options[{i}]");
            if o.text.is_none() && o.image_key.is_none() {
                return Err(AppError::Validation(format!(
                    "{ol}: an option needs text or an image (or both)"
                )));
            }
            let ov = |e: AppError| match e {
                AppError::Validation(m) => AppError::Validation(format!("{ol}.{m}")),
                other => other,
            };
            check_len("text", &o.text, 2000).map_err(ov)?;
            check_len("image_alt", &o.image_alt, 255).map_err(ov)?;
            check_image_key("image_key", &o.image_key).map_err(ov)?;
            options.push(o);
        }
        let correct = options.iter().filter(|o| o.is_correct).count();
        if !self.multi_select && correct > 1 {
            return Err(AppError::Validation(format!(
                "{label}: {correct} options are marked correct; set multi_select to true"
            )));
        }
        self.options = options;
        Ok(self)
    }

    pub fn image_keys(&self) -> HashSet<String> {
        let mut keys: HashSet<String> = [
            &self.image_key,
            &self.answer_image_key,
            &self.explanation_image_key,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
        keys.extend(self.options.iter().filter_map(|o| o.image_key.clone()));
        keys
    }
}

/// Insert a (normalized) question and its options inside a transaction. Returns the new id.
pub async fn insert_question(
    tx: &mut Transaction<'_, MySql>,
    quiz_id: i64,
    position: i32,
    q: &QuestionInput,
) -> AppResult<i64> {
    let res = sqlx::query(
        "INSERT INTO questions (quiz_id, position, prompt, image_key, image_alt, answer_text, \
         answer_image_key, answer_image_alt, explanation, explanation_image_key, multi_select) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(quiz_id)
    .bind(position)
    .bind(&q.prompt)
    .bind(&q.image_key)
    .bind(&q.image_alt)
    .bind(&q.answer_text)
    .bind(&q.answer_image_key)
    .bind(&q.answer_image_alt)
    .bind(&q.explanation)
    .bind(&q.explanation_image_key)
    .bind(q.multi_select)
    .execute(&mut **tx)
    .await?;
    let question_id = res.last_insert_id() as i64;
    insert_options(tx, question_id, &q.options).await?;
    Ok(question_id)
}

async fn insert_options(
    tx: &mut Transaction<'_, MySql>,
    question_id: i64,
    options: &[OptionInput],
) -> AppResult<()> {
    for (i, o) in options.iter().enumerate() {
        sqlx::query(
            "INSERT INTO question_options (question_id, position, text, image_key, image_alt, is_correct) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(question_id)
        .bind(i as i32)
        .bind(&o.text)
        .bind(&o.image_key)
        .bind(&o.image_alt)
        .bind(o.is_correct)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn ensure_subject(state: &AppState, subject_id: i64) -> AppResult<()> {
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM subjects WHERE id = ?")
        .bind(subject_id)
        .fetch_optional(&state.db)
        .await?;
    exists
        .map(|_| ())
        .ok_or(AppError::Validation("subject_id: subject not found".into()))
}

/// A quiz as seen by a manager: the full quiz plus who owns it and whether it is shared.
#[derive(Debug, Serialize)]
pub struct ManagedQuiz {
    #[serde(flatten)]
    pub detail: QuizDetail,
    #[serde(flatten)]
    pub access: AccessInfo,
}

impl Deref for ManagedQuiz {
    type Target = QuizDetail;
    fn deref(&self) -> &QuizDetail {
        &self.detail
    }
}

/// A quiz card in the manager's list.
#[derive(Debug, Serialize)]
pub struct ManagedQuizSummary {
    #[serde(flatten)]
    pub summary: QuizSummary,
    #[serde(flatten)]
    pub access: AccessInfo,
}

/// Load a quiz the manager may manage (owner or shared), else 404.
pub async fn load_quiz(state: &AppState, id: i64, user_id: i64) -> AppResult<ManagedQuiz> {
    let access = access::require(&state.db, Content::Quiz, id, user_id).await?;
    let detail = repo::quiz_detail(&state.db, &state.storage, id)
        .await?
        .ok_or(AppError::NotFound("quiz"))?;
    Ok(ManagedQuiz { detail, access })
}

// ---------- Quiz handlers ----------

#[derive(Debug, Deserialize)]
struct ListQuery {
    subject_id: Option<i64>,
}

/// Quizzes the manager owns or that were shared with them (drafts included).
async fn list_quizzes(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<ManagedQuizSummary>>> {
    let mut access = access::accessible(&state.db, Content::Quiz, user.id).await?;
    let quizzes = repo::quiz_summaries(&state.db, &state.storage, q.subject_id, false).await?;
    Ok(Json(
        quizzes
            .into_iter()
            .filter_map(|summary| {
                access
                    .remove(&summary.id)
                    .map(|access| ManagedQuizSummary { summary, access })
            })
            .collect(),
    ))
}

async fn get_quiz(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ManagedQuiz>> {
    Ok(Json(load_quiz(&state, id, user.id).await?))
}

async fn create_quiz(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Json(body): Json<QuizInput>,
) -> AppResult<(StatusCode, Json<ManagedQuiz>)> {
    let body = body.normalize()?;
    ensure_subject(&state, body.subject_id).await?;
    // New quizzes always start unpublished.
    let res = sqlx::query(
        "INSERT INTO quizzes (subject_id, title, description, cover_image_key, is_published, created_by) \
         VALUES (?, ?, ?, ?, FALSE, ?)",
    )
    .bind(body.subject_id)
    .bind(&body.title)
    .bind(&body.description)
    .bind(&body.cover_image_key)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    let id = res.last_insert_id() as i64;
    Ok((
        StatusCode::CREATED,
        Json(load_quiz(&state, id, user.id).await?),
    ))
}

async fn update_quiz(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<QuizInput>,
) -> AppResult<Json<ManagedQuiz>> {
    let body = body.normalize()?;
    let existing = load_quiz(&state, id, user.id).await?;
    ensure_subject(&state, body.subject_id).await?;
    sqlx::query(
        "UPDATE quizzes SET subject_id = ?, title = ?, description = ?, cover_image_key = ? WHERE id = ?",
    )
    .bind(body.subject_id)
    .bind(&body.title)
    .bind(&body.description)
    .bind(&body.cover_image_key)
    .bind(id)
    .execute(&state.db)
    .await?;

    if let Some(old) = existing.summary.cover_image_key.clone()
        && Some(&old) != body.cover_image_key.as_ref()
    {
        state.storage.delete_many_best_effort([old]).await;
    }
    Ok(Json(load_quiz(&state, id, user.id).await?))
}

async fn delete_quiz(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let quiz = load_quiz(&state, id, user.id).await?;
    quiz.access.require_owner("delete this quiz")?;
    let mut keys: Vec<String> = quiz
        .questions
        .iter()
        .flat_map(question_image_keys)
        .collect();
    keys.extend(quiz.summary.cover_image_key.clone());

    // Questions and options are removed by ON DELETE CASCADE.
    sqlx::query("DELETE FROM quizzes WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    state.storage.delete_many_best_effort(keys).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct PublishInput {
    is_published: bool,
}

async fn set_published(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<PublishInput>,
) -> AppResult<Json<ManagedQuiz>> {
    let quiz = load_quiz(&state, id, user.id).await?;
    if body.is_published && quiz.summary.available_modes.is_empty() {
        return Err(AppError::Validation(
            "this quiz cannot be published yet: no quiz mode is available. Add questions with \
             options (QCM) or answers (Flashcard) first"
                .into(),
        ));
    }
    sqlx::query("UPDATE quizzes SET is_published = ? WHERE id = ?")
        .bind(body.is_published)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(load_quiz(&state, id, user.id).await?))
}

// ---------- Question handlers ----------

async fn create_question(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<QuestionInput>,
) -> AppResult<(StatusCode, Json<ManagedQuiz>)> {
    let body = body.normalize("question")?;
    let quiz = load_quiz(&state, id, user.id).await?;
    let next_position = quiz
        .questions
        .iter()
        .map(|q| q.position + 1)
        .max()
        .unwrap_or(0);

    let mut tx = state.db.begin().await?;
    insert_question(&mut tx, id, next_position, &body).await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(load_quiz(&state, id, user.id).await?),
    ))
}

async fn update_question(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path((id, qid)): Path<(i64, i64)>,
    Json(body): Json<QuestionInput>,
) -> AppResult<Json<ManagedQuiz>> {
    let body = body.normalize("question")?;
    let quiz = load_quiz(&state, id, user.id).await?;
    let existing = quiz
        .questions
        .iter()
        .find(|q| q.id == qid)
        .ok_or(AppError::NotFound("question"))?;
    let old_keys: HashSet<String> = question_image_keys(existing).into_iter().collect();

    let mut tx = state.db.begin().await?;
    sqlx::query(
        "UPDATE questions SET prompt = ?, image_key = ?, image_alt = ?, answer_text = ?, \
         answer_image_key = ?, answer_image_alt = ?, explanation = ?, explanation_image_key = ?, \
         multi_select = ? WHERE id = ? AND quiz_id = ?",
    )
    .bind(&body.prompt)
    .bind(&body.image_key)
    .bind(&body.image_alt)
    .bind(&body.answer_text)
    .bind(&body.answer_image_key)
    .bind(&body.answer_image_alt)
    .bind(&body.explanation)
    .bind(&body.explanation_image_key)
    .bind(body.multi_select)
    .bind(qid)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    // Options are replaced as a whole; simpler and keeps positions consistent.
    sqlx::query("DELETE FROM question_options WHERE question_id = ?")
        .bind(qid)
        .execute(&mut *tx)
        .await?;
    insert_options(&mut tx, qid, &body.options).await?;
    tx.commit().await?;

    // Delete images that this question no longer uses.
    let new_keys = body.image_keys();
    let removed: Vec<&String> = old_keys.difference(&new_keys).collect();
    state.storage.delete_many_best_effort(removed).await;

    Ok(Json(load_quiz(&state, id, user.id).await?))
}

async fn delete_question(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path((id, qid)): Path<(i64, i64)>,
) -> AppResult<Json<ManagedQuiz>> {
    let quiz = load_quiz(&state, id, user.id).await?;
    let existing = quiz
        .questions
        .iter()
        .find(|q| q.id == qid)
        .ok_or(AppError::NotFound("question"))?;
    let keys = question_image_keys(existing);

    sqlx::query("DELETE FROM questions WHERE id = ? AND quiz_id = ?")
        .bind(qid)
        .bind(id)
        .execute(&state.db)
        .await?;
    state.storage.delete_many_best_effort(keys).await;
    Ok(Json(load_quiz(&state, id, user.id).await?))
}

#[derive(Debug, Deserialize)]
struct ReorderInput {
    question_ids: Vec<i64>,
}

async fn reorder_questions(
    RequireManager(user, _): QuizManager,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<ReorderInput>,
) -> AppResult<Json<ManagedQuiz>> {
    let quiz = load_quiz(&state, id, user.id).await?;
    let current: HashSet<i64> = quiz.questions.iter().map(|q| q.id).collect();
    let requested: HashSet<i64> = body.question_ids.iter().copied().collect();
    if current != requested || requested.len() != body.question_ids.len() {
        return Err(AppError::Validation(
            "question_ids must list every question of this quiz exactly once".into(),
        ));
    }
    let mut tx = state.db.begin().await?;
    for (pos, qid) in body.question_ids.iter().enumerate() {
        sqlx::query("UPDATE questions SET position = ? WHERE id = ? AND quiz_id = ?")
            .bind(pos as i32)
            .bind(qid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(load_quiz(&state, id, user.id).await?))
}
