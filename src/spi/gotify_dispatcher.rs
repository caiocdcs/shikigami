use std::pin::Pin;

use reqwest::Client;

use crate::core::{
    domain::DispatchError, domain::integration::IntegrationConfig,
    ports::notification_dispatcher::NotificationDispatcher,
};

#[derive(Debug, Clone)]
pub struct GotifyDispatcher {
    client: Client,
}

impl GotifyDispatcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl NotificationDispatcher for GotifyDispatcher {
    fn dispatch(
        &self,
        config: &IntegrationConfig,
        message: &str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), DispatchError>> + Send>> {
        let client = self.client.clone();
        let gotify = match config {
            IntegrationConfig::Gotify(c) => c.clone(),
            _ => {
                return Box::pin(async {
                    Err(DispatchError::Permanent("not a gotify config".to_string()))
                });
            }
        };
        let msg = message.to_string();

        Box::pin(async move {
            let url = format!(
                "{}/message?token={}",
                gotify.url.trim_end_matches('/'),
                gotify.token
            );

            let body = serde_json::json!({
                "title": "Shikigami Alert",
                "message": msg,
                "priority": gotify.priority,
            });

            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| DispatchError::Transient(format!("gotify request failed: {e}")))?;

            if resp.status().is_success() {
                Ok(())
            } else if resp.status().is_server_error() {
                Err(DispatchError::Transient(format!(
                    "gotify server error: {}",
                    resp.status()
                )))
            } else {
                Err(DispatchError::Permanent(format!(
                    "gotify client error: {}",
                    resp.status()
                )))
            }
        })
    }
}
