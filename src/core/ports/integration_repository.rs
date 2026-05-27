use crate::core::domain::{
    Integration,
    integration::{IntegrationError, IntegrationId, NewIntegration},
};

pub trait IntegrationRepository: Send + Sync + 'static {
    fn get_integrations(
        &self,
    ) -> impl Future<Output = Result<Vec<Integration>, IntegrationError>> + Send;
    fn get_integration(
        &self,
        integration_id: IntegrationId,
    ) -> impl Future<Output = Result<Option<Integration>, IntegrationError>> + Send;
    fn new_integration(
        &self,
        integration: NewIntegration,
    ) -> impl Future<Output = Result<Integration, IntegrationError>> + Send;
    fn delete_integration(
        &self,
        integration_id: IntegrationId,
    ) -> impl Future<Output = Result<(), IntegrationError>> + Send;
    fn update_integration(
        &self,
        integration: Integration,
    ) -> impl Future<Output = Result<(), IntegrationError>> + Send;
}
