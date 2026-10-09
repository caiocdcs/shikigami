use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};
use chrono::{DateTime, Utc};

use crate::{
    AppState,
    core::domain::{MonitorStatus, ScheduleType},
    error::AppError,
};

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

/// "90" -> "1m 30s", "3600" -> "1h". Compact, for schedules and grace periods.
fn format_duration(total_seconds: i64) -> String {
    let (d, rem) = (total_seconds / 86_400, total_seconds % 86_400);
    let (h, rem) = (rem / 3600, rem % 3600);
    let (m, s) = (rem / 60, rem % 60);
    let parts: Vec<String> = [(d, "d"), (h, "h"), (m, "m"), (s, "s")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, unit)| format!("{n}{unit}"))
        .collect();
    if parts.is_empty() {
        "0s".to_string()
    } else {
        parts.join(" ")
    }
}

/// Schedule as (human text, optional cron expression shown as code).
fn format_schedule(
    schedule_type: &str,
    cron_expr: Option<String>,
    timezone: Option<String>,
    interval_seconds: Option<i64>,
) -> (String, Option<String>) {
    match schedule_type {
        "cron" => {
            let label = match timezone {
                Some(tz) if !tz.is_empty() => format!("in {tz}"),
                _ => "in UTC".to_string(),
            };
            (label, cron_expr)
        }
        "interval" => (
            format!("Every {}", format_duration(interval_seconds.unwrap_or(0))),
            None,
        ),
        other => (other.to_string(), None),
    }
}

/// Words shown for a status. `status` stays the raw value for CSS classes.
fn status_label(status: MonitorStatus) -> &'static str {
    match status {
        MonitorStatus::Active => "Checking in",
        MonitorStatus::Missed => "Missed",
        MonitorStatus::Paused => "Paused",
    }
}

