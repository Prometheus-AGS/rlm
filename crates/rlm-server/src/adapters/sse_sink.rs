//! Server-Sent Events (SSE) event sink adapter.
//!
//! This module implements the EventSink trait for streaming RLM events
//! as Server-Sent Events over HTTP connections.

use async_trait::async_trait;
use axum::response::sse::Event;
use rlm_core::{
    ports::{EventSink, EventFilter, SinkMetadata, SinkStats},
    error::{RlmError, RlmResult},
    RlmEvent, RlmEventData,
};
use serde_json::json;
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// SSE event sink that converts RLM events to Server-Sent Events.
#[derive(Debug)]
pub struct SseEventSink {
    sender: mpsc::UnboundedSender<Result<Event, Infallible>>,
    request_id: String,
    is_closed: Arc<std::sync::atomic::AtomicBool>,
    stats: Arc<Mutex<SinkStats>>,
    filter: Arc<Mutex<EventFilter>>,
}

impl SseEventSink {
    /// Create a new SSE event sink with the given sender channel.
    pub fn new(
        sender: mpsc::UnboundedSender<Result<Event, Infallible>>,
        request_id: String,
    ) -> Self {
        Self {
            sender,
            request_id,
            is_closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stats: Arc::new(Mutex::new(SinkStats::new())),
            filter: Arc::new(Mutex::new(EventFilter::allow_all())),
        }
    }

