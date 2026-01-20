use rlm_core::{
    executor::RlmExecutor,
    ports::{EventSink, LlmProvider, ReplBackend, ModelInfo, SinkStats, SinkMetadata,
            EventFilter, ProviderMetadata, ReplMetadata},
    types::{
        CallStatus, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole,
        ChatChoice, RlmEvent, RlmEventData, RlmRequest, TokenUsage,
    },
    config::RlmConfig,
    RlmResult,
};
use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    collections::HashMap,
};

/// Test event sink that captures emitted events for verification.
#[derive(Debug, Clone)]
pub struct TestEventSink {
    events: Arc<Mutex<Vec<RlmEvent>>>,
}

impl TestEventSink {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_events(&self) -> Vec<RlmEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[async_trait]
impl EventSink for TestEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    async fn emit_batch(&self, events: Vec<RlmEvent>) -> RlmResult<()> {
        for event in events {
            self.emit(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        Ok(())
    }

    async fn close(&mut self) -> RlmResult<()> {
        Ok(())
    }

    async fn get_stats(&self) -> RlmResult<SinkStats> {
        use std::time::SystemTime;
        Ok(SinkStats {
            events_emitted: self.events.lock().unwrap().len() as u64,
            events_buffered: 0,
            events_failed: 0,
            avg_emit_latency_ms: None,
            created_at: SystemTime::now(),
            last_emit_at: None,
            buffer_utilization: 0.0,
        })
    }

    fn get_metadata(&self) -> SinkMetadata {
        SinkMetadata {
            sink_type: "TestEventSink".to_string(),
            version: "1.0.0".to_string(),
            max_buffer_size: None,
            supports_batch: true,
            supports_filtering: false,
            capabilities: vec!["test".to_string()],
            destination: None,
        }
    }

    async fn health_check(&self) -> RlmResult<bool> {
        Ok(true)
    }

    async fn set_filter(&mut self, _filter: EventFilter) -> RlmResult<()> {
        Ok(())
    }
}

/// Mock REPL backend for testing.
#[derive(Debug)]
pub struct MockReplBackend {
    variables: std::collections::HashMap<String, String>,
}

impl MockReplBackend {
    pub fn new() -> Self {
        Self {
            variables: std::collections::HashMap::new(),
        }
    }
}

#[async_trait]
impl ReplBackend for MockReplBackend {
    async fn reset(&mut self) -> RlmResult<()> {
        self.variables.clear();
        Ok(())
    }

    async fn execute(&mut self, _code: &str) -> RlmResult<String> {
        Ok("executed".to_string())
    }

    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()> {
        self.variables.insert(name.to_string(), value.to_string());
        Ok(())
    }

    async fn get_state(&self) -> RlmResult<HashMap<String, String>> {
        Ok(self.variables.clone())
    }

    async fn health_check(&self) -> RlmResult<bool> {
        Ok(true)
    }

    fn get_metadata(&self) -> ReplMetadata {
        ReplMetadata {
            backend_type: "MockReplBackend".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["variables".to_string()],
            max_code_length: Some(1000),
        }
    }
}

/// Mock LLM provider for testing.
#[derive(Debug)]
pub struct MockLlmProvider {
    response: String,
}

impl MockLlmProvider {
    pub fn new(response: String) -> Self {
        Self { response }
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, _request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        Ok(ChatCompletionResponse {
            id: "test-response-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567890,
            model: "gpt-4".to_string(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: ChatRole::Assistant,
                    content: self.response.clone(),
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 25,
                total_tokens: 75,
                recursive_calls: 0,
            },
        })
    }

    async fn complete_stream(&self, _request: &ChatCompletionRequest) -> RlmResult<std::pin::Pin<Box<dyn futures::stream::Stream<Item = RlmResult<ChatCompletionResponse>> + Send + 'static>>> {
        // Create a simple stream that yields the response once
        use futures::stream;
        let response = self.complete(_request).await?;
        Ok(Box::pin(stream::once(async { Ok(response) })))
    }

    fn validate_request(&self, _request: &ChatCompletionRequest) -> RlmResult<()> {
        Ok(())
    }

    async fn list_models(&self) -> RlmResult<Vec<ModelInfo>> {
        Ok(vec![])
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "mock".to_string(),
            version: "1.0.0".to_string(),
            base_url: "https://mock.example.com".to_string(),
            requires_auth: false,
            rate_limit_rpm: Some(1000),
            rate_limit_tpm: Some(60000),
            capabilities: vec!["chat".to_string(), "completions".to_string()],
        }
    }

    async fn health_check(&self) -> RlmResult<bool> {
        Ok(true)
    }
}

