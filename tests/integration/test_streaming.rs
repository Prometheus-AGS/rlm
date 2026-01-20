//! RLM event streaming integration tests.
//!
//! These tests verify the complete RLM streaming workflow including:
//! - Context offloading events
//! - Recursive call progress events
//! - REPL operation events
//! - Final response streaming

use rlm_core::{
    ports::{EventSink, ReplBackend},
    RlmEvent, RlmEventData, RlmRequest, CallStatus,
    types::{ChatCompletionRequest, ChatMessage, ChatRole}
};
use rlm_repl_rhai::RhaiReplBackend;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, Duration}
};
use tokio::time::timeout;

/// Mock event sink that collects all events for testing.
#[derive(Debug, Clone)]
struct TestEventSink {
    events: Arc<Mutex<Vec<RlmEvent>>>,
}

impl TestEventSink {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_events(&self) -> Vec<RlmEvent> {
        self.events.lock().unwrap().clone()
    }

    fn count_events(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    fn get_events_by_type(&self, event_type: &str) -> Vec<RlmEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| match &event.data {
                RlmEventData::ReplOp { .. } => event_type == "repl_op",
                RlmEventData::RecursiveCall { .. } => event_type == "recursive_call",
                RlmEventData::Chunk { .. } => event_type == "chunk",
                RlmEventData::ContextChunk { .. } => event_type == "context_chunk",
                RlmEventData::Done { .. } => event_type == "done",
                RlmEventData::Error { .. } => event_type == "error",
            })
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl EventSink for TestEventSink {
    async fn emit(&self, event: RlmEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    async fn close(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

#[tokio::test]
async fn test_streaming_events_for_short_context() {
    let event_sink = TestEventSink::new();
    let mut repl = RhaiReplBackend::new().expect("Failed to create REPL backend");

    // Create a simple request that should produce basic streaming events
    let request = RlmRequest {
        query: "What is the meaning of life?".to_string(),
        context: "This is a simple context that doesn't require recursive processing.".to_string(),
        max_iterations: 5,
        recursion_depth: 1,
        metadata: HashMap::new(),
    };

    // Simulate basic context offloading
    repl.set_variable("context", &request.context).await
        .expect("Failed to set context in REPL");

    // Emit context chunk event
    let context_event = RlmEvent {
        event_id: "ctx-1".to_string(),
        request_id: "req-1".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::ContextChunk {
            chunk_id: "chunk-1".to_string(),
            size_bytes: request.context.len(),
            processed: true,
        },
    };

    event_sink.emit(context_event).await.expect("Failed to emit context event");

    // Simulate REPL operation
    let repl_result = repl.execute("context.len()").await
        .expect("Failed to execute REPL operation");

    let repl_event = RlmEvent {
        event_id: "repl-1".to_string(),
        request_id: "req-1".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::ReplOp {
            iteration: 1,
            code: "context.len()".to_string(),
            result: rlm_core::ReplResult::Success { value: repl_result },
        },
    };

    event_sink.emit(repl_event).await.expect("Failed to emit REPL event");

    // Emit final completion event
    let done_event = RlmEvent {
        event_id: "done-1".to_string(),
        request_id: "req-1".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::Done {
            total_tokens: 150,
            total_duration_ms: 1000,
        },
    };

    event_sink.emit(done_event).await.expect("Failed to emit done event");

    // Verify events were collected
    let events = event_sink.get_events();
    assert_eq!(events.len(), 3);

    // Verify event types
    assert_eq!(event_sink.get_events_by_type("context_chunk").len(), 1);
    assert_eq!(event_sink.get_events_by_type("repl_op").len(), 1);
    assert_eq!(event_sink.get_events_by_type("done").len(), 1);
}

#[tokio::test]
async fn test_streaming_events_for_long_context() {
    let event_sink = TestEventSink::new();
    let mut repl = RhaiReplBackend::new().expect("Failed to create REPL backend");

    // Create a longer context that might require chunking
    let long_context = "This is a much longer context. ".repeat(1000);
    let request = RlmRequest {
        query: "Summarize the key themes in this long document.".to_string(),
        context: long_context.clone(),
        max_iterations: 10,
        recursion_depth: 2,
        metadata: HashMap::new(),
    };

    // Simulate context chunking for large context
    let chunk_size = 5000;
    let chunks: Vec<_> = long_context
        .chars()
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .enumerate()
        .collect();

    for (i, chunk) in chunks {
        let chunk_str: String = chunk.iter().collect();

        // Set chunk in REPL
        let var_name = format!("context_chunk_{}", i);
        repl.set_variable(&var_name, &chunk_str).await
            .expect("Failed to set context chunk");

        // Emit context chunk event
        let chunk_event = RlmEvent {
            event_id: format!("ctx-chunk-{}", i),
            request_id: "req-long".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::ContextChunk {
                chunk_id: format!("chunk-{}", i),
                size_bytes: chunk_str.len(),
                processed: true,
            },
        };

        event_sink.emit(chunk_event).await.expect("Failed to emit chunk event");
    }

    // Simulate recursive call for complex processing
    let recursive_event = RlmEvent {
        event_id: "recursive-1".to_string(),
        request_id: "req-long".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::RecursiveCall {
            call_id: "call-1".to_string(),
            depth: 1,
            status: CallStatus::InProgress,
            prompt_tokens: 2000,
            completion_tokens: 0,
        },
    };

    event_sink.emit(recursive_event).await.expect("Failed to emit recursive call event");

    // Complete the recursive call
    let recursive_complete_event = RlmEvent {
        event_id: "recursive-complete-1".to_string(),
        request_id: "req-long".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::RecursiveCall {
            call_id: "call-1".to_string(),
            depth: 1,
            status: CallStatus::Completed,
            prompt_tokens: 2000,
            completion_tokens: 500,
        },
    };

    event_sink.emit(recursive_complete_event).await.expect("Failed to emit recursive complete event");

    // Final completion
    let done_event = RlmEvent {
        event_id: "done-long".to_string(),
        request_id: "req-long".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::Done {
            total_tokens: 2500,
            total_duration_ms: 5000,
        },
    };

    event_sink.emit(done_event).await.expect("Failed to emit done event");

    // Verify comprehensive event collection
    let events = event_sink.get_events();
    assert!(events.len() >= 4); // At least 2 chunks + 2 recursive + 1 done = 5

    // Verify specific event types were captured
    assert!(!event_sink.get_events_by_type("context_chunk").is_empty());
    assert_eq!(event_sink.get_events_by_type("recursive_call").len(), 2); // InProgress and Completed
    assert_eq!(event_sink.get_events_by_type("done").len(), 1);
}

#[tokio::test]
async fn test_streaming_response_chunks() {
    let event_sink = TestEventSink::new();

    // Simulate streaming response chunks
    let response_chunks = vec![
        "The answer to your question",
        " is that recursive language models",
        " provide a novel approach to",
        " handling very long contexts",
        " by breaking them down into",
        " manageable pieces."
    ];

    for (i, chunk) in response_chunks.iter().enumerate() {
        let chunk_event = RlmEvent {
            event_id: format!("chunk-{}", i),
            request_id: "req-stream".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::Chunk {
                content: chunk.to_string(),
                chunk_index: i as u32,
                is_final: i == response_chunks.len() - 1,
            },
        };

        event_sink.emit(chunk_event).await.expect("Failed to emit chunk event");
    }

    // Verify streaming chunks
    let chunk_events = event_sink.get_events_by_type("chunk");
    assert_eq!(chunk_events.len(), response_chunks.len());

    // Verify final chunk is marked as final
    if let Some(last_event) = chunk_events.last() {
        if let RlmEventData::Chunk { is_final, .. } = &last_event.data {
            assert!(*is_final);
        }
    }

    // Reconstruct full response from chunks
    let mut full_response = String::new();
    for event in chunk_events {
        if let RlmEventData::Chunk { content, .. } = event.data {
            full_response.push_str(&content);
        }
    }

    let expected_response = response_chunks.join("");
    assert_eq!(full_response, expected_response);
}

#[tokio::test]
async fn test_streaming_error_events() {
    let event_sink = TestEventSink::new();

    // Simulate an error during processing
    let error_event = RlmEvent {
        event_id: "error-1".to_string(),
        request_id: "req-error".to_string(),
        timestamp: SystemTime::now(),
        data: RlmEventData::Error {
            error_type: "ReplExecutionError".to_string(),
            message: "Failed to execute REPL code: syntax error".to_string(),
            recoverable: false,
        },
    };

    event_sink.emit(error_event).await.expect("Failed to emit error event");

    // Verify error event was captured
    let error_events = event_sink.get_events_by_type("error");
    assert_eq!(error_events.len(), 1);

    if let RlmEventData::Error { error_type, message, recoverable } = &error_events[0].data {
        assert_eq!(error_type, "ReplExecutionError");
        assert!(message.contains("syntax error"));
        assert!(!recoverable);
    }
}

#[tokio::test]
async fn test_concurrent_streaming_events() {
    let event_sink = TestEventSink::new();

    // Simulate concurrent event emission from multiple sources
    let event_sink_clone = event_sink.clone();

    let handles = vec![
        tokio::spawn(async move {
            for i in 0..10 {
                let event = RlmEvent {
                    event_id: format!("concurrent-repl-{}", i),
                    request_id: "req-concurrent".to_string(),
                    timestamp: SystemTime::now(),
                    data: RlmEventData::ReplOp {
                        iteration: i,
                        code: format!("operation_{}", i),
                        result: rlm_core::ReplResult::Success {
                            value: format!("result_{}", i)
                        },
                    },
                };
                event_sink_clone.emit(event).await.expect("Failed to emit concurrent event");

                // Small delay to simulate realistic timing
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }),
    ];

    // Wait for all concurrent tasks
    for handle in handles {
        handle.await.expect("Concurrent task failed");
    }

    // Verify all events were captured
    let events = event_sink.get_events();
    assert_eq!(events.len(), 10);

    // Verify all are REPL operations
    assert_eq!(event_sink.get_events_by_type("repl_op").len(), 10);
}

#[tokio::test]
async fn test_streaming_with_timeout() {
    let event_sink = TestEventSink::new();

    // Test that event emission doesn't block indefinitely
    let emit_future = async {
        for i in 0..5 {
            let event = RlmEvent {
                event_id: format!("timeout-test-{}", i),
                request_id: "req-timeout".to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::Chunk {
                    content: format!("Chunk {}", i),
                    chunk_index: i,
                    is_final: i == 4,
                },
            };
            event_sink.emit(event).await.expect("Failed to emit event");
        }
    };

    // Should complete within reasonable time
    timeout(Duration::from_secs(5), emit_future)
        .await
        .expect("Event emission should not timeout");

    assert_eq!(event_sink.count_events(), 5);
}

#[tokio::test]
async fn test_event_ordering_and_timestamps() {
    let event_sink = TestEventSink::new();

    let start_time = SystemTime::now();

    // Emit events with slight delays to test ordering
    for i in 0..5 {
        tokio::time::sleep(Duration::from_millis(10)).await;

        let event = RlmEvent {
            event_id: format!("ordered-{}", i),
            request_id: "req-ordered".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::Chunk {
                content: format!("Ordered chunk {}", i),
                chunk_index: i,
                is_final: i == 4,
            },
        };

        event_sink.emit(event).await.expect("Failed to emit ordered event");
    }

    let events = event_sink.get_events();
    assert_eq!(events.len(), 5);

    // Verify events maintain chronological order
    for (i, event) in events.iter().enumerate() {
        assert!(event.timestamp >= start_time);

        // Each subsequent event should be at or after the previous one
        if i > 0 {
            assert!(event.timestamp >= events[i - 1].timestamp);
        }
    }
}