use anyhow::Context;
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Address the HTTP listener binds to.
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub database_url: SecretString,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    pub api_key: Option<SecretString>,

    #[serde(default)]
    pub ui_enabled: bool,

    /// Check-ins older than this many days are pruned by the retention worker.
    #[serde(default = "default_retention_days")]
    pub retention_days: i64,

    /// How often (in seconds) the retention worker prunes old check-ins.
    #[serde(default = "default_retention_interval_seconds")]
    pub retention_interval_seconds: u64,

    /// Terminal `notification_outbox` rows (sent/failed) older than this many days
    /// are pruned. Pending/sending rows are never pruned. `0` disables.
    #[serde(default = "default_outbox_retention_days")]
    pub outbox_retention_days: i64,

    // --- Worker tuning ---
    /// How often (in seconds) the notification worker drains the outbox.
    #[serde(default = "default_notification_interval_seconds")]
    pub notification_interval_seconds: u64,
    /// Max delivery attempts before an outbox entry is marked failed.
    #[serde(default = "default_notification_max_retries")]
    pub notification_max_retries: u32,
    /// How often (in seconds) the missed-monitor checker runs.
    #[serde(default = "default_checker_interval_seconds")]
    pub checker_interval_seconds: u64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_retention_days() -> i64 {
    30
}

fn default_retention_interval_seconds() -> u64 {
    3600
}

fn default_outbox_retention_days() -> i64 {
    30
}

fn default_notification_interval_seconds() -> u64 {
    30
}

fn default_notification_max_retries() -> u32 {
    3
}

fn default_checker_interval_seconds() -> u64 {
    60
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // In development, load from a .env file if present.
        // The .ok() is intentional: in production there is no .env file,
        // and that is fine.
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .context("failed to build configuration")?;

        let parsed: Config = config
            .try_deserialize()
            .context("failed to deserialize configuration")?;

        Ok(parsed)
    }

    /// Defaults for tests. Override fields with struct update syntax, e.g.
    /// `Config { ui_enabled: true, ..Config::for_test(url) }`.
    pub fn for_test(database_url: &str) -> Self {
        Self {
            host: default_host(),
            port: 3000,
            database_url: SecretString::new(database_url.to_string().into()),
            log_level: "debug".to_string(),
            api_key: None,
            ui_enabled: false,
            retention_days: default_retention_days(),
            retention_interval_seconds: default_retention_interval_seconds(),
            outbox_retention_days: default_outbox_retention_days(),
            notification_interval_seconds: default_notification_interval_seconds(),
            notification_max_retries: default_notification_max_retries(),
            checker_interval_seconds: default_checker_interval_seconds(),
        }
    }
}
