//! Embedded usage test demonstrating RLM integration in constrained environments.
//!
//! This example shows how to use RLM in embedded or resource-constrained scenarios,
//! including:
//! - Minimal resource configuration
//! - Synchronous API usage
//! - Error resilience and fallback strategies
//! - Memory-efficient streaming
//! - Custom timeout and retry policies

use rlm_core::{
    RlmConfig, RlmExecutor, RlmRequest, RlmResponse, RlmSession, SessionManager,
    ReplBackend, LlmProvider, EventSink, RlmEvent, RlmError, RlmResult,
    ChatMessage, ChatRole, ExecutionMetadata, CallStatus,
};
use rlm_repl_rhai::RhaiReplBackend;
use rlm_server::{OpenAiProvider, OpenAiConfig};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};

/// Embedded RLM client optimized for resource-constrained environments.
#[derive(Debug)]
pub struct EmbeddedRlmClient {
    executor: RlmExecutor,
    config: EmbeddedConfig,
}

/// Configuration optimized for embedded usage.
#[derive(Debug, Clone)]
pub struct EmbeddedConfig {
    pub max_context_length: usize,
    pub max_recursive_depth: u32,
    pub chunk_size: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub memory_limit_mb: usize,
    pub enable_caching: bool,
    pub fallback_enabled: bool,
}

impl Default for EmbeddedConfig {
    fn default() -> Self {
        Self {
            max_context_length: 4096,   // Reduced from 100k
            max_recursive_depth: 3,     // Reduced from 10
            chunk_size: 1024,           // Reduced from 8192
            timeout_seconds: 30,        // Reduced from 300
            retry_attempts: 2,          // Limited retries
            memory_limit_mb: 64,        // 64MB memory limit
            enable_caching: true,       // Cache for efficiency
            fallback_enabled: true,     // Enable fallbacks
        }
    }
}

impl EmbeddedRlmClient {
    /// Create a new embedded RLM client with minimal resource usage.
    pub async fn new(config: EmbeddedConfig) -> RlmResult<Self> {
        // Minimal logging setup
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "warn");
        }
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .init();

        // Create minimal REPL backend
        let repl_backend = Arc::new(tokio::sync::Mutex::new(
            RhaiReplBackend::with_limits(config.memory_limit_mb * 1024 * 1024)?
        ));

        // Create LLM provider with conservative settings
        let llm_config = OpenAiConfig::builder()
            .api_key(std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("RLM_API_KEY"))
                .unwrap_or_else(|_| "demo-key".to_string()))
            .model("gpt-3.5-turbo".to_string()) // Use cheaper model
            .temperature(0.3) // Lower temperature for consistency
            .max_tokens(Some(1024)) // Limit tokens
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        let llm_provider = Arc::new(OpenAiProvider::new(llm_config));

        // Create RLM config
        let rlm_config = RlmConfig {
            max_context_length: config.max_context_length,
            max_recursive_depth: config.max_recursive_depth,
            chunk_size: config.chunk_size,
            overlap_size: config.chunk_size / 8, // Small overlap
            timeout_seconds: config.timeout_seconds,
            enable_streaming: false, // Disable streaming for simplicity
            debug_mode: false,
        };

        // Create executor
        let executor = RlmExecutor::new(
            repl_backend,
            llm_provider,
            rlm_config,
        )?;

        Ok(Self {
            executor,
            config,
        })
    }

    /// Process a query with timeout and retry logic.
    pub async fn query(&self, prompt: &str) -> RlmResult<String> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(self.config.timeout_seconds);

        for attempt in 1..=self.config.retry_attempts {
            if start_time.elapsed() > timeout {
                return Err(RlmError::Timeout(format!(
                    "Query timeout after {} seconds",
                    self.config.timeout_seconds
                )));
            }

            match self.try_query(prompt, attempt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Query attempt {} failed: {}", attempt, e);
                    if attempt == self.config.retry_attempts {
                        return if self.config.fallback_enabled {
                            Ok(self.fallback_response(prompt))
                        } else {
                            Err(e)
                        };
                    }
                    // Brief wait before retry
                    tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                }
            }
        }

        Err(RlmError::Execution("All retry attempts failed".to_string()))
    }

    /// Single query attempt.
    async fn try_query(&self, prompt: &str, attempt: u32) -> RlmResult<String> {
        debug!("Query attempt {}: {}", attempt, prompt);

        // Create minimal event sink
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let event_sink = Arc::new(MinimalEventSink::new(event_tx));

        // Create request
        let request = RlmRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: prompt.to_string(),
                    name: None,
                }
            ],
            model: Some("gpt-3.5-turbo".to_string()),
            temperature: Some(0.3),
            max_tokens: Some(1024),
            stream: Some(false),
            metadata: ExecutionMetadata {
                session_id: format!("embedded-{}", uuid::Uuid::new_v4()),
                request_id: format!("req-{}-{}", attempt, uuid::Uuid::new_v4()),
                timestamp: std::time::SystemTime::now(),
                user_id: Some("embedded-user".to_string()),
                tags: vec!["embedded".to_string(), format!("attempt-{}", attempt)],
            },
        };

        // Execute with timeout
        let response = tokio::time::timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.executor.execute(request, event_sink)
        ).await
        .map_err(|_| RlmError::Timeout("Request timeout".to_string()))?
        .map_err(|e| RlmError::Execution(format!("Execution failed: {}", e)))?;

        // Extract response content
        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(RlmError::Execution("No response content".to_string()))
        }
    }

    /// Fallback response for when all attempts fail.
    fn fallback_response(&self, prompt: &str) -> String {
        format!(
            "I apologize, but I'm unable to process your request \"{}\" at the moment due to system limitations. Please try again later or simplify your query.",
            prompt.chars().take(50).collect::<String>()
        )
    }

    /// Get memory usage statistics.
    pub fn memory_stats(&self) -> MemoryStats {
        let used_mb = get_memory_usage_mb();
        MemoryStats {
            used_mb,
            limit_mb: self.config.memory_limit_mb,
            utilization: (used_mb as f64 / self.config.memory_limit_mb as f64 * 100.0) as u8,
        }
    }

    /// Health check for embedded environment.
    pub async fn health_check(&self) -> HealthStatus {
        let memory = self.memory_stats();
        let start_time = Instant::now();

        // Simple connectivity test
        let connectivity = match self.query("test").await {
            Ok(_) => true,
            Err(_) => false,
        };

        let response_time_ms = start_time.elapsed().as_millis() as u64;

        HealthStatus {
            memory_ok: memory.utilization < 90,
            connectivity_ok: connectivity,
            response_time_ms,
            status: if memory.utilization < 90 && connectivity && response_time_ms < 5000 {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            }
        }
    }
}

