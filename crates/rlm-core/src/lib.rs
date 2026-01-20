//! RLM (Recursive Language Model) core implementation.
//!
//! This crate provides the core types, executor, and port definitions
//! for the RLM architecture described in the paper:
//! "RLM: A Recursive Language Model for Long Contexts" (arXiv:2410.01855).
//!
//! # Quick Start
//!
//! The easiest way to get started with RLM is using the high-level client API:
//!
//! ```rust,no_run
//! use rlm_core::{RlmExecutor, RlmConfig, RlmRequest};
//! use rlm_repl_rhai::RhaiReplBackend;
//! use rlm_server::OpenAiProvider;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create REPL backend
//!     let repl = Arc::new(tokio::sync::Mutex::new(RhaiReplBackend::new()?));
//!
//!     // Create LLM provider
//!     let llm = Arc::new(OpenAiProvider::new(
//!         rlm_server::OpenAiConfig::builder()
//!             .api_key(std::env::var("OPENAI_API_KEY")?)
//!             .build()?
//!     ));
//!
//!     // Create RLM executor with optimized configuration
//!     let config = RlmConfig::new();
//!     let executor = RlmExecutor::new(repl, llm, config)?;
//!
//!     // Create a request
//!     let request = RlmRequest {
//!         query: "Analyze this large dataset: [data...]".to_string(),
//!         context: Some("dataset context".to_string()),
//!         max_iterations: 10,
//!         recursion_depth: 3,
//!         // ... other fields
//!     };
//!
//!     // Process the request
//!     // let response = executor.execute(request, event_sink).await?;
//!     // println!("Response: {}", response.content);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! RLM uses a ports-and-adapters (hexagonal) architecture:
//! - **Core**: Execution logic, events, types (this crate)
//! - **Ports**: Traits for REPL, LLM, event sinks
//! - **Adapters**: Concrete implementations (other crates)
//!
//! ## Key Components
//!
//! ### High-Level Client API
//!
//! The [`RlmExecutor`] provides the core execution interface:
//!
//! ```rust,no_run
//! # use rlm_core::{RlmExecutor, RlmRequest, RlmConfig};
//! # use std::sync::Arc;
//! # async fn example(executor: RlmExecutor<Arc<tokio::sync::Mutex<dyn rlm_core::ReplBackend>>, Arc<dyn rlm_core::LlmProvider>>) -> rlm_core::RlmResult<()> {
//! // Create a request
//! let request = RlmRequest {
//!     query: "What is 2+2?".to_string(),
//!     context: None,
//!     max_iterations: 5,
//!     recursion_depth: 2,
//!     // ... other fields
//! };
//!
//! // Execute with event sink
//! // let response = executor.execute(request, event_sink).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Builder Pattern
//!
//! All major components use builder patterns for configuration:
//!
//! ```rust,no_run
//! use rlm_core::RlmConfig;
//!
//! // Configure RLM for different scenarios
//! let config = RlmConfig::new()
//!     .with_max_context_length(500_000)
//!     .with_recursion_depth(5);
//!
//! # Ok::<(), rlm_core::RlmError>(())
//! ```
//!
//! ### Context Processing
//!
//! RLM handles large contexts efficiently through intelligent chunking:
//!
//! ```rust,no_run
//! use rlm_core::{ContextAnalyzer, ContextAnalysisConfig};
//!
//! // Configure context analysis
//! let config = ContextAnalysisConfig::new()
//!     .with_max_chunk_size(16384)
//!     .with_semantic_chunking(true);
//!
//! let analyzer = ContextAnalyzer::new(config);
//! let chunks = analyzer.analyze_context("large document content...", None).await?;
//! # Ok::<(), rlm_core::RlmError>(())
//! ```
//!
//! ### Session Management
//!
//! Sessions maintain conversation state and enable recursive processing:
//!
//! ```rust,no_run
//! use rlm_core::{SessionManager, RlmSession};
//!
//! let session_manager = SessionManager::new();
//! let session = session_manager.create_session().await?;
//!
//! // Session tracks messages, tokens, and state
//! println!("Session ID: {}", session.id());
//! println!("Message count: {}", session.message_count());
//! println!("Total tokens: {}", session.total_tokens());
//! # Ok::<(), rlm_core::RlmError>(())
//! ```
//!
//! ## Backend Integration
//!
//! RLM supports multiple REPL and LLM backends through traits:
//!
//! ```rust,no_run
//! use rlm_core::{ReplBackend, LlmProvider, BackendConfig, ProviderType};
//!
//! // Configure different LLM providers
//! let openai_config = BackendConfig::new(
//!     ProviderType::OpenAI,
//!     "your-key".to_string(),
//!     "gpt-4o".to_string(),
//! );
//!
//! let anthropic_config = BackendConfig::new(
//!     ProviderType::Anthropic,
//!     "your-key".to_string(),
//!     "claude-3-5-sonnet-20241022".to_string(),
//! );
//! # Ok::<(), rlm_core::RlmError>(())
//! ```
//!
//! ## Error Handling
//!
//! RLM uses structured error types for comprehensive error handling:
//!
//! ```rust,no_run
//! use rlm_core::{RlmError, RlmResult};
//!
//! async fn handle_errors() {
//!     let result: RlmResult<()> = Ok(());
//!     match result {
//!         Ok(_) => println!("Success"),
//!         Err(RlmError::Timeout(msg)) => println!("Request timed out: {}", msg),
//!         Err(RlmError::RateLimited { retry_after, .. }) => {
//!             println!("Rate limited, retry in {}ms", retry_after);
//!         },
//!         Err(RlmError::ContextTooLarge { size, max_size }) => {
//!             println!("Context too large: {} > {}", size, max_size);
//!         },
//!         Err(e) => println!("Other error: {}", e),
//!     }
//! }
//! ```
//!
//! ## Performance and Monitoring
//!
//! RLM provides comprehensive metrics and monitoring:
//!
//! ```rust,no_run
//! use rlm_core::RlmMetrics;
//!
//! // Create metrics collector
//! let metrics = RlmMetrics::new();
//! println!("Requests processed: {}", metrics.total_requests());
//! println!("Average response time: {:?}", metrics.average_response_time());
//! println!("Token usage: {}", metrics.total_tokens());
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod backend_config;
pub mod backend_router;
pub mod builder;
pub mod config;
pub mod context_analyzer;
pub mod error;
pub mod executor;
pub mod log_context;
pub mod metrics;
pub mod performance;
pub mod ports;
pub mod progress;
pub mod recursive_call;
pub mod session;
pub mod token_tracker;
pub mod types;

