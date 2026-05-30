use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use uuid::Uuid;

use crate::{
    AppState,
    api::dtos::monitor_dto::{
        CreateMonitorDto, LinkIntegrationDto, MonitorResponse, UpdateMonitorDto,
    },
    core::domain::{
        integration::IntegrationId,
        monitor::{CheckInOutcome, Monitor, MonitorId, MonitorStatus, ScheduleType},
    },
    error::{AppError, AppResult},
};

#[axum::debug_handler]
pub async fn create_monitor(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateMonitorDto>,
) -> AppResult<(StatusCode, Json<MonitorResponse>)> {
    let monitor = state
        .monitor_service
        .create_monitor(
            payload.name,
            payload.description,
            payload.slug,
            payload.schedule_type,
            payload.cron_expr,
            payload.interval_seconds,
            payload.grace_seconds,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(monitor.into())))
}

#[axum::debug_handler]
pub async fn get_monitor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<MonitorResponse>> {
    let monitor = state
        .monitor_service
        .get_monitor(MonitorId::from_uuid(id))
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(monitor.into()))
}

#[axum::debug_handler]
pub async fn get_monitors(State(state): State<AppState>) -> AppResult<Json<Vec<MonitorResponse>>> {
    let monitors = state.monitor_service.get_monitors().await?;
    Ok(Json(
        monitors.into_iter().map(MonitorResponse::from).collect(),
    ))
}

#[axum::debug_handler]
pub async fn delete_monitor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .monitor_service
        .delete_monitor(MonitorId::from_uuid(id))
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn update_monitor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateMonitorDto>,
) -> AppResult<StatusCode> {
    let schedule = match payload.schedule_type.as_str() {
        "cron" => {
            let expr = payload.cron_expr.ok_or_else(|| {
                AppError::Validation("cron_expr required for cron schedule".to_string())
            })?;
            ScheduleType::Cron { cron_expr: expr }
        }
        "interval" => {
            let secs = payload.interval_seconds.ok_or_else(|| {
                AppError::Validation("interval_seconds required for interval schedule".to_string())
            })?;
            ScheduleType::Interval {
                interval_seconds: secs,
            }
        }
        _ => {
            return Err(AppError::Validation(
                "schedule_type must be 'cron' or 'interval'".to_string(),
            ));
        }
    };
    let status = MonitorStatus::try_from(payload.status.as_str())
        .map_err(|_| AppError::Validation("invalid status value".to_string()))?;

    // Fetch existing monitor to preserve timestamps
    let existing = state
        .monitor_service
        .get_monitor(MonitorId::from_uuid(id))
        .await?
        .ok_or(AppError::NotFound)?;

    let monitor = Monitor {
        id: MonitorId::from_uuid(id),
        name: payload.name,
        description: payload.description,
        slug: payload.slug,
        schedule_type: schedule,
        status,
        grace_seconds: payload.grace_seconds,
        last_pinged_at: existing.last_pinged_at,
        next_expected_at: existing.next_expected_at,
        created_at: existing.created_at,
    };
    state.monitor_service.update_monitor(monitor).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn link_integration(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
    Json(payload): Json<LinkIntegrationDto>,
) -> AppResult<StatusCode> {
    let integration_uuid: Uuid = payload
        .integration_id
        .parse()
        .map_err(|_| AppError::Validation("invalid integration_id".to_string()))?;
    state
        .monitor_service
        .link_integration(
            MonitorId::from_uuid(monitor_id),
            IntegrationId::from_uuid(integration_uuid),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn unlink_integration(
    State(state): State<AppState>,
    Path((monitor_id, integration_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    state
        .monitor_service
        .unlink_integration(
            MonitorId::from_uuid(monitor_id),
            IntegrationId::from_uuid(integration_id),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn ping_monitor(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .monitor_service
        .ping(MonitorId::from_uuid(monitor_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn success_check_in(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .monitor_service
        .check_in(MonitorId::from_uuid(monitor_id), CheckInOutcome::Success)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn failure_check_in(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    state
        .monitor_service
        .check_in(MonitorId::from_uuid(monitor_id), CheckInOutcome::Failure)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
