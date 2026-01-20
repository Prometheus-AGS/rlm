//! Event sink port definition.
//!
//! This module defines the `EventSink` trait that enables RLM to emit
//! real-time events during execution for monitoring, logging, and UX.

use crate::error::RlmResult;
use crate::types::RlmEvent;
use async_trait::async_trait;
use std::time::SystemTime;

/// Sink for RLM execution events.
///
/// This trait abstracts different event handling strategies, allowing
/// the core executor to emit events without knowing how they're processed.
///
/// # Design Notes
///
/// - All operations are async to support I/O-bound sinks (SSE, WebSockets, files)
/// - Events are strongly typed for better debugging and monitoring
/// - Implementations should handle backpressure gracefully
/// - Events should be emitted in chronological order
///
/// # Example
///
/// ```rust,no_run
/// use rlm_core::ports::EventSink;
/// use rlm_core::types::{RlmEvent, ReplResult};
/// use std::time::SystemTime;
///
/// async fn execute_with_events<S: EventSink>(sink: &S) -> Result<(), Box<dyn std::error::Error>> {
///     sink.emit(RlmEvent::ExecutionStarted {
///         request_id: "req-123".to_string(),
///         timestamp: SystemTime::now(),
///     }).await?;
///
///     sink.emit(RlmEvent::ReplOp {
///         iteration: 1,
///         code: "let x = 42".to_string(),
///         result: ReplResult::Success { value: "42".to_string() },
///         timestamp: SystemTime::now(),
///     }).await?;
///
///     sink.emit(RlmEvent::Done {
///         request_id: "req-123".to_string(),
///         timestamp: SystemTime::now(),
///     }).await?;
///
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait EventSink: Send + Sync + std::fmt::Debug {
    /// Emit a single event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Event was successfully handled
    /// * `Err(RlmError)` - Event handling failed
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The underlying transport fails (network, file I/O, etc.)
    /// - The event sink is in an invalid state
    /// - Backpressure limits are exceeded
    async fn emit(&self, event: RlmEvent) -> RlmResult<()>;

    /// Emit multiple events as a batch.
    ///
    /// This is an optimization for sinks that can process multiple
    /// events more efficiently together.
    ///
    /// # Arguments
    ///
    /// * `events` - Vector of events to emit
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All events were successfully handled
    /// * `Err(RlmError)` - At least one event failed (may be partial)
    async fn emit_batch(&self, events: Vec<RlmEvent>) -> RlmResult<()>;

    /// Flush any buffered events.
    ///
    /// Ensures all previously emitted events have been fully processed
    /// by the underlying transport. Useful before shutdown or critical points.
    async fn flush(&self) -> RlmResult<()>;

    /// Close the event sink.
    ///
    /// Performs cleanup and ensures all events are flushed before closing.
    /// After this call, further emit operations may fail.
    async fn close(&mut self) -> RlmResult<()>;

    /// Get sink statistics and health information.
    ///
    /// Returns information about event throughput, buffer status, etc.
    /// Useful for monitoring and debugging.
    async fn get_stats(&self) -> RlmResult<SinkStats>;

    /// Get metadata about the event sink.
    fn get_metadata(&self) -> SinkMetadata;

    /// Check if the sink is healthy and ready to receive events.
    async fn health_check(&self) -> RlmResult<bool>;

    /// Set event filtering criteria.
    ///
    /// Allows selective event emission based on type, severity, etc.
    /// Implementations may ignore this if filtering is not supported.
    async fn set_filter(&mut self, filter: EventFilter) -> RlmResult<()>;
}

/// Statistics about event sink performance and state.
#[derive(Debug, Clone, PartialEq)]
pub struct SinkStats {
    /// Total events emitted since creation
    pub events_emitted: u64,
    /// Events currently buffered (if applicable)
    pub events_buffered: u32,
    /// Number of events that failed to emit
    pub events_failed: u64,
    /// Average emit latency in milliseconds (if available)
    pub avg_emit_latency_ms: Option<f64>,
    /// Sink creation timestamp
    pub created_at: SystemTime,
    /// Last successful emit timestamp (if any)
    pub last_emit_at: Option<SystemTime>,
    /// Current buffer capacity utilization (0.0 to 1.0)
    pub buffer_utilization: f64,
}

impl SinkStats {
    /// Create new sink statistics.
    pub fn new() -> Self {
        Self {
            events_emitted: 0,
            events_buffered: 0,
            events_failed: 0,
            avg_emit_latency_ms: None,
            created_at: SystemTime::now(),
            last_emit_at: None,
            buffer_utilization: 0.0,
        }
    }

    /// Get success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        if self.events_emitted == 0 {
            return 100.0;
        }
        let total = self.events_emitted + self.events_failed;
        (self.events_emitted as f64 / total as f64) * 100.0
    }

    /// Check if the sink is considered healthy.
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if success rate > 95% and buffer not full
        self.success_rate() > 95.0 && self.buffer_utilization < 0.9
    }
}

