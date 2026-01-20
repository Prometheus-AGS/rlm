//! Library integration example demonstrating how to use RLM as a library.
//!
//! This example shows how to integrate RLM into your own Rust application
//! as a library dependency, including:
//! - Setting up RLM configuration
//! - Creating and managing sessions
//! - Processing requests with streaming responses
//! - Handling errors and monitoring metrics
//! - Using custom backends

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

/// Example application demonstrating library integration.
#[derive(Debug)]
pub struct RlmApplication {
    executor: RlmExecutor,
    session_manager: SessionManager,
    config: RlmConfig,
}

impl RlmApplication {
    /// Create a new RLM application instance.
    pub async fn new() -> RlmResult<Self> {
        // Initialize tracing for logging
        tracing_subscriber::fmt::init();

        // Configure the RLM system
        let config = RlmConfig {
            max_context_length: 100_000,
            max_recursive_depth: 10,
            chunk_size: 8192,
            overlap_size: 512,
            timeout_seconds: 300,
            enable_streaming: true,
            debug_mode: false,
        };

        // Create REPL backend
        let repl_backend = Arc::new(tokio::sync::Mutex::new(
            RhaiReplBackend::new().map_err(|e| RlmError::Initialization(e.to_string()))?
        ));

        // Create LLM provider
        let llm_config = OpenAiConfig::builder()
            .api_key(std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo-key".to_string()))
            .model("gpt-4o".to_string())
            .temperature(0.7)
            .max_tokens(Some(4096))
            .build()?;

        let llm_provider = Arc::new(OpenAiProvider::new(llm_config));

        // Create executor
        let executor = RlmExecutor::new(
            repl_backend,
            llm_provider,
            config.clone(),
        )?;

        // Create session manager
        let session_manager = SessionManager::new();

        Ok(Self {
            executor,
            session_manager,
            config,
        })
    }

    /// Process a simple text query.
    pub async fn process_query(&self, query: &str) -> RlmResult<String> {
        info!("Processing query: {}", query);

        // Create a new session
        let session = self.session_manager.create_session().await?;
        let session_id = session.id().clone();

        // Create event sink for collecting results
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let event_sink = Arc::new(ChannelEventSink::new(event_tx));

        // Create request
        let request = RlmRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: query.to_string(),
                    name: None,
                }
            ],
            model: Some("gpt-4o".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(true),
            metadata: ExecutionMetadata {
                session_id: session_id.clone(),
                request_id: uuid::Uuid::new_v4().to_string(),
                timestamp: std::time::SystemTime::now(),
                user_id: None,
                tags: vec![],
            },
        };

        // Execute the request
        let response_future = self.executor.execute(request, event_sink);

        // Collect streaming results
        let mut final_content = String::new();
        let mut response_result = None;

        tokio::select! {
            response = response_future => {
                response_result = Some(response);
            }
            _ = async {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        RlmEvent::Chunk { content, .. } => {
                            final_content.push_str(&content);
                            debug!("Received chunk: {}", content);
                        }
                        RlmEvent::Done { session_id, .. } => {
                            info!("Session {} completed", session_id);
                            break;
                        }
                        RlmEvent::Error { error, .. } => {
                            error!("Error in session: {}", error);
                            return Err(RlmError::Execution(error));
                        }
                        RlmEvent::RecursiveCall { call_id, query, .. } => {
                            debug!("Recursive call {}: {}", call_id, query);
                        }
                        RlmEvent::ReplOp { code, result, .. } => {
                            debug!("REPL operation: {} -> {:?}", code, result);
                        }
                        RlmEvent::ContextChunk { chunk_id, .. } => {
                            debug!("Processing context chunk: {}", chunk_id);
                        }
                    }
                }
            } => {}
        }

        // Handle final response
        match response_result {
            Some(Ok(response)) => {
                if final_content.is_empty() {
                    // If no streaming content, use response content
                    if let Some(choice) = response.choices.first() {
                        Ok(choice.message.content.clone())
                    } else {
                        Err(RlmError::Execution("No response content".to_string()))
                    }
                } else {
                    Ok(final_content)
                }
            }
            Some(Err(e)) => Err(e),
            None => {
                if final_content.is_empty() {
                    Err(RlmError::Execution("No response received".to_string()))
                } else {
                    Ok(final_content)
                }
            }
        }
    }

    /// Process a complex request with context offloading.
    pub async fn process_with_context(&self, context: &str, query: &str) -> RlmResult<String> {
        info!("Processing query with context offloading");

        // Create session
        let session = self.session_manager.create_session().await?;

        // Create event sink
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let event_sink = Arc::new(ChannelEventSink::new(event_tx));

        // Create request with context message
        let request = RlmRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: format!(
                        "You have access to context data. Use the REPL to analyze it.\n\nContext:\n{}",
                        context
                    ),
                    name: None,
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: query.to_string(),
                    name: None,
                }
            ],
            model: Some("gpt-4o".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(true),
            metadata: ExecutionMetadata {
                session_id: session.id().clone(),
                request_id: uuid::Uuid::new_v4().to_string(),
                timestamp: std::time::SystemTime::now(),
                user_id: None,
                tags: vec!["context_offloading".to_string()],
            },
        };

        // Execute and collect results (similar to process_query)
        let response = self.executor.execute(request, event_sink).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(RlmError::Execution("No response content".to_string()))
        }
    }

    /// Get system metrics.
    pub fn get_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "active_sessions": self.session_manager.active_session_count(),
            "config": {
                "max_context_length": self.config.max_context_length,
                "max_recursive_depth": self.config.max_recursive_depth,
                "chunk_size": self.config.chunk_size,
                "timeout_seconds": self.config.timeout_seconds,
            }
        })
    }

    /// Shutdown the application gracefully.
    pub async fn shutdown(&self) -> RlmResult<()> {
        info!("Shutting down RLM application");
        self.session_manager.shutdown_all_sessions().await?;
        Ok(())
    }
}