#[tokio::test]
async fn test_executor_streaming_events() -> Result<(), Box<dyn std::error::Error>> {
    // Create test components
    let event_sink = TestEventSink::new();
    let repl_backend = MockReplBackend::new();
    let llm_provider = MockLlmProvider::new(
        "This is a test response that will be split into multiple chunks for streaming purposes.".to_string()
    );

    // Create executor with event sink
    let config = RlmConfig::default();
    let executor = RlmExecutor::new(config, repl_backend, llm_provider)
        .with_event_sink(Arc::new(event_sink.clone()));

    // Create test request
    let request = RlmRequest {
        query: "Test query".to_string(),
        context: "Test context with some background information".to_string(),
        max_iterations: 5,
        recursion_depth: 1,
        metadata: std::collections::HashMap::new(),
    };

    // Execute the request
    let result = executor.execute(request).await;
    assert!(result.is_ok(), "Execution should succeed");

    // Verify events were emitted
    let events = event_sink.get_events();
    assert!(!events.is_empty(), "Should have emitted events");

    // Check for expected event types
    let mut found_repl_op = false;
    let mut found_recursive_call = false;
    let mut found_chunk = false;
    let mut found_context_chunk = false;
    let mut found_done = false;

    for event in &events {
        match &event.data {
            RlmEventData::ReplOp { .. } => {
                found_repl_op = true;
                println!("✓ Found ReplOp event");
            }
            RlmEventData::RecursiveCall { status, depth, .. } => {
                found_recursive_call = true;
                assert_eq!(*depth, 0, "Should be at depth 0");
                assert!(
                    matches!(status, CallStatus::InProgress | CallStatus::Completed),
                    "Status should be InProgress or Completed"
                );
                println!("✓ Found RecursiveCall event with status: {:?}", status);
            }
            RlmEventData::Chunk { content, chunk_index, is_final, .. } => {
                found_chunk = true;
                assert!(!content.is_empty(), "Chunk content should not be empty");
                println!("✓ Found Chunk event {}: '{}...' (final: {})",
                         chunk_index,
                         &content.chars().take(20).collect::<String>(),
                         is_final);
            }
            RlmEventData::ContextChunk { .. } => {
                found_context_chunk = true;
                println!("✓ Found ContextChunk event");
            }
            RlmEventData::Done { total_tokens, .. } => {
                found_done = true;
                assert!(*total_tokens > 0, "Should have consumed tokens");
                println!("✓ Found Done event with {} tokens", total_tokens);
            }
            RlmEventData::Error { .. } => {
                panic!("Should not have error events in successful execution");
            }
        }
    }

    // Verify all expected event types were found
    assert!(found_repl_op, "Should have emitted ReplOp events");
    assert!(found_recursive_call, "Should have emitted RecursiveCall events");
    assert!(found_chunk, "Should have emitted Chunk events");
    assert!(found_context_chunk, "Should have emitted ContextChunk events");
    assert!(found_done, "Should have emitted Done event");

    println!("✅ All streaming event types verified successfully!");
    println!("Total events emitted: {}", events.len());

    Ok(())
}

#[tokio::test]
async fn test_executor_streaming_chunk_order() -> Result<(), Box<dyn std::error::Error>> {
    // Test that chunks are emitted in the correct order
    let event_sink = TestEventSink::new();
    let repl_backend = MockReplBackend::new();
    let llm_provider = MockLlmProvider::new(
        "First chunk content. Second chunk content. Third chunk content. Fourth chunk content.".to_string()
    );

    let config = RlmConfig::default();
    let executor = RlmExecutor::new(config, repl_backend, llm_provider)
        .with_event_sink(Arc::new(event_sink.clone()));

    let request = RlmRequest {
        query: "Test chunk ordering".to_string(),
        context: "Test context".to_string(),
        max_iterations: 5,
        recursion_depth: 1,
        metadata: std::collections::HashMap::new(),
    };

    let _result = executor.execute(request).await?;

    // Find all chunk events and verify ordering
    let events = event_sink.get_events();
    let chunk_events: Vec<&RlmEvent> = events.iter()
        .filter(|e| matches!(e.data, RlmEventData::Chunk { .. }))
        .collect();

    assert!(!chunk_events.is_empty(), "Should have chunk events");

    // Verify chunk indices are sequential
    for (i, event) in chunk_events.iter().enumerate() {
        if let RlmEventData::Chunk { chunk_index, is_final, .. } = &event.data {
            assert_eq!(*chunk_index, i as u32, "Chunk indices should be sequential");

            // Only the last chunk should be marked as final
            if i == chunk_events.len() - 1 {
                assert!(*is_final, "Last chunk should be marked as final");
            } else {
                assert!(!*is_final, "Non-final chunks should not be marked as final");
            }
        }
    }

    println!("✅ Chunk ordering verified successfully!");
    println!("Total chunks: {}", chunk_events.len());

    Ok(())
}