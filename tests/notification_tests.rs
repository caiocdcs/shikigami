use std::time::Duration;

use shikigami::{
    core::{
        domain::integration::{IntegrationChannel, IntegrationConfig, NewIntegration, NtfyConfig},
        notification_service::{DispatcherMap, NotificationService},
        ports::{
            integration_repository::IntegrationRepository,
            notification_dispatcher::OutboxRepository,
        },
    },
    spi::{
        gotify_dispatcher::GotifyDispatcher, integration_repository::SqliteIntegrationRepository,
        ntfy_dispatcher::NtfyDispatcher, outbox_repository::SqliteOutboxRepository,
        slack_dispatcher::SlackDispatcher,
    },
};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tokio_util::sync::CancellationToken;

fn fast_poll() -> Duration {
    Duration::from_millis(50)
}

async fn pool_with_migrations() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("failed to create pool");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed");
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("pragma failed");
    pool
}

#[tokio::test]
async fn outbox_state_pending_to_sent() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', 'i1', 'test', 0, 'pending', datetime('now'))",
    )
    .bind(&entry_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    let entries = repo.fetch_pending(10).await.expect("fetch failed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].retry_count, 0);

    repo.claim_sending(&entry_id).await.expect("claim failed");
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "sending");

    repo.mark_sent(&entry_id).await.expect("mark_sent failed");
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "sent");

    let entries = repo.fetch_pending(10).await.expect("fetch failed");
    assert_eq!(entries.len(), 0);
}

#[tokio::test]
async fn outbox_state_pending_to_failed() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', 'i1', 'test', 0, 'pending', datetime('now'))",
    )
    .bind(&entry_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    repo.claim_sending(&entry_id).await.expect("claim failed");
    repo.mark_failed(&entry_id)
        .await
        .expect("mark_failed failed");

    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn outbox_state_transient_retry_increments_count() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', 'i1', 'test', 0, 'pending', datetime('now'))",
    )
    .bind(&entry_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    repo.claim_sending(&entry_id).await.expect("claim failed");
    repo.retry_later(&entry_id)
        .await
        .expect("retry_later failed");

    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "pending");

    let retry: i32 = sqlx::query_scalar("SELECT retry_count FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(retry, 1);
}

#[tokio::test]
async fn outbox_resets_stale_sending_on_startup() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', 'i1', 'test', 0, 'sending', datetime('now'))",
    )
    .bind(&entry_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    let count = repo.reset_stale_sending().await.expect("reset failed");
    assert_eq!(count, 1);

    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn notification_service_handles_missing_integration() {
    let pool = pool_with_migrations().await;
    let int_repo = SqliteIntegrationRepository::new(pool.clone());

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', '00000000-0000-0000-0000-000000000000', 'test', 0, 'pending', datetime('now'))",
    )
    .bind(&entry_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    let client = reqwest::Client::new();
    let dispatchers = DispatcherMap::new(
        NtfyDispatcher::new(client.clone()),
        GotifyDispatcher::new(client.clone()),
        SlackDispatcher::new(client),
    );
    let outbox_repo = SqliteOutboxRepository::new(pool.clone());
    let service = NotificationService::new(outbox_repo, int_repo, dispatchers, fast_poll());
    service.run_once().await;

    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(
        status, "failed",
        "missing integration should mark as failed"
    );
}

#[tokio::test]
async fn notification_service_transient_retries() {
    let pool = pool_with_migrations().await;
    let int_repo = SqliteIntegrationRepository::new(pool.clone());

    let integration = int_repo
        .new_integration(NewIntegration {
            name: "test".to_string(),
            channel: IntegrationChannel::Ntfy,
            config: IntegrationConfig::Ntfy(NtfyConfig {
                url: "https://ntfy.sh".to_string(),
                topic: "test".to_string(),
                priority: 3,
                message: "alert".to_string(),
            }),
        })
        .await
        .expect("create integration");
    let int_id = integration.id.as_uuid().to_string();

    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, 'm1', ?, 'alert', 0, 'pending', datetime('now'))",
    )
    .bind(&entry_id)
    .bind(&int_id)
    .execute(&pool)
    .await
    .expect("insert failed");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .expect("client");
    let dispatchers = DispatcherMap::new(
        NtfyDispatcher::new(client.clone()),
        GotifyDispatcher::new(client.clone()),
        SlackDispatcher::new(client),
    );
    let outbox_repo = SqliteOutboxRepository::new(pool.clone());
    let service = NotificationService::new(outbox_repo, int_repo, dispatchers, fast_poll());
    service.run_once().await;

    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(status, "pending", "transient error should retry");

    let retry: i32 = sqlx::query_scalar("SELECT retry_count FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(retry, 1, "retry_count should increment");
}

#[tokio::test]
async fn notification_worker_responds_to_cancellation() {
    let pool = pool_with_migrations().await;
    let int_repo = SqliteIntegrationRepository::new(pool.clone());
    let outbox_repo = SqliteOutboxRepository::new(pool);
    let client = reqwest::Client::new();
    let dispatchers = DispatcherMap::new(
        NtfyDispatcher::new(client.clone()),
        GotifyDispatcher::new(client.clone()),
        SlackDispatcher::new(client),
    );
    let service = NotificationService::new(outbox_repo, int_repo, dispatchers, fast_poll());

    let token = CancellationToken::new();
    let child = token.child_token();

    let handle = tokio::spawn(async move {
        service.run(child).await;
    });

    token.cancel();

    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("worker did not shut down in time")
        .expect("worker panicked");
}
