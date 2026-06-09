use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::core::domain::Monitor;
use crate::core::domain::ScheduleType;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateMonitorDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    #[validate(
        length(min = 1, max = 50),
        custom(function = "crate::core::domain::monitor::validate_slug")
    )]
    pub slug: String,
    pub schedule_type: String,
    pub cron_expr: Option<String>,
    pub interval_seconds: Option<i64>,
    #[validate(range(min = 1))]
    pub grace_seconds: i64,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMonitorDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    #[validate(
        length(min = 1, max = 50),
        custom(function = "crate::core::domain::monitor::validate_slug")
    )]
    pub slug: String,
    pub schedule_type: String,
    pub cron_expr: Option<String>,
    pub interval_seconds: Option<i64>,
    #[validate(range(min = 1))]
    pub grace_seconds: i64,
    pub status: String,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkIntegrationDto {
    pub integration_id: String,
}

#[derive(Debug, Serialize)]
pub struct MonitorResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
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
}

impl From<Monitor> for MonitorResponse {
    fn from(monitor: Monitor) -> Self {
        let (schedule_type, cron_expr, interval_seconds, timezone) = match &monitor.schedule_type {
            ScheduleType::Cron {
                cron_expr,
                timezone,
            } => (
                "cron".to_string(),
                Some(cron_expr.clone()),
                None,
                Some(timezone.clone()),
            ),
            ScheduleType::Interval { interval_seconds } => {
                ("interval".to_string(), None, Some(*interval_seconds), None)
            }
        };
        Self {
            id: monitor.id.as_uuid().to_string(),
            name: monitor.name,
            description: monitor.description,
            slug: monitor.slug,
            status: monitor.status.to_string(),
            schedule_type,
            cron_expr,
            interval_seconds,
            grace_seconds: monitor.grace_seconds,
            last_pinged_at: monitor.last_pinged_at.map(|dt| dt.to_rfc3339()),
            next_expected_at: monitor.next_expected_at.map(|dt| dt.to_rfc3339()),
            created_at: monitor.created_at.to_rfc3339(),
            timezone,
        }
    }
}
