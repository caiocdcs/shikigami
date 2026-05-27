use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};

use crate::AppState;
use crate::api::handlers;

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
            get(handlers::get_integration).delete(handlers::delete_integration),
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
