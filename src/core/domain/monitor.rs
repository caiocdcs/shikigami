use std::fmt::Display;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone)]
pub struct MonitorId(Uuid);

impl Default for MonitorId {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct NewMonitor {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub slug: String,
    pub schedule_type: ScheduleType,
    #[validate(range(min = 1))]
    pub grace_seconds: i64,
}

#[derive(Debug, Clone, Validate)]
pub struct Monitor {
    pub id: MonitorId,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub slug: String,
    pub schedule_type: ScheduleType,
    pub status: MonitorStatus,
    pub grace_seconds: i64,
    pub last_pinged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Monitor {
    pub fn new(
        id: MonitorId,
        name: String,
        description: Option<String>,
        slug: String,
        schedule_type: ScheduleType,
        status: MonitorStatus,
        grace_seconds: i64,
        last_pinged_at: Option<chrono::DateTime<chrono::Utc>>,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            slug,
            schedule_type,
            status,
            grace_seconds,
            last_pinged_at,
            next_expected_at,
            created_at,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ScheduleType {
    Cron { cron_expr: String, timezone: String },
    Interval { interval_seconds: i64 },
}

impl ScheduleType {
    pub fn validate(&self) -> Result<(), MonitorError> {
        match self {
            ScheduleType::Cron {
                cron_expr,
                timezone,
            } => {
                if cron_expr.is_empty() {
                    return Err(MonitorError::InvalidConfig(
                        "cron_expr must not be empty".to_string(),
                    ));
                }
                timezone.parse::<chrono_tz::Tz>().map_err(|_| {
                    MonitorError::InvalidConfig(format!("invalid timezone '{timezone}'"))
                })?;
                Ok(())
            }
            ScheduleType::Interval { interval_seconds } => {
                if *interval_seconds <= 0 {
                    return Err(MonitorError::InvalidConfig(
                        "interval_seconds must be greater than 0".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Compute the next expected occurrence after `from`.
    ///
    /// For interval schedules this is simply `from + interval_seconds`.
    /// For cron schedules this uses the cron expression parser.
    pub fn next_occurrence_after(
        &self,
        from: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>, MonitorError> {
        match self {
            ScheduleType::Interval { interval_seconds } => {
                Ok(Some(*from + chrono::Duration::seconds(*interval_seconds)))
            }
            ScheduleType::Cron {
                cron_expr,
                timezone,
            } => {
                let tz: chrono_tz::Tz = timezone.parse().map_err(|_| {
                    MonitorError::InvalidConfig(format!("invalid timezone '{timezone}'"))
                })?;
                let cron = croner::Cron::from_str(cron_expr).map_err(|e| {
                    MonitorError::InvalidConfig(format!(
                        "invalid cron expression '{cron_expr}': {e}"
                    ))
                })?;
                let from_local = from.with_timezone(&tz);
                let next = cron.find_next_occurrence(&from_local, false).map_err(|e| {
                    MonitorError::InvalidConfig(format!(
                        "cron evaluation error for '{cron_expr}': {e}"
                    ))
                })?;
                Ok(Some(next.with_timezone(&chrono::Utc)))
            }
        }
    }
}

impl TryFrom<&str> for ScheduleType {
    type Error = MonitorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "cron" => Ok(ScheduleType::Cron {
                cron_expr: String::new(),
                timezone: "UTC".to_string(),
            }),
            "interval" => Ok(ScheduleType::Interval {
                interval_seconds: 0,
            }),
            _ => Err(MonitorError::InvalidConfig("schedule_type".to_string())),
        }
    }
}

impl Display for ScheduleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleType::Cron { .. } => write!(f, "cron"),
            ScheduleType::Interval { .. } => write!(f, "interval"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum MonitorStatus {
    Active,
    Paused,
    Missed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CheckInOutcome {
    Success,
    Failure,
}

impl Display for CheckInOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckInOutcome::Success => write!(f, "success"),
            CheckInOutcome::Failure => write!(f, "failure"),
        }
    }
}

impl TryFrom<&str> for CheckInOutcome {
    type Error = MonitorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "success" => Ok(CheckInOutcome::Success),
            "failure" => Ok(CheckInOutcome::Failure),
            _ => Err(MonitorError::InvalidConfig(
                "outcome must be 'success' or 'failure'".to_string(),
            )),
        }
    }
}

impl TryFrom<&str> for MonitorStatus {
    type Error = MonitorError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(MonitorStatus::Active),
            "paused" => Ok(MonitorStatus::Paused),
            "missed" => Ok(MonitorStatus::Missed),
            _ => Err(MonitorError::InvalidConfig("status".to_string())),
        }
    }
}

impl Display for MonitorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorStatus::Active => write!(f, "active"),
            MonitorStatus::Paused => write!(f, "paused"),
            MonitorStatus::Missed => write!(f, "missed"),
        }
    }
}

#[derive(Debug)]
pub enum MonitorError {
    InvalidConfig(String),
    NotFound(MonitorId),
    Conflict(String),
    Database(String),
}

impl std::error::Error for MonitorError {}

impl Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorError::InvalidConfig(field) => write!(f, "Invalid config: {field}"),
            MonitorError::NotFound(id) => write!(f, "Monitor not found: {}", id.as_uuid()),
            MonitorError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            MonitorError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl MonitorError {
    #[allow(clippy::needless_pass_by_value)]
    pub fn map_sqlx_error(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                Some("2067") => MonitorError::Conflict("duplicate entry".to_string()),
                Some("787") => {
                    MonitorError::InvalidConfig("referenced record not found".to_string())
                }
                _ => MonitorError::Database(e.to_string()),
            },
            _ => MonitorError::Database(e.to_string()),
        }
    }
}

