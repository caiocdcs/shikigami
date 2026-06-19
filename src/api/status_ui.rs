use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};
use chrono::{DateTime, Utc};

use crate::{AppState, error::AppError};

fn format_relative(dt: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let diff = now.signed_duration_since(dt);
    if diff.num_seconds() < 0 {
        let future = -diff;
        if future.num_minutes() < 1 {
            "in <1m".to_string()
        } else if future.num_hours() < 1 {
            format!("in {}m", future.num_minutes())
        } else if future.num_days() < 1 {
            format!("in {}h", future.num_hours())
        } else {
            format!("in {}d", future.num_days())
        }
    } else if diff.num_minutes() < 1 {
        "<1m ago".to_string()
    } else if diff.num_hours() < 1 {
        format!("{}m ago", diff.num_minutes())
    } else if diff.num_days() < 1 {
        format!("{}h ago", diff.num_hours())
    } else {
        format!("{}d ago", diff.num_days())
    }
}

fn display_time(
    dt: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    absent: &str,
) -> (String, Option<String>) {
    match dt {
        Some(t) => (format_relative(t, now), Some(t.to_rfc3339())),
        None => (absent.to_string(), None),
    }
}

fn format_schedule_display(
    schedule_type: String,
    cron_expr: Option<String>,
    timezone: Option<String>,
    interval_seconds: Option<i64>,
) -> String {
    match schedule_type.as_str() {
        "cron" => {
            let expr = cron_expr.unwrap_or_default();
            match timezone {
                Some(tz) if !tz.is_empty() => format!("{expr} ({tz})"),
                _ => expr,
            }
        }
        "interval" => {
            format!("every {}s", interval_seconds.unwrap_or(0))
        }
        _ => schedule_type,
    }
}

#[derive(Template)]
#[template(path = "status.html")]
struct StatusPage {
    total: usize,
    healthy: usize,
    missed: usize,
    paused: usize,
    updated_at: String,
    monitors: Vec<MonitorView>,
}

struct MonitorView {
    name: String,
    slug: String,
    status: String,
    schedule_display: String,
    last_ping_display: String,
    last_ping_absolute: Option<String>,
    next_expected_display: String,
    next_expected_absolute: Option<String>,
    integrations: i64,
}

struct IntegrationView {
    name: String,
    channel: String,
}

#[derive(Template)]
#[template(path = "monitor.html")]
struct MonitorPage {
    name: String,
    slug: String,
    status: String,
    schedule_display: String,
    grace_seconds: i64,
    last_ping_display: String,
    last_ping_absolute: Option<String>,
    next_expected_display: String,
    next_expected_absolute: Option<String>,
    integrations: Vec<IntegrationView>,
    check_ins: Vec<CheckInView>,
}

struct CheckInView {
    checked_in_at: String,
    outcome: String,
    message: Option<String>,
}

pub async fn status_index(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let report = state.monitor_service.status_report().await?;
    let now = Utc::now();

    let monitors = report
        .monitors
        .into_iter()
        .map(|e| {
            let (last_ping_display, last_ping_absolute) = display_time(e.last_pinged_at, now, "—");
            let (next_expected_display, next_expected_absolute) =
                display_time(e.next_expected_at, now, "—");
            MonitorView {
                name: e.name,
                slug: e.slug,
                status: e.status.to_string(),
                schedule_display: format_schedule_display(
                    e.schedule_type,
                    e.cron_expr,
                    e.timezone,
                    e.interval_seconds,
                ),
                last_ping_display,
                last_ping_absolute,
                next_expected_display,
                next_expected_absolute,
                integrations: e.integrations,
            }
        })
        .collect();

    let page = StatusPage {
        total: report.total,
        healthy: report.healthy,
        missed: report.missed,
        paused: report.paused,
        updated_at: now.to_rfc3339(),
        monitors,
    };

    let html = page
        .render()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Html(html))
}

pub async fn monitor_detail(
    Path(slug): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let monitor = state
        .monitor_service
        .get_monitor_by_slug(&slug)
        .await?
        .ok_or(AppError::NotFound)?;

    let check_ins = state
        .monitor_service
        .get_check_ins(monitor.id.clone(), 10, 0)
        .await?;

    let integrations = state
        .monitor_service
        .get_monitor_integrations(monitor.id.clone())
        .await?
        .into_iter()
        .map(|i| IntegrationView {
            name: i.name,
            channel: i.channel.to_string(),
        })
        .collect();

    let (cron_expr, timezone, interval_seconds) = match &monitor.schedule_type {
        crate::core::domain::ScheduleType::Cron {
            cron_expr: expr,
            timezone: tz,
        } => (
            Some(expr.clone()),
            (!tz.is_empty()).then(|| tz.clone()),
            None,
        ),
        crate::core::domain::ScheduleType::Interval {
            interval_seconds: secs,
        } => (None, None, Some(*secs)),
    };
    let schedule_display = format_schedule_display(
        monitor.schedule_type.to_string(),
        cron_expr,
        timezone,
        interval_seconds,
    );

    let now = Utc::now();
    let (last_ping_display, last_ping_absolute) =
        display_time(monitor.last_pinged_at, now, "never");
    let (next_expected_display, next_expected_absolute) =
        display_time(monitor.next_expected_at, now, "unknown");

    let page = MonitorPage {
        name: monitor.name,
        slug: monitor.slug,
        status: monitor.status.to_string(),
        schedule_display,
        grace_seconds: monitor.grace_seconds,
        last_ping_display,
        last_ping_absolute,
        next_expected_display,
        next_expected_absolute,
        integrations,
        check_ins: check_ins
            .check_ins
            .into_iter()
            .map(|c| CheckInView {
                checked_in_at: c.checked_in_at.to_rfc3339(),
                outcome: c.outcome.to_string(),
                message: c.message,
            })
            .collect(),
    };

    let html = page
        .render()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Html(html))
}