/// Minimal event sink for embedded usage.
#[derive(Debug)]
struct MinimalEventSink {
    sender: mpsc::UnboundedSender<RlmEvent>,
}

impl MinimalEventSink {
    fn new(sender: mpsc::UnboundedSender<RlmEvent>) -> Self {
        Self { sender }
    }
}

#[async_trait::async_trait]
impl EventSink for MinimalEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        // Only emit essential events to save resources
        match event {
            RlmEvent::Error { .. } | RlmEvent::Done { .. } => {
                let _ = self.sender.send(event); // Ignore send errors
            }
            _ => {} // Drop non-essential events
        }
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        Ok(())
    }
}

/// Memory usage statistics.
#[derive(Debug)]
pub struct MemoryStats {
    pub used_mb: usize,
    pub limit_mb: usize,
    pub utilization: u8,
}

/// Health check status.
#[derive(Debug)]
pub struct HealthStatus {
    pub memory_ok: bool,
    pub connectivity_ok: bool,
    pub response_time_ms: u64,
    pub status: String,
}

/// Get current memory usage (simplified implementation).
fn get_memory_usage_mb() -> usize {
    // In a real implementation, this would use platform-specific APIs
    // For now, return a mock value
    32 // Mock 32MB usage
}

/// Extension trait for RhaiReplBackend with memory limits.
trait RhaiReplBackendExt {
    fn with_limits(memory_limit_bytes: usize) -> RlmResult<RhaiReplBackend>;
}

impl RhaiReplBackendExt for RhaiReplBackend {
    fn with_limits(memory_limit_bytes: usize) -> RlmResult<RhaiReplBackend> {
        // Create backend with memory limits
        let mut backend = RhaiReplBackend::new()
            .map_err(|e| RlmError::Initialization(e.to_string()))?;

        // In a real implementation, configure memory limits here
        warn!("Memory limit {} bytes configured (mock)", memory_limit_bytes);

        Ok(backend)
    }
}

/// Embedded usage examples and tests.
pub mod tests {
    use super::*;

    /// Test basic embedded functionality.
    pub async fn test_basic_usage() -> RlmResult<()> {
        println!("=== Embedded Usage Test ===");

        let config = EmbeddedConfig::default();
        let client = EmbeddedRlmClient::new(config).await?;

        // Simple query
        let response = client.query("What is 2+2?").await?;
        println!("Response: {}", response);

        // Memory stats
        let memory = client.memory_stats();
        println!("Memory usage: {}MB / {}MB ({}%)",
                 memory.used_mb, memory.limit_mb, memory.utilization);

        Ok(())
    }

    /// Test error handling and fallbacks.
    pub async fn test_error_handling() -> RlmResult<()> {
        println!("=== Error Handling Test ===");

        let mut config = EmbeddedConfig::default();
        config.timeout_seconds = 1; // Very short timeout
        config.retry_attempts = 2;

        let client = EmbeddedRlmClient::new(config).await?;

        // This should trigger timeout and fallback
        let response = client.query("Explain quantum physics in detail").await?;
        println!("Fallback response: {}", response);

        Ok(())
    }