/// Channel-based event sink for collecting events.
#[derive(Debug)]
struct ChannelEventSink {
    sender: mpsc::UnboundedSender<RlmEvent>,
}

impl ChannelEventSink {
    fn new(sender: mpsc::UnboundedSender<RlmEvent>) -> Self {
        Self { sender }
    }
}

#[async_trait::async_trait]
impl EventSink for ChannelEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        self.sender.send(event)
            .map_err(|e| RlmError::Event(format!("Failed to send event: {}", e)))?;
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        // Channel automatically flushes
        Ok(())
    }
}

/// Example usage scenarios.
pub mod examples {
    use super::*;

    /// Simple question answering example.
    pub async fn simple_qa_example() -> RlmResult<()> {
        println!("=== Simple Q&A Example ===");

        let app = RlmApplication::new().await?;

        let query = "What is the capital of France?";
        let response = app.process_query(query).await?;

        println!("Q: {}", query);
        println!("A: {}", response);

        Ok(())
    }

    /// Context offloading example with large document.
    pub async fn context_offloading_example() -> RlmResult<()> {
        println!("=== Context Offloading Example ===");

        let app = RlmApplication::new().await?;

        // Simulate a large document
        let context = include_str!("../README.md");
        let query = "Summarize the key features of this project";

        let response = app.process_with_context(context, query).await?;

        println!("Context: {} characters", context.len());
        println!("Query: {}", query);
        println!("Response: {}", response);

        Ok(())
    }

    /// Recursive reasoning example.
    pub async fn recursive_reasoning_example() -> RlmResult<()> {
        println!("=== Recursive Reasoning Example ===");

        let app = RlmApplication::new().await?;

        let query = r#"
        Analyze this dataset and find patterns:
        Data: [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]

        Use recursive analysis to:
        1. Identify the pattern
        2. Predict the next 3 values
        3. Explain the mathematical relationship
        "#;

        let response = app.process_query(query).await?;

        println!("Query: {}", query.trim());
        println!("Response: {}", response);

        Ok(())
    }

    /// Metrics monitoring example.
    pub async fn metrics_example() -> RlmResult<()> {
        println!("=== Metrics Example ===");

        let app = RlmApplication::new().await?;

        // Process a few queries
        let queries = vec![
            "What is 2+2?",
            "Explain quantum computing",
            "Write a haiku about programming",
        ];

        for query in queries {
            let _ = app.process_query(query).await?;
            println!("Processed: {}", query);
        }

        // Display metrics
        let metrics = app.get_metrics();
        println!("System Metrics: {}", serde_json::to_string_pretty(&metrics)?);

        Ok(())
    }

    /// Error handling example.
    pub async fn error_handling_example() -> RlmResult<()> {
        println!("=== Error Handling Example ===");

        let app = RlmApplication::new().await?;

        // Intentionally problematic query
        let query = "Execute: rm -rf /"; // This should be safely handled

        match app.process_query(query).await {
            Ok(response) => {
                println!("Response: {}", response);
            }
            Err(error) => {
                println!("Error handled gracefully: {}", error);
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> RlmResult<()> {
    println!("RLM Library Integration Examples");
    println!("===============================");

    // Run examples
    if let Err(e) = examples::simple_qa_example().await {
        eprintln!("Simple Q&A example failed: {}", e);
    }

    if let Err(e) = examples::context_offloading_example().await {
        eprintln!("Context offloading example failed: {}", e);
    }

    if let Err(e) = examples::recursive_reasoning_example().await {
        eprintln!("Recursive reasoning example failed: {}", e);
    }

    if let Err(e) = examples::metrics_example().await {
        eprintln!("Metrics example failed: {}", e);
    }

    if let Err(e) = examples::error_handling_example().await {
        eprintln!("Error handling example failed: {}", e);
    }

    println!("Examples completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_application_creation() {
        let result = RlmApplication::new().await;
        assert!(result.is_ok(), "Should create application successfully");
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let app = RlmApplication::new().await.unwrap();
        let metrics = app.get_metrics();

        assert!(metrics.is_object());
        assert!(metrics.get("active_sessions").is_some());
        assert!(metrics.get("config").is_some());
    }

    #[tokio::test]
    async fn test_channel_event_sink() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sink = ChannelEventSink::new(tx);

        let test_event = RlmEvent::Done {
            session_id: "test".to_string(),
            request_id: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            final_status: CallStatus::Success,
            total_tokens: 100,
        };

        sink.emit(test_event).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            RlmEvent::Done { session_id, .. } => {
                assert_eq!(session_id, "test");
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let app = RlmApplication::new().await.unwrap();
        let result = app.shutdown().await;
        assert!(result.is_ok(), "Should shutdown gracefully");
    }
}