//! SSE streaming contract tests for RLM server.
//!
//! These tests verify that the RLM server properly implements Server-Sent Events
//! for real-time streaming of responses, progress events, and completion markers.

use axum::http::StatusCode;
use axum_test::TestServer;
use rlm_core::{ChatCompletionRequest, ChatMessage, ChatRole};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Create a test server instance for streaming tests.
async fn create_test_server() -> TestServer {
    let app = rlm_server::create_test_app().await;
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_streaming_chat_completion_headers() {
    let server = create_test_server().await;

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Hello, how are you?".to_string(),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: true,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    // Verify SSE headers are set correctly
    assert_eq!(response.status_code(), StatusCode::OK);

    let headers = response.headers();
    assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
    assert_eq!(headers.get("cache-control").unwrap(), "no-cache");
    assert_eq!(headers.get("connection").unwrap(), "keep-alive");
}

#[tokio::test]
async fn test_streaming_chat_completion_with_short_context() {
    let server = create_test_server().await;

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "What is 2+2? Give a brief answer.".to_string(),
            name: None,
        }],
        max_tokens: Some(50),
        temperature: Some(0.0),
        stream: true,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    // Get response body as text to parse SSE events
    let body = response.text();

    // Should contain at least one data chunk and a [DONE] marker
    assert!(body.contains("data: "));
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn test_streaming_progress_events() {
    let server = create_test_server().await;

    // Use a longer context to trigger RLM processing with recursive calls
    let long_context = "Context: ".repeat(1000);
    let query = format!("{}What is the main theme of this context?", long_context);

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: query,
            name: None,
        }],
        max_tokens: Some(200),
        temperature: Some(0.0),
        stream: true,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.text();

    // Should contain RLM-specific progress events
    // These would be emitted during context offloading and recursive processing
    assert!(body.contains("data: "));
    assert!(body.contains("data: [DONE]"));

    // Parse individual SSE events
    let events: Vec<&str> = body.split("data: ").filter(|s| !s.is_empty()).collect();
    assert!(!events.is_empty());

    // Last event should be [DONE]
    let last_event = events.last().unwrap().trim();
    assert_eq!(last_event, "[DONE]");
}

#[tokio::test]
async fn test_streaming_with_rlm_config() {
    let server = create_test_server().await;

    let request = json!({
        "model": "gpt-4",
        "messages": [
            {
                "role": "user",
                "content": "Explain the concept of recursive language models."
            }
        ],
        "stream": true,
        "max_tokens": 150,
        "temperature": 0.5,
        "rlm_config": {
            "max_recursive_depth": 2,
            "enable_progress_events": true,
            "context_chunk_size": 1000
        }
    });

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.text();

    // Should contain streaming data with RLM configuration applied
    assert!(body.contains("data: "));
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn test_streaming_error_handling() {
    let server = create_test_server().await;

    // Test with invalid model
    let request = ChatCompletionRequest {
        model: "invalid-model".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Test message".to_string(),
            name: None,
        }],
        stream: true,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    // Should return error status but still with proper SSE headers for error streaming
    assert!(response.status_code().is_client_error() || response.status_code().is_server_error());
}

#[tokio::test]
async fn test_streaming_connection_management() {
    let server = create_test_server().await;

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Generate a longer response about artificial intelligence.".to_string(),
            name: None,
        }],
        max_tokens: Some(500),
        temperature: Some(0.7),
        stream: true,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    // Test with timeout to ensure streaming doesn't hang indefinitely
    let response_future = server
        .post("/v1/chat/completions")
        .json(&request);

    let response = timeout(Duration::from_secs(30), response_future)
        .await
        .expect("Streaming response should complete within 30 seconds");

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.text();

    // Verify proper SSE event structure
    let lines: Vec<&str> = body.lines().collect();

    // Should have proper SSE format with event: and data: lines
    let data_lines: Vec<&str> = lines.iter().filter(|line| line.starts_with("data: ")).copied().collect();
    assert!(!data_lines.is_empty());

    // Should end with [DONE]
    assert!(data_lines.iter().any(|line| line.contains("[DONE]")));
}

#[tokio::test]
async fn test_non_streaming_vs_streaming_compatibility() {
    let server = create_test_server().await;

    let base_request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "What is the capital of France?".to_string(),
            name: None,
        }],
        max_tokens: Some(50),
        temperature: Some(0.0),
        stream: false, // Will be modified
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    // Test non-streaming
    let non_streaming_response = server
        .post("/v1/chat/completions")
        .json(&base_request)
        .await;

    assert_eq!(non_streaming_response.status_code(), StatusCode::OK);
    assert_eq!(
        non_streaming_response.headers().get("content-type").unwrap(),
        "application/json"
    );

    // Test streaming
    let mut streaming_request = base_request;
    streaming_request.stream = true;

    let streaming_response = server
        .post("/v1/chat/completions")
        .json(&streaming_request)
        .await;

    assert_eq!(streaming_response.status_code(), StatusCode::OK);
    assert_eq!(
        streaming_response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

#[tokio::test]
async fn test_streaming_with_stop_sequences() {
    let server = create_test_server().await;

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Count to 10: 1, 2, 3,".to_string(),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.0),
        stream: true,
        stop: Some(vec!["5".to_string()]),
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    assert_eq!(response.status_code(), StatusCode::OK);

    let body = response.text();

    // Should contain streaming data that stops at the specified sequence
    assert!(body.contains("data: "));
    assert!(body.contains("data: [DONE]"));
}