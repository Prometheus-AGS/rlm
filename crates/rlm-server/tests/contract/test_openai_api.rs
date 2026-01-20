//! OpenAI API contract tests for /v1/chat/completions endpoint.
//!
//! These tests verify that the RLM server provides complete OpenAI API compatibility
//! for chat completions, including request/response formats, error handling, and
//! all required/optional parameters.

use axum::http::StatusCode;
use axum_test::TestServer;
use rlm_core::types::{ChatCompletionRequest, ChatMessage, ChatRole, RlmConfig, AggregationStrategy};
use rlm_server::create_test_app;
use serde_json::json;

#[tokio::test]
async fn test_openai_chat_completions_basic() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Hello, world!".to_string(),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();

    // Verify OpenAI-compatible response structure
    assert!(body["id"].is_string());
    assert_eq!(body["object"], "chat.completion");
    assert!(body["created"].is_number());
    assert_eq!(body["model"], "gpt-4");
    assert!(body["choices"].is_array());
    assert!(body["usage"].is_object());

    // Verify choice structure
    let choices = &body["choices"];
    assert_eq!(choices.as_array().unwrap().len(), 1);

    let choice = &choices[0];
    assert_eq!(choice["index"], 0);
    assert!(choice["message"]["role"] == "assistant");
    assert!(choice["message"]["content"].is_string());
    assert!(choice["finish_reason"].is_string());

    // Verify usage structure
    let usage = &body["usage"];
    assert!(usage["prompt_tokens"].is_number());
    assert!(usage["completion_tokens"].is_number());
    assert!(usage["total_tokens"].is_number());
}

#[tokio::test]
async fn test_openai_chat_completions_with_system_message() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: "What is 2 + 2?".to_string(),
                name: None,
            },
        ],
        max_tokens: Some(50),
        temperature: Some(0.1),
        stream: false,
        stop: None,
        top_p: Some(1.0),
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["model"], "gpt-4");
    assert!(body["choices"][0]["message"]["content"].as_str().unwrap().contains("4"));
}

#[tokio::test]
async fn test_openai_chat_completions_with_conversation_history() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "My name is Alice.".to_string(),
                name: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Hello Alice! Nice to meet you.".to_string(),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: "What's my name?".to_string(),
                name: None,
            },
        ],
        max_tokens: Some(20),
        temperature: Some(0.0),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(content.to_lowercase().contains("alice"));
}

#[tokio::test]
async fn test_openai_chat_completions_long_context() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create a long context (simulating 100K+ tokens)
    let long_context = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(2000);
    let question = "What is the main topic of this document?";

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: format!("Document: {}", long_context),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: question.to_string(),
                name: None,
            },
        ],
        max_tokens: Some(100),
        temperature: Some(0.5),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: Some(RlmConfig {
            max_recursive_depth: Some(5),
        }),
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    // This should succeed with RLM processing the long context
    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["choices"][0]["message"]["content"].is_string());

    // Should have RLM-specific metadata in usage
    let usage = &body["usage"];
    assert!(usage["total_tokens"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_openai_chat_completions_error_invalid_model() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = json!({
        "model": "invalid-model-name",
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ]
    });

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert!(body["error"]["message"].as_str().unwrap().contains("model"));
}

#[tokio::test]
async fn test_openai_chat_completions_error_empty_messages() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = json!({
        "model": "gpt-4",
        "messages": []
    });

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["error"]["type"], "invalid_request_error");
    assert!(body["error"]["message"].as_str().unwrap().contains("messages"));
}

#[tokio::test]
async fn test_openai_chat_completions_with_stop_sequences() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Count from 1 to 10".to_string(),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.0),
        stream: false,
        stop: Some(vec!["5".to_string()]),
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["choices"][0]["finish_reason"], "stop");

    // Content should stop at or before "5"
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(!content.contains("6"));
}

#[tokio::test]
async fn test_openai_chat_completions_max_tokens_limit() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Write a very long story about a dragon.".to_string(),
            name: None,
        }],
        max_tokens: Some(10), // Very small limit
        temperature: Some(0.7),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["choices"][0]["finish_reason"], "length");

    // Token usage should not exceed max_tokens significantly
    let usage = &body["usage"];
    let completion_tokens = usage["completion_tokens"].as_u64().unwrap();
    assert!(completion_tokens <= 15); // Allow some buffer for encoding
}

#[tokio::test]
async fn test_openai_chat_completions_temperature_variations() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with temperature = 0 (deterministic)
    let request_deterministic = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "What is the capital of France?".to_string(),
            name: None,
        }],
        max_tokens: Some(10),
        temperature: Some(0.0),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    let response1 = server
        .post("/v1/chat/completions")
        .json(&request_deterministic)
        .await;

    let response2 = server
        .post("/v1/chat/completions")
        .json(&request_deterministic)
        .await;

    response1.assert_status(StatusCode::OK);
    response2.assert_status(StatusCode::OK);

    // With temperature=0, responses should be similar/identical
    let body1 = response1.json::<serde_json::Value>();
    let body2 = response2.json::<serde_json::Value>();

    let content1 = body1["choices"][0]["message"]["content"].as_str().unwrap();
    let content2 = body2["choices"][0]["message"]["content"].as_str().unwrap();

    // Both should mention Paris (deterministic response)
    assert!(content1.to_lowercase().contains("paris"));
    assert!(content2.to_lowercase().contains("paris"));
}

#[tokio::test]
async fn test_openai_chat_completions_content_type_validation() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Test with wrong content type
    let response = server
        .post("/v1/chat/completions")
        .header("content-type", "text/plain")
        .text("invalid json")
        .await;

    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn test_openai_chat_completions_malformed_json() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    let response = server
        .post("/v1/chat/completions")
        .header("content-type", "application/json")
        .text("{ invalid json }")
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["error"]["type"], "invalid_request_error");
}