impl From<sqlx::Error> for MonitorError {
    fn from(error: sqlx::Error) -> Self {
        Self::map_sqlx_error(error)
    }
}

impl From<serde_json::Error> for MonitorError {
    fn from(error: serde_json::Error) -> Self {
        MonitorError::InvalidConfig(error.to_string())
    }
}

impl From<uuid::Error> for MonitorError {
    fn from(error: uuid::Error) -> Self {
        MonitorError::Database(format!("invalid uuid: {error}"))
    }
}

impl From<chrono::ParseError> for MonitorError {
    fn from(error: chrono::ParseError) -> Self {
        MonitorError::Database(format!("invalid datetime: {error}"))
    }
}

impl From<validator::ValidationErrors> for MonitorError {
    fn from(errors: validator::ValidationErrors) -> Self {
        MonitorError::InvalidConfig(errors.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .and_utc()
    }

    #[test]
    fn next_occurrence_after_cron_evaluates_in_local_timezone() {
        // 9am daily in Sao_Paulo (UTC-3) == 12:00 UTC.
        let schedule = ScheduleType::Cron {
            cron_expr: "0 9 * * *".to_string(),
            timezone: "America/Sao_Paulo".to_string(),
        };
        let from = utc("2026-06-09 00:00:00");
        let next = schedule.next_occurrence_after(&from).unwrap().unwrap();
        assert_eq!(next, utc("2026-06-09 12:00:00"));
    }

    #[test]
    fn next_occurrence_after_cron_defaults_to_utc() {
        let schedule = ScheduleType::Cron {
            cron_expr: "0 9 * * *".to_string(),
            timezone: "UTC".to_string(),
        };
        let from = utc("2026-06-09 00:00:00");
        let next = schedule.next_occurrence_after(&from).unwrap().unwrap();
        assert_eq!(next, utc("2026-06-09 09:00:00"));
    }

    #[test]
    fn validate_rejects_invalid_timezone() {
        let schedule = ScheduleType::Cron {
            cron_expr: "0 9 * * *".to_string(),
            timezone: "Not/AZone".to_string(),
        };
        assert!(matches!(
            schedule.validate(),
            Err(MonitorError::InvalidConfig(_))
        ));
    }
}
