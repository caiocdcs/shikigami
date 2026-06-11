use anyhow::Context;
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,

    pub database_url: SecretString,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    pub api_key: Option<SecretString>,

    #[serde(default)]
    pub ui_enabled: bool,
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
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

    pub fn for_test(database_url: &str) -> Self {
        Self {
            port: 3000,
            database_url: SecretString::new(database_url.to_string().into()),
            log_level: "debug".to_string(),
            api_key: None,
            ui_enabled: false,
        }
    }

    pub fn for_test_with_key(database_url: &str, key: &str) -> Self {
        Self {
            port: 3000,
            database_url: SecretString::new(database_url.to_string().into()),
            log_level: "debug".to_string(),
            api_key: Some(SecretString::new(key.to_string().into())),
            ui_enabled: false,
        }
    }

    pub fn for_test_with_ui(database_url: &str, ui_enabled: bool) -> Self {
        Self {
            port: 3000,
            database_url: SecretString::new(database_url.to_string().into()),
            log_level: "debug".to_string(),
            api_key: None,
            ui_enabled,
        }
    }
}
