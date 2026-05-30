use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::core::{
    domain::{DispatchError, IntegrationConfig, IntegrationId},
    ports::{IntegrationRepository, NotificationDispatcher, OutboxRepository},
};

pub struct DispatcherMap {
    ntfy: Box<dyn NotificationDispatcher>,
    gotify: Box<dyn NotificationDispatcher>,
    slack: Box<dyn NotificationDispatcher>,
}

impl DispatcherMap {
    pub fn new(
        ntfy: impl NotificationDispatcher + 'static,
        gotify: impl NotificationDispatcher + 'static,
        slack: impl NotificationDispatcher + 'static,
    ) -> Self {
        Self {
            ntfy: Box::new(ntfy),
            gotify: Box::new(gotify),
            slack: Box::new(slack),
        }
    }

    async fn dispatch(
        &self,
        config: &IntegrationConfig,
        message: &str,
    ) -> Result<(), DispatchError> {
        match config {
            IntegrationConfig::Ntfy(_) => self.ntfy.dispatch(config, message).await,
            IntegrationConfig::Gotify(_) => self.gotify.dispatch(config, message).await,
            IntegrationConfig::Slack(_) => self.slack.dispatch(config, message).await,
            IntegrationConfig::Email(_) => Err(DispatchError::Permanent(
                "email dispatch not yet implemented".to_string(),
            )),
        }
    }
}

pub struct NotificationService<IR, OR> {
    outbox_repo: OR,
    integration_repo: IR,
    dispatchers: DispatcherMap,
    max_retries: u32,
    poll_interval: Duration,
}

impl<IR, OR> NotificationService<IR, OR>
where
    IR: IntegrationRepository,
    OR: OutboxRepository,
{
    pub fn new(
        outbox_repo: OR,
        integration_repo: IR,
        dispatchers: DispatcherMap,
        poll_interval: Duration,
    ) -> Self {
        Self {
            outbox_repo,
            integration_repo,
            dispatchers,
            max_retries: 3,
            poll_interval,
        }
    }

    pub async fn run(&self, shutdown_token: CancellationToken) {
        match self.outbox_repo.reset_stale_sending().await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!(count = %count, "reset stale sending outbox entries");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to reset stale sending entries");
            }
        }

        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    tracing::info!("notification worker shutting down");
                    break;
                }
                _ = tokio::time::sleep(self.poll_interval) => {
                    self.process_batch().await;
                }
            }
        }
    }

    /// Run a single batch of outbox processing (useful for testing).
    pub async fn run_once(&self) {
        self.process_batch().await;
    }

    async fn process_batch(&self) {
        let entries = match self.outbox_repo.fetch_pending(10).await {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = %e, "failed to fetch pending outbox entries");
                return;
            }
        };

        for entry in entries {
            if let Err(e) = self.outbox_repo.claim_sending(&entry.id).await {
                tracing::error!(entry_id = %entry.id, error = %e, "failed to claim outbox entry");
                continue;
            }

            let integration_id = match entry.integration_id.parse::<uuid::Uuid>() {
                Ok(u) => IntegrationId::from_uuid(u),
                Err(_) => {
                    tracing::error!(entry_id = %entry.id, "invalid integration_id in outbox");
                    let _ = self.outbox_repo.mark_failed(&entry.id).await;
                    continue;
                }
            };

            let integration = match self.integration_repo.get_integration(integration_id).await {
                Ok(Some(i)) => i,
                Ok(None) => {
                    tracing::warn!(entry_id = %entry.id, "integration not found, marking failed");
                    let _ = self.outbox_repo.mark_failed(&entry.id).await;
                    continue;
                }
                Err(e) => {
                    tracing::error!(entry_id = %entry.id, error = %e, "failed to fetch integration");
                    let _ = self.outbox_repo.retry_later(&entry.id).await;
                    continue;
                }
            };

            match self
                .dispatchers
                .dispatch(&integration.config, &entry.message)
                .await
            {
                Ok(()) => {
                    tracing::info!(entry_id = %entry.id, "notification sent");
                    if let Err(e) = self.outbox_repo.mark_sent(&entry.id).await {
                        tracing::error!(entry_id = %entry.id, error = %e, "failed to mark sent");
                    }
                }
                Err(DispatchError::Permanent(msg)) => {
                    tracing::warn!(entry_id = %entry.id, error = %msg, "permanent dispatch failure");
                    if let Err(e) = self.outbox_repo.mark_failed(&entry.id).await {
                        tracing::error!(entry_id = %entry.id, error = %e, "failed to mark failed");
                    }
                }
                Err(DispatchError::Transient(msg)) => {
                    tracing::warn!(entry_id = %entry.id, error = %msg, "transient dispatch failure");
                    let current_retries = entry.retry_count as u32;
                    if current_retries + 1 >= self.max_retries {
                        tracing::warn!(entry_id = %entry.id, retries = %current_retries, "max retries reached, marking failed");
                        if let Err(e) = self.outbox_repo.mark_failed(&entry.id).await {
                            tracing::error!(entry_id = %entry.id, error = %e, "failed to mark failed");
                        }
                    } else if let Err(e) = self.outbox_repo.retry_later(&entry.id).await {
                        tracing::error!(entry_id = %entry.id, error = %e, "failed to retry later");
                    }
                }
            }
        }
    }
}
