use crate::core::domain::{
    Integration, Monitor,
    integration::IntegrationId,
    monitor::{MonitorError, MonitorId, NewMonitor},
};

pub trait MonitorRepository: Send + Sync + 'static {
    fn get_monitors(&self) -> impl Future<Output = Result<Vec<Monitor>, MonitorError>> + Send;
    fn get_monitor(
        &self,
        monitor_id: MonitorId,
    ) -> impl Future<Output = Result<Option<Monitor>, MonitorError>> + Send;
    fn new_monitor(
        &self,
        monitor: NewMonitor,
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
}
