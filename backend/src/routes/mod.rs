//! HTTP routes. Everything is mounted under `/api` in [`crate::build_router`].

pub mod admin_quizzes;
pub mod auth;
pub mod import;
pub mod public;
pub mod reviews;
pub mod subjects;
pub mod uploads;
pub mod users;

use axum::Router;

use crate::{config::Config, state::AppState};

pub fn api(config: &Config) -> Router<AppState> {
    Router::new()
        .merge(public::router())
        .merge(auth::router())
        .merge(admin_quizzes::router())
        .merge(import::router())
        .merge(uploads::router(config))
        .merge(reviews::router(config))
        .merge(users::router())
        .merge(subjects::router())
}
