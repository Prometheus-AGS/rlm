//! Performance metrics collection for RLM core processing.
//!
//! This module provides comprehensive performance tracking for RLM execution,
//! including timing, token usage, recursive call depth, and context processing metrics.

use crate::{
    error::{RlmError, RlmResult},
    types::{ExecutionMetadata, RlmEvent, TokenUsage},
};
#[cfg(test)]
use std::time::SystemTime;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, instrument, warn};

/// Performance metrics collector for RLM operations.
#[derive(Debug, Clone)]
pub struct RlmMetricsCollector {
    /// Internal metrics storage.
    metrics: Arc<RwLock<RlmMetrics>>,
    /// Session-specific metrics.
    session_metrics: Arc<RwLock<HashMap<String, SessionMetrics>>>,
}

/// Core RLM performance metrics.
#[derive(Debug, Default)]
pub struct RlmMetrics {
    /// Total number of RLM requests processed.
    pub total_requests: u64,
    /// Total processing time across all requests.
    pub total_processing_time: Duration,
    /// Total recursive calls made.
    pub total_recursive_calls: u64,
    /// Maximum recursive depth reached.
    pub max_recursive_depth: u32,
    /// Total tokens processed.
    pub total_tokens_processed: u64,
    /// Total context chunks processed.
    pub total_context_chunks: u64,
    /// Total context bytes offloaded to REPL.
    pub total_context_bytes: u64,
    /// Average processing time per request.
    pub avg_processing_time: Duration,
    /// Average tokens per request.
    pub avg_tokens_per_request: f64,
    /// Distribution of recursive depths.
    pub recursive_depth_distribution: HashMap<u32, u64>,
    /// Error counts by type.
    pub error_counts: HashMap<String, u64>,
    /// Request completion rates.
    pub completion_rates: CompletionRates,
}

/// Session-specific performance metrics.
#[derive(Debug, Default, Clone)]
pub struct SessionMetrics {
    /// Session ID.
    pub session_id: String,
    /// Session start time.
    pub start_time: Option<Instant>,
    /// Number of requests in this session.
    pub request_count: u64,
    /// Total processing time for this session.
    pub session_processing_time: Duration,
    /// Recursive calls in this session.
    pub recursive_calls: u64,
    /// Maximum depth reached in this session.
    pub max_depth: u32,
    /// Token usage for this session.
    pub token_usage: TokenUsage,
    /// Context processing metrics.
    pub context_metrics: ContextProcessingMetrics,
    /// Error count for this session.
    pub error_count: u64,
}

/// Context processing performance metrics.
#[derive(Debug, Default, Clone)]
pub struct ContextProcessingMetrics {
    /// Number of context chunks processed.
    pub chunks_processed: u64,
    /// Total size of context offloaded to REPL.
    pub context_bytes_offloaded: u64,
    /// Time spent on context analysis.
    pub context_analysis_time: Duration,
    /// Time spent on chunk processing.
    pub chunk_processing_time: Duration,
    /// Average chunk size in bytes.
    pub avg_chunk_size: f64,
}

/// Request completion rate tracking.
#[derive(Debug, Default)]
pub struct CompletionRates {
    /// Successfully completed requests.
    pub successful_requests: u64,
    /// Failed requests.
    pub failed_requests: u64,
    /// Timed out requests.
    pub timed_out_requests: u64,
    /// Cancelled requests.
    pub cancelled_requests: u64,
}

/// Timing information for a specific operation.
#[derive(Debug)]
pub struct OperationTimer {
    /// Operation name.
    pub operation: String,
    /// Start time.
    pub start_time: Instant,
    /// Optional metadata.
    pub metadata: HashMap<String, String>,
}

