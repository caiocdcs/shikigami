use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_validated_extractors::ValidatedJson;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::dtos::{
        CheckInResponse, CheckInsPage, CreateMonitorDto, IntegrationResponse, LinkIntegrationDto,
        MonitorResponse, UpdateMonitorDto,
    },
    core::domain::{
        CheckInOutcome, IntegrationId, Monitor, MonitorId, MonitorStatus, ScheduleType,
    },
    error::{AppError, AppResult},
};

#[derive(Debug, Deserialize)]
pub struct CheckInsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

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
            payload.timezone,
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
            let expr = payload
                .cron_expr
                .ok_or_else(|| AppError::Validation("cron_expr required".to_string()))?;
            ScheduleType::Cron {
                cron_expr: expr,
                timezone: payload.timezone.unwrap_or_else(|| "UTC".to_string()),
            }
        }
        "interval" => {
            let secs = payload
                .interval_seconds
                .ok_or_else(|| AppError::Validation("interval_seconds required".to_string()))?;
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
        .map_err(|_| AppError::Validation("invalid status".to_string()))?;

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
    let iid: Uuid = payload
        .integration_id
        .parse()
        .map_err(|_| AppError::Validation("invalid integration_id".to_string()))?;
    state
        .monitor_service
        .link_integration(
            MonitorId::from_uuid(monitor_id),
            IntegrationId::from_uuid(iid),
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
pub async fn get_monitor_integrations(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
) -> AppResult<Json<Vec<IntegrationResponse>>> {
    let integrations = state
        .monitor_service
        .get_monitor_integrations(MonitorId::from_uuid(monitor_id))
        .await?;
    Ok(Json(
        integrations
            .into_iter()
            .map(IntegrationResponse::from)
            .collect(),
    ))
}

#[axum::debug_handler]
pub async fn get_monitor_check_ins(
    State(state): State<AppState>,
    Path(monitor_id): Path<Uuid>,
    Query(params): Query<CheckInsQuery>,
) -> AppResult<Json<CheckInsPage>> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    let page = state
        .monitor_service
        .get_check_ins(MonitorId::from_uuid(monitor_id), limit, offset)
        .await?;
    Ok(Json(CheckInsPage {
        items: page
            .check_ins
            .into_iter()
            .map(|c| CheckInResponse {
                id: c.id,
                monitor_id: c.monitor_id,
                checked_in_at: c.checked_in_at.to_rfc3339(),
                outcome: c.outcome.to_string(),
                message: c.message,
            })
            .collect(),
        total: page.total,
        limit,
        offset,
    }))
}

#[axum::debug_handler]
pub async fn ping_monitor(
    State(state): State<AppState>,
    Path(reference): Path<String>,
    body: String,
) -> AppResult<StatusCode> {
    let monitor_id = state.monitor_service.resolve_monitor_id(&reference).await?;
    let message = optional_message(body);
    state.monitor_service.ping(monitor_id, message).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn success_check_in(
    State(state): State<AppState>,
    Path(reference): Path<String>,
    body: String,
) -> AppResult<StatusCode> {
    let monitor_id = state.monitor_service.resolve_monitor_id(&reference).await?;
    let message = optional_message(body);
    state
        .monitor_service
        .check_in(monitor_id, CheckInOutcome::Success, message)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn failure_check_in(
    State(state): State<AppState>,
    Path(reference): Path<String>,
    body: String,
) -> AppResult<StatusCode> {
    let monitor_id = state.monitor_service.resolve_monitor_id(&reference).await?;
    let message = optional_message(body);
    state
        .monitor_service
        .check_in(monitor_id, CheckInOutcome::Failure, message)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Map a raw ingress body to an optional message.
///
/// Empty bodies (no `-d` on the curl) yield `None` so existing bodyless pings
/// keep working. Non-UTF-8 is rejected earlier by the `String` extractor (400);
/// oversize bodies are rejected earlier by `DefaultBodyLimit` on the ingress
/// router (413). Trimming is the caller's concern: store verbatim here so the
/// dashboard shows exactly what the job sent, and let notification formatting
/// decide how to present it.
fn optional_message(body: String) -> Option<String> {
    if body.is_empty() { None } else { Some(body) }
}
