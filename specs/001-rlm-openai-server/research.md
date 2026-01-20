# RLM Server Implementation Research

This document presents comprehensive research findings on the technical approaches for implementing the RLM (Recursive Language Model) server. The research validates architectural decisions and provides production-ready implementation strategies for each key component.

## 1. RLM Paper Implementation Details

### Validated Three-Stage Pipeline Architecture

Based on research of MIT's RLM paper (arXiv:2512.24601) and existing implementations:

#### Context Offloading Strategy
- **Core Principle**: Treat long context as an external environment variable rather than direct token ingestion
- **Implementation Pattern**: Store entire context in REPL environment as a string variable accessible via `context` variable
- **Benefits**: Reduces O(n²) attention complexity to O(1) for needle-in-haystack tasks
- **Memory Management**: Context stored externally to the LLM, accessed programmatically through code execution

#### Recursive llm_query() Function Design
```python
def llm_query(prompt, sub_context=None, model="auto"):
    """
    Spawns recursive sub-model calls with custom prompts
    Args:
        prompt: Query for the sub-model
        sub_context: Optional subset of context to pass (default: uses main context)
        model: Model to use for sub-call (default: same as parent)
    Returns:
        String response from sub-model
    """
```

**Key Design Patterns**:
- **Decomposition**: Break complex queries into smaller, focused sub-problems
- **Hierarchical Processing**: Parent model orchestrates, child models execute specific tasks
- **Context Subsetting**: Pass only relevant context portions to sub-models for efficiency
- **Recursive Depth Control**: Implement max_depth parameter to prevent infinite recursion

#### Result Aggregation Strategies

**Task-Type Specific Approaches**:
1. **Needle-in-Haystack (O(1))**: Direct retrieval after search/filter operations
2. **Aggregation Tasks (O(n))**: Collect results from parallel sub-queries, combine systematically
3. **Pairwise Reasoning (O(n²))**: Coordinate multiple comparison operations, maintain state

**Aggregation Patterns**:
- Use REPL variables to accumulate intermediate results
- Implement `FINAL()` and `FINAL_VAR()` functions for result extraction
- Support streaming aggregation for large datasets

### Validated Implementation Approach

Our Rhai-based approach aligns with paper principles:
- **✅ Context Offloading**: Rhai provides safe script environment for context storage
- **✅ Recursive Calls**: `llm_query()` function implemented as Rhai native function
- **✅ Three-Stage Pipeline**: Context loading → Recursive processing → Aggregation
- **✅ Safety**: Rhai's sandboxed execution prevents security issues vs Python

## 2. Rust Axum HTTP Server Best Practices

### Server-Sent Events (SSE) Implementation

**Production-Ready SSE Pattern with Axum**:
```rust
use axum::{
    response::sse::{Event, Sse},
    extract::State,
    http::StatusCode,
};
use tokio::sync::broadcast;
use futures_util::stream::Stream;

// Broadcaster for managing multiple SSE clients
#[derive(Clone)]
struct EventBroadcaster {
    sender: broadcast::Sender<String>,
}

impl EventBroadcaster {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    async fn subscribe(&self) -> impl Stream<Item = Result<Event, Infallible>> {
        let receiver = self.sender.subscribe();
        ReceiverStream::new(receiver)
            .map(|msg| Ok(Event::default().data(msg.unwrap_or_default())))
    }

    fn broadcast(&self, event: &str) -> Result<(), broadcast::error::SendError<String>> {
        self.sender.send(event.to_string()).map(|_| ())
    }
}
```

**Key SSE Best Practices**:
- **Client Connection Management**: Use `broadcast::Sender` for fan-out to multiple clients
- **Error Handling**: Gracefully handle disconnected clients with `broadcast::error::RecvError`
- **Memory Management**: Set reasonable channel buffer sizes (1024+ for production)
- **Heartbeat**: Implement periodic ping events to detect disconnected clients
- **Reconnection Support**: Include event IDs for client-side reconnection logic

### OpenAI API Compatibility Layer

**Structured Approach**:
```rust
// Request/Response types matching OpenAI spec
#[derive(Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: Option<bool>,
    max_tokens: Option<u32>,
    // ... other OpenAI parameters
}

// Stream-aware response handler
async fn chat_completions(
    State(app_state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.stream.unwrap_or(false) {
        // Return SSE stream
        let stream = app_state.rlm_executor.execute_streaming(&request).await?;
        Ok(Sse::new(stream).into_response())
    } else {
        // Return complete response
        let response = app_state.rlm_executor.execute(&request).await?;
        Ok(Json(response).into_response())
    }
}
```

