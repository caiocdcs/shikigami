use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::core::domain::{IntegrationError, MonitorError};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        let body = serde_json::json!({ "error": { "message": message } });
        (status, Json(body)).into_response()
    }
}

impl From<IntegrationError> for AppError {
    fn from(err: IntegrationError) -> Self {
        match err {
            IntegrationError::InvalidConfig(field) => AppError::Validation(field),
            IntegrationError::NotFound(_) => AppError::NotFound,
            IntegrationError::Conflict(msg) => AppError::Conflict(msg),
            IntegrationError::Database(msg) => AppError::Internal(msg),
        }
    }
}

impl From<MonitorError> for AppError {
    fn from(err: MonitorError) -> Self {
        match err {
            MonitorError::InvalidConfig(field) => AppError::Validation(field),
            MonitorError::NotFound(_) => AppError::NotFound,
            MonitorError::Conflict(msg) => AppError::Conflict(msg),
            MonitorError::Database(msg) => AppError::Internal(msg),
        }
    }
}
