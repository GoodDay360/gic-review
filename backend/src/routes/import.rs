//! `POST /admin/quizzes/import`: create a whole quiz from JSON in one DB transaction.
//! The JSON format is documented in `docs/AI-HOW-ADD-QUIZ.md`.

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use serde::Deserialize;

use crate::{
    auth::RequireStaff,
    error::{AppError, AppResult},
    images,
    models::QuizDetail,
    repo,
    routes::{
        admin_quizzes::{OptionInput, QuestionInput, insert_question},
        uploads::store_image,
    },
    state::AppState,
    validation::{check_image_key, check_len, clean},
};

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/admin/quizzes/import",
        post(import_quiz).layer(DefaultBodyLimit::max(4 * 1024 * 1024)),
    )
}

const MAX_QUESTIONS: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportQuiz {
    pub subject_slug: Option<String>,
    pub subject_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_key: Option<String>,
    pub cover_image_url: Option<String>,
    pub questions: Vec<ImportQuestion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportQuestion {
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
    #[serde(default)]
    pub multi_select: bool,
    #[serde(default)]
    pub options: Vec<ImportOption>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportOption {
    pub text: Option<String>,
    pub image_key: Option<String>,
    pub image_url: Option<String>,
    pub image_alt: Option<String>,
    #[serde(default)]
    pub is_correct: bool,
}

/// An image given either as an existing key or as a URL to download.
fn pick(label: &str, key: &Option<String>, url: &Option<String>) -> AppResult<Option<ImageSource>> {
    match (clean(key.clone()), clean(url.clone())) {
        (Some(_), Some(_)) => Err(AppError::Validation(format!(
            "{label}: give either *_key or *_url, not both"
        ))),
        (Some(k), None) => {
            check_image_key(label, &Some(k.clone()))?;
            Ok(Some(ImageSource::Key(k)))
        }
        (None, Some(u)) => Ok(Some(ImageSource::Url(u))),
        (None, None) => Ok(None),
    }
}

#[derive(Debug, Clone)]
enum ImageSource {
    Key(String),
    Url(String),
}

/// A syntactically valid placeholder used during the validation pass.
fn placeholder(src: &Option<ImageSource>) -> Option<String> {
    src.as_ref()
        .map(|_| format!("tmp/{}.png", uuid::Uuid::nil()))
}

struct PlannedQuestion {
    input: QuestionInput,
    image: Option<ImageSource>,
    answer_image: Option<ImageSource>,
    explanation_image: Option<ImageSource>,
    option_images: Vec<Option<ImageSource>>,
}

fn plan(questions: Vec<ImportQuestion>) -> AppResult<Vec<PlannedQuestion>> {
    let mut planned = Vec::with_capacity(questions.len());
    for (i, q) in questions.into_iter().enumerate() {
        let label = format!("questions[{i}]");
        let image = pick(&format!("{label}.image"), &q.image_key, &q.image_url)?;
        let answer_image = pick(
            &format!("{label}.answer_image"),
            &q.answer_image_key,
            &q.answer_image_url,
        )?;
        let explanation_image = pick(
            &format!("{label}.explanation_image"),
            &q.explanation_image_key,
            &q.explanation_image_url,
        )?;
        let mut option_images = Vec::new();
        let mut options = Vec::new();
        for (j, o) in q.options.into_iter().enumerate() {
            let img = pick(
                &format!("{label}.options[{j}].image"),
                &o.image_key,
                &o.image_url,
            )?;
            options.push(OptionInput {
                text: o.text,
                image_key: placeholder(&img),
                image_alt: o.image_alt,
                is_correct: o.is_correct,
            });
            option_images.push(img);
        }
        let input = QuestionInput {
            prompt: q.prompt,
            image_key: placeholder(&image),
            image_alt: q.image_alt,
            answer_text: q.answer_text,
            answer_image_key: placeholder(&answer_image),
            answer_image_alt: q.answer_image_alt,
            explanation: q.explanation,
            explanation_image_key: placeholder(&explanation_image),
            multi_select: q.multi_select,
            options,
        }
        // Validation pass with placeholder keys: fail before downloading anything.
        .normalize(&label)?;
        planned.push(PlannedQuestion {
            input,
            image,
            answer_image,
            explanation_image,
            option_images,
        });
    }
    Ok(planned)
}

async fn resolve(
    state: &AppState,
    quiz_id: i64,
    src: &Option<ImageSource>,
    uploaded: &mut Vec<String>,
) -> AppResult<Option<String>> {
    match src {
        None => Ok(None),
        Some(ImageSource::Key(k)) => Ok(Some(k.clone())),
        Some(ImageSource::Url(url)) => {
            let bytes = images::download(url, state.config.max_image_size_bytes).await?;
            let name = url
                .rsplit('/')
                .next()
                .map(|s| s.split('?').next().unwrap_or(s));
            let name = name.filter(|n| images::ImageKind::from_filename(n).is_some());
            let stored = store_image(state, bytes, name.map(str::to_string), Some(quiz_id)).await?;
            uploaded.push(stored.key.clone());
            Ok(Some(stored.key))
        }
    }
}

async fn import_quiz(
    RequireStaff(user): RequireStaff,
    State(state): State<AppState>,
    Json(body): Json<ImportQuiz>,
) -> AppResult<(StatusCode, Json<QuizDetail>)> {
    // ---- Validate everything up front ----
    let subject_id = match (&body.subject_id, clean(body.subject_slug.clone())) {
        (Some(id), _) => *id,
        (None, Some(slug)) => {
            repo::subject_by_slug(&state.db, &slug)
                .await?
                .ok_or_else(|| AppError::Validation(format!("subject_slug '{slug}' not found")))?
                .id
        }
        (None, None) => {
            return Err(AppError::Validation(
                "subject_slug (or subject_id) is required".into(),
            ));
        }
    };
    let subject_exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM subjects WHERE id = ?")
        .bind(subject_id)
        .fetch_optional(&state.db)
        .await?;
    if subject_exists.is_none() {
        return Err(AppError::Validation("subject_id: subject not found".into()));
    }
    let title = body.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(AppError::Validation(
            "title: is required (max 200 characters)".into(),
        ));
    }
    let description = clean(body.description.clone());
    check_len("description", &description, 5000)?;
    if body.questions.is_empty() || body.questions.len() > MAX_QUESTIONS {
        return Err(AppError::Validation(format!(
            "questions: between 1 and {MAX_QUESTIONS} questions are required"
        )));
    }
    let cover = pick("cover_image", &body.cover_image_key, &body.cover_image_url)?;
    let planned = plan(body.questions)?;

    // ---- Write in one transaction; clean up downloaded images on failure ----
    let mut uploaded: Vec<String> = Vec::new();
    let result = async {
        let mut tx = state.db.begin().await?;
        let res = sqlx::query(
            "INSERT INTO quizzes (subject_id, title, description, is_published, created_by) \
             VALUES (?, ?, ?, FALSE, ?)",
        )
        .bind(subject_id)
        .bind(&title)
        .bind(&description)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
        let quiz_id = res.last_insert_id() as i64;

        let cover_key = resolve(&state, quiz_id, &cover, &mut uploaded).await?;
        sqlx::query("UPDATE quizzes SET cover_image_key = ? WHERE id = ?")
            .bind(&cover_key)
            .bind(quiz_id)
            .execute(&mut *tx)
            .await?;

        for (pos, p) in planned.into_iter().enumerate() {
            let mut q = p.input;
            q.image_key = resolve(&state, quiz_id, &p.image, &mut uploaded).await?;
            q.answer_image_key = resolve(&state, quiz_id, &p.answer_image, &mut uploaded).await?;
            q.explanation_image_key =
                resolve(&state, quiz_id, &p.explanation_image, &mut uploaded).await?;
            for (o, src) in q.options.iter_mut().zip(p.option_images.iter()) {
                o.image_key = resolve(&state, quiz_id, src, &mut uploaded).await?;
            }
            insert_question(&mut tx, quiz_id, pos as i32, &q).await?;
        }
        tx.commit().await?;
        Ok::<i64, AppError>(quiz_id)
    }
    .await;

    match result {
        Ok(quiz_id) => {
            let quiz = repo::quiz_detail(&state.db, &state.storage, quiz_id)
                .await?
                .ok_or(AppError::NotFound("quiz"))?;
            Ok((StatusCode::CREATED, Json(quiz)))
        }
        Err(err) => {
            state.storage.delete_many_best_effort(&uploaded).await;
            Err(err)
        }
    }
}
