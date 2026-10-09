use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::core::ports::{MonitorRepository, OutboxRepository};

/// Background worker that bounds `SQLite` growth by pruning old `check_ins`
/// and terminal (`sent`/`failed`) `notification_outbox` rows.
///
/// Pending and sending outbox rows are never pruned: they represent in-flight
/// or undelivered alerts. Cutoffs are business decisions computed here
/// (`now - N days`); the repositories only execute the deletes. A window of
/// `0` (or less) disables that prune.
pub struct RetentionChecker<MR: MonitorRepository, OR: OutboxRepository> {
    monitor_repo: MR,
    outbox_repo: OR,
    interval: Duration,
    check_in_retention_days: i64,
    outbox_retention_days: i64,
}

impl<MR: MonitorRepository, OR: OutboxRepository> RetentionChecker<MR, OR> {
    pub fn new(
        monitor_repo: MR,
        outbox_repo: OR,
        interval: Duration,
        check_in_retention_days: i64,
        outbox_retention_days: i64,
    ) -> Self {
        Self {
            monitor_repo,
            outbox_repo,
            interval,
            check_in_retention_days,
            outbox_retention_days,
        }
    }

    pub async fn run(&self, shutdown_token: CancellationToken) {
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("retention checker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.interval) => {
                    self.run_once().await;
                }
            }
        }
    }

    pub async fn run_once(&self) {
        if let Some(cutoff) = cutoff(self.check_in_retention_days) {
            match self.monitor_repo.prune_old_check_ins(cutoff).await {
                Ok(pruned) => tracing::info!(pruned, "check-in retention prune complete"),
                Err(e) => tracing::error!(error = %e, "check-in retention prune failed"),
            }
        }
        if let Some(cutoff) = cutoff(self.outbox_retention_days) {
            match self.outbox_repo.prune_old_outbox_entries(cutoff).await {
                Ok(pruned) => tracing::info!(pruned, "outbox retention prune complete"),
                Err(e) => tracing::error!(error = %e, "outbox retention prune failed"),
            }
        }
    }
}

fn cutoff(retention_days: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    (retention_days > 0).then(|| chrono::Utc::now() - chrono::Duration::days(retention_days))
}
