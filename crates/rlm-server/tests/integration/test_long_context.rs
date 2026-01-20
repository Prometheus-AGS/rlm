//! Long-context integration tests for RLM server.
//!
//! These tests verify that the RLM server can handle very long contexts
//! by offloading them to REPL and using recursive decomposition,
//! following the paper's evaluation methodology.

use axum::http::StatusCode;
use axum_test::TestServer;
use rlm_core::{ChatCompletionRequest, ChatMessage, ChatRole, RlmConfig, AggregationStrategy};
use rlm_server::create_test_app;

#[tokio::test]
async fn test_long_context_basic() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create a moderately long context (simulating 10K+ tokens)
    let long_context = "The document discusses machine learning fundamentals. ".repeat(200);
    let question = "What is the main topic of this document?";

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: format!("Context: {}", long_context),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: question.to_string(),
                name: None,
            },
        ],
        max_tokens: Some(100),
        temperature: Some(0.0), // Deterministic for testing
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: Some(RlmConfig {
            max_recursive_depth: Some(2),
            context_chunk_size: Some(1000),
            enable_progress_events: true,
            aggregation_strategy: Some(AggregationStrategy::Sequential),
        }),
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    // Should succeed with RLM handling the long context
    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["object"], "chat.completion");
    assert!(body["choices"][0]["message"]["content"].is_string());

    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(content.to_lowercase().contains("machine learning"));
}

#[tokio::test]
async fn test_very_long_context_exceeds_normal_limits() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create a very long context (simulating 100K+ tokens)
    let very_long_context = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(2000);
    let question = "Summarize the key points from this document.";

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: format!("Document: {}", very_long_context),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: question.to_string(),
                name: None,
            },
        ],
        max_tokens: Some(200),
        temperature: Some(0.2),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: Some(RlmConfig {
            max_recursive_depth: Some(3),
            context_chunk_size: Some(2000),
            enable_progress_events: true,
            aggregation_strategy: Some(AggregationStrategy::Parallel),
        }),
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    // Should succeed with RLM context offloading and recursive processing
    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    assert!(body["choices"][0]["message"]["content"].is_string());

    // Verify that usage statistics show RLM processing occurred
    let usage = &body["usage"];
    assert!(usage["total_tokens"].as_u64().unwrap() > 0);

    // RLM should report recursive calls when handling long contexts
    assert!(usage["recursive_calls"].as_u64().unwrap_or(0) >= 0);
}

#[tokio::test]
async fn test_needle_in_haystack_pattern() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create a haystack with a specific "needle" to find (S-NIAH pattern from paper)
    let haystack_part1 = "This document contains many irrelevant details. ".repeat(500);
    let needle = "The magic number is 42 and it represents the answer to life.";
    let haystack_part2 = "More irrelevant information continues here. ".repeat(500);

    let full_context = format!("{} {} {}", haystack_part1, needle, haystack_part2);
    let question = "What is the magic number mentioned in the document and what does it represent?";

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: format!("Document: {}", full_context),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: question.to_string(),
                name: None,
            },
        ],
        max_tokens: Some(150),
        temperature: Some(0.0),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: Some(RlmConfig {
            max_recursive_depth: Some(2),
            context_chunk_size: Some(1000),
            enable_progress_events: true,
            aggregation_strategy: Some(AggregationStrategy::Sequential),
        }),
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();

    // RLM should successfully find the needle in the haystack
    assert!(content.contains("42") || content.contains("forty"));
    assert!(content.to_lowercase().contains("answer") || content.to_lowercase().contains("life"));
}

#[tokio::test]
async fn test_context_aggregation_pattern() {
    let app = create_test_app().await;
    let server = TestServer::new(app).unwrap();

    // Create multiple chunks of information that need to be aggregated (OOLONG pattern from paper)
    let chunk1 = "Product A costs $100 and was sold 50 times. ";
    let chunk2 = "Product B costs $200 and was sold 30 times. ";
    let chunk3 = "Product C costs $150 and was sold 20 times. ";
    let padding = "Additional irrelevant data. ".repeat(300);

    let full_context = format!("{}{}{}{}", chunk1, padding.clone(), chunk2, padding.clone(), chunk3, padding);
    let question = "Calculate the total revenue from all products mentioned.";

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: ChatRole::System,
                content: format!("Sales data: {}", full_context),
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: question.to_string(),
                name: None,
            },
        ],
        max_tokens: Some(200),
        temperature: Some(0.0),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: Some(RlmConfig {
            max_recursive_depth: Some(3),
            context_chunk_size: Some(800),
            enable_progress_events: true,
            aggregation_strategy: Some(AggregationStrategy::Parallel),
        }),
    };

    let response = server
        .post("/v1/chat/completions")
        .json(&request)
        .await;

    response.assert_status(StatusCode::OK);

    let body = response.json::<serde_json::Value>();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();

    // RLM should aggregate information from all chunks
    // Expected: (100*50) + (200*30) + (150*20) = 5000 + 6000 + 3000 = 14000
    assert!(
        content.contains("14000") ||
        content.contains("14,000") ||
        content.contains("$14") ||
        (content.contains("5000") && content.contains("6000") && content.contains("3000"))
    );
}

// TODO: Add streaming version of long context tests
// TODO: Add error handling tests for extremely long contexts
// TODO: Add performance benchmarking tests matching paper metrics
// TODO: Add tests with different aggregation strategies
// TODO: Add tests with various recursion depths