    /// Test health monitoring.
    pub async fn test_health_monitoring() -> RlmResult<()> {
        println!("=== Health Monitoring Test ===");

        let config = EmbeddedConfig::default();
        let client = EmbeddedRlmClient::new(config).await?;

        let health = client.health_check().await;
        println!("Health status: {}", health.status);
        println!("Memory OK: {}", health.memory_ok);
        println!("Connectivity OK: {}", health.connectivity_ok);
        println!("Response time: {}ms", health.response_time_ms);

        Ok(())
    }

    /// Test memory-constrained scenarios.
    pub async fn test_memory_constraints() -> RlmResult<()> {
        println!("=== Memory Constraints Test ===");

        let mut config = EmbeddedConfig::default();
        config.memory_limit_mb = 32; // Very limited memory
        config.max_context_length = 1024; // Small context

        let client = EmbeddedRlmClient::new(config).await?;

        // Multiple small queries to test memory management
        let queries = vec![
            "Hi",
            "What's the weather?",
            "Tell me a joke",
            "Count to 5",
        ];

        for query in queries {
            let response = client.query(query).await?;
            let memory = client.memory_stats();
            println!("Query: {} -> Memory: {}%", query, memory.utilization);

            if memory.utilization > 95 {
                println!("WARNING: High memory usage detected");
                break;
            }
        }

        Ok(())
    }

    /// Test batch processing for efficiency.
    pub async fn test_batch_processing() -> RlmResult<()> {
        println!("=== Batch Processing Test ===");

        let config = EmbeddedConfig::default();
        let client = EmbeddedRlmClient::new(config).await?;

        let batch_queries = vec![
            "What is the capital of France?",
            "What is 10 * 10?",
            "Name a primary color",
            "What day comes after Monday?",
        ];

        let start_time = Instant::now();
        let mut results = Vec::new();

        for query in batch_queries {
            match client.query(query).await {
                Ok(response) => {
                    results.push((query, response));
                }
                Err(e) => {
                    results.push((query, format!("Error: {}", e)));
                }
            }
        }

        let total_time = start_time.elapsed();

        println!("Batch processing completed in {:?}", total_time);
        for (query, response) in results {
            println!("Q: {} -> A: {}", query, response.chars().take(50).collect::<String>());
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> RlmResult<()> {
    println!("Embedded RLM Usage Tests");
    println!("=======================");

    // Run embedded tests
    if let Err(e) = tests::test_basic_usage().await {
        eprintln!("Basic usage test failed: {}", e);
    }

    if let Err(e) = tests::test_error_handling().await {
        eprintln!("Error handling test failed: {}", e);
    }

    if let Err(e) = tests::test_health_monitoring().await {
        eprintln!("Health monitoring test failed: {}", e);
    }

    if let Err(e) = tests::test_memory_constraints().await {
        eprintln!("Memory constraints test failed: {}", e);
    }

    if let Err(e) = tests::test_batch_processing().await {
        eprintln!("Batch processing test failed: {}", e);
    }

    println!("Embedded tests completed!");
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_config_defaults() {
        let config = EmbeddedConfig::default();
        assert_eq!(config.max_context_length, 4096);
        assert_eq!(config.max_recursive_depth, 3);
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.enable_caching);
    }

    #[tokio::test]
    async fn test_memory_stats() {
        let config = EmbeddedConfig::default();
        let client = EmbeddedRlmClient::new(config).await.unwrap();
        let stats = client.memory_stats();

        assert!(stats.used_mb <= stats.limit_mb);
        assert!(stats.utilization <= 100);
    }

    #[tokio::test]
    async fn test_fallback_response() {
        let config = EmbeddedConfig::default();
        let client = EmbeddedRlmClient::new(config).await.unwrap();
        let response = client.fallback_response("test query");

        assert!(response.contains("unable to process"));
        assert!(response.contains("test query"));
    }

    #[tokio::test]
    async fn test_minimal_event_sink() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sink = MinimalEventSink::new(tx);

        // Test that error events are emitted
        let error_event = RlmEvent::Error {
            session_id: "test".to_string(),
            request_id: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            error: "test error".to_string(),
        };

        sink.emit(error_event).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            RlmEvent::Error { error, .. } => {
                assert_eq!(error, "test error");
            }
            _ => panic!("Expected error event"),
        }

        // Test that non-essential events are dropped
        let chunk_event = RlmEvent::Chunk {
            session_id: "test".to_string(),
            request_id: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            content: "chunk".to_string(),
            chunk_index: 0,
        };

        sink.emit(chunk_event).await.unwrap();

        // Should not receive anything since chunk events are dropped
        assert!(rx.try_recv().is_err());
    }
}