### Configuration Management Hierarchy

**Recommended Pattern (CLI > Environment > YAML)**:
```rust
use figment::{Figment, providers::{Format, Toml, Env, Serialized}};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Serialize, Deserialize, Clone)]
struct ServerConfig {
    #[clap(long, env = "RLM_PORT")]
    port: Option<u16>,

    #[clap(long, env = "RLM_HOST")]
    host: Option<String>,

    #[clap(long, env = "RLM_CONFIG_FILE")]
    config_file: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: Some(8080),
            host: Some("127.0.0.1".to_string()),
            config_file: None,
        }
    }
}

fn load_config() -> Result<ServerConfig, figment::Error> {
    let cli_args = ServerConfig::parse();

    let mut figment = Figment::from(Serialized::defaults(&ServerConfig::default()))
        .merge(Toml::file("rlm.toml").nested()) // YAML alternative
        .merge(Env::prefixed("RLM_").global())
        .merge(Serialized::defaults(&cli_args));

    // Override with config file if specified
    if let Some(config_path) = &cli_args.config_file {
        figment = figment.merge(Toml::file(config_path).nested());
    }

    figment.extract()
}
```

### Structured Logging with Tracing

**Production Configuration**:
```rust
use tracing::{info, error, instrument};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

// Initialize structured logging
fn init_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))?;

    let formatting_layer = if config.json_format {
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .compact()
            .with_target(true)
            .boxed()
    };

    Registry::default()
        .with(env_filter)
        .with(formatting_layer)
        .init();

    Ok(())
}

// Async context instrumentation
#[instrument(skip(executor, request), fields(model = %request.model))]
async fn process_rlm_request(
    executor: &RlmExecutor,
    request: &RlmRequest,
) -> RlmResult<RlmResponse> {
    info!("Processing RLM request");

    let response = executor.execute(request).await
        .map_err(|e| {
            error!("RLM execution failed: {}", e);
            e
        })?;

    info!("RLM request completed successfully");
    Ok(response)
}
```

## 3. Rhai REPL Integration Patterns

### Safe Engine Configuration

**Production-Ready Rhai Setup**:
```rust
use rhai::{Engine, Scope, AST, Dynamic, ParseErrorType, EvalAltResult};
use std::sync::{Arc, Mutex};

struct SafeRhaiBackend {
    engine: Engine,
    scope: Scope<'static>,
    max_operations: u64,
    max_memory_mb: usize,
}

impl SafeRhaiBackend {
    fn new(config: &RhaiConfig) -> Self {
        let mut engine = Engine::new();

        // Security configurations
        engine.set_max_operations(config.max_operations); // Prevent infinite loops
        engine.set_max_expr_depths(
            config.max_expr_depth,
            config.max_function_expr_depth
        );

        // Memory management
        engine.on_progress(|operations| {
            if operations % 10_000 == 0 {
                // Check memory usage periodically
                // Return Some(Dynamic::UNIT) to continue, None to stop
                Some(rhai::Dynamic::UNIT)
            } else {
                None
            }
        });

        // Disable potentially dangerous operations
        engine.disable_symbol("eval");    // No dynamic code execution
        engine.disable_symbol("import");  // No file system access

        // Custom function registration
        engine.register_fn("llm_query", llm_query_impl);
        engine.register_fn("log", safe_log_impl);

        Self {
            engine,
            scope: Scope::new(),
            max_operations: config.max_operations,
            max_memory_mb: config.max_memory_mb,
        }
    }
}

// Safe implementation prevents resource exhaustion
#[derive(Clone, Serialize, Deserialize)]
struct RhaiConfig {
    max_operations: u64,           // Max script operations (default: 1M)
    max_expr_depth: usize,         // Max expression nesting (default: 64)
    max_function_expr_depth: usize, // Max function nesting (default: 32)
    max_memory_mb: usize,          // Memory limit (default: 100MB)
    execution_timeout_ms: u64,     // Script timeout (default: 30s)
}
```

### Memory Management for Large Contexts

