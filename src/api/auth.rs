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
        Some(token) if constant_time_eq(token.as_bytes(), expected.expose_secret().as_bytes()) => {
            Ok(next.run(req).await)
        }
        _ => Err(AppError::Unauthorized),
    }
}

/// Compare without short-circuiting on the first differing byte, so response
/// timing does not reveal how much of a guessed key is correct. Only the
/// length can leak, which says nothing about the key's content.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let diff = a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y));
    std::hint::black_box(diff) == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn constant_time_eq_matches_equality() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secret2"));
        assert!(!constant_time_eq(b"", b"x"));
    }
}
