use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::core::{
    domain::CheckInOutcome, monitor_service::MonitorService, ports::MonitorRepository,
};

pub struct MonitorChecker<R: MonitorRepository> {
    monitor_service: MonitorService<R>,
    check_interval: Duration,
}

impl<R: MonitorRepository> MonitorChecker<R> {
    pub fn new(monitor_service: MonitorService<R>, check_interval: Duration) -> Self {
        Self {
            monitor_service,
            check_interval,
        }
    }

    pub async fn run(&self, shutdown_token: CancellationToken) {
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("monitor checker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.check_interval) => {
                    self.run_once().await;
                }
            }
        }
    }

    pub async fn run_once(&self) {
        let missed = match self.monitor_service.find_missed_monitors().await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(error = %e, "failed to find missed monitors");
                return;
            }
        };

        for monitor_id in missed {
            match self
                .monitor_service
                .check_in(monitor_id.clone(), CheckInOutcome::Failure)
                .await
            {
                Ok(()) => {
                    tracing::info!(monitor_id = %monitor_id.as_uuid(), "monitor missed");
                }
                Err(e) => {
                    tracing::error!(monitor_id = %monitor_id.as_uuid(), error = %e, "failed to check_in missed monitor");
                }
            }
        }
    }
}
