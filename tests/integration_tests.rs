use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use shikigami::{config::Config, create_app_with_pool};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn test_app() -> axum::Router {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("failed to create test pool");
    let config = Config::for_test("sqlite::memory:");
    create_app_with_pool(pool, config)
        .await
        .expect("failed to create test app")
}

async fn response_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("failed to read body")
        .to_bytes();
    serde_json::from_slice(&body).expect("failed to parse json")
}

#[tokio::test]
async fn create_integration_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "homelab-ntfy",
                        "channel": "ntfy",
                        "config": {
                            "url": "https://ntfy.sh",
                            "topic": "alerts",
                            "priority": 4,
                            "message": "monitor down"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["name"], "homelab-ntfy");
    assert_eq!(body["channel"], "ntfy");
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn get_integration_returns_200() {
    let app = test_app().await;

    // Create first
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "homelab-ntfy",
                        "channel": "ntfy",
                        "config": {
                            "url": "https://ntfy.sh",
                            "topic": "alerts",
                            "priority": 4,
                            "message": "monitor down"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = response_json(create_response).await;
    let id = create_body["id"].as_str().unwrap();

    // Then get
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/integrations/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = response_json(get_response).await;
    assert_eq!(body["id"], id);
    assert_eq!(body["name"], "homelab-ntfy");
}

#[tokio::test]
async fn get_integration_not_found_returns_404() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/integrations/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_integration_returns_204() {
    let app = test_app().await;

    // Create first
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "to-delete",
                        "channel": "ntfy",
                        "config": {
                            "url": "https://ntfy.sh",
                            "topic": "del",
                            "priority": 4,
                            "message": "gone"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = response_json(create_response).await;
    let id = create_body["id"].as_str().unwrap();

    // Delete
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/integrations/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Verify it's gone
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/integrations/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_integration_invalid_channel_returns_400() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "bad-channel",
                        "channel": "carrier-pigeon",
                        "config": {}
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_integration_empty_field_returns_400() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "empty-url",
                        "channel": "ntfy",
                        "config": {
                            "url": "",
                            "topic": "alerts",
                            "priority": 4,
                            "message": "down"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_gotify_integration_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "homelab-gotify",
                        "channel": "gotify",
                        "config": {
                            "url": "https://gotify.example.com",
                            "token": "abc123",
                            "priority": 5
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["name"], "homelab-gotify");
    assert_eq!(body["channel"], "gotify");
}

#[tokio::test]
async fn create_email_integration_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "homelab-email",
                        "channel": "email",
                        "config": {
                            "smtp_host": "smtp.example.com",
                            "smtp_port": 587,
                            "to": "me@example.com",
                            "from": "shikigami@example.com"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["name"], "homelab-email");
    assert_eq!(body["channel"], "email");
}

#[tokio::test]
async fn create_slack_integration_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "homelab-slack",
                        "channel": "slack",
                        "config": {
                            "webhook_url": "https://hooks.slack.com/services/xxx"
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["name"], "homelab-slack");
    assert_eq!(body["channel"], "slack");
}
