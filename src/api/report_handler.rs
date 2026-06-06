use axum::{Json, extract::State};

use serde::Serialize;

use crate::{AppState, error::AppResult};

#[derive(Serialize)]
pub struct MonitorReportEntry {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub schedule_type: String,
    pub cron_expr: Option<String>,
    pub interval_seconds: Option<i64>,
    pub grace_seconds: i64,
    pub last_pinged_at: Option<String>,
    pub next_expected_at: Option<String>,
    pub created_at: String,
    pub integrations: i64,
    pub outbox_pending: i64,
}

#[derive(Serialize)]
pub struct ReportResponse {
    pub total: usize,
    pub healthy: usize,
    pub missed: usize,
    pub paused: usize,
    pub monitors: Vec<MonitorReportEntry>,
}

pub async fn health_report(State(state): State<AppState>) -> AppResult<Json<ReportResponse>> {
    let monitors = state.monitor_service.get_monitors().await?;

    let mut entries = Vec::with_capacity(monitors.len());
    let mut healthy = 0usize;
    let mut missed = 0usize;
    let mut paused = 0usize;

    for m in monitors {
        let mon_id = m.id.as_uuid().to_string();
        let (schedule_type, cron_expr, interval_seconds) = match &m.schedule_type {
            crate::core::domain::ScheduleType::Cron { cron_expr } => {
                ("cron".to_string(), Some(cron_expr.clone()), None)
            }
            crate::core::domain::ScheduleType::Interval { interval_seconds } => {
                ("interval".to_string(), None, Some(*interval_seconds))
            }
        };

        let integrations: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM monitor_integrations WHERE monitor_id = ?")
                .bind(&mon_id)
                .fetch_one(&state.pg_pool)
                .await
                .unwrap_or(0);

        let outbox_pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notification_outbox WHERE monitor_id = ? AND status = 'pending'",
        )
        .bind(&mon_id)
        .fetch_one(&state.pg_pool)
        .await
        .unwrap_or(0);

        match m.status {
            crate::core::domain::MonitorStatus::Active => healthy += 1,
            crate::core::domain::MonitorStatus::Missed => missed += 1,
            crate::core::domain::MonitorStatus::Paused => paused += 1,
        }

        entries.push(MonitorReportEntry {
            id: mon_id,
            name: m.name,
            slug: m.slug,
            status: m.status.to_string(),
            schedule_type,
            cron_expr,
            interval_seconds,
            grace_seconds: m.grace_seconds,
            last_pinged_at: m.last_pinged_at.map(|dt| dt.to_rfc3339()),
            next_expected_at: m.next_expected_at.map(|dt| dt.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            integrations,
            outbox_pending,
        });
    }

    Ok(Json(ReportResponse {
        total: entries.len(),
        healthy,
        missed,
        paused,
        monitors: entries,
    }))
}
