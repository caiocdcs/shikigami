use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use secrecy::ExposeSecret;

use crate::{AppState, error::AppError};

pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(expected) = state.config.api_key.as_ref() else {
        return Ok(next.run(req).await);
    };

    let provided = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match provided {
        Some(token) if token == expected.expose_secret() => Ok(next.run(req).await),
        _ => Err(AppError::Unauthorized),
    }
}