pub use backend_config::{BackendConfig, BackendConfigError, ProviderType, RateLimits, RetryPolicy};
pub use backend_router::{BackendRouter, BackendHealth, RoutedBackend, RoutingStrategy};
pub use builder::{RlmConfigBuilder, BackendConfigBuilder, ContextAnalysisConfigBuilder};
pub use config::RlmConfig;
pub use context_analyzer::{
    ChunkingStrategy, ContentType, ContextAnalysis, ContextAnalysisConfig, ContextAnalyzer,
    ContextChunk,
};
pub use error::{RlmError, RlmResult};
pub use executor::RlmExecutor;
pub use log_context::{
    CallStatus as LogCallStatus, ContextSpan, LogContext, RecursiveCallContext, SessionContext,
    SessionStats, SessionStatus as LogSessionStatus,
};
pub use metrics::{
    CompletionRates, ContextProcessingMetrics, OperationTimer, RlmMetrics, RlmMetricsCollector,
    SessionMetrics,
};
pub use performance::{
    CachedContext, OptimizedContextProcessor, PerformanceConfig, PerformanceStats, ProcessedContext,
};
pub use ports::{
    EventFilter, EventSeverity, EventSink, LlmProvider, ModelInfo, NullEventSink,
    ProviderMetadata, ReplBackend, ReplMetadata, SinkMetadata, SinkStats,
};
pub use progress::{CallProgress, ProgressTracker, SessionProgress};
pub use recursive_call::{RecursiveCall, RecursiveCallTree, TreeStatistics};
pub use session::{RlmSession, SessionManager, SessionStatus};
pub use token_tracker::{
    CallTokenStatus, GlobalTokenStats, RecursiveCallTokens, SessionTokenUsage, TokenPricing,
    TokenTracker, TokenUsageSnapshot,
};
pub use types::{
    AggregationStrategy, CallStatus, ChatChoice, ChatCompletionRequest, ChatCompletionResponse,
    ChatMessage, ChatRole, ExecutionMetadata, ReplResult, RlmRequest, RlmResponse, RlmEvent, RlmEventData, TokenUsage,
};