impl RlmMetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(RlmMetrics::default())),
            session_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start timing an operation.
    #[instrument(skip(self), fields(operation = %operation))]
    pub fn start_timer(&self, operation: &str) -> OperationTimer {
        OperationTimer {
            operation: operation.to_string(),
            start_time: Instant::now(),
            metadata: HashMap::new(),
        }
    }

    /// Start timing an operation with metadata.
    #[instrument(skip(self, metadata), fields(operation = %operation))]
    pub fn start_timer_with_metadata(
        &self,
        operation: &str,
        metadata: HashMap<String, String>,
    ) -> OperationTimer {
        OperationTimer {
            operation: operation.to_string(),
            start_time: Instant::now(),
            metadata,
        }
    }

    /// Record the completion of an RLM request.
    #[instrument(skip(self, execution_metadata), fields(session_id = %session_id))]
    pub fn record_request_completion(
        &self,
        session_id: &str,
        timer: OperationTimer,
        execution_metadata: &ExecutionMetadata,
        success: bool,
    ) -> RlmResult<()> {
        let processing_time = timer.start_time.elapsed();

        // Update global metrics
        {
            let mut metrics = self
                .metrics
                .write()
                .map_err(|e| RlmError::Other(format!("Failed to acquire metrics lock: {}", e)))?;

            metrics.total_requests += 1;
            metrics.total_processing_time += processing_time;
            metrics.total_recursive_calls += execution_metadata.recursive_calls as u64;
            metrics.max_recursive_depth = metrics
                .max_recursive_depth
                .max(execution_metadata.recursive_calls);
            metrics.total_tokens_processed += execution_metadata.total_tokens as u64;

            // Update recursive depth distribution
            *metrics
                .recursive_depth_distribution
                .entry(execution_metadata.recursive_calls)
                .or_insert(0) += 1;

            // Update completion rates
            if success {
                metrics.completion_rates.successful_requests += 1;
            } else {
                metrics.completion_rates.failed_requests += 1;
            }

            // Recalculate averages
            metrics.avg_processing_time =
                metrics.total_processing_time / metrics.total_requests as u32;
            metrics.avg_tokens_per_request =
                metrics.total_tokens_processed as f64 / metrics.total_requests as f64;
        }

        // Update session metrics
        {
            let mut session_metrics = self.session_metrics.write().map_err(|e| {
                RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
            })?;

            let session_metric = session_metrics
                .entry(session_id.to_string())
                .or_insert_with(|| SessionMetrics {
                    session_id: session_id.to_string(),
                    start_time: Some(Instant::now()),
                    ..Default::default()
                });

            session_metric.request_count += 1;
            session_metric.session_processing_time += processing_time;
            session_metric.recursive_calls += execution_metadata.recursive_calls as u64;
            session_metric.max_depth = session_metric
                .max_depth
                .max(execution_metadata.recursive_calls);
            session_metric.token_usage.total_tokens += execution_metadata.total_tokens;
            // Token details not available in ExecutionMetadata, using total_tokens
            session_metric.token_usage.total_tokens += execution_metadata.total_tokens;

            if !success {
                session_metric.error_count += 1;
            }
        }

        debug!(
            operation = %timer.operation,
            session_id = %session_id,
            processing_time_ms = processing_time.as_millis(),
            recursive_calls = execution_metadata.recursive_calls,
            tokens_used = execution_metadata.total_tokens,
            success = success,
            "Recorded request completion metrics"
        );

        Ok(())
    }

    /// Record context processing metrics.
    #[instrument(skip(self), fields(session_id = %session_id))]
    pub fn record_context_processing(
        &self,
        session_id: &str,
        chunks_processed: u64,
        context_bytes: u64,
        analysis_time: Duration,
        processing_time: Duration,
    ) -> RlmResult<()> {
        // Update global metrics
        {
            let mut metrics = self
                .metrics
                .write()
                .map_err(|e| RlmError::Other(format!("Failed to acquire metrics lock: {}", e)))?;

            metrics.total_context_chunks += chunks_processed;
            metrics.total_context_bytes += context_bytes;
        }

        // Update session metrics
        {
            let mut session_metrics = self.session_metrics.write().map_err(|e| {
                RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
            })?;

            let session_metric = session_metrics
                .entry(session_id.to_string())
                .or_insert_with(|| SessionMetrics {
                    session_id: session_id.to_string(),
                    start_time: Some(Instant::now()),
                    ..Default::default()
                });

            session_metric.context_metrics.chunks_processed += chunks_processed;
            session_metric.context_metrics.context_bytes_offloaded += context_bytes;
            session_metric.context_metrics.context_analysis_time += analysis_time;
            session_metric.context_metrics.chunk_processing_time += processing_time;

            // Update average chunk size
            if session_metric.context_metrics.chunks_processed > 0 {
                session_metric.context_metrics.avg_chunk_size =
                    session_metric.context_metrics.context_bytes_offloaded as f64
                        / session_metric.context_metrics.chunks_processed as f64;
            }
        }

        debug!(
            session_id = %session_id,
            chunks_processed = chunks_processed,
            context_bytes = context_bytes,
            analysis_time_ms = analysis_time.as_millis(),
            processing_time_ms = processing_time.as_millis(),
            "Recorded context processing metrics"
        );

        Ok(())
    }

    /// Record an error occurrence.
    #[instrument(skip(self), fields(session_id = %session_id, error_type = %error_type))]
    pub fn record_error(
        &self,
        session_id: &str,
        error_type: &str,
        error: &RlmError,
    ) -> RlmResult<()> {
        // Update global error counts
        {
            let mut metrics = self
                .metrics
                .write()
                .map_err(|e| RlmError::Other(format!("Failed to acquire metrics lock: {}", e)))?;

            *metrics
                .error_counts
                .entry(error_type.to_string())
                .or_insert(0) += 1;
        }

        // Update session error count
        {
            let mut session_metrics = self.session_metrics.write().map_err(|e| {
                RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
            })?;

            if let Some(session_metric) = session_metrics.get_mut(session_id) {
                session_metric.error_count += 1;
            }
        }

        warn!(
            session_id = %session_id,
            error_type = %error_type,
            error = %error,
            "Recorded error metrics"
        );

        Ok(())
    }

    /// Record a timeout occurrence.
    #[instrument(skip(self), fields(session_id = %session_id))]
    pub fn record_timeout(&self, session_id: &str, operation: &str) -> RlmResult<()> {
        {
            let mut metrics = self
                .metrics
                .write()
                .map_err(|e| RlmError::Other(format!("Failed to acquire metrics lock: {}", e)))?;

            metrics.completion_rates.timed_out_requests += 1;
        }

        self.record_error(
            session_id,
            "timeout",
            &RlmError::Other(format!("Operation '{}' timed out", operation)),
        )
    }

    /// Get current global metrics snapshot.
    pub fn get_global_metrics(&self) -> RlmResult<RlmMetrics> {
        self.metrics
            .read()
            .map_err(|e| RlmError::Other(format!("Failed to acquire metrics lock: {}", e)))?
            .clone()
            .into()
    }

    /// Get session metrics for a specific session.
    pub fn get_session_metrics(&self, session_id: &str) -> RlmResult<Option<SessionMetrics>> {
        let session_metrics = self.session_metrics.read().map_err(|e| {
            RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
        })?;

        Ok(session_metrics.get(session_id).cloned())
    }

    /// Get all active session metrics.
    pub fn get_all_session_metrics(&self) -> RlmResult<HashMap<String, SessionMetrics>> {
        let session_metrics = self.session_metrics.read().map_err(|e| {
            RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
        })?;
        Ok(session_metrics.clone())
    }

    /// Clear old session metrics based on age.
    #[instrument(skip(self))]
    pub fn cleanup_old_sessions(&self, max_age: Duration) -> RlmResult<u64> {
        let mut session_metrics = self.session_metrics.write().map_err(|e| {
            RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
        })?;

        let now = Instant::now();
        let mut removed_count = 0;

        session_metrics.retain(|_session_id, metrics| {
            if let Some(start_time) = metrics.start_time {
                let age = now.duration_since(start_time);
                if age > max_age {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            } else {
                // Keep sessions without start time
                true
            }
        });

        if removed_count > 0 {
            debug!(
                removed_sessions = removed_count,
                "Cleaned up old session metrics"
            );
        }

        Ok(removed_count)
    }

    /// Process RLM event for metrics collection.
    #[instrument(skip(self, event))]
    pub fn process_event(&self, event: &RlmEvent, session_id: &str) -> RlmResult<()> {
        match &event.data {
            crate::types::RlmEventData::RecursiveCall { depth, .. } => {
                // Record recursive call depth
                let mut session_metrics = self.session_metrics.write().map_err(|e| {
                    RlmError::Other(format!("Failed to acquire session metrics lock: {}", e))
                })?;

                let session_metric = session_metrics
                    .entry(session_id.to_string())
                    .or_insert_with(|| SessionMetrics {
                        session_id: session_id.to_string(),
                        start_time: Some(Instant::now()),
                        ..Default::default()
                    });

                session_metric.recursive_calls += 1;
                session_metric.max_depth = session_metric.max_depth.max(*depth);
            }
            crate::types::RlmEventData::ContextChunk {
                chunk_id: _,
                size_bytes,
                processed,
            } => {
                // Record context processing
                if *processed {
                    self.record_context_processing(
                        session_id,
                        1, // Single chunk
                        *size_bytes as u64,
                        Duration::from_millis(0), // Analysis time not available in event
                        Duration::from_millis(0), // Processing time not available in event
                    )?;
                }
            }
            crate::types::RlmEventData::Error { message, .. } => {
                // Record error
                self.record_error(
                    session_id,
                    "processing_error",
                    &RlmError::Other(message.clone()),
                )?;
            }
            _ => {
                // Other events don't require specific metrics processing
            }
        }

        Ok(())
    }
}

