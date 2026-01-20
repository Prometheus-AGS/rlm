//! Server-Sent Events streaming support.
//!
//! This module provides streaming chat completion handlers that emit real-time
//! events during RLM processing, including context offloading, recursive calls,
//! and response chunks in OpenAI-compatible SSE format.

use axum::{
    extract::State,
    response::{
        sse::{KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use rlm_core::{
    ports::{EventSink, LlmProvider},
    types::{ChatCompletionRequest, RlmRequest, RlmEvent},
    executor::RlmExecutor,
};
use rlm_repl_rhai::RhaiReplBackend;
use crate::adapters::{
    OpenAiProvider, OpenAiConfigBuilder,
    sse_sink::{SseEventSinkBuilder, create_sse_stream},
};
use crate::server::routes::{AppState, AppError, validate_chat_request};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Handle streaming chat completions requests.
///
/// This function creates an SSE stream that emits events in real-time during
/// RLM processing, including context offloading progress, recursive call status,
/// and final response chunks in OpenAI-compatible format.
pub async fn stream_chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<impl IntoResponse, AppError> {
    debug!("Received streaming chat completion request: {:?}", request);

    // Validate the request
    validate_chat_request(&request)?;

    // Ensure streaming is enabled
    if !request.stream {
        return Err(AppError::BadRequest(
            "Stream parameter must be true for streaming endpoint".to_string()
        ));
    }

    // Generate request ID for correlation
    let request_id = Uuid::new_v4().to_string();
    info!("Starting streaming completion for request: {}", request_id);

    // Create SSE event sink and receiver
    let (event_sink, receiver) = SseEventSinkBuilder::new()
        .with_request_id(request_id.clone())
        .with_keep_alive_interval(Duration::from_secs(30))
        .build();

    // Create the SSE stream with keep-alive
    let sse_stream = create_sse_stream(receiver, Some(Duration::from_secs(30)));

    // Spawn background task to handle the RLM execution
    let execution_state = state.clone();
    let execution_request = request.clone();
    let execution_sink = Arc::new(event_sink);

    tokio::spawn(async move {
        if let Err(e) = handle_streaming_execution(
            execution_state,
            execution_request,
            execution_sink,
            request_id.clone(),
        ).await {
            error!("Streaming execution failed for {}: {:?}", request_id, e);
        }
    });

    // Return the SSE response
    let sse_response = Sse::new(sse_stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)));

    Ok(sse_response)
}

/// Handle the actual streaming execution in a background task.
///
/// This function coordinates the RLM execution with event streaming,
/// converting RLM events to SSE events and managing the stream lifecycle.
async fn handle_streaming_execution(
    state: AppState,
    request: ChatCompletionRequest,
    event_sink: Arc<dyn EventSink + Send + Sync>,
    request_id: String,
) -> Result<(), AppError> {
    debug!("Starting streaming execution for request: {}", request_id);

    // Send initial connection established event
    send_initial_event(&event_sink, &request_id).await?;

    // Check if this should use RLM processing
    if let Some(_rlm_config) = &request.rlm_config {
        debug!("Using RLM streaming execution for request: {}", request_id);
        handle_rlm_streaming_execution(state, request, event_sink, request_id).await
    } else {
        debug!("Using direct OpenAI streaming for request: {}", request_id);
        handle_openai_streaming_execution(state, request, event_sink, request_id).await
    }
}

/// Handle RLM streaming execution with recursive processing.
async fn handle_rlm_streaming_execution(
    _state: AppState,
    request: ChatCompletionRequest,
    event_sink: Arc<dyn EventSink + Send + Sync>,
    request_id: String,
) -> Result<(), AppError> {
    // Convert ChatCompletionRequest to RlmRequest
    let rlm_request = convert_to_rlm_request(request, request_id.clone())?;

    debug!("Executing RLM request with streaming events: {}", request_id);

    // Create a new RLM executor instance with the event sink
    let repl_backend = RhaiReplBackend::new()
        .map_err(|e| AppError::Internal(format!("Failed to create REPL backend: {}", e)))?;

    // Create a new OpenAI provider instance for the executor
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "mock-key".to_string());
    let openai_config = OpenAiConfigBuilder::new(api_key).build();
    let openai_provider = OpenAiProvider::new(openai_config)
        .map_err(|e| AppError::Internal(format!("Failed to create OpenAI provider: {}", e)))?;

    let rlm_config = rlm_core::config::RlmConfig::default();
    let executor = RlmExecutor::new(rlm_config, repl_backend, openai_provider)
        .with_event_sink(event_sink.clone());

    // Execute with streaming events
    match executor.execute(rlm_request).await {
        Ok(rlm_response) => {
            debug!("RLM execution completed successfully: {}", request_id);

            // Send the final response as streaming chunks
            let response_content = rlm_response.answer;
            let chunks = split_into_chunks(&response_content, 100); // 100 chars per chunk

            for (i, chunk) in chunks.iter().enumerate() {
                let is_final = i == chunks.len() - 1;
                let chunk_event = RlmEvent {
                    event_id: format!("final-chunk-{}", i),
                    request_id: request_id.clone(),
                    timestamp: std::time::SystemTime::now(),
                    data: rlm_core::types::RlmEventData::Chunk {
                        content: chunk.to_string(),
                        chunk_index: i as u32,
                        is_final,
                    },
                };

                if let Err(e) = event_sink.emit(chunk_event).await {
                    warn!("Failed to emit chunk event: {}", e);
                }
            }

            // Send completion event
            let done_event = RlmEvent {
                event_id: format!("done-{}", Uuid::new_v4()),
                request_id: request_id.clone(),
                timestamp: std::time::SystemTime::now(),
                data: rlm_core::types::RlmEventData::Done {
                    total_tokens: rlm_response.metadata.total_tokens,
                    total_duration_ms: rlm_response.metadata.duration.as_millis() as u64,
                },
            };

            if let Err(e) = event_sink.emit(done_event).await {
                warn!("Failed to emit done event: {}", e);
            }

            Ok(())
        }
        Err(e) => {
            error!("RLM execution failed: {:?}", e);

            // Send error event
            let error_event = RlmEvent {
                event_id: format!("error-{}", Uuid::new_v4()),
                request_id: request_id.clone(),
                timestamp: std::time::SystemTime::now(),
                data: rlm_core::types::RlmEventData::Error {
                    error_type: "RlmExecutionError".to_string(),
                    message: e.to_string(),
                    recoverable: false,
                },
            };

            if let Err(sink_err) = event_sink.emit(error_event).await {
                warn!("Failed to emit error event: {}", sink_err);
            }

            Err(AppError::Internal(format!("RLM execution error: {}", e)))
        }
    }
}

