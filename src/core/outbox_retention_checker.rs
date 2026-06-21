use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::core::ports::OutboxRepository;

/// Background worker that prunes terminal `notification_outbox` rows
/// (`sent`/`failed`) older than the configured retention window.
///
/// Pending and sending rows are never pruned: they represent in-flight or
/// undelivered alerts, which are symptoms to investigate rather than data to
/// silently delete. Mirrors `RetentionChecker` (own timer + shutdown token)
/// but owns only the outbox concern and depends solely on `OutboxRepository`.
pub struct OutboxRetentionChecker<OR: OutboxRepository> {
    repo: OR,
    interval: Duration,
    retention_days: i64,
}

impl<OR: OutboxRepository> OutboxRetentionChecker<OR> {
    pub fn new(repo: OR, interval: Duration, retention_days: i64) -> Self {
        Self {
            repo,
            interval,
            retention_days,
        }
    }

    pub async fn run(&self, shutdown_token: CancellationToken) {
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("outbox retention checker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.interval) => {
                    self.run_once().await;
                }
            }
        }
    }

    pub async fn run_once(&self) {
        match self.prune().await {
            Ok(pruned) => {
                tracing::info!(pruned, "outbox retention prune complete");
            }
            Err(e) => {
                tracing::error!(error = %e, "outbox retention prune failed");
            }
        }
    }

    async fn prune(&self) -> Result<u64, sqlx::Error> {
        if self.retention_days <= 0 {
            // Retention disabled. Lets operators turn pruning off via config
            // without a code change.
            return Ok(0);
        }
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.retention_days);
        self.repo.prune_old_outbox_entries(cutoff).await
    }
}