    /// Convert an RLM event to an SSE event.
    fn convert_to_sse_event(&self, event: RlmEvent) -> Result<Event, RlmError> {
        debug!("Converting RLM event to SSE: {:?}", event.event_id);

        let timestamp = event.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        match event.data {
            RlmEventData::ReplOp { iteration, code, result } => {
                let data = json!({
                    "type": "repl_op",
                    "event_id": event.event_id,
                    "request_id": event.request_id,
                    "timestamp": timestamp,
                    "data": {
                        "iteration": iteration,
                        "code": code,
                        "result": result,
                    }
                });

                Event::default()
                    .event("repl_op")
                    .json_data(data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }

            RlmEventData::RecursiveCall {
                call_id,
                depth,
                status,
                prompt_tokens,
                completion_tokens,
            } => {
                let data = json!({
                    "type": "recursive_call",
                    "event_id": event.event_id,
                    "request_id": event.request_id,
                    "timestamp": timestamp,
                    "data": {
                        "call_id": call_id,
                        "depth": depth,
                        "status": status,
                        "prompt_tokens": prompt_tokens,
                        "completion_tokens": completion_tokens,
                    }
                });

                Event::default()
                    .event("recursive_call")
                    .json_data(data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }

            RlmEventData::Chunk {
                content,
                chunk_index: _,
                is_final,
            } => {
                // For chunk events, send in OpenAI streaming format
                let finish_reason: Option<String> = if is_final { Some("stop".to_string()) } else { None };
                let choice_data = json!({
                    "index": 0,
                    "delta": {
                        "content": content,
                    },
                    "finish_reason": finish_reason
                });

                let data = json!({
                    "id": event.event_id,
                    "object": "chat.completion.chunk",
                    "created": timestamp / 1000, // Convert to seconds
                    "model": "rlm-enabled", // This would be set from the actual model
                    "choices": [choice_data],
                });

                Event::default()
                    .event("chunk")
                    .json_data(data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }

            RlmEventData::ContextChunk {
                chunk_id,
                size_bytes,
                processed,
            } => {
                let data = json!({
                    "type": "context_chunk",
                    "event_id": event.event_id,
                    "request_id": event.request_id,
                    "timestamp": timestamp,
                    "data": {
                        "chunk_id": chunk_id,
                        "size_bytes": size_bytes,
                        "processed": processed,
                    }
                });

                Event::default()
                    .event("context_chunk")
                    .json_data(data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }

            RlmEventData::Done {
                total_tokens,
                total_duration_ms,
            } => {
                // Send RLM completion event
                let rlm_data = json!({
                    "type": "done",
                    "event_id": event.event_id,
                    "request_id": event.request_id,
                    "timestamp": timestamp,
                    "data": {
                        "total_tokens": total_tokens,
                        "total_duration_ms": total_duration_ms,
                    }
                });

                Event::default()
                    .event("done")
                    .json_data(rlm_data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }

            RlmEventData::Error {
                error_type,
                message,
                recoverable,
            } => {
                let data = json!({
                    "type": "error",
                    "event_id": event.event_id,
                    "request_id": event.request_id,
                    "timestamp": timestamp,
                    "data": {
                        "error_type": error_type,
                        "message": message,
                        "recoverable": recoverable,
                    }
                });

                Event::default()
                    .event("error")
                    .json_data(data)
                    .map_err(|e| RlmError::Other(format!("Failed to create SSE event: {}", e)))
            }
        }
    }

    /// Send the OpenAI [DONE] marker to terminate the stream.
    async fn send_done_marker(&self) -> RlmResult<()> {
        debug!("Sending [DONE] marker for request: {}", self.request_id);

        let done_event = Event::default().data("[DONE]");

        self.sender
            .send(Ok(done_event))
            .map_err(|_| RlmError::Other("Failed to send [DONE] marker".to_string()))?;

        Ok(())
    }

    /// Update statistics after emitting an event.
    fn update_stats(&self, success: bool) {
        if let Ok(mut stats) = self.stats.lock() {
            if success {
                stats.events_emitted += 1;
                stats.last_emit_at = Some(SystemTime::now());
            } else {
                stats.events_failed += 1;
            }
        }
    }
}

#[async_trait]
impl EventSink for SseEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        // Check if the sink has been closed
        if self.is_closed.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("Attempted to emit event to closed SSE sink: {}", event.event_id);
            return Err(RlmError::Other("SSE sink is closed".to_string()));
        }

        // Check filter
        if let Ok(filter) = self.filter.lock() {
            if !filter.should_emit(&event) {
                debug!("Event filtered out: {}", event.event_id);
                return Ok(());
            }
        }

        // Convert RLM event to SSE event
        let sse_event = self.convert_to_sse_event(event.clone())?;

        // Send the event
        let send_result = self.sender.send(Ok(sse_event));
        let success = send_result.is_ok();

        if !success {
            error!("Failed to send SSE event: {}", event.event_id);
            self.update_stats(false);
            return Err(RlmError::Other("Failed to send SSE event".to_string()));
        }

        self.update_stats(true);

        // For Done events, also send the [DONE] marker
        if matches!(event.data, RlmEventData::Done { .. }) {
            self.send_done_marker().await?;
        }

        debug!("Successfully emitted SSE event: {}", event.event_id);
        Ok(())
    }

    async fn emit_batch(&self, events: Vec<RlmEvent>) -> RlmResult<()> {
        for event in events {
            self.emit(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        // For SSE, events are immediately sent, so no buffering to flush
        Ok(())
    }

    async fn close(&mut self) -> RlmResult<()> {
        debug!("Closing SSE event sink for request: {}", self.request_id);

        // Mark as closed
        self.is_closed.store(true, std::sync::atomic::Ordering::Relaxed);

        // Send final [DONE] marker if not already sent
        if let Err(e) = self.send_done_marker().await {
            warn!("Failed to send final [DONE] marker: {}", e);
        }

        Ok(())
    }

    async fn get_stats(&self) -> RlmResult<SinkStats> {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .map_err(|_| RlmError::Other("Failed to get stats".to_string()))
    }

    fn get_metadata(&self) -> SinkMetadata {
        SinkMetadata::new("sse", "1.0", true, true)
            .with_destination(format!("sse://request/{}", self.request_id))
            .with_capabilities(vec![
                "streaming".to_string(),
                "real_time".to_string(),
                "openai_compatible".to_string(),
            ])
    }

    async fn health_check(&self) -> RlmResult<bool> {
        Ok(!self.is_closed.load(std::sync::atomic::Ordering::Relaxed))
    }

    async fn set_filter(&mut self, filter: EventFilter) -> RlmResult<()> {
        self.filter
            .lock()
            .map(|mut f| *f = filter)
            .map_err(|_| RlmError::Other("Failed to set filter".to_string()))?;
        Ok(())
    }
}

/// Builder for creating SSE event sinks with configuration options.
#[derive(Debug)]
pub struct SseEventSinkBuilder {
    request_id: Option<String>,
    keep_alive_interval: Option<Duration>,
    buffer_size: Option<usize>,
}

impl SseEventSinkBuilder {
    /// Create a new SSE event sink builder.
    pub fn new() -> Self {
        Self {
            request_id: None,
            keep_alive_interval: Some(Duration::from_secs(30)),
            buffer_size: Some(100),
        }
    }

    /// Set the request ID for event correlation.
    pub fn with_request_id<S: Into<String>>(mut self, request_id: S) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set the keep-alive interval for the SSE connection.
    pub fn with_keep_alive_interval(mut self, interval: Duration) -> Self {
        self.keep_alive_interval = Some(interval);
        self
    }

    /// Disable keep-alive messages.
    pub fn without_keep_alive(mut self) -> Self {
        self.keep_alive_interval = None;
        self
    }

    /// Set the buffer size for the event channel.
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    /// Build the SSE event sink and return both the sink and the receiver.
    pub fn build(self) -> (SseEventSink, mpsc::UnboundedReceiver<Result<Event, Infallible>>) {
        let request_id = self.request_id.unwrap_or_else(|| {
            uuid::Uuid::new_v4().to_string()
        });

        let (sender, receiver) = mpsc::unbounded_channel();

        let sink = SseEventSink::new(sender, request_id);

        (sink, receiver)
    }
}

impl Default for SseEventSinkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create an SSE stream from an event receiver.
pub fn create_sse_stream(
    receiver: mpsc::UnboundedReceiver<Result<Event, Infallible>>,
    keep_alive: Option<Duration>,
) -> impl futures::Stream<Item = Result<Event, Infallible>> + Send {
    use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

    let stream = UnboundedReceiverStream::new(receiver);

    // Add keep-alive if configured
    if let Some(interval) = keep_alive {
        let keep_alive_stream = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(interval)
        ).map(|_| Ok(Event::default().comment("keep-alive")));

        Box::pin(futures::stream::select(stream, keep_alive_stream))
            as std::pin::Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>
    } else {
        Box::pin(stream)
            as std::pin::Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::ReplResult;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_sse_event_sink_creation() {
        let builder = SseEventSinkBuilder::new()
            .with_request_id("test-request")
            .with_keep_alive_interval(Duration::from_secs(15));

        let (sink, _receiver) = builder.build();

        assert_eq!(sink.request_id, "test-request");
        assert!(!sink.is_closed.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_repl_op_event_conversion() {
        let (sink, mut receiver) = SseEventSinkBuilder::new()
            .with_request_id("test")
            .build();

        let event = RlmEvent {
            event_id: "repl-1".to_string(),
            request_id: "test".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::ReplOp {
                iteration: 1,
                code: "test_code".to_string(),
                result: ReplResult::Success {
                    value: "42".to_string(),
                },
            },
        };

        sink.emit(event).await.expect("Failed to emit event");

        // Check that event was received
        let sse_event = receiver.recv().await.expect("Should receive SSE event");
        assert!(sse_event.is_ok());
    }

    #[tokio::test]
    async fn test_chunk_event_openai_format() {
        let (sink, mut receiver) = SseEventSinkBuilder::new()
            .with_request_id("test")
            .build();

        let event = RlmEvent {
            event_id: "chunk-1".to_string(),
            request_id: "test".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::Chunk {
                content: "Hello world".to_string(),
                chunk_index: 0,
                is_final: false,
            },
        };

        sink.emit(event).await.expect("Failed to emit chunk event");

        let sse_event = receiver.recv().await.expect("Should receive SSE event");
        assert!(sse_event.is_ok());
    }

    #[tokio::test]
    async fn test_done_event_with_marker() {
        let (sink, mut receiver) = SseEventSinkBuilder::new()
            .with_request_id("test")
            .build();

        let event = RlmEvent {
            event_id: "done-1".to_string(),
            request_id: "test".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::Done {
                total_tokens: 100,
                total_duration_ms: 5000,
            },
        };

        sink.emit(event).await.expect("Failed to emit done event");

        // Should receive the done event
        let done_event = receiver.recv().await.expect("Should receive done event");
        assert!(done_event.is_ok());

        // Should also receive the [DONE] marker
        let marker_event = receiver.recv().await.expect("Should receive [DONE] marker");
        assert!(marker_event.is_ok());
    }

    #[tokio::test]
    async fn test_sink_close() {
        let (mut sink, mut receiver) = SseEventSinkBuilder::new()
            .with_request_id("test")
            .build();

        sink.close().await.expect("Failed to close sink");

        assert!(sink.is_closed.load(std::sync::atomic::Ordering::Relaxed));

        // Should receive [DONE] marker on close
        let marker_event = receiver.recv().await.expect("Should receive [DONE] marker on close");
        assert!(marker_event.is_ok());
    }

    #[tokio::test]
    async fn test_emit_after_close() {
        let (mut sink, _receiver) = SseEventSinkBuilder::new()
            .with_request_id("test")
            .build();

        sink.close().await.expect("Failed to close sink");

        let event = RlmEvent {
            event_id: "after-close".to_string(),
            request_id: "test".to_string(),
            timestamp: SystemTime::now(),
            data: RlmEventData::Error {
                error_type: "TestError".to_string(),
                message: "Test error".to_string(),
                recoverable: false,
            },
        };

        let result = sink.emit(event).await;
        assert!(result.is_err());
    }
}