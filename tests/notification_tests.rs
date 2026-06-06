use std::time::Duration;

use shikigami::{
    core::{
        domain::integration::{IntegrationChannel, IntegrationConfig, NewIntegration, NtfyConfig},
        monitor_checker::MonitorChecker,
        monitor_service::MonitorService,
        notification_service::{DispatcherMap, NotificationService},
        ports::{
            MonitorRepository, integration_repository::IntegrationRepository,
            notification_dispatcher::OutboxRepository,
        },
    },
    spi::{
        gotify_dispatcher::GotifyDispatcher, integration_repository::SqliteIntegrationRepository,
        monitor_repository::SqliteMonitorRepository, ntfy_dispatcher::NtfyDispatcher,
        outbox_repository::SqliteOutboxRepository, slack_dispatcher::SlackDispatcher,
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
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("pragma failed");
    pool
}

/// Insert a valid monitor row and return its ID.
async fn insert_monitor(pool: &SqlitePool) -> String {
    let mon_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let next = now + chrono::Duration::seconds(60);
    sqlx::query("INSERT INTO monitors (id, name, slug, status, schedule_type, interval_seconds, grace_seconds, last_pinged_at, next_expected_at, created_at) VALUES (?, 'test', 'test', 'active', 'interval', 60, 10, ?, ?, datetime('now'))")
        .bind(&mon_id)
        .bind(now.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(next.format("%Y-%m-%d %H:%M:%S").to_string())
        .execute(pool).await.expect("insert monitor");
    mon_id
}

/// Insert a valid ntfy integration row and return its ID.
async fn insert_integration(pool: &SqlitePool) -> String {
    let int_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO integrations (id, name, channel_type, config_json, status, created_at) VALUES (?, 'test', 'ntfy', '{\"url\":\"https://ntfy.sh\",\"topic\":\"t\",\"priority\":3,\"message\":\"alert\"}', 'active', datetime('now'))")
        .bind(&int_id)
        .execute(pool).await.expect("insert integration");
    int_id
}

/// Insert an outbox entry referencing valid parent records. Returns (entry_id, monitor_id, integration_id).
async fn insert_outbox(pool: &SqlitePool, status: &str) -> (String, String, String) {
    let mon_id = insert_monitor(pool).await;
    let int_id = insert_integration(pool).await;
    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, ?, ?, 'test', 0, ?, datetime('now'))")
        .bind(&entry_id)
        .bind(&mon_id)
        .bind(&int_id)
        .bind(status)
        .execute(pool).await.expect("insert outbox");
    (entry_id, mon_id, int_id)
}

// ---- Outbox Tests ----

#[tokio::test]
async fn outbox_state_pending_to_sent() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());
    let (entry_id, _, _) = insert_outbox(&pool, "pending").await;
    let entries = repo.fetch_pending(10).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].retry_count, 0);
    repo.claim_sending(&entry_id).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "sending");
    repo.mark_sent(&entry_id).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "sent");
    let entries = repo.fetch_pending(10).await.unwrap();
    assert_eq!(entries.len(), 0);
}

#[tokio::test]
async fn outbox_state_pending_to_failed() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());
    let (entry_id, _, _) = insert_outbox(&pool, "pending").await;
    repo.claim_sending(&entry_id).await.unwrap();
    repo.mark_failed(&entry_id).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[tokio::test]
async fn outbox_state_transient_retry_increments_count() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());
    let (entry_id, _, _) = insert_outbox(&pool, "pending").await;
    repo.claim_sending(&entry_id).await.unwrap();
    repo.retry_later(&entry_id).await.unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
    let retry: i32 = sqlx::query_scalar("SELECT retry_count FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(retry, 1);
}

#[tokio::test]
async fn outbox_resets_stale_sending_on_startup() {
    let pool = pool_with_migrations().await;
    let repo = SqliteOutboxRepository::new(pool.clone());
    let (entry_id, _, _) = insert_outbox(&pool, "sending").await;
    let count = repo.reset_stale_sending().await.unwrap();
    assert_eq!(count, 1);
    let status: String = sqlx::query_scalar("SELECT status FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending");
}