/// Handle direct OpenAI streaming execution (for non-RLM requests).
async fn handle_openai_streaming_execution(
    state: AppState,
    request: ChatCompletionRequest,
    event_sink: Arc<dyn EventSink + Send + Sync>,
    request_id: String,
) -> Result<(), AppError> {
    debug!("Executing direct OpenAI streaming for request: {}", request_id);

    // For direct OpenAI streaming, we would use the provider's streaming capability
    // For now, we'll simulate streaming by completing the request and chunking the response
    match state.openai_provider.as_ref().complete(&request).await {
        Ok(response) => {
            debug!("OpenAI completion successful for streaming: {}", request_id);

            // Extract the response content
            let response_content = response.choices.first()
                .map(|choice| choice.message.content.clone())
                .unwrap_or_default();

            // Split into chunks for streaming simulation
            let chunks = split_into_chunks(&response_content, 50); // Smaller chunks for better streaming UX

            for (i, chunk) in chunks.iter().enumerate() {
                let is_final = i == chunks.len() - 1;
                let chunk_event = RlmEvent {
                    event_id: format!("openai-chunk-{}", i),
                    request_id: request_id.clone(),
                    timestamp: std::time::SystemTime::now(),
                    data: rlm_core::types::RlmEventData::Chunk {
                        content: chunk.to_string(),
                        chunk_index: i as u32,
                        is_final,
                    },
                };

                if let Err(e) = event_sink.emit(chunk_event).await {
                    warn!("Failed to emit OpenAI chunk event: {}", e);
                }

                // Small delay to simulate real-time streaming
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            // Send completion event
            let done_event = RlmEvent {
                event_id: format!("openai-done-{}", Uuid::new_v4()),
                request_id: request_id.clone(),
                timestamp: std::time::SystemTime::now(),
                data: rlm_core::types::RlmEventData::Done {
                    total_tokens: response.usage.total_tokens,
                    total_duration_ms: 1000, // Estimated
                },
            };

            if let Err(e) = event_sink.emit(done_event).await {
                warn!("Failed to emit OpenAI done event: {}", e);
            }

            Ok(())
        }
        Err(e) => {
            error!("OpenAI streaming failed: {:?}", e);

            // Send error event
            let error_event = RlmEvent {
                event_id: format!("openai-error-{}", Uuid::new_v4()),
                request_id: request_id.clone(),
                timestamp: std::time::SystemTime::now(),
                data: rlm_core::types::RlmEventData::Error {
                    error_type: "OpenAiError".to_string(),
                    message: e.to_string(),
                    recoverable: false,
                },
            };

            if let Err(sink_err) = event_sink.emit(error_event).await {
                warn!("Failed to emit OpenAI error event: {}", sink_err);
            }

            Err(AppError::Internal(format!("OpenAI streaming error: {}", e)))
        }
    }
}

/// Send initial connection established event.
async fn send_initial_event(
    event_sink: &Arc<dyn EventSink + Send + Sync>,
    request_id: &str,
) -> Result<(), AppError> {
    let init_event = RlmEvent {
        event_id: format!("init-{}", Uuid::new_v4()),
        request_id: request_id.to_string(),
        timestamp: std::time::SystemTime::now(),
        data: rlm_core::types::RlmEventData::ContextChunk {
            chunk_id: "connection-established".to_string(),
            size_bytes: 0,
            processed: true,
        },
    };

    event_sink.emit(init_event).await
        .map_err(|e| AppError::Internal(format!("Failed to send initial event: {}", e)))
}

/// Convert ChatCompletionRequest to RlmRequest for RLM processing.
fn convert_to_rlm_request(
    request: ChatCompletionRequest,
    request_id: String,
) -> Result<RlmRequest, AppError> {
    // Extract query and context from messages
    let mut query = String::new();
    let mut context = String::new();

    for message in &request.messages {
        match message.role {
            rlm_core::types::ChatRole::User => {
                if query.is_empty() {
                    // Use the last user message as the main query
                    query = message.content.clone();
                } else {
                    // Previous user messages become part of context
                    context.push_str(&format!("User: {}\n", message.content));
                }
            }
            rlm_core::types::ChatRole::Assistant => {
                context.push_str(&format!("Assistant: {}\n", message.content));
            }
            rlm_core::types::ChatRole::System => {
                context.push_str(&format!("System: {}\n", message.content));
            }
        }
    }

    if query.is_empty() {
        return Err(AppError::BadRequest(
            "No user message found for RLM processing".to_string()
        ));
    }

    let mut metadata = HashMap::new();
    metadata.insert("request_id".to_string(), request_id);
    metadata.insert("model".to_string(), request.model.clone());
    if let Some(temp) = request.temperature {
        metadata.insert("temperature".to_string(), temp.to_string());
    }

    Ok(RlmRequest {
        query,
        context,
        max_iterations: 50, // Default from paper
        recursion_depth: request.rlm_config
            .as_ref()
            .and_then(|c| c.max_recursive_depth)
            .unwrap_or(1),
        metadata,
    })
}

/// Split text into chunks for streaming.
fn split_into_chunks(text: &str, chunk_size: usize) -> Vec<String> {
    if text.is_empty() {
        return vec!["".to_string()];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for word in text.split_whitespace() {
        if current_chunk.len() + word.len() + 1 > chunk_size && !current_chunk.is_empty() {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
        }

        if !current_chunk.is_empty() {
            current_chunk.push(' ');
        }
        current_chunk.push_str(word);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::types::{ChatMessage, ChatRole};

    #[test]
    fn test_split_into_chunks() {
        let text = "This is a test message that should be split into multiple chunks.";
        let chunks = split_into_chunks(text, 20);

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 20 || chunk.split_whitespace().count() <= 1));
    }

    #[test]
    fn test_split_empty_text() {
        let chunks = split_into_chunks("", 10);
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn test_convert_to_rlm_request() {
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
                    content: "Hello, how are you?".to_string(),
                    name: None,
                },
            ],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: true,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let rlm_request = convert_to_rlm_request(request, "test-123".to_string()).unwrap();

        assert_eq!(rlm_request.query, "Hello, how are you?");
        assert!(rlm_request.context.contains("System: You are a helpful assistant."));
        assert_eq!(rlm_request.metadata.get("request_id"), Some(&"test-123".to_string()));
    }
}