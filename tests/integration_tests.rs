use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use shikigami::{config::Config, create_app_with_pool};
use sqlx::SqlitePool;
use tower::ServiceExt;

async fn test_app() -> axum::Router {
    let (app, _pool) = test_app_with_pool().await;
    app
}

async fn test_app_with_pool() -> (axum::Router, SqlitePool) {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("failed to create test pool");
    let config = Config::for_test("sqlite::memory:");
    let app = create_app_with_pool(
        pool.clone(),
        config,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("failed to create test app");
    (app, pool)
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

async fn create_test_monitor(app: axum::Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "test-monitor",
                        "slug": "test-monitor",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let body = response_json(response).await;
    body["id"].as_str().expect("id").to_string()
}

async fn create_test_cron_monitor(app: axum::Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "test-cron",
                        "slug": "test-cron",
                        "schedule_type": "cron",
                        "cron_expr": "0 * * * *",
                        "grace_seconds": 300
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let body = response_json(response).await;
    body["id"].as_str().expect("id").to_string()
}

async fn create_test_integration(app: axum::Router) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "test-ntfy",
                        "channel": "ntfy",
                        "config": {
                            "url": "https://ntfy.sh",
                            "topic": "alerts",
                            "priority": 4,
                            "message": "down"
                        }
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    let body = response_json(response).await;
    body["id"].as_str().expect("id").to_string()
}

async fn link_monitor_integration(app: axum::Router, monitor_id: &str, integration_id: &str) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/monitors/{monitor_id}/integrations"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "integration_id": integration_id
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
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

// --- Monitor tests ---

#[tokio::test]
async fn create_interval_monitor_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "nightly-backup",
                        "description": "Home lab backup job",
                        "slug": "nightly-backup",
                        "schedule_type": "interval",
                        "interval_seconds": 86400,
                        "grace_seconds": 3600
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["name"], "nightly-backup");
    assert_eq!(body["status"], "active");
    assert_eq!(body["schedule_type"], "interval");
}

#[tokio::test]
async fn create_cron_monitor_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "cron-job",
                        "slug": "cron-job",
                        "schedule_type": "cron",
                        "cron_expr": "0 2 * * *",
                        "grace_seconds": 300
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    assert_eq!(body["schedule_type"], "cron");
    assert_eq!(body["cron_expr"], "0 2 * * *");
}

