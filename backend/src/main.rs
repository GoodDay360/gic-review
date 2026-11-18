use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Context;
use gic_review_api::{
    bootstrap, build_router, config::Config, rate_limit::RateLimiter, state::AppState,
    storage::Storage,
};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
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

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("\n{msg}\n");
            std::process::exit(1);
        }
    };
    tracing::info!(?config, "configuration loaded");

    let connect = MySqlConnectOptions::new()
        .host(&config.db_host)
        .port(config.db_port)
        .username(&config.db_user)
        .password(&config.db_password)
        .database(&config.db_name)
        .charset("utf8mb4");
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

    let state = AppState {
        db,
        storage: Storage::new(&config),
        config: Arc::new(config),
        login_limiter: RateLimiter::new(5, Duration::from_secs(60)),
    };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("could not bind {addr}"))?;
    tracing::info!("GIC Review API listening on http://{addr}");
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
