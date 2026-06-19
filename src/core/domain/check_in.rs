//! Check-in domain entity.
//!
//! A check-in is the record produced when a monitored job pings an ingress
//! endpoint. It carries the outcome (success/failure) and an optional message
//! supplied by the caller (e.g. the reason a job reported failure, or context
//! attached to a heartbeat). The message is stored verbatim, surfaced via the
//! JSON API and the status UI, and -- for failures -- folded into the
//! notification body.

use chrono::{DateTime, Utc};

use crate::core::domain::CheckInOutcome;

#[derive(Debug, Clone)]
pub struct CheckIn {
    pub id: String,
    pub monitor_id: String,
    pub checked_in_at: DateTime<Utc>,
    pub outcome: CheckInOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CheckInsResult {
    pub check_ins: Vec<CheckIn>,
    pub total: i64,
}
