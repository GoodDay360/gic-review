//! The single place where quiz modes and their availability rules live.
//!
//! To add a mode: add a variant to [`QuizMode`], add it to [`QuizMode::ALL`], and write its
//! rule in [`QuizMode::is_available`]. The API then reports it in `available_modes` and the
//! frontend registry (`frontend/src/quiz-modes/registry.ts`) decides how to render it.
//! See `docs/DEV-ADD-QUIZ-MODE.md`.

use serde::{Deserialize, Serialize};

use crate::models::QuestionDto;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuizMode {
    /// Multiple choice ("questions à choix multiples").
    Qcm,
    /// Front/back study cards.
    Flashcard,
}

impl QuizMode {
    /// Every mode, in the order they are shown to students.
    pub const ALL: &'static [QuizMode] = &[QuizMode::Qcm, QuizMode::Flashcard];

    /// A quiz supports a mode only if *every* question supports it.
    pub fn is_available(self, questions: &[QuestionDto]) -> bool {
        if questions.is_empty() {
            return false;
        }
        match self {
            // >= 2 options (each with text or an image) and >= 1 correct option.
            QuizMode::Qcm => questions.iter().all(|q| {
                let valid_options = q.options.iter().filter(|o| has_text_or_image(o)).count();
                valid_options >= 2 && q.options.iter().any(|o| o.is_correct)
            }),
            // An answer (text or image) for the back of the card, or a correct option to show.
            QuizMode::Flashcard => questions.iter().all(|q| {
                non_empty(q.answer_text.as_deref())
                    || q.answer_image_key.is_some()
                    || q.options.iter().any(|o| o.is_correct)
            }),
        }
    }
}

fn non_empty(s: Option<&str>) -> bool {
    s.is_some_and(|s| !s.trim().is_empty())
}

fn has_text_or_image(o: &crate::models::OptionDto) -> bool {
    non_empty(o.text.as_deref()) || o.image_key.is_some()
}

/// Modes available for a quiz, computed by the backend so the frontend never guesses.
pub fn available_modes(questions: &[QuestionDto]) -> Vec<QuizMode> {
    QuizMode::ALL
        .iter()
        .copied()
        .filter(|m| m.is_available(questions))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OptionDto, QuestionDto};

    fn opt(text: &str, correct: bool) -> OptionDto {
        OptionDto {
            id: 0,
            position: 0,
            text: Some(text.into()),
            image_key: None,
            image_url: None,
            image_alt: None,
            is_correct: correct,
        }
    }

    fn question(options: Vec<OptionDto>, answer_text: Option<&str>) -> QuestionDto {
        QuestionDto {
            id: 0,
            position: 0,
            prompt: Some("Q".into()),
            image_key: None,
            image_url: None,
            image_alt: None,
            answer_text: answer_text.map(Into::into),
            answer_image_key: None,
            answer_image_url: None,
            answer_image_alt: None,
            explanation: None,
            explanation_image_key: None,
            explanation_image_url: None,
            multi_select: false,
            text_format: Default::default(),
            options,
        }
    }

    #[test]
    fn empty_quiz_has_no_modes() {
        assert!(available_modes(&[]).is_empty());
    }

    #[test]
    fn options_with_a_correct_answer_enable_both_modes() {
        let qs = vec![question(vec![opt("a", true), opt("b", false)], None)];
        assert_eq!(
            available_modes(&qs),
            vec![QuizMode::Qcm, QuizMode::Flashcard]
        );
    }

    #[test]
    fn answer_text_only_enables_flashcard_only() {
        let qs = vec![question(vec![], Some("answer"))];
        assert_eq!(available_modes(&qs), vec![QuizMode::Flashcard]);
    }

    #[test]
    fn one_option_is_not_enough_for_qcm() {
        let qs = vec![question(vec![opt("a", true)], None)];
        assert_eq!(available_modes(&qs), vec![QuizMode::Flashcard]);
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(serde_json::to_string(&QuizMode::Qcm).unwrap(), "\"qcm\"");
    }
}
