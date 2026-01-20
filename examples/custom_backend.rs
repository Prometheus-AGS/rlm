//! Custom backend integration guide and examples.
//!
//! This example demonstrates how to create custom REPL and LLM backends
//! for RLM, including:
//! - Implementing the ReplBackend trait
//! - Implementing the LlmProvider trait
//! - Custom event sinks for monitoring
//! - Integration patterns and best practices

use rlm_core::{
    ReplBackend, LlmProvider, EventSink, RlmEvent, RlmError, RlmResult,
    ReplResult, ChatCompletionRequest, ChatCompletionResponse, ChatChoice,
    ChatMessage, ChatRole, TokenUsage, ModelInfo, ProviderMetadata,
    ReplMetadata, SinkMetadata, SinkStats, EventFilter, EventSeverity,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tracing::{info, warn, debug, error};
use serde_json::Value;

/// Example custom REPL backend using JavaScript V8 engine.
///
/// This demonstrates how to integrate any scripting language or runtime
/// as a REPL backend for RLM.
#[derive(Debug)]
pub struct JavaScriptReplBackend {
    /// V8 isolate or runtime handle (mock for example).
    runtime: MockV8Runtime,
    /// Current execution context.
    context: HashMap<String, Value>,
    /// Execution statistics.
    stats: ReplStats,
}

/// Mock V8 runtime for demonstration.
#[derive(Debug)]
struct MockV8Runtime {
    globals: HashMap<String, Value>,
}

/// REPL execution statistics.
#[derive(Debug, Default)]
struct ReplStats {
    operations_count: u64,
    total_execution_time: Duration,
    memory_usage_bytes: usize,
    last_operation_time: Option<SystemTime>,
}

impl JavaScriptReplBackend {
    /// Create a new JavaScript REPL backend.
    pub fn new() -> RlmResult<Self> {
        info!("Initializing JavaScript REPL backend");

        let runtime = MockV8Runtime {
            globals: HashMap::new(),
        };

        Ok(Self {
            runtime,
            context: HashMap::new(),
            stats: ReplStats::default(),
        })
    }

    /// Execute JavaScript code with timeout and sandboxing.
    fn execute_js(&mut self, code: &str) -> RlmResult<String> {
        let start_time = SystemTime::now();

        // Simulate JavaScript execution
        let result = match code.trim() {
            "Math.PI" => "3.141592653589793".to_string(),
            "2 + 2" => "4".to_string(),
            "console.log('Hello')" => {
                // Side effects go to context
                self.context.insert("console_output".to_string(),
                    Value::String("Hello".to_string()));
                "undefined".to_string()
            }
            "throw new Error('test')" => {
                return Err(RlmError::Repl("JavaScript error: test".to_string()));
            }
            _ if code.starts_with("var ") || code.starts_with("let ") || code.starts_with("const ") => {
                // Variable declarations
                self.context.insert("last_declaration".to_string(),
                    Value::String(code.to_string()));
                "undefined".to_string()
            }
            _ => {
                // Default: echo the input as a string
                format!("\"{}\"", code)
            }
        };

        // Update statistics
        let execution_time = start_time.elapsed().unwrap_or(Duration::ZERO);
        self.stats.operations_count += 1;
        self.stats.total_execution_time += execution_time;
        self.stats.last_operation_time = Some(SystemTime::now());
        self.stats.memory_usage_bytes = self.context.len() * 64; // Mock calculation

        debug!("Executed JavaScript: {} -> {}", code, result);
        Ok(result)
    }
}

#[async_trait]
impl ReplBackend for JavaScriptReplBackend {
    async fn execute(&mut self, code: &str) -> RlmResult<ReplResult> {
        match self.execute_js(code) {
            Ok(output) => Ok(ReplResult::Success { value: output }),
            Err(e) => Ok(ReplResult::Error {
                error: e.to_string(),
                traceback: None,
            }),
        }
    }

    async fn reset(&mut self) -> RlmResult<()> {
        info!("Resetting JavaScript REPL backend");
        self.context.clear();
        self.runtime.globals.clear();
        Ok(())
    }

    async fn get_state(&self) -> RlmResult<HashMap<String, String>> {
        let mut state = HashMap::new();
        state.insert("runtime".to_string(), "v8".to_string());
        state.insert("context_size".to_string(), self.context.len().to_string());
        state.insert("operations_count".to_string(), self.stats.operations_count.to_string());
        state.insert("memory_usage_bytes".to_string(), self.stats.memory_usage_bytes.to_string());
        Ok(state)
    }

    fn metadata(&self) -> ReplMetadata {
        ReplMetadata {
            name: "JavaScript V8 REPL".to_string(),
            version: "1.0.0".to_string(),
            language: "javascript".to_string(),
            features: vec![
                "sandboxing".to_string(),
                "async_execution".to_string(),
                "es6_modules".to_string(),
                "console_output".to_string(),
            ],
            limitations: vec![
                "no_file_io".to_string(),
                "no_network_access".to_string(),
                "memory_limited".to_string(),
            ],
        }
    }
}

/// Example custom LLM provider for local models.
///
/// This demonstrates how to integrate local models, custom APIs,
/// or other LLM services with RLM.
#[derive(Debug)]
pub struct LocalLlamaProvider {
    /// Model configuration.
    config: LocalLlamaConfig,
    /// Request statistics.
    stats: LlmStats,
    /// Model information.
    model_info: ModelInfo,
}

/// Configuration for local Llama model.
#[derive(Debug, Clone)]
pub struct LocalLlamaConfig {
    /// Path to model weights.
    pub model_path: String,
    /// Maximum context length.
    pub max_context_length: usize,
    /// Temperature for sampling.
    pub temperature: f32,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// API endpoint for local server.
    pub endpoint: String,
}

/// LLM provider statistics.
#[derive(Debug, Default)]
struct LlmStats {
    requests_count: u64,
    total_tokens: u64,
    total_latency: Duration,
    error_count: u64,
}

impl LocalLlamaProvider {
    /// Create a new local Llama provider.
    pub fn new(config: LocalLlamaConfig) -> RlmResult<Self> {
        info!("Initializing Local Llama provider with model: {}", config.model_path);

        let model_info = ModelInfo {
            name: "llama-3.2-8b".to_string(),
            provider: "local".to_string(),
            max_tokens: config.max_context_length as u32,
            supports_streaming: true,
            supports_functions: false,
            cost_per_token: None, // Local models are free
        };

        Ok(Self {
            config,
            stats: LlmStats::default(),
            model_info,
        })
    }

    /// Generate response using local model.
    async fn generate_response(&mut self, request: &ChatCompletionRequest) -> RlmResult<String> {
        let start_time = SystemTime::now();

        // Simulate API call to local Llama server
        let prompt = self.build_prompt(&request.messages);

        debug!("Sending request to local Llama: {}", prompt);

        // Mock response based on input
        let response = match prompt.to_lowercase() {
            p if p.contains("hello") => "Hello! How can I help you today?",
            p if p.contains("math") || p.contains("calculate") => {
                "I can help with mathematical calculations. What would you like to compute?"
            }
            p if p.contains("code") || p.contains("programming") => {
                "I can assist with programming and code-related questions. What language are you working with?"
            }
            _ => "I understand your request. Let me think about this and provide a helpful response.",
        };

        // Update statistics
        let latency = start_time.elapsed().unwrap_or(Duration::ZERO);
        self.stats.requests_count += 1;
        self.stats.total_tokens += response.len() as u64;
        self.stats.total_latency += latency;

        Ok(response.to_string())
    }

    /// Build prompt from messages.
    fn build_prompt(&self, messages: &[ChatMessage]) -> String {
        let mut prompt = String::new();

        for message in messages {
            match message.role {
                ChatRole::System => prompt.push_str(&format!("System: {}\n", message.content)),
                ChatRole::User => prompt.push_str(&format!("Human: {}\n", message.content)),
                ChatRole::Assistant => prompt.push_str(&format!("Assistant: {}\n", message.content)),
                ChatRole::Function => prompt.push_str(&format!("Function: {}\n", message.content)),
            }
        }

        prompt.push_str("Assistant: ");
        prompt
    }
}

#[async_trait]
impl LlmProvider for LocalLlamaProvider {
    async fn complete(&mut self, request: ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        match self.generate_response(&request).await {
            Ok(content) => {
                let choice = ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: ChatRole::Assistant,
                        content,
                        name: None,
                    },
                    finish_reason: Some("stop".to_string()),
                };

                Ok(ChatCompletionResponse {
                    id: format!("local-{}", uuid::Uuid::new_v4()),
                    object: "chat.completion".to_string(),
                    created: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    model: self.model_info.name.clone(),
                    choices: vec![choice],
                    usage: Some(TokenUsage {
                        prompt_tokens: request.messages.iter()
                            .map(|m| m.content.len() / 4)
                            .sum::<usize>() as u32,
                        completion_tokens: 50, // Mock value
                        total_tokens: 0, // Will be calculated
                    }),
                })
            }
            Err(e) => {
                self.stats.error_count += 1;
                Err(e)
            }
        }
    }

    async fn stream(&mut self, _request: ChatCompletionRequest) -> RlmResult<Box<dyn futures::Stream<Item = RlmResult<String>> + Send + Unpin>> {
        // For this example, streaming is not implemented
        Err(RlmError::Other("Streaming not implemented for LocalLlamaProvider".to_string()))
    }

    fn model_info(&self) -> &ModelInfo {
        &self.model_info
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "Local Llama Provider".to_string(),
            version: "1.0.0".to_string(),
            supported_models: vec![
                "llama-3.2-8b".to_string(),
                "llama-3.2-3b".to_string(),
            ],
            features: vec![
                "local_execution".to_string(),
                "no_api_key_required".to_string(),
                "privacy_preserving".to_string(),
            ],
            limitations: vec![
                "requires_gpu".to_string(),
                "slower_than_api".to_string(),
                "no_function_calling".to_string(),
            ],
        }
    }
}

