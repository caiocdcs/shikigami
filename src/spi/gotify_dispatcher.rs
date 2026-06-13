use std::pin::Pin;

use reqwest::Client;

use crate::core::{
    domain::{DispatchError, IntegrationConfig, NotificationContent},
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
        notification: &NotificationContent,
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
        let title = notification.title.clone();
        let body = notification.body.clone();

        Box::pin(async move {
            let url = format!(
                "{}/message?token={}",
                gotify.url.trim_end_matches('/'),
                gotify.token
            );

            let payload = serde_json::json!({
                "title": title,
                "message": body,
                "priority": gotify.priority,
            });

            let resp = client
                .post(&url)
                .json(&payload)
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
