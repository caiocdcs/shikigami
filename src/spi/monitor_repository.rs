use sqlx::{FromRow, SqlitePool};

use crate::core::{
    domain::{
        Integration, Monitor, MonitorError,
        integration::{IntegrationChannel, IntegrationConfig, IntegrationId, IntegrationStatus},
        monitor::{MonitorId, MonitorStatus, NewMonitor, ScheduleType},
    },
    ports::monitor_repository::MonitorRepository,
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
            Ok(ScheduleType::Cron { cron_expr })
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
            r#"SELECT id as "id!", name as "name!", description, slug as "slug!", status as "status!", schedule_type as "schedule_type!", cron_expr, interval_seconds, grace_seconds as "grace_seconds!", last_pinged_at, next_expected_at, created_at as "created_at!" FROM monitors"#
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
            r#"SELECT id as "id!", name as "name!", description, slug as "slug!", status as "status!", schedule_type as "schedule_type!", cron_expr, interval_seconds, grace_seconds as "grace_seconds!", last_pinged_at, next_expected_at, created_at as "created_at!" FROM monitors WHERE id = ?"#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        row.map(Monitor::try_from).transpose()
    }

    async fn new_monitor(&self, monitor: NewMonitor) -> Result<Monitor, MonitorError> {
        monitor.schedule_type.validate()?;

        let id = MonitorId::new();
        let id_str = id.as_uuid().to_string();
        let schedule_type_str = monitor.schedule_type.to_string();
        let (cron_expr, interval_seconds) = match &monitor.schedule_type {
            ScheduleType::Cron { cron_expr } => (Some(cron_expr.clone()), None),
            ScheduleType::Interval { interval_seconds } => (None, Some(*interval_seconds)),
        };

        sqlx::query!(
            r#"INSERT INTO monitors (id, name, description, slug, status, schedule_type, cron_expr, interval_seconds, grace_seconds, last_pinged_at, next_expected_at, created_at) VALUES (?, ?, ?, ?, 'active', ?, ?, ?, ?, NULL, NULL, datetime('now'))"#,
            id_str,
            monitor.name,
            monitor.description,
            monitor.slug,
            schedule_type_str,
            cron_expr,
            interval_seconds,
            monitor.grace_seconds
        )
        .execute(&self.pool)
        .await
        .map_err(MonitorError::map_sqlx_error)?;

        Ok(Monitor::new(
            id,
            monitor.name,
            monitor.description,
            monitor.slug,
            monitor.schedule_type,
            MonitorStatus::Active,
            monitor.grace_seconds,
            None,
            None,
            chrono::Utc::now(),
        ))
    }

    async fn delete_monitor(&self, monitor_id: MonitorId) -> Result<(), MonitorError> {
        let id_str = monitor_id.as_uuid().to_string();

        sqlx::query!(r#"DELETE FROM monitors WHERE id = ?"#, id_str)
            .execute(&self.pool)
            .await
            .map_err(MonitorError::map_sqlx_error)?;

        Ok(())
    }

    async fn update_monitor(&self, monitor: Monitor) -> Result<(), MonitorError> {
        let id_str = monitor.id.as_uuid().to_string();
        let schedule_type_str = monitor.schedule_type.to_string();
        let status_str = monitor.status.to_string();
        let (cron_expr, interval_seconds) = match &monitor.schedule_type {
            ScheduleType::Cron { cron_expr } => (Some(cron_expr.clone()), None),
            ScheduleType::Interval { interval_seconds } => (None, Some(*interval_seconds)),
        };
        let last_pinged_at = monitor
            .last_pinged_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
        let next_expected_at = monitor
            .next_expected_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        sqlx::query!(
            r#"UPDATE monitors SET name = ?, description = ?, slug = ?, status = ?, schedule_type = ?, cron_expr = ?, interval_seconds = ?, grace_seconds = ?, last_pinged_at = ?, next_expected_at = ? WHERE id = ?"#,
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
}
