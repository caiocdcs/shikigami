use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use uuid::Uuid;

use crate::{
    AppState,
    api::dtos::{CreateIntegrationDto, IntegrationResponse, UpdateIntegrationDto},
    core::domain::{Integration, IntegrationChannel, IntegrationConfig, IntegrationId},
    error::{AppError, AppResult},
};

#[axum::debug_handler]
pub async fn create_integration(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateIntegrationDto>,
) -> AppResult<(StatusCode, Json<IntegrationResponse>)> {
    let integration = state
        .integration_service
        .create_integration(payload.name, payload.channel, payload.config)
        .await?;

    Ok((StatusCode::CREATED, Json(integration.into())))
}

#[axum::debug_handler]
pub async fn get_integration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<IntegrationResponse>> {
    let integration = state
        .integration_service
        .get_integration(IntegrationId::from_uuid(id))
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(integration.into()))
}

#[axum::debug_handler]
pub async fn get_integrations(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<IntegrationResponse>>> {
    let integrations = state.integration_service.get_integrations().await?;
    Ok(Json(
        integrations
            .into_iter()
            .map(IntegrationResponse::from)
            .collect(),
    ))
}

#[axum::debug_handler]
pub async fn delete_integration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .integration_service
        .delete_integration(IntegrationId::from_uuid(id))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn update_integration(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateIntegrationDto>,
) -> AppResult<StatusCode> {
    let existing = state
        .integration_service
        .get_integration(IntegrationId::from_uuid(id))
        .await?
        .ok_or(AppError::NotFound)?;

    let channel = IntegrationChannel::try_from(payload.channel.as_str())
        .map_err(|_| AppError::Validation("invalid channel".to_string()))?;

    let config = IntegrationConfig::parse(
        &channel,
        &serde_json::to_string(&payload.config).unwrap_or_default(),
    )
    .map_err(|_| AppError::Validation("invalid config".to_string()))?;

    let integration = Integration::new(
        existing.id,
        payload.name,
        channel,
        config,
        existing.status,
        existing.created_at,
    );

    state
        .integration_service
        .update_integration(integration)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
