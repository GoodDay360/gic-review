//! HTTP routes. Everything is mounted under `/api` in [`crate::build_router`].

pub mod admin_quizzes;
pub mod api_keys;
pub mod auth;
pub mod import;
pub mod public;
pub mod reviews;
pub mod shares;
pub mod subjects;
pub mod uploads;
pub mod users;
pub mod categories;

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
        .merge(categories::router())
        .merge(api_keys::router())
        .merge(shares::router())
}
