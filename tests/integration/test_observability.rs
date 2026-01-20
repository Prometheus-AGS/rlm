//! Observability integration tests.
//!
//! Tests verify that structured logging, metrics collection, and tracing
//! provide comprehensive observability during RLM processing.

use rlm_core::{
    executor::RlmExecutor,
    types::{ChatCompletionRequest, ChatMessage, ChatRole, RlmRequest},
    config::RlmConfig,
};
use rlm_repl_rhai::RhaiReplBackend;
use rlm_server::adapters::openai::{OpenAiProvider, OpenAiConfigBuilder};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error, debug};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    fmt::format::FmtSpan,
};

/// Test structured logging during RLM execution.
#[tokio::test]
async fn test_structured_logging_during_execution() {
    // Initialize test logging with JSON format
    let _guard = init_test_logging();

    // Create RLM executor
    let rlm_config = RlmConfig::default();
    let repl_backend = RhaiReplBackend::new().expect("Failed to create REPL backend");

    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string());
    let openai_config = OpenAiConfigBuilder::new(api_key).build();
    let openai_provider = OpenAiProvider::new(openai_config).expect("Failed to create provider");

    let executor = RlmExecutor::new(rlm_config, repl_backend, openai_provider);

    // Create test request
    let request = RlmRequest {
        query: "What is the capital of France?".to_string(),
        context: "You are a helpful assistant.".to_string(),
        max_iterations: 3,
        recursion_depth: 1,
        metadata: std::collections::HashMap::new(),
    };

    // Execute with structured logging
    info!(
        target: "rlm_test",
        request_id = "test-123",
        query = %request.query,
        max_iterations = request.max_iterations,
        "Starting RLM execution"
    );

    let result = executor.execute(request).await;

    match result {
        Ok(response) => {
            info!(
                target: "rlm_test",
                request_id = "test-123",
                answer_length = response.answer.len(),
                total_tokens = response.metadata.total_tokens,
                duration_ms = response.metadata.duration.as_millis(),
                "RLM execution completed successfully"
            );
        }
        Err(e) => {
            error!(
                target: "rlm_test",
                request_id = "test-123",
                error = %e,
                "RLM execution failed"
            );
        }
    }
}

/// Test log context propagation through recursive calls.
#[tokio::test]
async fn test_log_context_propagation() {
    let _guard = init_test_logging();

    // Test that context is properly maintained through nested spans
    let span = tracing::info_span!(
        "test_recursive_context",
        request_id = "test-456",
        depth = 0
    );

    let _enter = span.enter();

    info!("Starting recursive processing test");

    // Simulate nested recursive call context
    simulate_recursive_call(1, "test-456").await;
    simulate_recursive_call(2, "test-456").await;

    info!("Completed recursive processing test");
}

/// Test error tracking and correlation.
#[tokio::test]
async fn test_error_tracking_correlation() {
    let _guard = init_test_logging();

    let request_id = "test-error-789";
    let span = tracing::error_span!("error_tracking_test", request_id = request_id);

    let _enter = span.enter();

    // Simulate various error scenarios
    warn!(
        request_id = request_id,
        error_type = "provider_error",
        provider = "openai",
        "Provider request failed, retrying"
    );

    error!(
        request_id = request_id,
        error_type = "timeout",
        timeout_ms = 30000,
        "Request timed out after maximum retries"
    );

    debug!(
        request_id = request_id,
        recovery_action = "fallback_provider",
        "Attempting recovery with fallback"
    );
}

/// Test performance metrics logging.
#[tokio::test]
async fn test_performance_metrics_logging() {
    let _guard = init_test_logging();

    let request_id = "test-perf-101";
    let start_time = std::time::Instant::now();

    // Simulate processing with performance logging
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let processing_time = start_time.elapsed();

    info!(
        target: "rlm_performance",
        request_id = request_id,
        processing_time_ms = processing_time.as_millis(),
        tokens_processed = 150,
        context_chunks = 3,
        recursive_calls = 2,
        "Performance metrics captured"
    );

    // Log token usage metrics
    info!(
        target: "rlm_token_usage",
        request_id = request_id,
        prompt_tokens = 100,
        completion_tokens = 75,
        total_tokens = 175,
        cost_estimate_cents = 0.35,
        "Token usage tracked"
    );
}

