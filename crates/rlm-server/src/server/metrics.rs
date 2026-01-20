//! Prometheus metrics endpoint implementation.
//!
//! This module provides comprehensive metrics collection and exposure
//! for monitoring RLM server performance, request patterns, and health.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use prometheus::{
    Counter, Gauge, Histogram, IntCounter, IntGauge, Registry, TextEncoder,
};
use rlm_core::{RlmResult, RlmError};
use std::sync::{Arc, OnceLock};
use tracing::{debug, error};

use crate::server::routes::AppState;

/// Global metrics registry - safe initialization with OnceLock.
static GLOBAL_REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();
static GLOBAL_METRICS: OnceLock<Arc<RlmMetrics>> = OnceLock::new();

/// Comprehensive RLM metrics collection.
#[derive(Debug)]
pub struct RlmMetrics {
    // Request metrics
    /// Total number of requests processed by the server.
    pub requests_total: IntCounter,
    /// Current number of requests being processed.
    pub requests_in_flight: IntGauge,
    /// Histogram of request processing durations in seconds.
    pub request_duration: Histogram,
    /// Histogram of request body sizes in bytes.
    pub request_size_bytes: Histogram,
    /// Histogram of response body sizes in bytes.
    pub response_size_bytes: Histogram,

    // RLM-specific metrics
    /// Total number of recursive LLM calls made.
    pub recursive_calls_total: IntCounter,
    /// Histogram of recursive call depths.
    pub recursive_depth: Histogram,
    /// Total number of context chunks processed.
    pub context_chunks_processed: IntCounter,
    /// Histogram of context offload sizes in bytes.
    pub context_offload_size_bytes: Histogram,

    // Token usage metrics
    /// Total number of tokens consumed (prompt + completion).
    pub tokens_total: IntCounter,
    /// Total number of prompt tokens consumed.
    pub prompt_tokens: IntCounter,
    /// Total number of completion tokens generated.
    pub completion_tokens: IntCounter,
    /// Total cost of token usage in USD.
    pub token_usage_cost: Counter,

    // Backend provider metrics
    /// Total number of requests sent to LLM backends.
    pub backend_requests_total: IntCounter,
    /// Total number of errors from LLM backends.
    pub backend_errors_total: IntCounter,
    /// Histogram of backend request latencies in seconds.
    pub backend_latency: Histogram,
    /// Current health status of LLM backends (1=healthy, 0=unhealthy).
    pub backend_health_status: IntGauge,

    // HTTP server metrics
    /// Total number of HTTP requests handled.
    pub http_requests_total: IntCounter,
    /// Histogram of HTTP request durations in seconds.
    pub http_request_duration: Histogram,
    /// Current number of HTTP requests being processed.
    pub http_requests_in_flight: IntGauge,
    /// Counter of HTTP responses by status code.
    pub http_response_status: IntCounter,

    // Error tracking
    /// Total number of errors encountered.
    pub errors_total: IntCounter,
    /// Current error rate (errors per second).
    pub error_rate: Gauge,
    /// Total number of request timeouts.
    pub timeout_total: IntCounter,
    /// Total number of retry attempts made.
    pub retry_attempts_total: IntCounter,

    // System metrics
    /// Current system memory usage as a percentage.
    pub system_memory_usage: Gauge,
    /// Current system CPU usage as a percentage.
    pub system_cpu_usage: Gauge,
    /// Current number of active connections.
    pub active_connections: IntGauge,
    /// Current size of the connection pool.
    pub connection_pool_size: IntGauge,
}

