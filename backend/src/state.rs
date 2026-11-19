use std::sync::Arc;

use sqlx::MySqlPool;
use tokio::sync::Semaphore;

use crate::{cache::ResponseCache, config::Config, rate_limit::RateLimiter, storage::Storage};

/// Shared application state, available to handlers through Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub storage: Storage,
    pub config: Arc<Config>,
    pub login_limiter: RateLimiter,
    /// Limits how many image uploads are processed and stored at once, for all clients
    /// together (`MAX_CONCURRENT_IMAGE_UPLOADS`). Extra uploads wait their turn.
    pub image_upload_slots: Arc<Semaphore>,
    /// Public read-API response cache (see `cache`).
    pub cache: ResponseCache,
}