/// Custom event sink for advanced monitoring and alerting.
///
/// This demonstrates how to create sophisticated event processing
/// for monitoring, alerting, and analytics.
#[derive(Debug)]
pub struct AdvancedEventSink {
    /// Configuration for the sink.
    config: EventSinkConfig,
    /// Event buffer for batching.
    buffer: Vec<RlmEvent>,
    /// Statistics.
    stats: EventStats,
    /// Event filter.
    filter: EventFilter,
}

/// Configuration for advanced event sink.
#[derive(Debug, Clone)]
pub struct EventSinkConfig {
    /// Maximum buffer size before flushing.
    pub max_buffer_size: usize,
    /// Flush interval.
    pub flush_interval: Duration,
    /// Enable metrics collection.
    pub enable_metrics: bool,
    /// Enable alerting.
    pub enable_alerting: bool,
    /// Webhook URL for alerts.
    pub webhook_url: Option<String>,
}

/// Event processing statistics.
#[derive(Debug, Default)]
struct EventStats {
    events_processed: u64,
    events_filtered: u64,
    events_buffered: u64,
    alerts_sent: u64,
    last_flush: Option<SystemTime>,
}

impl AdvancedEventSink {
    /// Create a new advanced event sink.
    pub fn new(config: EventSinkConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
            stats: EventStats::default(),
            filter: EventFilter::default(),
        }
    }

    /// Process and analyze an event.
    async fn process_event(&mut self, event: &RlmEvent) -> RlmResult<()> {
        // Check if event should trigger an alert
        if self.config.enable_alerting {
            if let Some(alert) = self.check_for_alerts(event) {
                self.send_alert(alert).await?;
            }
        }

        // Update metrics
        if self.config.enable_metrics {
            self.update_metrics(event);
        }

        // Buffer event for batch processing
        self.buffer.push(event.clone());
        self.stats.events_buffered += 1;

        // Flush if buffer is full
        if self.buffer.len() >= self.config.max_buffer_size {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Check if event should trigger an alert.
    fn check_for_alerts(&self, event: &RlmEvent) -> Option<Alert> {
        match event {
            RlmEvent::Error { error, .. } => {
                Some(Alert {
                    level: AlertLevel::Error,
                    message: format!("RLM execution error: {}", error),
                    timestamp: SystemTime::now(),
                    metadata: HashMap::new(),
                })
            }
            RlmEvent::RecursiveCall { call_id, depth, .. } if depth > &5 => {
                Some(Alert {
                    level: AlertLevel::Warning,
                    message: format!("Deep recursion detected: {} at depth {}", call_id, depth),
                    timestamp: SystemTime::now(),
                    metadata: HashMap::new(),
                })
            }
            _ => None,
        }
    }

    /// Send alert to configured webhook.
    async fn send_alert(&mut self, alert: Alert) -> RlmResult<()> {
        if let Some(webhook_url) = &self.config.webhook_url {
            info!("Sending alert to webhook: {} - {}", webhook_url, alert.message);
            // In a real implementation, send HTTP request to webhook
            self.stats.alerts_sent += 1;
        }
        Ok(())
    }

    /// Update event metrics.
    fn update_metrics(&mut self, event: &RlmEvent) {
        // Update counters based on event type
        self.stats.events_processed += 1;

        // In a real implementation, update Prometheus metrics, etc.
        debug!("Updated metrics for event: {:?}", event);
    }

    /// Flush buffered events.
    async fn flush_buffer(&mut self) -> RlmResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        info!("Flushing {} events from buffer", self.buffer.len());

        // In a real implementation, send events to analytics system,
        // write to database, etc.
        for event in &self.buffer {
            debug!("Processing buffered event: {:?}", event);
        }

        self.buffer.clear();
        self.stats.last_flush = Some(SystemTime::now());
        Ok(())
    }
}

