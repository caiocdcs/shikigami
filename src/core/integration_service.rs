use crate::core::{
    domain::{
        Integration,
        integration::{
            IntegrationChannel, IntegrationConfig, IntegrationError, IntegrationId, NewIntegration,
        },
    },
    ports::integration_repository::IntegrationRepository,
};

#[derive(Debug, Clone)]
pub struct IntegrationService<R: IntegrationRepository> {
    repo: R,
}

impl<R: IntegrationRepository> IntegrationService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create_integration(
        &self,
        name: String,
        channel: String,
        config: serde_json::Value,
    ) -> Result<Integration, IntegrationError> {
        let channel = IntegrationChannel::try_from(channel.as_str())?;
        let config_json = serde_json::to_string(&config)?;
        let config = IntegrationConfig::parse(&channel, &config_json)?;

        let new_integration = NewIntegration {
            name,
            channel,
            config,
        };

        self.repo.new_integration(new_integration).await
    }

    pub async fn get_integrations(&self) -> Result<Vec<Integration>, IntegrationError> {
        self.repo.get_integrations().await
    }

    pub async fn get_integration(
        &self,
        integration_id: IntegrationId,
    ) -> Result<Option<Integration>, IntegrationError> {
        self.repo.get_integration(integration_id).await
    }

    pub async fn delete_integration(
        &self,
        integration_id: IntegrationId,
    ) -> Result<(), IntegrationError> {
        self.repo.delete_integration(integration_id).await
    }

    pub async fn update_integration(
        &self,
        integration: Integration,
    ) -> Result<(), IntegrationError> {
        self.repo.update_integration(integration).await
    }
}
