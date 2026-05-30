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
use tokio_util::sync::CancellationToken;
use tower_http::{
    compression::CompressionLayer,
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    core::{
        integration_service::IntegrationService,
        monitor_service::MonitorService,
        notification_service::{DispatcherMap, NotificationService},
    },
    spi::{
        gotify_dispatcher::GotifyDispatcher, integration_repository::SqliteIntegrationRepository,
        monitor_repository::SqliteMonitorRepository, ntfy_dispatcher::NtfyDispatcher,
        outbox_repository::SqliteOutboxRepository, slack_dispatcher::SlackDispatcher,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pg_pool: Pool<Sqlite>,
    pub integration_service: IntegrationService<SqliteIntegrationRepository>,
    pub monitor_service: MonitorService<SqliteMonitorRepository>,
}

impl AppState {
    pub fn new(config: Config, pg_pool: Pool<Sqlite>) -> Self {
        let integration_repository = SqliteIntegrationRepository::new(pg_pool.clone());
        let integration_service = IntegrationService::new(integration_repository);
        let monitor_repository = SqliteMonitorRepository::new(pg_pool.clone());
        let monitor_service = MonitorService::new(monitor_repository);
        Self {
            config: Arc::new(config),
            pg_pool,
            integration_service,
            monitor_service,
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

pub fn build_notification_service(
    pool: Pool<Sqlite>,
) -> NotificationService<SqliteIntegrationRepository, SqliteOutboxRepository> {
    let integration_repo = SqliteIntegrationRepository::new(pool.clone());
    let outbox_repo = SqliteOutboxRepository::new(pool);
    let http_client = reqwest::Client::new();
    let dispatchers = DispatcherMap::new(
        NtfyDispatcher::new(http_client.clone()),
        GotifyDispatcher::new(http_client.clone()),
        SlackDispatcher::new(http_client.clone()),
    );
    NotificationService::new(
        outbox_repo,
        integration_repo,
        dispatchers,
        Duration::from_secs(30),
    )
}

pub async fn create_app_with_pool(pool: Pool<Sqlite>, config: Config) -> anyhow::Result<Router> {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    let state = AppState::new(config, pool.clone());

    // Build and spawn the notification worker
    let notification_service = build_notification_service(pool);
    let shutdown_token = CancellationToken::new();
    let worker_token = shutdown_token.child_token();
    tokio::spawn(async move {
        notification_service.run(worker_token).await;
    });

    let router = api::routes::router(state);

    let app = router
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());

    Ok(app)
}
