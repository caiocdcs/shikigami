#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::ignored_unit_patterns)]

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
        monitor_checker::MonitorChecker,
        monitor_service::MonitorService,
        notification_service::{DispatcherMap, NotificationService},
        retention_checker::RetentionChecker,
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

pub async fn create_pool(config: &Config) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(config.pool_max_connections)
        .min_connections(config.pool_min_connections)
        .acquire_timeout(Duration::from_secs(config.pool_acquire_timeout_seconds))
        .idle_timeout(Duration::from_secs(config.pool_idle_timeout_seconds))
        .after_connect(|conn, _meta| {
            Box::pin(async {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(config.database_url.expose_secret())
        .await
        .context("failed to connect to the database")?;

    Ok(pool)
}

pub async fn create_app(
    config: Config,
    shutdown_token: CancellationToken,
) -> anyhow::Result<Router> {
    let pool = create_pool(&config).await?;
    create_app_with_pool(pool, config, shutdown_token).await
}

pub fn build_notification_service(
    config: &Config,
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
        Duration::from_secs(config.notification_interval_seconds),
        config.notification_max_retries,
    )
}

pub async fn create_app_with_pool(
    pool: Pool<Sqlite>,
    config: Config,
    shutdown_token: CancellationToken,
) -> anyhow::Result<Router> {
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    let state = AppState::new(config.clone(), pool.clone());

    if state.config.api_key.is_none() {
        tracing::warn!(
            "API_KEY not set: CRUD endpoints are UNAUTHENTICATED. Set API_KEY to protect them."
        );
    }

    // Build and spawn the notification worker
    let notification_service = build_notification_service(&config, pool.clone());
    let worker_token = shutdown_token.child_token();
    tokio::spawn(async move {
        notification_service.run(worker_token).await;
    });

    // Build and spawn the missed-monitor checker
    let checker_repo = SqliteMonitorRepository::new(pool.clone());
    let checker_service = MonitorService::new(checker_repo);
    let checker = MonitorChecker::new(
        checker_service,
        Duration::from_secs(config.checker_interval_seconds),
    );
    let checker_token = shutdown_token.child_token();
    tokio::spawn(async move {
        checker.run(checker_token).await;
    });

    // Build and spawn the check-in retention worker
    let retention_repo = SqliteMonitorRepository::new(pool.clone());
    let retention = RetentionChecker::new(
        retention_repo,
        Duration::from_secs(config.retention_interval_seconds),
        config.retention_days,
    );
    let retention_token = shutdown_token.child_token();
    tokio::spawn(async move {
        retention.run(retention_token).await;
    });

    let router = api::routes::router(state);

    let app = router
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());

    Ok(app)
}
