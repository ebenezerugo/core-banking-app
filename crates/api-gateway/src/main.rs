mod config;
mod error;
mod event_store;
mod middleware;
mod router;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = config::AppConfig::from_env()?;

    let pool = sqlx::PgPool::connect(&config.database_url).await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    let app = router::build_router(pool, config.clone());

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Starting core banking API on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
