use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use uuid::Uuid;

use crate::{
    AppState,
    api::dtos::integration_dto::{CreateIntegrationDto, IntegrationResponse},
    core::domain::integration::IntegrationId,
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
