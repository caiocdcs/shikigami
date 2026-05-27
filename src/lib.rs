pub mod api;
pub mod config;
pub mod core;
pub mod error;
pub mod spi;

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum::Router;
use config::Config;
use secrecy::ExposeSecret;
use sqlx::{Pool, Sqlite, SqlitePool, sqlite::SqlitePoolOptions};
use tower_http::{
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    core::integration_service::IntegrationService,
    spi::integration_repository::SqliteIntegrationRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pg_pool: Pool<Sqlite>,
    pub integration_service: IntegrationService<SqliteIntegrationRepository>,
}

impl AppState {
    pub fn new(config: Config, pg_pool: Pool<Sqlite>) -> Self {
        let integration_repository = SqliteIntegrationRepository::new(pg_pool.clone());
        let integration_service = IntegrationService::new(integration_repository);
        Self {
            config: Arc::new(config),
            pg_pool,
            integration_service,
        }
    }
}

pub async fn create_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(3))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
        .context("failed to connect to the database")?;

    Ok(pool)
}

pub async fn create_app(config: Config) -> anyhow::Result<Router> {
    let pool = create_pool(config.database_url.expose_secret()).await?;
    create_app_with_pool(pool, config).await
}

pub async fn create_app_with_pool(pool: Pool<Sqlite>, config: Config) -> anyhow::Result<Router> {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    let state = AppState::new(config, pool);
    let router = api::routes::router(state);

    let app = router
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());

    Ok(app)
}