/// Problems first, then healthy, then paused; alphabetical within a group.
fn status_rank(status: MonitorStatus) -> u8 {
    match status {
        MonitorStatus::Missed => 0,
        MonitorStatus::Active => 1,
        MonitorStatus::Paused => 2,
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// The page headline: one sentence answering "is anything broken?".
/// Returns (sentence, tone) where tone is `ok`, `missed`, `paused` or `empty`.
fn verdict(total: usize, healthy: usize, missed: usize, paused: usize) -> (String, &'static str) {
    if total == 0 {
        ("No monitors yet.".to_string(), "empty")
    } else if missed == 1 {
        (
            format!("1 of {total} monitors missed its check-in."),
            "missed",
        )
    } else if missed > 1 {
        (
            format!("{missed} of {total} monitors missed their check-in."),
            "missed",
        )
    } else if healthy == 0 {
        ("Every monitor is paused.".to_string(), "paused")
    } else if paused == 0 {
        let subject = plural(healthy, "monitor is", "monitors are");
        let subject = if healthy == 1 {
            subject
        } else {
            format!("All {subject}")
        };
        (format!("{subject} checking in."), "ok")
    } else {
        (
            format!(
                "{} checking in.",
                plural(healthy, "monitor is", "monitors are")
            ),
            "ok",
        )
    }
}

#[derive(Template)]
#[template(path = "status.html")]
struct StatusPage {
    verdict: String,
    tone: &'static str,
    breakdown: String,
    missed: usize,
    updated_at: String,
    updated_absolute: String,
    monitors: Vec<MonitorView>,
}

struct MonitorView {
    name: String,
    slug: String,
    status: String,
    status_label: &'static str,
    schedule_label: String,
    cron_expr: Option<String>,
    last_ping_display: String,
    last_ping_absolute: Option<String>,
    next_expected_display: String,
    next_expected_absolute: Option<String>,
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
    status_label: &'static str,
    summary: String,
    schedule_label: String,
    cron_expr: Option<String>,
    grace: String,
    last_ping_display: String,
    last_ping_absolute: Option<String>,
    next_expected_display: String,
    next_expected_absolute: Option<String>,
    integrations: Vec<IntegrationView>,
    check_ins: Vec<CheckInView>,
}

struct CheckInView {
    when: String,
    when_absolute: String,
    outcome: String,
    message: Option<String>,
}

pub async fn status_index(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let report = state.monitor_service.status_report().await?;
    let now = Utc::now();

    let mut entries = report.monitors;
    entries.sort_by(|a, b| {
        status_rank(a.status)
            .cmp(&status_rank(b.status))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let monitors = entries
        .into_iter()
        .map(|e| {
            let (last_ping_display, last_ping_absolute) =
                display_time(e.last_pinged_at, now, "never");
            let (next_expected_display, next_expected_absolute) =
                display_time(e.next_expected_at, now, "unknown");
            let (schedule_label, cron_expr) = format_schedule(
                &e.schedule_type,
                e.cron_expr,
                e.timezone,
                e.interval_seconds,
            );
            MonitorView {
                name: e.name,
                slug: e.slug,
                status_label: status_label(e.status),
                status: e.status.to_string(),
                schedule_label,
                cron_expr,
                last_ping_display,
                last_ping_absolute,
                next_expected_display,
                next_expected_absolute,
            }
        })
        .collect();

    let (verdict, tone) = verdict(report.total, report.healthy, report.missed, report.paused);
    let breakdown = format!(
        "{} checking in, {} missed, {} paused.",
        report.healthy, report.missed, report.paused
    );

    let page = StatusPage {
        verdict,
        tone,
        breakdown,
        missed: report.missed,
        updated_at: now.format("%H:%M UTC").to_string(),
        updated_absolute: now.to_rfc3339(),
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

    let (schedule_label, cron_expr) = match &monitor.schedule_type {
        ScheduleType::Cron {
            cron_expr,
            timezone,
        } => format_schedule(
            "cron",
            Some(cron_expr.clone()),
            Some(timezone.clone()),
            None,
        ),
        ScheduleType::Interval { interval_seconds } => {
            format_schedule("interval", None, None, Some(*interval_seconds))
        }
    };

    let now = Utc::now();
    let (last_ping_display, last_ping_absolute) =
        display_time(monitor.last_pinged_at, now, "never");
    let (next_expected_display, next_expected_absolute) =
        display_time(monitor.next_expected_at, now, "unknown");

    let summary = match monitor.status {
        MonitorStatus::Active => "Its last check-in arrived on time.".to_string(),
        MonitorStatus::Missed => {
            "Missed its last check-in. Linked integrations were notified.".to_string()
        }
        MonitorStatus::Paused => {
            "Paused. Check-ins are still recorded, but it won't be marked missed or send alerts."
                .to_string()
        }
    };

    let page = MonitorPage {
        name: monitor.name,
        slug: monitor.slug,
        status_label: status_label(monitor.status),
        status: monitor.status.to_string(),
        summary,
        schedule_label,
        cron_expr,
        grace: format_duration(monitor.grace_seconds),
        last_ping_display,
        last_ping_absolute,
        next_expected_display,
        next_expected_absolute,
        integrations,
        check_ins: check_ins
            .check_ins
            .into_iter()
            .map(|c| CheckInView {
                when: format!(
                    "{} ({})",
                    c.checked_in_at.format("%b %-d, %H:%M UTC"),
                    format_relative(c.checked_in_at, now)
                ),
                when_absolute: c.checked_in_at.to_rfc3339(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_is_compact() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(86_400 + 7200), "1d 2h");
    }

    #[test]
    fn verdict_prioritises_missed() {
        assert_eq!(verdict(0, 0, 0, 0).1, "empty");
        assert_eq!(
            verdict(5, 4, 1, 0).0,
            "1 of 5 monitors missed its check-in."
        );
        assert_eq!(
            verdict(5, 2, 2, 1).0,
            "2 of 5 monitors missed their check-in."
        );
        assert_eq!(verdict(3, 3, 0, 0).0, "All 3 monitors are checking in.");
        assert_eq!(verdict(1, 1, 0, 0).0, "1 monitor is checking in.");
        assert_eq!(verdict(4, 3, 0, 1).0, "3 monitors are checking in.");
        assert_eq!(verdict(2, 0, 0, 2).1, "paused");
    }
}
