use std::pin::Pin;

use reqwest::Client;

use crate::core::{
    domain::DispatchError, domain::integration::IntegrationConfig,
    ports::notification_dispatcher::NotificationDispatcher,
};

#[derive(Debug, Clone)]
pub struct NtfyDispatcher {
    client: Client,
}

impl NtfyDispatcher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

impl NotificationDispatcher for NtfyDispatcher {
    fn dispatch(
        &self,
        config: &IntegrationConfig,
        message: &str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), DispatchError>> + Send>> {
        let client = self.client.clone();
        let ntfy = match config {
            IntegrationConfig::Ntfy(c) => c.clone(),
            _ => {
                return Box::pin(async {
                    Err(DispatchError::Permanent("not an ntfy config".to_string()))
                });
            }
        };
        let msg = message.to_string();

        Box::pin(async move {
            let url = format!("{}/{}", ntfy.url.trim_end_matches('/'), ntfy.topic);

            let resp = client
                .post(&url)
                .header("Title", "Shikigami Alert")
                .header("Priority", ntfy.priority.to_string())
                .body(msg)
                .send()
                .await
                .map_err(|e| DispatchError::Transient(format!("ntfy request failed: {e}")))?;

            if resp.status().is_success() {
                Ok(())
            } else if resp.status().is_server_error() {
                Err(DispatchError::Transient(format!(
                    "ntfy server error: {}",
                    resp.status()
                )))
            } else {
                Err(DispatchError::Permanent(format!(
                    "ntfy client error: {}",
                    resp.status()
                )))
            }
        })
    }
}