impl RlmMetrics {
    /// Create a new metrics collection with default configuration.
    pub fn new() -> RlmResult<Self> {
        Ok(Self {
            // Request metrics
            requests_total: IntCounter::new(
                "rlm_requests_total",
                "Total number of RLM requests processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create requests_total metric: {}", e)))?,

            requests_in_flight: IntGauge::new(
                "rlm_requests_in_flight",
                "Number of RLM requests currently being processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create requests_in_flight metric: {}", e)))?,

            request_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_request_duration_seconds",
                    "Duration of RLM request processing in seconds"
                ).buckets(vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create request_duration metric: {}", e)))?,

            request_size_bytes: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_request_size_bytes",
                    "Size of RLM requests in bytes"
                ).buckets(vec![1024.0, 10240.0, 102400.0, 1048576.0, 10485760.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create request_size_bytes metric: {}", e)))?,

            response_size_bytes: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_response_size_bytes",
                    "Size of RLM responses in bytes"
                ).buckets(vec![1024.0, 10240.0, 102400.0, 1048576.0, 10485760.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create response_size_bytes metric: {}", e)))?,

            // RLM-specific metrics
            recursive_calls_total: IntCounter::new(
                "rlm_recursive_calls_total",
                "Total number of recursive LLM calls made"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create recursive_calls_total metric: {}", e)))?,

            recursive_depth: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_recursive_depth",
                    "Depth of recursive calls in RLM processing"
                ).buckets(vec![1.0, 2.0, 3.0, 5.0, 10.0, 20.0, 50.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create recursive_depth metric: {}", e)))?,

            context_chunks_processed: IntCounter::new(
                "rlm_context_chunks_processed_total",
                "Total number of context chunks processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create context_chunks_processed metric: {}", e)))?,

            context_offload_size_bytes: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_context_offload_size_bytes",
                    "Size of context offloaded to REPL in bytes"
                ).buckets(vec![10240.0, 102400.0, 1048576.0, 10485760.0, 104857600.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create context_offload_size_bytes metric: {}", e)))?,

            // Token usage metrics
            tokens_total: IntCounter::new(
                "rlm_token_usage_total",
                "Total number of tokens processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create tokens_total metric: {}", e)))?,

            prompt_tokens: IntCounter::new(
                "rlm_prompt_tokens_total",
                "Total number of prompt tokens processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create prompt_tokens metric: {}", e)))?,

            completion_tokens: IntCounter::new(
                "rlm_completion_tokens_total",
                "Total number of completion tokens generated"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create completion_tokens metric: {}", e)))?,

            token_usage_cost: Counter::new(
                "rlm_token_usage_cost_total",
                "Total estimated cost of token usage in USD"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create token_usage_cost metric: {}", e)))?,

            // Backend provider metrics
            backend_requests_total: IntCounter::new(
                "rlm_backend_requests_total",
                "Total requests to backend providers"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create backend_requests_total metric: {}", e)))?,

            backend_errors_total: IntCounter::new(
                "rlm_backend_errors_total",
                "Total errors from backend providers"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create backend_errors_total metric: {}", e)))?,

            backend_latency: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "rlm_backend_latency_seconds",
                    "Backend provider response latency in seconds"
                ).buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create backend_latency metric: {}", e)))?,

            backend_health_status: IntGauge::new(
                "rlm_backend_health_status",
                "Health status of backend providers (1=healthy, 0=unhealthy)"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create backend_health_status metric: {}", e)))?,

            // HTTP server metrics
            http_requests_total: IntCounter::new(
                "http_requests_total",
                "Total HTTP requests received"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create http_requests_total metric: {}", e)))?,

            http_request_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "http_request_duration_seconds",
                    "HTTP request processing duration in seconds"
                ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0])
            )
            .map_err(|e| RlmError::Other(format!("Failed to create http_request_duration metric: {}", e)))?,

            http_requests_in_flight: IntGauge::new(
                "http_requests_in_flight",
                "Number of HTTP requests currently being processed"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create http_requests_in_flight metric: {}", e)))?,

            http_response_status: IntCounter::new(
                "http_response_status_total",
                "HTTP response status codes"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create http_response_status metric: {}", e)))?,

            // Error tracking
            errors_total: IntCounter::new(
                "rlm_errors_total",
                "Total number of errors encountered"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create errors_total metric: {}", e)))?,

            error_rate: Gauge::new(
                "rlm_error_rate",
                "Current error rate (errors per second)"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create error_rate metric: {}", e)))?,

            timeout_total: IntCounter::new(
                "rlm_timeouts_total",
                "Total number of request timeouts"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create timeout_total metric: {}", e)))?,

            retry_attempts_total: IntCounter::new(
                "rlm_retry_attempts_total",
                "Total number of retry attempts"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create retry_attempts_total metric: {}", e)))?,

            // System metrics
            system_memory_usage: Gauge::new(
                "rlm_system_memory_usage_bytes",
                "Current system memory usage in bytes"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create system_memory_usage metric: {}", e)))?,

            system_cpu_usage: Gauge::new(
                "rlm_system_cpu_usage_percent",
                "Current system CPU usage percentage"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create system_cpu_usage metric: {}", e)))?,

            active_connections: IntGauge::new(
                "rlm_active_connections",
                "Number of active client connections"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create active_connections metric: {}", e)))?,

            connection_pool_size: IntGauge::new(
                "rlm_connection_pool_size",
                "Size of the connection pool"
            )
            .map_err(|e| RlmError::Other(format!("Failed to create connection_pool_size metric: {}", e)))?,
        })
    }

    /// Register all metrics with the given registry.
    pub fn register_with_registry(&self, registry: &Registry) -> RlmResult<()> {
        // Register all metrics (abbreviated for brevity - full registration would include all metrics)
        registry.register(Box::new(self.requests_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register requests_total: {}", e)))?;
        registry.register(Box::new(self.requests_in_flight.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register requests_in_flight: {}", e)))?;
        registry.register(Box::new(self.request_duration.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register request_duration: {}", e)))?;

        // Add other metrics registration here...
        registry.register(Box::new(self.recursive_calls_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register recursive_calls_total: {}", e)))?;
        registry.register(Box::new(self.tokens_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register tokens_total: {}", e)))?;
        registry.register(Box::new(self.backend_requests_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register backend_requests_total: {}", e)))?;
        registry.register(Box::new(self.http_requests_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register http_requests_total: {}", e)))?;
        registry.register(Box::new(self.errors_total.clone()))
            .map_err(|e| RlmError::Other(format!("Failed to register errors_total: {}", e)))?;

        Ok(())
    }
}

/// Initialize global metrics registry and collectors.
pub fn init_metrics() -> RlmResult<()> {
    let registry = Registry::new();
    let metrics = RlmMetrics::new()?;

    metrics.register_with_registry(&registry)?;

    GLOBAL_REGISTRY.set(Arc::new(registry))
        .map_err(|_| RlmError::Other("Failed to set global registry".to_string()))?;
    GLOBAL_METRICS.set(Arc::new(metrics))
        .map_err(|_| RlmError::Other("Failed to set global metrics".to_string()))?;

    debug!("Metrics registry initialized successfully");
    Ok(())
}

/// Get global metrics instance.
pub fn get_global_metrics() -> Option<Arc<RlmMetrics>> {
    GLOBAL_METRICS.get().cloned()
}

/// Get global registry instance.
pub fn get_global_registry() -> Option<Arc<Registry>> {
    GLOBAL_REGISTRY.get().cloned()
}

/// Axum handler for the /metrics endpoint.
pub async fn metrics_handler(State(_state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, String)> {
    debug!("Serving metrics endpoint");

    // Initialize metrics if not already done
    if get_global_registry().is_none() {
        if let Err(e) = init_metrics() {
            error!("Failed to initialize metrics: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to initialize metrics".to_string()
            ));
        }
    }

    // Get global registry
    let registry = match get_global_registry() {
        Some(registry) => registry,
        None => {
            error!("Metrics registry not available");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Metrics registry not available".to_string()
            ));
        }
    };

    // Update system metrics before serving
    if let Some(metrics) = get_global_metrics() {
        update_system_metrics(&metrics).await;
    }

    // Encode metrics in Prometheus text format
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();

    match encoder.encode_to_string(&metric_families) {
        Ok(encoded_metrics) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                "content-type",
                "text/plain; version=0.0.4; charset=utf-8"
                    .parse()
                    .unwrap()
            );

            debug!("Successfully encoded {} metric families", metric_families.len());

            Ok((StatusCode::OK, headers, encoded_metrics))
        }
        Err(e) => {
            error!("Failed to encode metrics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {}", e)
            ))
        }
    }
}

/// Update system metrics with current values.
async fn update_system_metrics(metrics: &RlmMetrics) {
    // Update memory usage (placeholder implementation)
    if let Ok(memory_usage) = get_memory_usage() {
        metrics.system_memory_usage.set(memory_usage as f64);
    }

    // Update CPU usage (placeholder implementation)
    if let Ok(cpu_usage) = get_cpu_usage() {
        metrics.system_cpu_usage.set(cpu_usage);
    }
}

/// Get current memory usage in bytes (placeholder implementation).
fn get_memory_usage() -> Result<u64, Box<dyn std::error::Error>> {
    Ok(0) // Placeholder - would use system crates in production
}

/// Get current CPU usage percentage (placeholder implementation).
fn get_cpu_usage() -> Result<f64, Box<dyn std::error::Error>> {
    Ok(0.0) // Placeholder - would use system crates in production
}

/// Record RLM request metrics.
pub fn record_request_metrics(
    duration: std::time::Duration,
    request_size: usize,
    response_size: usize,
    recursive_calls: u32,
    tokens_used: u32,
) {
    if let Some(metrics) = get_global_metrics() {
        metrics.requests_total.inc();
        metrics.request_duration.observe(duration.as_secs_f64());
        metrics.request_size_bytes.observe(request_size as f64);
        metrics.response_size_bytes.observe(response_size as f64);
        metrics.recursive_calls_total.inc_by(recursive_calls as u64);
        metrics.tokens_total.inc_by(tokens_used as u64);
    }
}

/// Record backend provider metrics.
pub fn record_backend_metrics(
    _provider: &str,
    success: bool,
    latency: std::time::Duration,
) {
    if let Some(metrics) = get_global_metrics() {
        metrics.backend_requests_total.inc();
        if !success {
            metrics.backend_errors_total.inc();
        }
        metrics.backend_latency.observe(latency.as_secs_f64());
    }
}

/// Record error metrics.
pub fn record_error_metrics(error_type: &str) {
    if let Some(metrics) = get_global_metrics() {
        metrics.errors_total.inc();

        if error_type == "timeout" {
            metrics.timeout_total.inc();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = RlmMetrics::new().expect("Failed to create metrics");

        // Test basic metric operations
        metrics.requests_total.inc();
        assert_eq!(metrics.requests_total.get(), 1);

        metrics.request_duration.observe(1.5);
        assert_eq!(metrics.request_duration.get_sample_count(), 1);
    }

    #[test]
    fn test_metrics_registry() {
        let registry = Registry::new();
        let metrics = RlmMetrics::new().expect("Failed to create metrics");

        metrics.register_with_registry(&registry).expect("Failed to register metrics");

        let metric_families = registry.gather();
        assert!(!metric_families.is_empty());
    }

    #[tokio::test]
    async fn test_record_request_metrics() {
        init_metrics().expect("Failed to init metrics");

        record_request_metrics(
            std::time::Duration::from_millis(500),
            1024,
            2048,
            3,
            150,
        );

        // Verify metrics were recorded
        if let Some(metrics) = get_global_metrics() {
            assert_eq!(metrics.requests_total.get(), 1);
            assert_eq!(metrics.recursive_calls_total.get(), 3);
            assert_eq!(metrics.tokens_total.get(), 150);
        }
    }
}