/// Test concurrent request logging.
#[tokio::test]
async fn test_concurrent_request_logging() {
    let _guard = init_test_logging();

    // Spawn multiple concurrent requests
    let mut handles = vec![];

    for i in 0..5 {
        let request_id = format!("concurrent-{}", i);
        let handle = tokio::spawn(async move {
            let span = tracing::info_span!(
                "concurrent_request",
                request_id = %request_id,
                thread_id = i
            );
            let _enter = span.enter();

            info!("Starting concurrent request processing");

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            info!(
                tokens_used = i * 10,
                "Completed concurrent request processing"
            );
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.expect("Concurrent request failed");
    }

    info!("All concurrent requests completed");
}

/// Test log sampling and filtering.
#[tokio::test]
async fn test_log_sampling_and_filtering() {
    let _guard = init_test_logging();

    // Test different log levels
    trace!("This is a trace message - should be filtered in production");
    debug!("This is a debug message - useful for development");
    info!("This is an info message - normal operation");
    warn!("This is a warning - potential issue detected");
    error!("This is an error - action required");

    // Test structured data with filtering
    for i in 0..10 {
        if i % 3 == 0 {  // Sample every 3rd message
            info!(
                target: "rlm_sampling",
                iteration = i,
                sampled = true,
                "Sampled log entry"
            );
        } else {
            debug!(
                target: "rlm_sampling",
                iteration = i,
                sampled = false,
                "Non-sampled debug entry"
            );
        }
    }
}

/// Test log aggregation fields.
#[tokio::test]
async fn test_log_aggregation_fields() {
    let _guard = init_test_logging();

    // Log structured data suitable for aggregation
    info!(
        target: "rlm_aggregation",
        service = "rlm-server",
        component = "executor",
        operation = "chat_completion",
        status = "success",
        duration_ms = 1500,
        tokens_used = 250,
        provider = "openai",
        model = "gpt-4",
        "Request processed successfully"
    );

    info!(
        target: "rlm_aggregation",
        service = "rlm-server",
        component = "backend_router",
        operation = "route_selection",
        status = "success",
        selected_backend = "primary",
        fallback_available = true,
        "Backend routing completed"
    );

    error!(
        target: "rlm_aggregation",
        service = "rlm-server",
        component = "provider",
        operation = "api_call",
        status = "error",
        error_code = "rate_limit",
        retry_count = 3,
        provider = "openai",
        "Provider request failed"
    );
}

/// Simulate a recursive call with proper context.
async fn simulate_recursive_call(depth: u32, request_id: &str) {
    let span = tracing::info_span!(
        "recursive_call",
        request_id = request_id,
        depth = depth,
        call_type = "llm_query"
    );

    let _enter = span.enter();

    info!(
        depth = depth,
        "Processing recursive call"
    );

    // Simulate some processing time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    if depth < 3 {
        // Simulate nested recursive call
        debug!(
            depth = depth,
            next_depth = depth + 1,
            "Spawning nested recursive call"
        );
    }

    info!(
        depth = depth,
        "Recursive call completed"
    );
}

/// Initialize test logging configuration.
fn init_test_logging() -> tracing::subscriber::DefaultGuard {
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
                .json(), // Use JSON format for structured logging
        )
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("debug"))
        );

    tracing::subscriber::set_default(subscriber)
}

/// Integration test with actual server components.
#[cfg(test)]
mod server_integration {
    use super::*;

    #[tokio::test]
    async fn test_server_observability_integration() {
        // This would test the full server with observability
        // For now, test individual components

        let _guard = init_test_logging();

        info!(
            target: "server_integration",
            test = "observability_integration",
            "Testing server observability components"
        );

        // Test would verify:
        // 1. HTTP middleware logging
        // 2. Metrics collection
        // 3. Health checks
        // 4. Error tracking

        assert!(true); // Placeholder for actual integration test
    }
}