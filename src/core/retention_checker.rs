use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::core::{domain::MonitorError, ports::MonitorRepository};

/// Background worker that prunes old `check_ins` rows to bound `SQLite` growth.
///
/// Mirrors `MonitorChecker`: owns its own timer + shutdown token so the prune
/// cadence stays decoupled from missed-monitor detection. The cutoff is a
/// business decision computed here (`now - retention_days`); the repository only
/// executes the delete with the supplied timestamp.
pub struct RetentionChecker<R: MonitorRepository> {
    repo: R,
    interval: Duration,
    retention_days: i64,
}

impl<R: MonitorRepository> RetentionChecker<R> {
    pub fn new(repo: R, interval: Duration, retention_days: i64) -> Self {
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
        match self.prune().await {
            Ok(pruned) => {
                tracing::info!(pruned, "retention prune complete");
            }
            Err(e) => {
                tracing::error!(error = %e, "retention prune failed");
            }
        }
    }

    async fn prune(&self) -> Result<u64, MonitorError> {
        if self.retention_days <= 0 {
            // Retention disabled. Lets operators turn pruning off via config
            // without a code change.
            return Ok(0);
        }
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.retention_days);
        self.repo.prune_old_check_ins(cutoff).await
    }
}

#[cfg(test)]
mod tests {
    // Worker logic is exercised at the repo/integration level (see
    // tests/notification_tests.rs). The worker itself is pure orchestration:
    // compute cutoff, call repo, log. No domain computation to unit-test here.
}