impl Clone for RlmMetrics {
    fn clone(&self) -> Self {
        Self {
            total_requests: self.total_requests,
            total_processing_time: self.total_processing_time,
            total_recursive_calls: self.total_recursive_calls,
            max_recursive_depth: self.max_recursive_depth,
            total_tokens_processed: self.total_tokens_processed,
            total_context_chunks: self.total_context_chunks,
            total_context_bytes: self.total_context_bytes,
            avg_processing_time: self.avg_processing_time,
            avg_tokens_per_request: self.avg_tokens_per_request,
            recursive_depth_distribution: self.recursive_depth_distribution.clone(),
            error_counts: self.error_counts.clone(),
            completion_rates: CompletionRates {
                successful_requests: self.completion_rates.successful_requests,
                failed_requests: self.completion_rates.failed_requests,
                timed_out_requests: self.completion_rates.timed_out_requests,
                cancelled_requests: self.completion_rates.cancelled_requests,
            },
        }
    }
}

impl From<RlmMetrics> for RlmResult<RlmMetrics> {
    fn from(metrics: RlmMetrics) -> Self {
        Ok(metrics)
    }
}

impl Default for RlmMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationTimer {
    /// Add metadata to the timer.
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Get the elapsed time since timer started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = RlmMetricsCollector::new();
        let metrics = collector
            .get_global_metrics()
            .expect("Failed to get metrics");

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.total_recursive_calls, 0);
    }

    #[test]
    fn test_operation_timer() {
        let collector = RlmMetricsCollector::new();
        let timer = collector.start_timer("test_operation");

        assert_eq!(timer.operation, "test_operation");
        assert!(timer.elapsed().as_nanos() > 0);
    }

    #[test]
    fn test_request_completion_recording() {
        let collector = RlmMetricsCollector::new();
        let timer = collector.start_timer("test_request");

        let execution_metadata = ExecutionMetadata {
            iterations: 1,
            recursive_calls: 2,
            total_tokens: 150,
            duration: Duration::from_millis(100),
            started_at: SystemTime::now(),
            success: true,
        };

        collector
            .record_request_completion("test_session", timer, &execution_metadata, true)
            .expect("Failed to record request completion");

        let metrics = collector
            .get_global_metrics()
            .expect("Failed to get metrics");
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.total_recursive_calls, 2);
        assert_eq!(metrics.max_recursive_depth, 3);
        assert_eq!(metrics.total_tokens_processed, 150);
    }

    #[test]
    fn test_session_metrics() {
        let collector = RlmMetricsCollector::new();
        let timer = collector.start_timer("test_request");

        let execution_metadata = ExecutionMetadata {
            iterations: 1,
            recursive_calls: 1,
            total_tokens: 150,
            duration: Duration::from_millis(100),
            started_at: SystemTime::now(),
            success: true,
        };

        collector
            .record_request_completion("session_123", timer, &execution_metadata, true)
            .expect("Failed to record request completion");

        let session_metrics = collector
            .get_session_metrics("session_123")
            .expect("Failed to get session metrics")
            .expect("Session metrics not found");

        assert_eq!(session_metrics.session_id, "session_123");
        assert_eq!(session_metrics.request_count, 1);
        assert_eq!(session_metrics.recursive_calls, 1);
        assert_eq!(session_metrics.max_depth, 2);
        assert_eq!(session_metrics.token_usage.total_tokens, 150);
    }

    #[test]
    fn test_error_recording() {
        let collector = RlmMetricsCollector::new();
        let error = RlmError::Other("Test error".to_string());

        collector
            .record_error("session_123", "test_error", &error)
            .expect("Failed to record error");

        let metrics = collector
            .get_global_metrics()
            .expect("Failed to get metrics");
        assert_eq!(metrics.error_counts.get("test_error"), Some(&1));
    }

    #[test]
    fn test_context_processing_recording() {
        let collector = RlmMetricsCollector::new();

        collector
            .record_context_processing(
                "session_123",
                5,
                10240,
                Duration::from_millis(50),
                Duration::from_millis(200),
            )
            .expect("Failed to record context processing");

        let metrics = collector
            .get_global_metrics()
            .expect("Failed to get metrics");
        assert_eq!(metrics.total_context_chunks, 5);
        assert_eq!(metrics.total_context_bytes, 10240);

        let session_metrics = collector
            .get_session_metrics("session_123")
            .expect("Failed to get session metrics")
            .expect("Session metrics not found");

        assert_eq!(session_metrics.context_metrics.chunks_processed, 5);
        assert_eq!(
            session_metrics.context_metrics.context_bytes_offloaded,
            10240
        );
        assert_eq!(session_metrics.context_metrics.avg_chunk_size, 2048.0);
    }
}
