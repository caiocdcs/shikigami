use std::pin::Pin;

use lettre::{
    Address, AsyncSmtpTransport, Message, Tokio1Executor,
    message::header::ContentType,
    transport::{
        AsyncTransport,
        smtp::{
            authentication::Credentials,
            client::{Tls, TlsParameters},
        },
    },
};
use secrecy::ExposeSecret;

use crate::core::domain::{
    DispatchError, EmailConfig, IntegrationConfig, NotificationContent, SmtpEncryption,
};
use crate::core::ports::notification_dispatcher::NotificationDispatcher;

#[derive(Debug, Clone)]
pub struct EmailDispatcher;

impl EmailDispatcher {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmailDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationDispatcher for EmailDispatcher {
    fn dispatch(
        &self,
        config: &IntegrationConfig,
        notification: &NotificationContent,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), DispatchError>> + Send>> {
        let email = match config {
            IntegrationConfig::Email(c) => c.clone(),
            _ => {
                return Box::pin(async {
                    Err(DispatchError::Permanent("not an email config".to_string()))
                });
            }
        };

        let title = notification.title.clone();
        let body = notification.body.clone();

        Box::pin(async move { send_email(&email, &title, &body).await })
    }
}

async fn send_email(config: &EmailConfig, title: &str, body: &str) -> Result<(), DispatchError> {
    let from_addr: Address = config
        .from
        .parse()
        .map_err(|e| DispatchError::Permanent(format!("invalid from address: {e}")))?;
    let to_addr: Address = config
        .to
        .parse()
        .map_err(|e| DispatchError::Permanent(format!("invalid to address: {e}")))?;

    let email = Message::builder()
        .from(from_addr.into())
        .to(to_addr.into())
        .subject(title)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| DispatchError::Permanent(format!("failed to build email: {e}")))?;

    let creds = Credentials::new(
        config.smtp_username.clone(),
        config.smtp_password.expose_secret().to_string(),
    );

    let mailer = build_transport(
        &config.smtp_host,
        config.smtp_port,
        config.smtp_encryption,
        creds,
    )
    .map_err(|e| DispatchError::Permanent(format!("failed to configure SMTP: {e}")))?;

    mailer
        .send(email)
        .await
        .map_err(|e| DispatchError::Transient(format!("SMTP send failed: {e}")))?;

    Ok(())
}

fn build_transport(
    host: &str,
    port: u16,
    encryption: SmtpEncryption,
    creds: Credentials,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, DispatchError> {
    match encryption {
        SmtpEncryption::Tls => {
            let tls_params = TlsParameters::builder(host.to_string())
                .build_rustls()
                .map_err(|e| DispatchError::Permanent(format!("TLS config error: {e}")))?;
            let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .tls(Tls::Wrapper(tls_params))
                .port(port)
                .credentials(creds)
                .build();
            Ok(mailer)
        }
        SmtpEncryption::Starttls => {
            let tls_params = TlsParameters::builder(host.to_string())
                .build_rustls()
                .map_err(|e| DispatchError::Permanent(format!("TLS config error: {e}")))?;
            let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .tls(Tls::Required(tls_params))
                .port(port)
                .credentials(creds)
                .build();
            Ok(mailer)
        }
        SmtpEncryption::None => {
            let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .tls(Tls::None)
                .port(port)
                .credentials(creds)
                .build();
            Ok(mailer)
        }
    }
}
