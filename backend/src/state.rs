use std::sync::Arc;

use sqlx::MySqlPool;

use crate::{config::Config, rate_limit::RateLimiter, storage::Storage};

/// Shared application state, available to handlers through Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub storage: Storage,
    pub config: Arc<Config>,
    pub login_limiter: RateLimiter,
}