#[async_trait]
impl EventSink for AdvancedEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        // Apply filter
        if !self.filter.should_emit(&event) {
            return Ok(());
        }

        // For this example, just log the event
        // In a real implementation, you would process it properly
        debug!("Emitting event: {:?}", event);
        Ok(())
    }

    async fn emit_batch(&self, events: Vec<RlmEvent>) -> RlmResult<()> {
        for event in events {
            self.emit(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        // For this example, just log
        debug!("Flushing event sink");
        Ok(())
    }

    async fn close(&mut self) -> RlmResult<()> {
        self.flush().await?;
        info!("Closed advanced event sink");
        Ok(())
    }

    fn get_stats(&self) -> SinkStats {
        SinkStats {
            events_processed: self.stats.events_processed,
            events_dropped: self.stats.events_filtered,
            last_activity: self.stats.last_flush,
            is_healthy: true,
        }
    }

    fn get_metadata(&self) -> SinkMetadata {
        SinkMetadata {
            name: "Advanced Event Sink".to_string(),
            version: "1.0.0".to_string(),
            sink_type: "advanced_monitoring".to_string(),
            capabilities: vec![
                "batching".to_string(),
                "filtering".to_string(),
                "alerting".to_string(),
                "metrics".to_string(),
            ],
        }
    }

    async fn health_check(&self) -> RlmResult<bool> {
        // Check if sink is healthy
        Ok(self.buffer.len() < self.config.max_buffer_size * 2)
    }

    fn set_filter(&mut self, filter: EventFilter) {
        self.filter = filter;
    }
}

/// Alert structure for notifications.
#[derive(Debug)]
struct Alert {
    level: AlertLevel,
    message: String,
    timestamp: SystemTime,
    metadata: HashMap<String, String>,
}

/// Alert severity levels.
#[derive(Debug)]
enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Integration examples and patterns.
pub mod examples {
    use super::*;

    /// Example: Complete custom backend integration.
    pub async fn complete_custom_integration() -> RlmResult<()> {
        println!("=== Complete Custom Backend Integration ===");

        // Create custom REPL backend
        let mut repl = JavaScriptReplBackend::new()?;

        // Test REPL operations
        let result = repl.execute("2 + 2").await?;
        println!("REPL result: {:?}", result);

        // Get REPL metadata
        let repl_meta = repl.metadata();
        println!("REPL: {} v{} ({})", repl_meta.name, repl_meta.version, repl_meta.language);

        // Create custom LLM provider
        let mut llm = LocalLlamaProvider::new(LocalLlamaConfig {
            model_path: "/path/to/llama-3.2-8b.gguf".to_string(),
            max_context_length: 8192,
            temperature: 0.7,
            max_tokens: 512,
            endpoint: "http://localhost:8080/v1".to_string(),
        })?;

        // Test LLM completion
        let request = ChatCompletionRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "Hello, how are you?".to_string(),
                    name: None,
                }
            ],
            model: Some("llama-3.2-8b".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
        };

        let response = llm.complete(request).await?;
        println!("LLM response: {}", response.choices[0].message.content);

        // Get LLM metadata
        let llm_meta = llm.metadata();
        println!("LLM: {} v{}", llm_meta.name, llm_meta.version);

        // Create custom event sink
        let sink = AdvancedEventSink::new(EventSinkConfig {
            max_buffer_size: 100,
            flush_interval: Duration::from_secs(30),
            enable_metrics: true,
            enable_alerting: true,
            webhook_url: Some("https://alerts.example.com/webhook".to_string()),
        });

        // Test event emission
        let test_event = RlmEvent::ReplOp {
            session_id: "test".to_string(),
            request_id: "test".to_string(),
            iteration: 1,
            timestamp: SystemTime::now(),
            code: "console.log('test')".to_string(),
            result: ReplResult::Success { value: "undefined".to_string() },
        };

        sink.emit(test_event).await?;
        println!("Event emitted successfully");

        Ok(())
    }

    /// Example: Performance monitoring integration.
    pub async fn performance_monitoring_example() -> RlmResult<()> {
        println!("=== Performance Monitoring Integration ===");

        // Create monitoring event sink
        let mut sink = AdvancedEventSink::new(EventSinkConfig {
            max_buffer_size: 50,
            flush_interval: Duration::from_secs(10),
            enable_metrics: true,
            enable_alerting: true,
            webhook_url: None,
        });

        // Simulate various events for monitoring
        let events = vec![
            RlmEvent::ReplOp {
                session_id: "perf-test".to_string(),
                request_id: "req-1".to_string(),
                iteration: 1,
                timestamp: SystemTime::now(),
                code: "performance.now()".to_string(),
                result: ReplResult::Success { value: "1234.567".to_string() },
            },
            RlmEvent::RecursiveCall {
                session_id: "perf-test".to_string(),
                request_id: "req-1".to_string(),
                call_id: "recursive-1".to_string(),
                parent_id: None,
                depth: 1,
                timestamp: SystemTime::now(),
                query: "Analyze this data recursively".to_string(),
            },
            RlmEvent::Error {
                session_id: "perf-test".to_string(),
                request_id: "req-2".to_string(),
                timestamp: SystemTime::now(),
                error: "Connection timeout".to_string(),
            },
        ];

        for event in events {
            sink.emit(event).await?;
        }

        // Get sink statistics
        let stats = sink.get_stats();
        println!("Sink stats: {} events processed, {} dropped",
                 stats.events_processed, stats.events_dropped);

        // Health check
        let healthy = sink.health_check().await?;
        println!("Sink healthy: {}", healthy);

        Ok(())
    }

    /// Example: Multi-backend routing.
    pub async fn multi_backend_routing() -> RlmResult<()> {
        println!("=== Multi-Backend Routing Example ===");

        // Create multiple LLM providers
        let llm1 = LocalLlamaProvider::new(LocalLlamaConfig {
            model_path: "/path/to/small-model.gguf".to_string(),
            max_context_length: 4096,
            temperature: 0.3,
            max_tokens: 256,
            endpoint: "http://localhost:8080/v1".to_string(),
        })?;

        let llm2 = LocalLlamaProvider::new(LocalLlamaConfig {
            model_path: "/path/to/large-model.gguf".to_string(),
            max_context_length: 32768,
            temperature: 0.7,
            max_tokens: 1024,
            endpoint: "http://localhost:8081/v1".to_string(),
        })?;

        // Route based on request characteristics
        let simple_request = ChatCompletionRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "What is 2+2?".to_string(),
                    name: None,
                }
            ],
            model: Some("auto".to_string()),
            temperature: Some(0.3),
            max_tokens: Some(50),
            stream: Some(false),
        };

        // Route simple request to smaller model
        println!("Routing simple request to small model");
        println!("Model: {}", llm1.model_info().name);

        let complex_request = ChatCompletionRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "Analyze this complex dataset and provide insights...".to_string(),
                    name: None,
                }
            ],
            model: Some("auto".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: Some(false),
        };

        // Route complex request to larger model
        println!("Routing complex request to large model");
        println!("Model: {}", llm2.model_info().name);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> RlmResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Custom Backend Integration Guide");
    println!("===============================");

    // Run examples
    if let Err(e) = examples::complete_custom_integration().await {
        eprintln!("Custom integration example failed: {}", e);
    }

    if let Err(e) = examples::performance_monitoring_example().await {
        eprintln!("Performance monitoring example failed: {}", e);
    }

    if let Err(e) = examples::multi_backend_routing().await {
        eprintln!("Multi-backend routing example failed: {}", e);
    }

    println!("Custom backend examples completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_javascript_repl_backend() {
        let mut repl = JavaScriptReplBackend::new().unwrap();

        // Test basic execution
        let result = repl.execute("2 + 2").await.unwrap();
        assert!(matches!(result, ReplResult::Success { .. }));

        // Test error handling
        let result = repl.execute("throw new Error('test')").await.unwrap();
        assert!(matches!(result, ReplResult::Error { .. }));

        // Test state management
        let state = repl.get_state().await.unwrap();
        assert!(state.contains_key("operations_count"));
    }

    #[tokio::test]
    async fn test_local_llama_provider() {
        let mut provider = LocalLlamaProvider::new(LocalLlamaConfig {
            model_path: "test-model.gguf".to_string(),
            max_context_length: 4096,
            temperature: 0.7,
            max_tokens: 256,
            endpoint: "http://localhost:8080".to_string(),
        }).unwrap();

        let request = ChatCompletionRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                    name: None,
                }
            ],
            model: Some("llama-3.2-8b".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
        };

        let response = provider.complete(request).await.unwrap();
        assert!(!response.choices.is_empty());
        assert!(!response.choices[0].message.content.is_empty());
    }

    #[test]
    fn test_advanced_event_sink_creation() {
        let sink = AdvancedEventSink::new(EventSinkConfig {
            max_buffer_size: 100,
            flush_interval: Duration::from_secs(30),
            enable_metrics: true,
            enable_alerting: false,
            webhook_url: None,
        });

        let metadata = sink.get_metadata();
        assert_eq!(metadata.name, "Advanced Event Sink");
        assert!(metadata.capabilities.contains(&"batching".to_string()));
    }

    #[tokio::test]
    async fn test_event_sink_health_check() {
        let sink = AdvancedEventSink::new(EventSinkConfig {
            max_buffer_size: 10,
            flush_interval: Duration::from_secs(5),
            enable_metrics: false,
            enable_alerting: false,
            webhook_url: None,
        });

        let healthy = sink.health_check().await.unwrap();
        assert!(healthy);
    }
}