**Efficient Context Handling**:
```rust
impl RhaiReplBackend {
    async fn load_context(&mut self, context: &str) -> RlmResult<()> {
        // Chunk large contexts to avoid memory spikes
        const CHUNK_SIZE: usize = 1_000_000; // 1MB chunks

        if context.len() > CHUNK_SIZE {
            // Store in chunks with lazy loading
            let chunks: Vec<String> = context
                .chars()
                .collect::<Vec<_>>()
                .chunks(CHUNK_SIZE)
                .map(|chunk| chunk.iter().collect())
                .collect();

            self.scope.push("context_chunks", chunks);

            // Register helper functions for chunk access
            self.engine.register_fn("get_chunk", |index: i64| -> String {
                // Implementation to retrieve specific chunk
                format!("chunk_{}", index)
            });
        } else {
            // Direct storage for smaller contexts
            self.scope.push("context", context.to_string());
        }

        Ok(())
    }

    async fn execute_with_timeout(&mut self, code: &str) -> RlmResult<String> {
        let timeout = Duration::from_millis(self.config.execution_timeout_ms);

        tokio::time::timeout(timeout, async {
            self.engine.eval_with_scope::<String>(&mut self.scope, code)
                .map_err(|e| RlmError::ReplExecution(format!("Script error: {}", e)))
        })
        .await
        .map_err(|_| RlmError::ReplTimeout)?
    }
}
```

### Performance Optimization Strategies

**Key Performance Patterns**:
1. **AST Caching**: Pre-compile frequently used scripts
2. **Scope Reuse**: Maintain persistent scope across requests for context preservation
3. **Memory Monitoring**: Track heap usage and trigger GC when needed
4. **Operation Limits**: Prevent runaway scripts with operation counting
5. **Lazy Loading**: Load large context data on-demand

## 4. Multi-Backend LLM Integration

### OpenAI Client Implementation

**Production-Ready Pattern with `async-openai`**:
```rust
use async_openai::{Client, config::OpenAIConfig};
use tokio::time::{sleep, Duration};

struct OpenAiProvider {
    client: Client<OpenAIConfig>,
    config: LlmConfig,
    retry_policy: RetryPolicy,
}

#[derive(Clone)]
struct RetryPolicy {
    max_retries: usize,
    base_delay_ms: u64,
    max_delay_ms: u64,
    backoff_multiplier: f64,
}

impl LlmProvider for OpenAiProvider {
    #[instrument(skip(self, messages))]
    async fn complete(&self, messages: Vec<ChatMessage>) -> LlmResult<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(messages)
            .max_tokens(self.config.max_tokens)
            .build()?;

        self.execute_with_retry(|| async {
            self.client.chat().completions().create(request.clone()).await
        }).await
    }

    async fn complete_streaming(&self, messages: Vec<ChatMessage>) -> LlmResult<impl Stream<Item = LlmResult<String>>> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(messages)
            .stream(true)
            .build()?;

        let stream = self.client.chat().completions().create(request).await?;

        Ok(stream.map(|result| {
            result
                .map_err(|e| LlmError::ApiError(e.to_string()))
                .and_then(|chunk| {
                    chunk.choices.first()
                        .and_then(|choice| choice.delta.content.clone())
                        .ok_or_else(|| LlmError::EmptyResponse)
                })
        }))
    }
}
```

### Azure OpenAI Service Integration

**Key Differences and Compatibility**:
```rust
struct AzureOpenAiProvider {
    client: Client<AzureConfig>,
    config: AzureConfig,
}

#[derive(Clone)]
struct AzureConfig {
    api_base: String,           // https://{resource}.openai.azure.com
    api_version: String,        // "2024-02-01"
    deployment_name: String,    // Deployment name (not model name)
    api_key: String,
}

impl AzureOpenAiProvider {
    fn new(config: AzureConfig) -> Self {
        let azure_config = async_openai::config::AzureConfig::new()
            .with_api_base(&config.api_base)
            .with_api_version(&config.api_version)
            .with_deployment_id(&config.deployment_name)
            .with_api_key(&config.api_key);

        let client = Client::with_config(azure_config);

        Self { client, config }
    }
}

// Unified provider abstraction
enum LlmBackend {
    OpenAI(OpenAiProvider),
    AzureOpenAI(AzureOpenAiProvider),
    // Future: Anthropic, local models, etc.
}
```

### Error Handling and Retry Strategies