#[tokio::test]
async fn get_monitor_returns_200() {
    let app = test_app().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "test-monitor",
                        "slug": "test-monitor",
                        "schedule_type": "interval",
                        "interval_seconds": 3600,
                        "grace_seconds": 60
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = response_json(create_response).await;
    let id = create_body["id"].as_str().unwrap();

    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/monitors/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let body = response_json(get_response).await;
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn delete_monitor_returns_204() {
    let app = test_app().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "to-delete",
                        "slug": "to-delete",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = response_json(create_response).await;
    let id = create_body["id"].as_str().unwrap();

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/monitors/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let get_response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/monitors/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_monitor_invalid_schedule_returns_400() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "bad-schedule",
                        "slug": "bad-schedule",
                        "schedule_type": "cron",
                        "grace_seconds": 60
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
async fn link_integration_to_monitor_returns_200() {
    let app = test_app().await;

    // Create integration
    let int_response = app
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
    let int_body = response_json(int_response).await;
    let int_id = int_body["id"].as_str().unwrap();

    // Create monitor
    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "nightly-backup",
                        "slug": "nightly-backup",
                        "schedule_type": "interval",
                        "interval_seconds": 86400,
                        "grace_seconds": 3600
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    // Link
    let link_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/monitors/{mon_id}/integrations"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "integration_id": int_id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(link_response.status(), StatusCode::NO_CONTENT);

    // Unlink
    let unlink_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/monitors/{mon_id}/integrations/{int_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(unlink_response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn duplicate_slug_returns_409() {
    let app = test_app().await;

    let body = serde_json::to_string(&serde_json::json!({
        "name": "first",
        "slug": "same-slug",
        "schedule_type": "interval",
        "interval_seconds": 60,
        "grace_seconds": 10
    }))
    .unwrap();

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first.status(), StatusCode::CREATED);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn link_nonexistent_integration_returns_400() {
    let app = test_app().await;

    // Create monitor
    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "test",
                        "slug": "fk-test",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    // Link with nonexistent integration
    let link_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/monitors/{mon_id}/integrations"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "integration_id": "00000000-0000-0000-0000-000000000000"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(link_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ping_updates_timestamps() {
    let app = test_app().await;

    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "interval-ping",
                        "slug": "interval-ping",
                        "schedule_type": "interval",
                        "interval_seconds": 3600,
                        "grace_seconds": 600
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    let ping_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/ping/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(ping_response.status(), StatusCode::NO_CONTENT);

    // Verify timestamps were updated
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/monitors/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response_json(get_response).await;
    assert!(body["last_pinged_at"].is_string());
    assert!(body["next_expected_at"].is_string());
}

#[tokio::test]
async fn ping_nonexistent_monitor_returns_404() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/ping/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn success_creates_check_in() {
    let app = test_app().await;

    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "success-check",
                        "slug": "success-check",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/success/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn failure_creates_check_in() {
    let (app, pool) = test_app_with_pool().await;

    let int_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "failure-ntfy",
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
    let int_body = response_json(int_response).await;
    let int_id = int_body["id"].as_str().unwrap();

    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "failure-check",
                        "slug": "failure-check",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    // Link integration to monitor
    let _link_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/monitors/{mon_id}/integrations"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "integration_id": int_id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/failure/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify notification_outbox has an entry
    let outbox_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_outbox WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
    assert_eq!(
        outbox_count, 1,
        "notification_outbox should have 1 entry on failure"
    );
}

#[tokio::test]
async fn success_nonexistent_monitor_returns_404() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/success/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn failure_nonexistent_monitor_returns_404() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/failure/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ping_with_cron_updates_next_expected_at() {
    let app = test_app().await;

    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "cron-ping",
                        "slug": "cron-ping",
                        "schedule_type": "cron",
                        "cron_expr": "0 0 * * *",
                        "grace_seconds": 600
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    let ping_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/ping/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(ping_response.status(), StatusCode::NO_CONTENT);

    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/monitors/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response_json(get_response).await;
    assert!(body["last_pinged_at"].is_string());
    assert!(body["next_expected_at"].is_string());
}

#[tokio::test]
async fn delete_monitor_cascades_to_checkins_integrations_outbox() {
    let (app, pool) = test_app_with_pool().await;

    // Create integration
    let int_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/integrations")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "cascade-ntfy",
                        "channel": "ntfy",
                        "config": {
                            "url": "https://ntfy.sh",
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
    let int_body = response_json(int_response).await;
    let int_id = int_body["id"].as_str().unwrap();

    // Create monitor
    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "cascade-monitor",
                        "slug": "cascade-monitor",
                        "schedule_type": "interval",
                        "interval_seconds": 60,
                        "grace_seconds": 10
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;
    let mon_id = mon_body["id"].as_str().unwrap();

    // Link integration
    let _link = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/monitors/{mon_id}/integrations"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "integration_id": int_id
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create a failure check-in (writes to outbox)
    let _failure = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/failure/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify data exists before delete
    let check_ins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM check_ins WHERE monitor_id = ?")
        .bind(mon_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(check_ins, 1, "should have 1 check-in before delete");

    let links: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM monitor_integrations WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(links, 1, "should have 1 integration link before delete");

    let outbox: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_outbox WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(outbox, 1, "should have 1 outbox entry before delete");

    // Delete the monitor
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/monitors/{mon_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Verify cascade: check-ins, links, and outbox are gone
    let check_ins_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM check_ins WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(check_ins_after, 0, "check-ins should be cascade-deleted");

    let links_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM monitor_integrations WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        links_after, 0,
        "integration links should be cascade-deleted"
    );

    let outbox_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notification_outbox WHERE monitor_id = ?")
            .bind(mon_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(outbox_after, 0, "outbox entries should be cascade-deleted");

    // Integration itself should still exist
    let int_still: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM integrations WHERE id = ?")
        .bind(int_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(int_still, 1, "integration should survive monitor deletion");
}

#[tokio::test]
async fn new_monitor_has_initial_next_expected_at() {
    let app = test_app().await;

    // Create interval monitor
    let mon_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "initial-next",
                        "slug": "initial-next",
                        "schedule_type": "interval",
                        "interval_seconds": 300,
                        "grace_seconds": 30
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let mon_body = response_json(mon_response).await;

    assert!(
        mon_body["next_expected_at"].is_string(),
        "new monitor should have next_expected_at set"
    );
    assert!(
        mon_body["last_pinged_at"].is_null(),
        "new monitor should not have last_pinged_at"
    );

    // Create cron monitor
    let cron_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "cron-initial",
                        "slug": "cron-initial",
                        "schedule_type": "cron",
                        "cron_expr": "0 * * * *",
                        "grace_seconds": 300
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cron_body = response_json(cron_response).await;
    assert!(
        cron_body["next_expected_at"].is_string(),
        "new cron monitor should have next_expected_at set"
    );
}

#[tokio::test]
async fn create_cron_monitor_with_timezone_evaluates_in_local_time() {
    let app = test_app().await;

    // 9am daily in Sao_Paulo (UTC-3, no DST) -> next_expected_at at 12:00 UTC.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "tz-cron",
                        "slug": "tz-cron",
                        "schedule_type": "cron",
                        "cron_expr": "0 9 * * *",
                        "grace_seconds": 300,
                        "timezone": "America/Sao_Paulo"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["timezone"], "America/Sao_Paulo");
    let next = body["next_expected_at"].as_str().unwrap();
    assert!(
        next.contains("T12:00:00"),
        "expected 9am Sao_Paulo stored as 12:00 UTC, got {next}"
    );
}

#[tokio::test]
async fn create_cron_monitor_without_timezone_defaults_to_utc() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "utc-cron",
                        "slug": "utc-cron",
                        "schedule_type": "cron",
                        "cron_expr": "0 9 * * *",
                        "grace_seconds": 300
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response_json(response).await;
    assert_eq!(body["timezone"], "UTC");
    let next = body["next_expected_at"].as_str().unwrap();
    assert!(next.contains("T09:00:00"), "expected 9am UTC, got {next}");
}

#[tokio::test]
async fn create_cron_monitor_invalid_timezone_returns_400() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/monitors")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "name": "bad-tz",
                        "slug": "bad-tz",
                        "schedule_type": "cron",
                        "cron_expr": "0 9 * * *",
                        "grace_seconds": 300,
                        "timezone": "Not/AZone"
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
async fn delete_nonexistent_monitor_returns_404() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/monitors/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