impl Default for SinkStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about an event sink implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkMetadata {
    /// Sink type identifier (e.g., "sse", "websocket", "file", "memory")
    pub sink_type: String,
    /// Sink version or implementation version
    pub version: String,
    /// Maximum buffer size (if applicable)
    pub max_buffer_size: Option<u32>,
    /// Whether the sink supports batch operations
    pub supports_batch: bool,
    /// Whether the sink supports event filtering
    pub supports_filtering: bool,
    /// List of supported capabilities
    pub capabilities: Vec<String>,
    /// Target destination (e.g., file path, URL, etc.)
    pub destination: Option<String>,
}

impl SinkMetadata {
    /// Create new sink metadata.
    pub fn new(
        sink_type: impl Into<String>,
        version: impl Into<String>,
        supports_batch: bool,
        supports_filtering: bool,
    ) -> Self {
        Self {
            sink_type: sink_type.into(),
            version: version.into(),
            max_buffer_size: None,
            supports_batch,
            supports_filtering,
            capabilities: Vec::new(),
            destination: None,
        }
    }

    /// Check if the sink supports a specific capability.
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    /// Set buffer size information.
    pub fn with_buffer_size(mut self, size: u32) -> Self {
        self.max_buffer_size = Some(size);
        self
    }

    /// Set destination information.
    pub fn with_destination(mut self, destination: impl Into<String>) -> Self {
        self.destination = Some(destination.into());
        self
    }

    /// Add supported capabilities.
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Event filtering configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFilter {
    /// Minimum event severity to emit
    pub min_severity: EventSeverity,
    /// Event types to include (empty means all)
    pub include_types: Vec<String>,
    /// Event types to exclude
    pub exclude_types: Vec<String>,
    /// Maximum events per second (rate limiting)
    pub max_events_per_second: Option<u32>,
    /// Only emit events for specific request IDs
    pub request_id_filter: Option<Vec<String>>,
}

impl EventFilter {
    /// Create a new event filter that allows all events.
    pub fn allow_all() -> Self {
        Self {
            min_severity: EventSeverity::Debug,
            include_types: Vec::new(),
            exclude_types: Vec::new(),
            max_events_per_second: None,
            request_id_filter: None,
        }
    }

    /// Create a filter for critical events only.
    pub fn critical_only() -> Self {
        Self {
            min_severity: EventSeverity::Error,
            include_types: Vec::new(),
            exclude_types: Vec::new(),
            max_events_per_second: None,
            request_id_filter: None,
        }
    }

    /// Check if an event should be emitted based on this filter.
    pub fn should_emit(&self, _event: &RlmEvent) -> bool {
        // Implementation would check event against filter criteria
        // This is a placeholder for the actual filtering logic
        true
    }
}

/// Event severity levels for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    /// Debug/trace level events
    Debug = 0,
    /// Informational events
    Info = 1,
    /// Warning events
    Warn = 2,
    /// Error events
    Error = 3,
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::allow_all()
    }
}

/// A no-op event sink that discards all events.
///
/// Useful for testing or when event handling is not needed.
#[derive(Debug, Default)]
pub struct NullEventSink;

#[async_trait]
impl EventSink for NullEventSink {
    async fn emit(&self, _event: RlmEvent) -> RlmResult<()> {
        // Discard the event
        Ok(())
    }

    async fn emit_batch(&self, _events: Vec<RlmEvent>) -> RlmResult<()> {
        // Discard all events
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        // Nothing to flush
        Ok(())
    }

    async fn close(&mut self) -> RlmResult<()> {
        // Nothing to close
        Ok(())
    }

    async fn get_stats(&self) -> RlmResult<SinkStats> {
        Ok(SinkStats::new())
    }

    fn get_metadata(&self) -> SinkMetadata {
        SinkMetadata::new("null", "1.0", false, false)
            .with_destination("null".to_string())
    }

    async fn health_check(&self) -> RlmResult<bool> {
        Ok(true)
    }

    async fn set_filter(&mut self, _filter: EventFilter) -> RlmResult<()> {
        // Filter is ignored for null sink
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sink_stats() {
        let stats = SinkStats {
            events_emitted: 950,
            events_failed: 50,
            buffer_utilization: 0.8,
            ..SinkStats::new()
        };

        assert_eq!(stats.success_rate(), 95.0);
        assert!(stats.is_healthy());
    }

    #[test]
    fn test_sink_stats_unhealthy() {
        let stats = SinkStats {
            events_emitted: 800,
            events_failed: 200,
            buffer_utilization: 0.95,
            ..SinkStats::new()
        };

        assert_eq!(stats.success_rate(), 80.0);
        assert!(!stats.is_healthy());
    }

    #[test]
    fn test_sink_metadata_builder() {
        let metadata = SinkMetadata::new("test", "1.0", true, false)
            .with_buffer_size(1000)
            .with_destination("file://test.log")
            .with_capabilities(vec!["compression".to_string()]);

        assert_eq!(metadata.sink_type, "test");
        assert!(metadata.supports_batch);
        assert!(!metadata.supports_filtering);
        assert_eq!(metadata.max_buffer_size, Some(1000));
        assert!(metadata.supports("compression"));
        assert!(!metadata.supports("encryption"));
    }

    #[test]
    fn test_event_filter_creation() {
        let filter = EventFilter::critical_only();
        assert_eq!(filter.min_severity, EventSeverity::Error);

        let filter = EventFilter::allow_all();
        assert_eq!(filter.min_severity, EventSeverity::Debug);
    }
}