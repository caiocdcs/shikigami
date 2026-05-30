use sqlx::{FromRow, SqlitePool};

use crate::core::{
    domain::{
        Integration,
        integration::{
            IntegrationChannel, IntegrationConfig, IntegrationError, IntegrationId,
            IntegrationStatus, NewIntegration,
        },
    },
    ports::integration_repository::IntegrationRepository,
};

#[derive(FromRow)]
struct IntegrationRow {
    id: String,
    name: String,
    channel_type: String,
    config_json: String,
    status: String,
    created_at: String,
}

impl TryFrom<IntegrationRow> for Integration {
    type Error = IntegrationError;

    fn try_from(row: IntegrationRow) -> Result<Self, Self::Error> {
        let uuid = row.id.parse::<uuid::Uuid>()?;
        let naive = chrono::NaiveDateTime::parse_from_str(&row.created_at, "%Y-%m-%d %H:%M:%S")?;
        let parsed_at =
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
        let channel = IntegrationChannel::try_from(row.channel_type.as_str())?;
        let status = IntegrationStatus::try_from(row.status.as_str())?;

        Ok(Integration {
            id: IntegrationId::from_uuid(uuid),
            name: row.name,
            channel: channel.clone(),
            config: IntegrationConfig::parse(&channel, &row.config_json)?,
            status,
            created_at: parsed_at,
        })
    }
}

#[derive(Clone)]
pub struct SqliteIntegrationRepository {
    pool: SqlitePool,
}

impl SqliteIntegrationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl IntegrationRepository for SqliteIntegrationRepository {
    async fn get_integrations(&self) -> Result<Vec<Integration>, IntegrationError> {
        sqlx::query_as!(
            IntegrationRow,
            r#"SELECT id as "id!", name as "name!", channel_type as "channel_type!", config_json as "config_json!", status as "status!", created_at as "created_at!" FROM integrations"#
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Integration::try_from)
        .collect()
    }

    async fn get_integration(
        &self,
        integration_id: IntegrationId,
    ) -> Result<Option<Integration>, IntegrationError> {
        let id_str = integration_id.as_uuid().to_string();
        let row = sqlx::query_as!(
            IntegrationRow,
            r#"SELECT id as "id!", name as "name!", channel_type as "channel_type!", config_json as "config_json!", status as "status!", created_at as "created_at!" FROM integrations WHERE id = ?"#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(IntegrationError::map_sqlx_error)?;

        row.map(Integration::try_from).transpose()
    }

    async fn new_integration(
        &self,
        integration: NewIntegration,
    ) -> Result<Integration, IntegrationError> {
        let id = IntegrationId::new();
        let id_str = id.as_uuid().to_string();
        let channel_str = integration.channel.to_string();
        let config_json = integration.config.to_json();

        sqlx::query!(
            r#"INSERT INTO integrations (id, name, channel_type, config_json, status, created_at) VALUES (?, ?, ?, ?, 'active', datetime('now'))"#,
            id_str,
            integration.name,
            channel_str,
            config_json
        )
        .execute(&self.pool)
        .await
        .map_err(IntegrationError::map_sqlx_error)?;

        Ok(Integration::new(
            id,
            integration.name,
            integration.channel,
            integration.config,
            IntegrationStatus::Active,
            chrono::Utc::now(),
        ))
    }

    async fn delete_integration(
        &self,
        integration_id: IntegrationId,
    ) -> Result<(), IntegrationError> {
        let id_str = integration_id.as_uuid().to_string();

        sqlx::query!(r#"DELETE FROM integrations WHERE id = ?"#, id_str)
            .execute(&self.pool)
            .await
            .map_err(IntegrationError::map_sqlx_error)?;

        Ok(())
    }

    async fn update_integration(&self, integration: Integration) -> Result<(), IntegrationError> {
        let id_str = integration.id.as_uuid().to_string();
        let channel_str = integration.channel.to_string();
        let config_json = integration.config.to_json();
        let status_str = integration.status.to_string();

        sqlx::query!(
            r#"UPDATE integrations SET name = ?, channel_type = ?, config_json = ?, status = ? WHERE id = ?"#,
            integration.name,
            channel_str,
            config_json,
            status_str,
            id_str
        )
        .execute(&self.pool)
        .await
        .map_err(IntegrationError::map_sqlx_error)?;

        Ok(())
    }
}