#[tokio::test]
async fn notification_service_handles_missing_integration() {
    let pool = pool_with_migrations().await;
    let int_repo = SqliteIntegrationRepository::new(pool.clone());
    let mon_id = insert_monitor(&pool).await;
    // Use a valid UUID that doesn't exist as an integration
    let fake_int_id = uuid::Uuid::new_v4().to_string();
    let entry_id = uuid::Uuid::new_v4().to_string();
    // Temporarily disable FK check to insert outbox with non-existent integration
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("pragma failed");
    sqlx::query("INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, ?, ?, 'test', 0, 'pending', datetime('now'))")
        .bind(&entry_id).bind(&mon_id).bind(&fake_int_id).execute(&pool).await.expect("insert");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("pragma failed");
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
        .unwrap();
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
            name: "test".into(),
            channel: IntegrationChannel::Ntfy,
            config: IntegrationConfig::Ntfy(NtfyConfig {
                url: "https://ntfy.sh".into(),
                topic: "test".into(),
                priority: 3,
                message: "alert".into(),
            }),
        })
        .await
        .unwrap();
    let int_id = integration.id.as_uuid().to_string();
    let mon_id = insert_monitor(&pool).await;
    let entry_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, ?, ?, 'alert', 0, 'pending', datetime('now'))")
        .bind(&entry_id).bind(&mon_id).bind(&int_id).execute(&pool).await.unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
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
        .unwrap();
    assert_eq!(status, "pending", "transient error should retry");
    let retry: i32 = sqlx::query_scalar("SELECT retry_count FROM notification_outbox WHERE id = ?")
        .bind(&entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
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
        .unwrap()
        .unwrap();
}

// ---- Monitor Checker Tests ----

#[tokio::test]
async fn find_missed_monitors_returns_expired() {
    let pool = pool_with_migrations().await;
    let repo = SqliteMonitorRepository::new(pool.clone());
    let mon_id = insert_monitor(&pool).await;
    // Set next_expected_at to the past so it's missed
    let past = chrono::Utc::now() - chrono::Duration::seconds(60);
    sqlx::query("UPDATE monitors SET next_expected_at = ?, last_pinged_at = ? WHERE id = ?")
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(&mon_id)
        .execute(&pool)
        .await
        .unwrap();
    let missed = repo.find_missed_monitors().await.unwrap();
    assert_eq!(missed.len(), 1);
    assert_eq!(missed[0].as_uuid().to_string(), mon_id);
}

#[tokio::test]
async fn find_missed_monitors_skips_paused() {
    let pool = pool_with_migrations().await;
    let repo = SqliteMonitorRepository::new(pool.clone());
    let mon_id = insert_monitor(&pool).await;
    let past = chrono::Utc::now() - chrono::Duration::seconds(60);
    sqlx::query("UPDATE monitors SET status = 'paused', next_expected_at = ?, last_pinged_at = ? WHERE id = ?")
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(&mon_id)
        .execute(&pool)
        .await
        .unwrap();
    let missed = repo.find_missed_monitors().await.unwrap();
    assert_eq!(missed.len(), 0);
}

#[tokio::test]
async fn find_missed_monitors_skips_null_next_expected() {
    let pool = pool_with_migrations().await;
    let repo = SqliteMonitorRepository::new(pool.clone());
    let mon_id = insert_monitor(&pool).await;
    sqlx::query("UPDATE monitors SET next_expected_at = NULL, last_pinged_at = NULL WHERE id = ?")
        .bind(&mon_id)
        .execute(&pool)
        .await
        .unwrap();
    let missed = repo.find_missed_monitors().await.unwrap();
    assert_eq!(missed.len(), 0);
}

#[tokio::test]
async fn monitor_checker_run_once_creates_check_in_and_outbox() {
    let pool = pool_with_migrations().await;
    let repo = SqliteMonitorRepository::new(pool.clone());
    let int_id = insert_integration(&pool).await;
    let mon_id = insert_monitor(&pool).await;
    let past = chrono::Utc::now() - chrono::Duration::seconds(60);
    sqlx::query("UPDATE monitors SET next_expected_at = ?, last_pinged_at = ? WHERE id = ?")
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(past.format("%Y-%m-%d %H:%M:%S").to_string())
        .bind(&mon_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO monitor_integrations (monitor_id, integration_id) VALUES (?, ?)")
        .bind(&mon_id)
        .bind(&int_id)
        .execute(&pool)
        .await
        .unwrap();
    let service = MonitorService::new(repo);
    let checker = MonitorChecker::new(service, Duration::from_secs(60));
    checker.run_once().await;
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM check_ins WHERE monitor_id = ?")
        .bind(&mon_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "check_in record created");
    let outbox_count: i32 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_outbox WHERE monitor_id = ?")
            .bind(&mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        outbox_count, 1,
        "outbox entry created via check_in failure path"
    );
}
