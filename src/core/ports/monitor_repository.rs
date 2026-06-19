use crate::core::domain::{
    CheckInsResult, Integration, Monitor, NotificationContent,
    integration::IntegrationId,
    monitor::{
        CheckInOutcome, MonitorError, MonitorId, MonitorStatus, NewMonitor, StatusReportEntry,
    },
};

pub trait MonitorRepository: Send + Sync + 'static {
    fn get_monitors(&self) -> impl Future<Output = Result<Vec<Monitor>, MonitorError>> + Send;
    fn get_monitor(
        &self,
        monitor_id: MonitorId,
    ) -> impl Future<Output = Result<Option<Monitor>, MonitorError>> + Send;
    fn get_monitor_by_slug(
        &self,
        slug: &str,
    ) -> impl Future<Output = Result<Option<Monitor>, MonitorError>> + Send;
    fn new_monitor(
        &self,
        monitor: NewMonitor,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl Future<Output = Result<Monitor, MonitorError>> + Send;
    fn delete_monitor(
        &self,
        monitor_id: MonitorId,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;
    fn update_monitor(
        &self,
        monitor: Monitor,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;
    fn link_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;
    fn unlink_integration(
        &self,
        monitor_id: MonitorId,
        integration_id: IntegrationId,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;
    fn get_monitor_integrations(
        &self,
        monitor_id: MonitorId,
    ) -> impl Future<Output = Result<Vec<Integration>, MonitorError>> + Send;
    fn get_check_ins(
        &self,
        monitor_id: MonitorId,
        limit: i64,
        offset: i64,
    ) -> impl Future<Output = Result<CheckInsResult, MonitorError>> + Send;
    fn find_missed_monitors(
        &self,
    ) -> impl Future<Output = Result<Vec<MonitorId>, MonitorError>> + Send;
    fn ping(
        &self,
        monitor_id: MonitorId,
        timestamp: chrono::DateTime<chrono::Utc>,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;
    fn check_in(
        &self,
        monitor_id: MonitorId,
        outcome: CheckInOutcome,
        timestamp: chrono::DateTime<chrono::Utc>,
        next_expected_at: Option<chrono::DateTime<chrono::Utc>>,
        new_status: MonitorStatus,
        message: Option<String>,
        notification: Option<NotificationContent>,
    ) -> impl Future<Output = Result<(), MonitorError>> + Send;

    fn status_report(
        &self,
    ) -> impl Future<Output = Result<Vec<StatusReportEntry>, MonitorError>> + Send;
}
