use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::core::domain::Integration;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateIntegrationDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub channel: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct IntegrationResponse {
    pub id: String,
    pub name: String,
    pub channel: String,
    pub config: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Integration> for IntegrationResponse {
    fn from(integration: Integration) -> Self {
        Self {
            id: integration.id.as_uuid().to_string(),
            name: integration.name,
            channel: integration.channel.to_string(),
            config: integration.config.to_json(),
            status: integration.status.to_string(),
            created_at: integration.created_at,
        }
    }
}
