use sqlx::{FromRow, SqlitePool};

use crate::core::{
    domain::{
        CheckIn, CheckInOutcome, CheckInsResult, Integration, Monitor, MonitorError,
        NotificationContent,
        integration::{IntegrationChannel, IntegrationConfig, IntegrationId, IntegrationStatus},
        monitor::{MonitorId, MonitorStatus, NewMonitor, ScheduleType, StatusReportEntry},
    },
    ports::MonitorRepository,
};

#[derive(FromRow)]
struct MonitorRow {
    id: String,
    name: String,
    description: Option<String>,
    slug: String,
    status: String,
    schedule_type: String,
    cron_expr: Option<String>,
    interval_seconds: Option<i64>,
    grace_seconds: i64,
    last_pinged_at: Option<String>,
    next_expected_at: Option<String>,
    created_at: String,
    timezone: Option<String>,
}

fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>, MonitorError> {
    let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")?;
    Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
        naive,
        chrono::Utc,
    ))
}

fn parse_optional_datetime(
    s: Option<&String>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, MonitorError> {
    s.map(|v| parse_datetime(v)).transpose()
}

fn parse_schedule_type(row: &MonitorRow) -> Result<ScheduleType, MonitorError> {
    match row.schedule_type.as_str() {
        "cron" => {
            let cron_expr = row.cron_expr.clone().unwrap_or_default();
            let timezone = row.timezone.clone().unwrap_or_else(|| "UTC".to_string());
            Ok(ScheduleType::Cron {
                cron_expr,
                timezone,
            })
        }
        "interval" => {
            let interval_seconds = row.interval_seconds.unwrap_or(0);
            Ok(ScheduleType::Interval { interval_seconds })
        }
        _ => Err(MonitorError::InvalidConfig("schedule_type".to_string())),
    }
}

impl TryFrom<MonitorRow> for Monitor {
    type Error = MonitorError;

    fn try_from(row: MonitorRow) -> Result<Self, Self::Error> {
        let uuid = row.id.parse::<uuid::Uuid>()?;
        let created_at = parse_datetime(&row.created_at)?;
        let last_pinged_at = parse_optional_datetime(row.last_pinged_at.as_ref())?;
        let next_expected_at = parse_optional_datetime(row.next_expected_at.as_ref())?;
        let schedule_type = parse_schedule_type(&row)?;
        let status = MonitorStatus::try_from(row.status.as_str())?;

        Ok(Monitor::new(
            MonitorId::from_uuid(uuid),
            row.name,
            row.description,
            row.slug,
            schedule_type,
            status,
            row.grace_seconds,
            last_pinged_at,
            next_expected_at,
            created_at,
        ))
    }
}

#[derive(FromRow)]
struct IntegrationRow {
    id: String,
    name: String,
    channel_type: String,
    config_json: String,
    status: String,
    created_at: String,
}

impl TryFrom<IntegrationRow> for Integration {
    type Error = MonitorError;

    fn try_from(row: IntegrationRow) -> Result<Self, Self::Error> {
        let uuid = row.id.parse::<uuid::Uuid>()?;
        let naive = chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S")?;
        let created_at =
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
        let channel = IntegrationChannel::try_from(row.channel_type.as_str())
            .map_err(|e| MonitorError::Database(e.to_string()))?;
        let status = IntegrationStatus::try_from(row.status.as_str())
            .map_err(|e| MonitorError::Database(e.to_string()))?;
        let config = IntegrationConfig::parse(&channel, &row.config_json)
            .map_err(|e| MonitorError::Database(e.to_string()))?;

        Ok(Integration::new(
            IntegrationId::from_uuid(uuid),
            row.name,
            channel,
            config,
            status,
            created_at,
        ))
    }
}

#[derive(FromRow)]
struct CheckInRow {
    id: String,
    monitor_id: String,
    checked_in_at: String,
    outcome: String,
    message: Option<String>,
}

#[derive(FromRow)]
struct StatusReportRow {
    id: String,
    name: String,
    slug: String,
    status: String,
    schedule_type: String,
    cron_expr: Option<String>,
    interval_seconds: Option<i64>,
    grace_seconds: i64,
    last_pinged_at: Option<String>,
    next_expected_at: Option<String>,
    created_at: String,
    timezone: Option<String>,
    integrations: i64,
    outbox_pending: i64,
}

#[derive(Clone)]
pub struct SqliteMonitorRepository {
    pool: SqlitePool,
}

impl SqliteMonitorRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl MonitorRepository for SqliteMonitorRepository {
    async fn get_monitors(&self) -> Result<Vec<Monitor>, MonitorError> {
        sqlx::query_as!(
            MonitorRow,
            r#"SELECT id as "id!", name as "name!", description, slug as "slug!", status as "status!", schedule_type as "schedule_type!", cron_expr, interval_seconds, grace_seconds as "grace_seconds!", last_pinged_at, next_expected_at, created_at as "created_at!", timezone FROM monitors"#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Monitor::try_from)
        .collect()
    }

    async fn get_monitor(&self, monitor_id: MonitorId) -> Result<Option<Monitor>, MonitorError> {
        let id_str = monitor_id.as_uuid().to_string();
        let row = sqlx::query_as!(
            MonitorRow,
            r#"SELECT id as "id!", name as "name!", description, slug as "slug!", status as "status!", schedule_type as "schedule_type!", cron_expr, interval_seconds, grace_seconds as "grace_seconds!", last_pinged_at, next_expected_at, created_at as "created_at!", timezone FROM monitors WHERE id = ?"#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        row.map(Monitor::try_from).transpose()
    }

    async fn get_monitor_by_slug(&self, slug: &str) -> Result<Option<Monitor>, MonitorError> {
        let row = sqlx::query_as!(
            MonitorRow,
            r#"SELECT id as "id!", name as "name!", description, slug as "slug!", status as "status!", schedule_type as "schedule_type!", cron_expr, interval_seconds, grace_seconds as "grace_seconds!", last_pinged_at, next_expected_at, created_at as "created_at!", timezone FROM monitors WHERE slug = ?"#,
            slug
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        row.map(Monitor::try_from).transpose()
    }

    async fn new_monitor(
        &self,
        monitor: NewMonitor,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Monitor, MonitorError> {
        monitor.schedule_type.validate()?;

        let id = MonitorId::new();
        let id_str = id.as_uuid().to_string();
        let schedule_type_str = monitor.schedule_type.to_string();
        let (cron_expr, interval_seconds, timezone) = match &monitor.schedule_type {
            ScheduleType::Cron {
                cron_expr,
                timezone,
            } => (Some(cron_expr.clone()), None, Some(timezone.clone())),
            ScheduleType::Interval { interval_seconds } => (None, Some(*interval_seconds), None),
        };
        let next_expected_str =
            next_expected_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        sqlx::query!
            (r#"INSERT INTO monitors (id, name, description, slug, status, schedule_type, cron_expr, interval_seconds, grace_seconds, last_pinged_at, next_expected_at, created_at, timezone) VALUES (?, ?, ?, ?, 'active', ?, ?, ?, ?, NULL, ?, datetime('now'), ?)"#,
            id_str,
            monitor.name,
            monitor.description,
            monitor.slug,
            schedule_type_str,
            cron_expr,
            interval_seconds,
            monitor.grace_seconds,
            next_expected_str,
            timezone
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        let created_at = chrono::Utc::now();

        Ok(Monitor::new(
            id,
            monitor.name,
            monitor.description,
            monitor.slug,
            monitor.schedule_type,
            MonitorStatus::Active,
            monitor.grace_seconds,
            None,
            next_expected_at,
            created_at,
        ))
    }

    async fn delete_monitor(&self, monitor_id: MonitorId) -> Result<(), MonitorError> {
        let id_str = monitor_id.as_uuid().to_string();

        let result = sqlx::query!(r#"DELETE FROM monitors WHERE id = ?"#, id_str)
            .execute(&self.pool)
            .await
            .map_err(MonitorError::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(MonitorError::NotFound(monitor_id));
        }

        Ok(())
    }

    async fn update_monitor(&self, monitor: Monitor) -> Result<(), MonitorError> {
        let id_str = monitor.id.as_uuid().to_string();
        let schedule_type_str = monitor.schedule_type.to_string();
        let status_str = monitor.status.to_string();
        let (cron_expr, interval_seconds, timezone) = match &monitor.schedule_type {
            ScheduleType::Cron {
                cron_expr,
                timezone,
            } => (Some(cron_expr.clone()), None, Some(timezone.clone())),
            ScheduleType::Interval { interval_seconds } => (None, Some(*interval_seconds), None),
        };
        let last_pinged_at = monitor
            .last_pinged_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
        let next_expected_at = monitor
            .next_expected_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        sqlx::query!(
            r#"UPDATE monitors SET name = ?, description = ?, slug = ?, status = ?, schedule_type = ?, cron_expr = ?, interval_seconds = ?, grace_seconds = ?, last_pinged_at = ?, next_expected_at = ?, timezone = ? WHERE id = ?"#,
            monitor.name,
            monitor.description,
            monitor.slug,
            status_str,
            schedule_type_str,
            cron_expr,
            interval_seconds,
            monitor.grace_seconds,
            last_pinged_at,
            next_expected_at,
            timezone,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        Ok(())
    }

    async fn link_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> Result<(), MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let integration_id_str = integration_id.as_uuid().to_string();

        sqlx::query!(
            r#"INSERT INTO monitor_integrations (monitor_id, integration_id) VALUES (?, ?)"#,
            monitor_id_str,
            integration_id_str
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        Ok(())
    }

    async fn unlink_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> Result<(), MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let integration_id_str = integration_id.as_uuid().to_string();

        sqlx::query!(
            r#"DELETE FROM monitor_integrations WHERE monitor_id = ? AND integration_id = ?"#,
            monitor_id_str,
            integration_id_str
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        Ok(())
    }

    async fn get_monitor_integrations(
        &self,
        monitor_id: MonitorId,
    ) -> Result<Vec<Integration>, MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let rows = sqlx::query_as!(
            IntegrationRow,
            r#"SELECT i.id as "id!", i.name as "name!", i.channel_type as "channel_type!", i.config_json as "config_json!", i.status as "status!", i.created_at as "created_at!" FROM integrations i JOIN monitor_integrations mi ON i.id = mi.integration_id WHERE mi.monitor_id = ?"#,
            monitor_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Integration::try_from).collect()
    }

    async fn get_check_ins(
        &self,
        monitor_id: MonitorId,
        limit: i64,
        offset: i64,
    ) -> Result<CheckInsResult, MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM check_ins WHERE monitor_id = ?")
            .bind(&monitor_id_str)
            .fetch_one(&self.pool)
            .await
            .map_err(MonitorError::map_sqlx_error)?;
        let rows = sqlx::query_as!(
            CheckInRow,
            r#"SELECT id as "id!", monitor_id as "monitor_id!", checked_in_at as "checked_in_at!", outcome as "outcome!", message FROM check_ins WHERE monitor_id = ? ORDER BY checked_in_at DESC LIMIT ? OFFSET ?"#,
            monitor_id_str,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| {
                let naive =
                    chrono::NaiveDateTime::parse_from_str(&row.checked_in_at, "%Y-%m-%d %H:%M:%S")?;
                Ok(CheckIn {
                    id: row.id,
                    monitor_id: row.monitor_id,
                    checked_in_at: chrono::DateTime::from_naive_utc_and_offset(naive, chrono::Utc),
                    outcome: CheckInOutcome::try_from(row.outcome.as_str())?,
                    message: row.message,
                })
            })
            .collect::<Result<Vec<CheckIn>, MonitorError>>()?;

        Ok(CheckInsResult {
            check_ins: items,
            total,
        })
    }

    async fn find_missed_monitors(&self) -> Result<Vec<MonitorId>, MonitorError> {
        let rows: Vec<String> = sqlx::query_scalar(
            r"SELECT id FROM monitors
               WHERE status NOT IN ('paused', 'missed')
                 AND next_expected_at IS NOT NULL
                 AND datetime(next_expected_at, '+' || grace_seconds || ' seconds') < datetime('now')",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|s| {
                let uuid = s.parse::<uuid::Uuid>()?;
                Ok(MonitorId::from_uuid(uuid))
            })
            .collect()
    }

    async fn ping(
        &self,
        monitor_id: MonitorId,
        timestamp: chrono::DateTime<chrono::Utc>,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let now_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let next_expected_str =
            next_expected_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        sqlx::query!(
            r#"UPDATE monitors SET last_pinged_at = ?, next_expected_at = ?, status = 'active' WHERE id = ?"#,
            now_str,
            next_expected_str,
            monitor_id_str
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        Ok(())
    }

    async fn check_in(
        &self,
        monitor_id: MonitorId,
        outcome: CheckInOutcome,
        timestamp: chrono::DateTime<chrono::Utc>,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
        new_status: MonitorStatus,
        message: Option<String>,
        notification: Option<NotificationContent>,
    ) -> Result<(), MonitorError> {
        let monitor_id_str = monitor_id.as_uuid().to_string();
        let now_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let next_expected_str =
            next_expected_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
        let outcome_str = outcome.to_string();
        let status_str = new_status.to_string();

        // Insert check_in record
        let check_in_id = uuid::Uuid::new_v4().to_string();
        sqlx::query!(
            r#"INSERT INTO check_ins (id, monitor_id, checked_in_at, outcome, message) VALUES (?, ?, ?, ?, ?)"#,
            check_in_id,
            monitor_id_str,
            now_str,
            outcome_str,
            message
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        // Update monitor timestamps and status (status decided by service)
        sqlx::query!(
            r#"UPDATE monitors SET last_pinged_at = ?, next_expected_at = ?, status = ? WHERE id = ?"#,
            now_str,
            next_expected_str,
            status_str,
            monitor_id_str
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        // On failure, write to notification_outbox for each linked integration
        if let Some(notification) = notification {
            let integration_ids: Vec<String> = sqlx::query_scalar!(
                r#"SELECT integration_id FROM monitor_integrations WHERE monitor_id = ?"#,
                monitor_id_str
            )
            .fetch_all(&self.pool)
            .await?;

            let notification_json =
                serde_json::to_string(&notification).map_err(MonitorError::from)?;

            for int_id in integration_ids {
                let outbox_id = uuid::Uuid::new_v4().to_string();
                sqlx::query!(
                    r#"INSERT INTO notification_outbox (id, monitor_id, integration_id, message, retry_count, status, created_at) VALUES (?, ?, ?, ?, 0, 'pending', datetime('now'))"#,
                    outbox_id,
                    monitor_id_str,
                    int_id,
                    notification_json
                )
                .execute(&self.pool)
                .await
                .map_err(MonitorError::map_sqlx_error)?;
            }
        }

        Ok(())
    }

    async fn status_report(&self) -> Result<Vec<StatusReportEntry>, MonitorError> {
        let rows = sqlx::query_as!(
            StatusReportRow,
            r#"SELECT
                m.id as "id!",
                m.name as "name!",
                m.slug as "slug!",
                m.status as "status!",
                m.schedule_type as "schedule_type!",
                m.cron_expr,
                m.interval_seconds,
                m.grace_seconds as "grace_seconds!",
                m.last_pinged_at,
                m.next_expected_at,
                m.created_at as "created_at!",
                m.timezone,
                COALESCE(mi.integrations, 0) as "integrations!",
                COALESCE(o.outbox_pending, 0) as "outbox_pending!"
            FROM monitors m
            LEFT JOIN (
                SELECT monitor_id, COUNT(*) as integrations
                FROM monitor_integrations
                GROUP BY monitor_id
            ) mi ON m.id = mi.monitor_id
            LEFT JOIN (
                SELECT monitor_id, COUNT(*) as outbox_pending
                FROM notification_outbox
                WHERE status = 'pending'
                GROUP BY monitor_id
            ) o ON m.id = o.monitor_id
            ORDER BY m.name"#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let status = MonitorStatus::try_from(row.status.as_str())?;
                let created_at = parse_datetime(&row.created_at)?;
                let last_pinged_at = parse_optional_datetime(row.last_pinged_at.as_ref())?;
                let next_expected_at = parse_optional_datetime(row.next_expected_at.as_ref())?;
                let schedule_type = row.schedule_type.clone();
                Ok(StatusReportEntry {
                    id: row.id,
                    name: row.name,
                    slug: row.slug,
                    status,
                    schedule_type,
                    cron_expr: row.cron_expr,
                    interval_seconds: row.interval_seconds,
                    grace_seconds: row.grace_seconds,
                    last_pinged_at,
                    next_expected_at,
                    created_at,
                    timezone: row.timezone,
                    integrations: row.integrations,
                    outbox_pending: row.outbox_pending,
                })
            })
            .collect()
    }
}
