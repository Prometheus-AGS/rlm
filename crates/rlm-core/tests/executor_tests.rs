//! RLM core executor unit tests.
//!
//! These tests verify core executor functionality and integration.

use rlm_core::{RlmConfig, ports::ReplBackend};
use rlm_repl_rhai::RhaiReplBackend;

#[tokio::test]
async fn test_executor_creation_with_mock_providers() {
    use rlm_repl_rhai::functions::MockLlmProvider;
    use std::sync::Arc;

    let config = RlmConfig::default();
    let repl = RhaiReplBackend::new().expect("Failed to create REPL backend");
    let _llm = Arc::new(MockLlmProvider::with_response("Mock LLM response"));

    // This would require a mock implementation of the LlmProvider trait
    // For now, verify that the components can be created separately
    assert!(repl.health_check().await.unwrap());

    // Test basic config validation
    assert_eq!(config.repl.backend, "rhai");
    assert_eq!(config.llm.provider, "openai");
    assert_eq!(config.limits.max_iterations, 50);
    assert_eq!(config.limits.max_recursion_depth, 1);
}

#[test]
fn test_config_defaults() {
    let config = RlmConfig::default();

    // Test REPL config defaults
    assert_eq!(config.repl.backend, "rhai");
    assert!(config.repl.sandbox);
    assert_eq!(config.repl.max_memory_mb, 512);

    // Test LLM config defaults
    assert_eq!(config.llm.provider, "openai");
    assert_eq!(config.llm.model, "gpt-4-turbo");
    assert_eq!(config.llm.temperature, 0.0);
    assert_eq!(config.llm.max_tokens, 4096);

    // Test execution limits defaults
    assert_eq!(config.limits.max_iterations, 50);
    assert_eq!(config.limits.max_recursion_depth, 1);
    assert_eq!(config.limits.chunk_size_tokens, 4096);

    // Test streaming config defaults
    assert!(config.streaming.enabled);
    assert_eq!(config.streaming.buffer_size, 100);
    assert!(!config.streaming.include_repl_state);
}

#[test]
fn test_rlm_request_creation() {
    use rlm_core::RlmRequest;
    use std::collections::HashMap;

    let request = RlmRequest {
        query: "What is the main topic?".to_string(),
        context: "This is about machine learning.".to_string(),
        recursion_depth: 1,
        max_iterations: 10,
        metadata: HashMap::new(),
    };

    assert_eq!(request.query, "What is the main topic?");
    assert_eq!(request.context, "This is about machine learning.");
    assert_eq!(request.recursion_depth, 1);
    assert_eq!(request.max_iterations, 10);
    assert!(request.metadata.is_empty());
}

#[test]
fn test_chat_types_creation() {
    use rlm_core::{ChatMessage, ChatRole, ChatCompletionRequest};

    let message = ChatMessage {
        role: ChatRole::User,
        content: "Hello!".to_string(),
        name: None,
    };

    assert!(matches!(message.role, ChatRole::User));
    assert_eq!(message.content, "Hello!");
    assert!(message.name.is_none());

    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![message],
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    assert_eq!(request.model, "gpt-4");
    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.max_tokens, Some(100));
    assert_eq!(request.temperature, Some(0.7));
    assert!(!request.stream);
}

// TODO: Add integration tests once REPL and LLM adapters are implemented
// TODO: Add mock provider tests for complete execution workflow
// TODO: Add error handling tests for various failure scenarios