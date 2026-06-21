use std::future::Future;
use std::pin::Pin;

use crate::core::domain::{DispatchError, IntegrationConfig, NotificationContent};

#[derive(Debug, Clone)]
pub struct OutboxEntry {
    pub id: String,
    pub monitor_id: String,
    pub integration_id: String,
    pub notification: NotificationContent,
    pub retry_count: i32,
}

pub trait OutboxRepository: Send + Sync + 'static {
    fn fetch_pending(
        &self,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<OutboxEntry>, sqlx::Error>> + Send;

    fn claim_sending(&self, id: &str) -> impl Future<Output = Result<(), sqlx::Error>> + Send;

    fn mark_sent(&self, id: &str) -> impl Future<Output = Result<(), sqlx::Error>> + Send;

    fn mark_failed(&self, id: &str) -> impl Future<Output = Result<(), sqlx::Error>> + Send;

    fn retry_later(&self, id: &str) -> impl Future<Output = Result<(), sqlx::Error>> + Send;

    fn reset_stale_sending(&self) -> impl Future<Output = Result<u64, sqlx::Error>> + Send;

    /// Delete terminal outbox entries (status `sent` or `failed`) older than
    /// `cutoff`. Pending and sending rows are never pruned: they represent
    /// in-flight or undelivered alerts. Returns the number of pruned rows.
    fn prune_old_outbox_entries(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = Result<u64, sqlx::Error>> + Send;
}

pub trait NotificationDispatcher: Send + Sync + 'static {
    fn dispatch(
        &self,
        config: &IntegrationConfig,
        notification: &NotificationContent,
    ) -> Pin<Box<dyn Future<Output = Result<(), DispatchError>> + Send>>;
}
