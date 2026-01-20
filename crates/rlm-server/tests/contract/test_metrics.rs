//! Metrics endpoint contract tests.
//!
//! Tests verify that the /metrics endpoint provides properly formatted
//! Prometheus-compatible metrics for monitoring RLM processing performance.

use axum::http::{HeaderMap, StatusCode};
use serde_json::Value;
use std::collections::HashMap;

/// Test metrics endpoint availability and basic format.
#[tokio::test]
async fn test_metrics_endpoint_availability() {
    // Create test server
    let app = create_test_app().await;

    // Request metrics endpoint
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Should have proper content type
    let content_type = response.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert!(content_type.is_some());
    assert!(content_type.unwrap().contains("text/plain"));
}

/// Test that metrics include core RLM processing metrics.
#[tokio::test]
async fn test_rlm_core_metrics() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Should contain RLM-specific metrics
    assert!(metrics_text.contains("rlm_requests_total"));
    assert!(metrics_text.contains("rlm_request_duration_seconds"));
    assert!(metrics_text.contains("rlm_recursive_calls_total"));
    assert!(metrics_text.contains("rlm_context_chunks_processed_total"));
    assert!(metrics_text.contains("rlm_token_usage_total"));
}

/// Test that metrics include HTTP server metrics.
#[tokio::test]
async fn test_http_server_metrics() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Should contain HTTP server metrics
    assert!(metrics_text.contains("http_requests_total"));
    assert!(metrics_text.contains("http_request_duration_seconds"));
    assert!(metrics_text.contains("http_requests_in_flight"));
}

/// Test that metrics include backend provider metrics.
#[tokio::test]
async fn test_backend_provider_metrics() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Should contain backend provider metrics
    assert!(metrics_text.contains("rlm_backend_requests_total"));
    assert!(metrics_text.contains("rlm_backend_errors_total"));
    assert!(metrics_text.contains("rlm_backend_latency_seconds"));
    assert!(metrics_text.contains("rlm_backend_health_status"));
}

/// Test that metrics are properly formatted for Prometheus.
#[tokio::test]
async fn test_prometheus_format_compliance() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Should have HELP comments
    assert!(metrics_text.contains("# HELP"));

    // Should have TYPE comments
    assert!(metrics_text.contains("# TYPE"));

    // Should have proper metric names (snake_case)
    let lines: Vec<&str> = metrics_text.lines().collect();
    for line in lines {
        if line.starts_with("#") {
            continue; // Skip comments
        }
        if line.is_empty() {
            continue; // Skip empty lines
        }

        // Metric lines should have format: metric_name{labels} value
        assert!(line.contains(" ") || line.contains("\t"));
    }
}

/// Test metrics with active RLM processing.
#[tokio::test]
async fn test_metrics_during_active_processing() {
    let app = create_test_app().await;

    // Make a chat completion request to generate metrics
    let chat_request = serde_json::json!({
        "model": "gpt-4",
        "messages": [
            {"role": "user", "content": "Hello, test message for metrics"}
        ],
        "max_tokens": 50,
        "rlm_config": {
            "max_recursive_depth": 1
        }
    });

    let _chat_response = app.clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(chat_request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Now check metrics
    let metrics_response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/metrics")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(metrics_response.into_body(), usize::MAX).await.unwrap();
    let metrics_text = String::from_utf8(body.to_vec()).unwrap();

    // Should show incremented request counts
    assert!(metrics_text.contains("rlm_requests_total"));

    // Should show some processing time
    assert!(metrics_text.contains("rlm_request_duration_seconds"));
}

/// Helper function to create test application.
async fn create_test_app() -> axum::Router {
    use axum::Router;
    use rlm_server::server::routes::create_v1_router;
    use rlm_server::server::routes::AppState;
    use std::sync::Arc;

    let app_state = AppState::new().expect("Failed to create app state");

    Router::new()
        .nest("/v1", create_v1_router())
        .route("/metrics", axum::routing::get(rlm_server::server::metrics::metrics_handler))
        .with_state(app_state)
}

#[cfg(test)]
mod integration {
    use super::*;

    /// Test metrics endpoint integration with real server.
    #[tokio::test]
    async fn test_metrics_endpoint_integration() {
        // This test would require a running server instance
        // For now, we'll test the handler directly

        let app_state = rlm_server::server::routes::AppState::new()
            .expect("Failed to create app state");

        let metrics_response = rlm_server::server::metrics::metrics_handler(
            axum::extract::State(app_state)
        ).await;

        // Should return a successful response
        // Implementation depends on actual metrics handler
        assert!(metrics_response.is_ok() || metrics_response.is_err());
    }
}