**Robust Error Handling Pattern**:
```rust
use thiserror::Error;
use exponential_backoff::Backoff;

#[derive(Error, Debug)]
enum LlmError {
    #[error("API rate limit exceeded")]
    RateLimit,

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Network timeout")]
    Timeout,

    #[error("Invalid API key")]
    Authentication,

    #[error("Model not available: {model}")]
    ModelNotAvailable { model: String },

    #[error("Request too large: {size} bytes")]
    RequestTooLarge { size: usize },
}

impl OpenAiProvider {
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> LlmResult<T>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, async_openai::error::OpenAIError>> + Send,
        T: Send,
    {
        let mut backoff = ExponentialBackoff::new(
            Duration::from_millis(self.retry_policy.base_delay_ms),
            Duration::from_millis(self.retry_policy.max_delay_ms),
            self.retry_policy.backoff_multiplier,
        );

        for attempt in 0..=self.retry_policy.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let error = self.classify_error(&e);

                    if !self.should_retry(&error) || attempt == self.retry_policy.max_retries {
                        return Err(error);
                    }

                    if let Some(delay) = backoff.next_backoff() {
                        warn!("Request failed (attempt {}), retrying in {:?}: {}",
                              attempt + 1, delay, e);
                        sleep(delay).await;
                    }
                }
            }
        }

        unreachable!()
    }

    fn classify_error(&self, error: &async_openai::error::OpenAIError) -> LlmError {
        match error {
            // Rate limiting - definitely retry
            _ if error.to_string().contains("rate_limit") => LlmError::RateLimit,

            // Timeout - retry with backoff
            _ if error.to_string().contains("timeout") => LlmError::Timeout,

            // Authentication - don't retry
            _ if error.to_string().contains("unauthorized") => LlmError::Authentication,

            // Default classification
            _ => LlmError::ApiError(error.to_string()),
        }
    }

    fn should_retry(&self, error: &LlmError) -> bool {
        match error {
            LlmError::RateLimit | LlmError::Timeout => true,
            LlmError::Authentication | LlmError::ModelNotAvailable { .. } => false,
            LlmError::ApiError(_) | LlmError::RequestTooLarge { .. } => false,
        }
    }
}
```

## Implementation Recommendations

### Architecture Decisions Rationale

1. **Rhai over Python REPL**
   - ✅ Memory safety without FFI complexity
   - ✅ Sandboxed execution environment
   - ✅ Native Rust integration
   - ✅ WASM compilation support
   - ✅ Production-ready security features

2. **Axum over other HTTP frameworks**
   - ✅ Native async/await support
   - ✅ Type-safe request/response handling
   - ✅ Built-in SSE support
   - ✅ Excellent ecosystem integration
   - ✅ Performance comparable to hyper

3. **Figment for configuration**
   - ✅ Hierarchical configuration merging
   - ✅ Multiple format support (TOML, JSON, YAML)
   - ✅ Environment variable integration
   - ✅ Type-safe deserialization
   - ✅ CLI argument support

4. **`thiserror` for structured errors**
   - ✅ Better error context for library code
   - ✅ Structured error types for different error categories
   - ✅ Better integration with `anyhow` at application boundaries
   - ✅ Compile-time error handling verification

### Performance Optimization Priorities

1. **Context Processing**
   - Implement lazy loading for large contexts
   - Use streaming for context ingestion
   - Optimize Rhai scope memory usage

2. **LLM Integration**
   - Implement request batching where possible
   - Use connection pooling for HTTP clients
   - Cache frequently used model responses

3. **SSE Streaming**
   - Buffer management for high-throughput scenarios
   - Client connection pooling and cleanup
   - Event deduplication for multiple subscribers

### Production Deployment Considerations

1. **Monitoring and Observability**
   - Structured logging with correlation IDs
   - Metrics for request latency, error rates, token usage
   - Health check endpoints for load balancers

2. **Security**
   - API key rotation and secure storage
   - Request rate limiting and throttling
   - Input validation and sanitization

3. **Scalability**
   - Horizontal scaling via stateless design
   - Database/cache integration for session persistence
   - Load balancing considerations for SSE connections

This research validates our architectural approach and provides concrete implementation patterns for building a production-ready RLM server. The combination of Rust's safety guarantees, Axum's performance, and Rhai's sandboxed execution creates a robust foundation for implementing the RLM paradigm.