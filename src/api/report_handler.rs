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
    pub timezone: Option<String>,
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
    let report = state.monitor_service.status_report().await?;

    let monitors = report
        .monitors
        .into_iter()
        .map(|entry| MonitorReportEntry {
            id: entry.id,
            name: entry.name,
            slug: entry.slug,
            status: entry.status.to_string(),
            schedule_type: entry.schedule_type,
            cron_expr: entry.cron_expr,
            interval_seconds: entry.interval_seconds,
            grace_seconds: entry.grace_seconds,
            last_pinged_at: entry.last_pinged_at.map(|dt| dt.to_rfc3339()),
            next_expected_at: entry.next_expected_at.map(|dt| dt.to_rfc3339()),
            created_at: entry.created_at.to_rfc3339(),
            timezone: entry.timezone,
            integrations: entry.integrations,
            outbox_pending: entry.outbox_pending,
        })
        .collect();

    Ok(Json(ReportResponse {
        total: report.total,
        healthy: report.healthy,
        missed: report.missed,
        paused: report.paused,
        monitors,
    }))
}
