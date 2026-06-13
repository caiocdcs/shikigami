use validator::Validate;

use crate::core::{
    domain::{
        CheckInOutcome, Integration, IntegrationId, Monitor, MonitorError, MonitorId,
        MonitorStatus, NewMonitor, NotificationContent, ScheduleType, monitor::StatusReport,
    },
    ports::{CheckIn, MonitorRepository},
};

#[derive(Debug, Clone)]
pub struct MonitorService<R: MonitorRepository> {
    repo: R,
}

impl<R: MonitorRepository> MonitorService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create_monitor(
        &self,
        name: String,
        description: Option<String>,
        slug: String,
        schedule_type: String,
        cron_expr: Option<String>,
        interval_seconds: Option<i64>,
        grace_seconds: i64,
        timezone: Option<String>,
    ) -> Result<Monitor, MonitorError> {
        let schedule = match schedule_type.as_str() {
            "cron" => {
                let expr = cron_expr.ok_or_else(|| {
                    MonitorError::InvalidConfig("cron_expr required for cron schedule".to_string())
                })?;
                if expr.is_empty() {
                    return Err(MonitorError::InvalidConfig(
                        "cron_expr must not be empty".to_string(),
                    ));
                }
                ScheduleType::Cron {
                    cron_expr: expr,
                    timezone: timezone.unwrap_or_else(|| "UTC".to_string()),
                }
            }
            "interval" => {
                let secs = interval_seconds.ok_or_else(|| {
                    MonitorError::InvalidConfig(
                        "interval_seconds required for interval schedule".to_string(),
                    )
                })?;
                if secs <= 0 {
                    return Err(MonitorError::InvalidConfig(
                        "interval_seconds must be greater than 0".to_string(),
                    ));
                }
                ScheduleType::Interval {
                    interval_seconds: secs,
                }
            }
            _ => {
                return Err(MonitorError::InvalidConfig(
                    "schedule_type must be 'cron' or 'interval'".to_string(),
                ));
            }
        };

        let new_monitor = NewMonitor {
            name,
            description,
            slug,
            schedule_type: schedule,
            grace_seconds,
        };

        new_monitor.validate()?;

        let now = chrono::Utc::now();
        let next_expected_at = new_monitor.schedule_type.next_occurrence_after(&now)?;

        self.repo.new_monitor(new_monitor, next_expected_at).await
    }

    pub async fn get_monitors(&self) -> Result<Vec<Monitor>, MonitorError> {
        self.repo.get_monitors().await
    }

    pub async fn get_monitor(
        &self,
        monitor_id: MonitorId,
    ) -> Result<Option<Monitor>, MonitorError> {
        self.repo.get_monitor(monitor_id).await
    }

    pub async fn get_monitor_by_slug(&self, slug: &str) -> Result<Option<Monitor>, MonitorError> {
        self.repo.get_monitor_by_slug(slug).await
    }

    pub async fn resolve_monitor_id(&self, reference: &str) -> Result<MonitorId, MonitorError> {
        if let Ok(uuid) = reference.parse::<uuid::Uuid>() {
            return Ok(MonitorId::from_uuid(uuid));
        }
        let monitor = self
            .repo
            .get_monitor_by_slug(reference)
            .await?
            .ok_or_else(|| MonitorError::NotFoundBySlug(reference.to_string()))?;
        Ok(monitor.id)
    }

    pub async fn delete_monitor(&self, monitor_id: MonitorId) -> Result<(), MonitorError> {
        self.repo.delete_monitor(monitor_id).await
    }

    pub async fn update_monitor(&self, monitor: Monitor) -> Result<(), MonitorError> {
        monitor.validate()?;
        monitor.schedule_type.validate()?;
        self.repo.update_monitor(monitor).await
    }

    pub async fn link_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> Result<(), MonitorError> {
        self.repo.link_integration(monitor_id, integration_id).await
    }

    pub async fn unlink_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> Result<(), MonitorError> {
        self.repo
            .unlink_integration(monitor_id, integration_id)
            .await
    }

    pub async fn get_monitor_integrations(
        &self,
        monitor_id: MonitorId,
    ) -> Result<Vec<Integration>, MonitorError> {
        self.repo.get_monitor_integrations(monitor_id).await
    }

    pub async fn get_check_ins(
        &self,
        monitor_id: MonitorId,
        limit: i64,
    ) -> Result<Vec<CheckIn>, MonitorError> {
        self.repo.get_check_ins(monitor_id, limit).await
    }
    pub async fn find_missed_monitors(&self) -> Result<Vec<MonitorId>, MonitorError> {
        self.repo.find_missed_monitors().await
    }

    pub async fn ping(&self, monitor_id: MonitorId) -> Result<(), MonitorError> {
        self.check_in(monitor_id, CheckInOutcome::Success).await
    }

    pub async fn check_in(
        &self,
        monitor_id: MonitorId,
        outcome: CheckInOutcome,
    ) -> Result<(), MonitorError> {
        let monitor = self
            .repo
            .get_monitor(monitor_id.clone())
            .await?
            .ok_or(MonitorError::NotFound(monitor_id.clone()))?;

        let now = chrono::Utc::now();
        let next_expected = monitor.schedule_type.next_occurrence_after(&now)?;

        // Business rule: failure marks monitor as missed; success/ping marks active
        let new_status = match outcome {
            CheckInOutcome::Success => MonitorStatus::Active,
            CheckInOutcome::Failure => MonitorStatus::Missed,
        };

        let notification = if matches!(outcome, CheckInOutcome::Failure) {
            Some(NotificationContent::for_failure(
                &monitor.name,
                &monitor.slug,
                monitor.last_pinged_at,
            ))
        } else {
            None
        };

        self.repo
            .check_in(
                monitor_id,
                outcome,
                now,
                next_expected,
                new_status,
                notification,
            )
            .await
    }

    pub async fn status_report(&self) -> Result<StatusReport, MonitorError> {
        let entries = self.repo.status_report().await?;
        let total = entries.len();
        let mut healthy = 0usize;
        let mut missed = 0usize;
        let mut paused = 0usize;

        for entry in &entries {
            match entry.status {
                MonitorStatus::Active => healthy += 1,
                MonitorStatus::Missed => missed += 1,
                MonitorStatus::Paused => paused += 1,
            }
        }

        Ok(StatusReport {
            total,
            healthy,
            missed,
            paused,
            monitors: entries,
        })
    }
}
