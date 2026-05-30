use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
};

use crate::AppState;
use crate::api::handlers;
use crate::api::monitor_handlers;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route(
            "/integrations",
            post(handlers::create_integration).get(handlers::get_integrations),
        )
        .route(
            "/integrations/{id}",
            get(handlers::get_integration)
                .delete(handlers::delete_integration)
                .put(handlers::update_integration),
        )
        .route(
            "/monitors",
            post(monitor_handlers::create_monitor).get(monitor_handlers::get_monitors),
        )
        .route(
            "/monitors/{id}",
            get(monitor_handlers::get_monitor)
                .delete(monitor_handlers::delete_monitor)
                .put(monitor_handlers::update_monitor),
        )
        .route(
            "/monitors/{id}/check-ins",
            get(monitor_handlers::get_monitor_check_ins),
        )
        .route(
            "/monitors/{monitor_id}/integrations",
            post(monitor_handlers::link_integration)
                .get(monitor_handlers::get_monitor_integrations),
        )
        .route(
            "/monitors/{monitor_id}/integrations/{integration_id}",
            delete(monitor_handlers::unlink_integration),
        )
        .route("/ping/{monitor_id}", post(monitor_handlers::ping_monitor))
        .route(
            "/success/{monitor_id}",
            post(monitor_handlers::success_check_in),
        )
        .route(
            "/failure/{monitor_id}",
            post(monitor_handlers::failure_check_in),
        )
        .with_state(state)
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn readiness_check(State(state): State<AppState>) -> StatusCode {
    if sqlx::query("SELECT 1")
        .execute(&state.pg_pool)
        .await
        .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
