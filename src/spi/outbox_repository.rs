use sqlx::{FromRow, SqlitePool};

use crate::core::{
    domain::NotificationContent,
    ports::notification_dispatcher::{OutboxEntry, OutboxRepository},
};

#[derive(FromRow)]
struct OutboxRow {
    id: String,
    monitor_id: String,
    integration_id: String,
    message: String,
    retry_count: i32,
}

#[derive(Clone)]
pub struct SqliteOutboxRepository {
    pool: SqlitePool,
}

impl SqliteOutboxRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl OutboxRepository for SqliteOutboxRepository {
    async fn fetch_pending(&self, limit: i64) -> Result<Vec<OutboxEntry>, sqlx::Error> {
        let rows = sqlx::query_as::<_, OutboxRow>(
            r"SELECT id, monitor_id, integration_id, message, retry_count
               FROM notification_outbox
               WHERE status = 'pending'
               ORDER BY created_at ASC
               LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let notification = match serde_json::from_str::<NotificationContent>(&r.message) {
                    Ok(n) => n,
                    Err(e) => {
                        tracing::warn!(
                            entry_id = %r.id,
                            error = %e,
                            "failed to deserialize notification content, skipping"
                        );
                        return None;
                    }
                };
                Some(OutboxEntry {
                    id: r.id,
                    monitor_id: r.monitor_id,
                    integration_id: r.integration_id,
                    notification,
                    retry_count: r.retry_count,
                })
            })
            .collect())
    }

    async fn claim_sending(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notification_outbox SET status = 'sending' WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_sent(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notification_outbox SET status = 'sent' WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn mark_failed(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE notification_outbox SET status = 'failed' WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn retry_later(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE notification_outbox SET status = 'pending', retry_count = retry_count + 1 WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn reset_stale_sending(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE notification_outbox SET status = 'pending' WHERE status = 'sending'",
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn prune_old_outbox_entries(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, sqlx::Error> {
        // created_at is stored as TEXT via datetime('now'), i.e. "YYYY-MM-DD
        // HH:MM:SS" UTC. Format the cutoff the same way so the textual
        // comparison is chronological. Only terminal rows are eligible: pruning
        // pending/sending would silently drop in-flight or undelivered alerts.
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
        let result = sqlx::query(
            r"DELETE FROM notification_outbox
               WHERE status IN ('sent', 'failed') AND created_at < ?",
        )
        .bind(cutoff_str)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}
