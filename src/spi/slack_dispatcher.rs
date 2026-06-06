use std::pin::Pin;

use reqwest::Client;

use crate::core::{
    domain::DispatchError, domain::integration::IntegrationConfig,
    ports::notification_dispatcher::NotificationDispatcher,
};

#[derive(Debug, Clone)]
pub struct SlackDispatcher {
    client: Client,
}

impl SlackDispatcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl NotificationDispatcher for SlackDispatcher {
    fn dispatch(
        &self,
        config: &IntegrationConfig,
        message: &str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), DispatchError>> + Send>> {
        let client = self.client.clone();
        let slack = match config {
            IntegrationConfig::Slack(c) => c.clone(),
            _ => {
                return Box::pin(async {
                    Err(DispatchError::Permanent("not a slack config".to_string()))
                });
            }
        };
        let msg = message.to_string();

        Box::pin(async move {
            let body = serde_json::json!({
                "text": format!("*Shikigami Alert*\n{}", msg),
            });

            let resp = client
                .post(&slack.webhook_url)
                .json(&body)
                .send()
                .await
                .map_err(|e| DispatchError::Transient(format!("slack request failed: {e}")))?;

            if resp.status().is_success() {
                Ok(())
            } else if resp.status().is_server_error() {
                Err(DispatchError::Transient(format!(
                    "slack server error: {}",
                    resp.status()
                )))
            } else {
                Err(DispatchError::Permanent(format!(
                    "slack client error: {}",
                    resp.status()
                )))
            }
        })
    }
}
