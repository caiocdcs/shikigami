use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone)]
pub struct IntegrationId(Uuid);

impl Default for IntegrationId {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct NewIntegration {
    pub name: String,
    pub channel: IntegrationChannel,
    pub config: IntegrationConfig,
}

#[derive(Debug, Clone)]
pub struct Integration {
    pub id: IntegrationId,
    pub name: String,
    pub channel: IntegrationChannel,
    pub config: IntegrationConfig,
    pub status: IntegrationStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Integration {
    pub fn new(
        id: IntegrationId,
        name: String,
        channel: IntegrationChannel,
        config: IntegrationConfig,
        status: IntegrationStatus,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            id,
            name,
            channel,
            config,
            status,
            created_at,
        }
    }
}

#[derive(Debug)]
pub enum IntegrationError {
    InvalidConfig(String),
    NotFound(IntegrationId),
    Conflict(String),
    Database(String),
}

impl std::error::Error for IntegrationError {}

impl Display for IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationError::InvalidConfig(field) => write!(f, "Invalid config: {field}"),
            IntegrationError::NotFound(id) => write!(f, "Integration not found: {}", id.as_uuid()),
            IntegrationError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            IntegrationError::Database(msg) => write!(f, "Database error: {msg}"),
        }
    }
}

impl IntegrationError {
    #[allow(clippy::needless_pass_by_value)]
    pub fn map_sqlx_error(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
                Some("2067") => IntegrationError::Conflict("duplicate entry".to_string()),
                Some("787") => {
                    IntegrationError::InvalidConfig("referenced record not found".to_string())
                }
                _ => IntegrationError::Database(e.to_string()),
            },
            _ => IntegrationError::Database(e.to_string()),
        }
    }
}

impl From<sqlx::Error> for IntegrationError {
    fn from(error: sqlx::Error) -> Self {
        Self::map_sqlx_error(error)
    }
}

impl From<serde_json::Error> for IntegrationError {
    fn from(error: serde_json::Error) -> Self {
        IntegrationError::InvalidConfig(error.to_string())
    }
}

impl From<uuid::Error> for IntegrationError {
    fn from(error: uuid::Error) -> Self {
        IntegrationError::Database(format!("invalid uuid: {error}"))
    }
}

impl From<chrono::ParseError> for IntegrationError {
    fn from(error: chrono::ParseError) -> Self {
        IntegrationError::Database(format!("invalid datetime: {error}"))
    }
}

impl From<validator::ValidationErrors> for IntegrationError {
    fn from(errors: validator::ValidationErrors) -> Self {
        IntegrationError::InvalidConfig(errors.to_string())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum IntegrationConfig {
    Ntfy(NtfyConfig),
    Gotify(GotifyConfig),
    Email(EmailConfig),
}

impl IntegrationConfig {
    pub fn to_json(&self) -> String {
        match self {
            IntegrationConfig::Ntfy(c) => serde_json::to_string(c).unwrap_or_default(),
            IntegrationConfig::Gotify(c) => serde_json::to_string(c).unwrap_or_default(),
            IntegrationConfig::Email(c) => serde_json::to_string(c).unwrap_or_default(),
        }
    }

    pub fn parse(channel: &IntegrationChannel, json: &str) -> Result<Self, IntegrationError> {
        match channel {
            IntegrationChannel::Ntfy => {
                let config: NtfyConfig = serde_json::from_str(json)?;
                config.validate()?;
                Ok(IntegrationConfig::Ntfy(config))
            }
            IntegrationChannel::Gotify => {
                let config: GotifyConfig = serde_json::from_str(json)?;
                config.validate()?;
                Ok(IntegrationConfig::Gotify(config))
            }
            IntegrationChannel::Email => {
                let config: EmailConfig = serde_json::from_str(json)?;
                if config.smtp_host.is_empty() {
                    return Err(IntegrationError::InvalidConfig(
                        "smtp_host is required".to_string(),
                    ));
                }
                if config.to.is_empty() || !config.to.contains('@') {
                    return Err(IntegrationError::InvalidConfig(
                        "to must be a valid email".to_string(),
                    ));
                }
                if config.from.is_empty() || !config.from.contains('@') {
                    return Err(IntegrationError::InvalidConfig(
                        "from must be a valid email".to_string(),
                    ));
                }
                Ok(IntegrationConfig::Email(config))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct NtfyConfig {
    #[validate(length(min = 1))]
    pub url: String,
    #[validate(length(min = 1))]
    pub topic: String,
    pub priority: u8,
    #[validate(length(min = 1))]
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct GotifyConfig {
    #[validate(length(min = 1))]
    pub url: String,
    #[validate(length(min = 1))]
    pub token: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_encryption: SmtpEncryption,
    pub to: String,
    pub from: String,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum IntegrationChannel {
    Ntfy,
    Gotify,
    Email,
}

impl TryFrom<&str> for IntegrationChannel {
    type Error = IntegrationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "ntfy" => Ok(IntegrationChannel::Ntfy),
            "gotify" => Ok(IntegrationChannel::Gotify),
            "email" => Ok(IntegrationChannel::Email),

            _ => Err(IntegrationError::InvalidConfig("channel".to_string())),
        }
    }
}

impl Display for IntegrationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationChannel::Ntfy => write!(f, "ntfy"),
            IntegrationChannel::Gotify => write!(f, "gotify"),
            IntegrationChannel::Email => write!(f, "email"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SmtpEncryption {
    Tls,
    Starttls,
    None,
}

impl Display for SmtpEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmtpEncryption::Tls => write!(f, "tls"),
            SmtpEncryption::Starttls => write!(f, "starttls"),
            SmtpEncryption::None => write!(f, "none"),
        }
    }
}

impl TryFrom<&str> for SmtpEncryption {
    type Error = IntegrationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "tls" => Ok(SmtpEncryption::Tls),
            "starttls" => Ok(SmtpEncryption::Starttls),
            "none" => Ok(SmtpEncryption::None),
            _ => Err(IntegrationError::InvalidConfig(format!(
                "invalid smtp_encryption: {value}, expected tls, starttls, or none"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum IntegrationStatus {
    Active,
    Inactive,
}

impl TryFrom<&str> for IntegrationStatus {
    type Error = IntegrationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(IntegrationStatus::Active),
            "inactive" => Ok(IntegrationStatus::Inactive),
            _ => Err(IntegrationError::InvalidConfig("status".to_string())),
        }
    }
}

impl Display for IntegrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegrationStatus::Active => write!(f, "active"),
            IntegrationStatus::Inactive => write!(f, "inactive"),
        }
    }
}
