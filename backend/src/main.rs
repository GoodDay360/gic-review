use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use gic_review_api::{
    bootstrap, build_router,
    cache::ResponseCache,
    config::{Config, db_options_from_env},
    rate_limit::RateLimiter,
    routes,
    state::AppState,
    storage::Storage,
};
use sqlx::mysql::MySqlPoolOptions;
use tokio::sync::Semaphore;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // backend/.env is optional (real environment variables also work).
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gic_review_api=info,tower_http=info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // `cargo run -- migrate`: apply migrations using only the DB_* values, then exit.
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        return migrate_only().await;
    }

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("\n{msg}\n");
            std::process::exit(1);
        }
    };
    tracing::info!(?config, "configuration loaded");

    let connect = config.connect_options();
    let db = MySqlPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(connect)
        .await
        .with_context(|| {
            format!(
                "could not connect to MySQL at {}:{} (database '{}'); check DB_* in backend/.env",
                config.db_host, config.db_port, config.db_name
            )
        })?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("running database migrations failed")?;
    bootstrap::ensure_admin(&db, &config).await?;

    let addr: SocketAddr = format!("{}:{}", config.backend_host, config.backend_port)
        .parse()
        .context("BACKEND_HOST/BACKEND_PORT do not form a valid address")?;

    let cache = ResponseCache::new(
        config.cache_max_bytes,
        Duration::from_secs(config.cache_ttl_seconds.max(1)),
    );
    tracing::info!(
        max_mb = config.cache_max_bytes / 1024 / 1024,
        ttl_seconds = config.cache_ttl_seconds,
        "response cache {}",
        if config.cache_max_bytes > 0 {
            "enabled"
        } else {
            "disabled"
        }
    );
    let image_upload_slots = Arc::new(Semaphore::new(config.max_concurrent_image_uploads));
    let state = AppState {
        db,
        storage: Storage::new(&config),
        config: Arc::new(config),
        login_limiter: RateLimiter::new(5, Duration::from_secs(60)),
        image_upload_slots,
        cache,
    };
    let run_cleanup = state.config.is_production();
    let cleanup_state = state.clone();
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("could not bind {addr}"))?;
    tracing::info!("GIC Review API listening on http://{addr}");

    // Orphaned-image cleanup: once at startup on its own thread (started after the listener
    // is bound, so it doesn't delay the first request), then daily. Production only: a dev
    // database doesn't know production's images, so running it locally against a shared
    // bucket would delete images that live quizzes still use.
    if run_cleanup {
        routes::uploads::spawn_orphan_cleanup_worker(cleanup_state);
    } else {
        tracing::info!("orphan cleanup worker disabled outside production");
    }
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

async fn migrate_only() -> anyhow::Result<()> {
    let options = match db_options_from_env() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("\n{msg}\n");
            std::process::exit(1);
        }
    };
    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .context("could not connect to MySQL; check DB_* in backend/.env")?;
    let migrator = sqlx::migrate!("./migrations");
    migrator
        .run(&db)
        .await
        .context("running database migrations failed")?;
    println!(
        "Database is up to date ({} migrations).",
        migrator.iter().count()
    );
    Ok(())
}
