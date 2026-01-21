//! Core RLM executor implementation.

use crate::{
    ports::{EventSink, LlmProvider, NullEventSink, ReplBackend},
    BackendRouter, CallStatus, ExecutionMetadata, OptimizedContextProcessor, ProgressTracker,
    ReplResult, RlmConfig, RlmError, RlmEvent, RlmEventData, RlmRequest, RlmResponse, RlmResult,
    SessionManager,
};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::time::timeout;
use tracing::{debug, info, instrument, warn};

/// Main RLM executor.
///
/// Implements the three-stage pipeline from the paper (Fig 2):
/// 1. Context offloading → REPL
/// 2. LLM execution with recursive calls
/// 3. Aggregation of results
pub struct RlmExecutor<R, L>
where
    R: ReplBackend,
    L: LlmProvider,
{
    config: RlmConfig,
    repl: Arc<tokio::sync::Mutex<R>>,
    llm: Arc<L>,
    event_sink: Arc<dyn EventSink>,
    session_manager: Arc<tokio::sync::Mutex<SessionManager>>,
    progress_tracker: Option<Arc<ProgressTracker>>,
    backend_router: Option<Arc<BackendRouter>>,
    context_processor: Option<Arc<OptimizedContextProcessor>>,
}

impl<R, L> std::fmt::Debug for RlmExecutor<R, L>
where
    R: ReplBackend,
    L: LlmProvider,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RlmExecutor")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<R, L> RlmExecutor<R, L>
where
    R: ReplBackend + 'static,
    L: LlmProvider + 'static,
{
    /// Create a new executor.
    pub fn new(config: RlmConfig, repl: R, llm: L) -> Self {
        let session_timeout = config.limits.timeout;
        Self {
            config,
            repl: Arc::new(tokio::sync::Mutex::new(repl)),
            llm: Arc::new(llm),
            event_sink: Arc::new(NullEventSink),
            session_manager: Arc::new(tokio::sync::Mutex::new(SessionManager::new(
                session_timeout,
            ))),
            progress_tracker: None,
            backend_router: None,
            context_processor: None,
        }
    }

    /// Set custom event sink.
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = sink;
        self
    }

    /// Set a progress tracker for detailed call monitoring.
    pub fn with_progress_tracker(mut self, tracker: Arc<ProgressTracker>) -> Self {
        self.progress_tracker = Some(tracker);
        self
    }

    /// Set a backend router for multi-provider support.
    pub fn with_backend_router(mut self, router: Arc<BackendRouter>) -> Self {
        self.backend_router = Some(router);
        self
    }

    /// Set an optimized context processor for large context handling.
    pub fn with_context_processor(mut self, processor: Arc<OptimizedContextProcessor>) -> Self {
        self.context_processor = Some(processor);
        self
    }

    /// Execute an RLM request.
    #[instrument(skip(self, request), fields(query = %request.query))]
    pub async fn execute(&self, request: RlmRequest) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        info!("Starting RLM execution: {}", request_id);

        // Create session for tracking
        let context_tokens = (request.context.len() / 4) as u32; // Rough token estimation
        let session_id = {
            let mut session_mgr = self.session_manager.lock().await;
            session_mgr.create_session(request_id.clone(), request.context.len(), context_tokens)
        };

        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.clone(),
                timestamp: started_at,
                data: RlmEventData::ContextChunk {
                    chunk_id: "initial".to_string(),
                    size_bytes: request.context.len(),
                    processed: false,
                },
            })
            .await?;

        let result = timeout(
            self.config.limits.timeout,
            self.execute_inner(request, &request_id, &session_id),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                info!("RLM execution completed successfully");
                Ok(response)
            }
            Ok(Err(error)) => {
                warn!("RLM execution failed: {}", error);

                // Mark session as failed
                {
                    let mut session_mgr = self.session_manager.lock().await;
                    if let Some(session) = session_mgr.get_session_mut(&session_id) {
                        session.mark_failed(error.to_string());
                    }
                }

                self.event_sink
                    .emit(RlmEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        request_id: request_id.to_string(),
                        timestamp: SystemTime::now(),
                        data: RlmEventData::Error {
                            error_type: "execution_error".to_string(),
                            message: error.to_string(),
                            recoverable: false,
                        },
                    })
                    .await?;
                Err(error)
            }
            Err(_) => {
                let error = RlmError::Timeout(self.config.limits.timeout);
                warn!("RLM execution timed out");

                // Mark session as failed due to timeout
                {
                    let mut session_mgr = self.session_manager.lock().await;
                    if let Some(session) = session_mgr.get_session_mut(&session_id) {
                        session.mark_failed(error.to_string());
                    }
                }

                self.event_sink
                    .emit(RlmEvent {
                        event_id: uuid::Uuid::new_v4().to_string(),
                        request_id: request_id.to_string(),
                        timestamp: SystemTime::now(),
                        data: RlmEventData::Error {
                            error_type: "execution_error".to_string(),
                            message: error.to_string(),
                            recoverable: false,
                        },
                    })
                    .await?;
                Err(error)
            }
        }
    }

    async fn execute_inner(
        &self,
        request: RlmRequest,
        request_id: &str,
        session_id: &str,
    ) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();

        debug!("Stage 1: Offloading context to REPL");
        self.stage1_context_offload(&request, request_id).await?;

        // Update session status to ContextLoaded
        {
            let mut session_mgr = self.session_manager.lock().await;
            if let Some(session) = session_mgr.get_session_mut(session_id) {
                session.update_status(crate::SessionStatus::ContextLoaded);
            }
        }

        debug!("Stage 2: LLM execution with REPL");

        // Update session status to Processing
        {
            let mut session_mgr = self.session_manager.lock().await;
            if let Some(session) = session_mgr.get_session_mut(session_id) {
                session.update_status(crate::SessionStatus::Processing);
            }
        }

        let (answer, iterations, recursive_calls, total_tokens) = self
            .stage2_llm_execution(&request, request_id, session_id, 0)
            .await?;

        debug!("Stage 3: Final aggregation");

        // Update session status to Aggregating
        {
            let mut session_mgr = self.session_manager.lock().await;
            if let Some(session) = session_mgr.get_session_mut(session_id) {
                session.update_status(crate::SessionStatus::Aggregating);
            }
        }

        let duration = started_at.elapsed().unwrap_or_default();
        let metadata = ExecutionMetadata {
            iterations,
            recursive_calls,
            total_tokens,
            duration,
            started_at,
            success: true,
        };

        let response = RlmResponse {
            answer: answer.clone(),
            metadata: metadata.clone(),
            repl_state: None,
        };

        // Update session status to Completed and record final metrics
        {
            let mut session_mgr = self.session_manager.lock().await;
            if let Some(session) = session_mgr.get_session_mut(session_id) {
                session.total_recursive_calls = recursive_calls;
                session.total_tokens_used = total_tokens;
                session.mark_completed();
            }
        }

        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::Done {
                    total_tokens,
                    total_duration_ms: duration.as_millis() as u64,
                },
            })
            .await?;

        Ok(response)
    }

    async fn stage1_context_offload(
        &self,
        request: &RlmRequest,
        request_id: &str,
    ) -> RlmResult<()> {
        let mut repl = self.repl.lock().await;

        // Reset REPL to clean state and emit event
        debug!("Resetting REPL to clean state");
        repl.reset().await?;

        // Emit REPL reset operation event
        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::ReplOp {
                    iteration: 0,
                    code: "reset()".to_string(),
                    result: ReplResult::Success {
                        value: "REPL reset successfully".to_string(),
                    },
                },
            })
            .await?;

        // Preprocess context if performance optimization is enabled
        if let Some(ref processor) = self.context_processor {
            debug!(
                "Using optimized context processing for {} bytes",
                request.context.len()
            );
            let processed = processor.process_context(&request.context).await?;

            // Emit processing performance metrics
            self.event_sink
                .emit(RlmEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    request_id: request_id.to_string(),
                    timestamp: SystemTime::now(),
                    data: RlmEventData::ContextChunk {
                        chunk_id: "performance-metrics".to_string(),
                        size_bytes: format!(
                            "Processing: {}ms, Cache hit: {}, Compression ratio: {:.2}",
                            processed.processing_time_ms,
                            processed.cache_hit,
                            processed.compression_ratio
                        )
                        .len(),
                        processed: true,
                    },
                })
                .await?;

            info!(
                "Context processing completed: {}ms, {} chunks, cache_hit={}, compression_ratio={:.2}",
                processed.processing_time_ms,
                processed.chunks.len(),
                processed.cache_hit,
                processed.compression_ratio
            );
        }

        // Set context variable and emit event
        debug!(
            "Setting context variable with {} bytes",
            request.context.len()
        );
        repl.set_variable("context", &request.context).await?;

        // Emit REPL set variable operation event
        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::ReplOp {
                    iteration: 0,
                    code: format!("context = <{} bytes of data>", request.context.len()),
                    result: ReplResult::Success {
                        value: "Context variable set successfully".to_string(),
                    },
                },
            })
            .await?;

        // For now, estimate token count based on character count (rough approximation)
        // TODO: Implement proper token counting in LLM provider trait
        let context_tokens = (request.context.len() / 4) as u32;

        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::ContextChunk {
                    chunk_id: "context-0".to_string(),
                    size_bytes: request.context.len(),
                    processed: true,
                },
            })
            .await?;

        debug!("Context offloaded: {} tokens", context_tokens);
        Ok(())
    }

    async fn stage2_llm_execution(
        &self,
        request: &RlmRequest,
        request_id: &str,
        _session_id: &str,
        depth: u32,
    ) -> RlmResult<(String, u32, u32, u32)> {
        if depth > request.recursion_depth {
            return Err(RlmError::MaxRecursion(request.recursion_depth));
        }

        if request.max_iterations == 0 {
            return Err(RlmError::MaxIterations(0));
        }

        let iterations = 1u32;
        let recursive_calls = 0u32;
        let mut total_tokens = 0u32;

        // Generate unique call ID for this LLM execution
        let call_id = uuid::Uuid::new_v4().to_string();

        // Create a chat completion request using the OpenAI-compatible interface
        let prompt = self.build_prompt(&request.query);
        let prompt_tokens = (prompt.len() / 4) as u32; // Rough estimate

        // Start tracking with ProgressTracker if available
        if let Some(ref tracker) = self.progress_tracker {
            tracker
                .start_call(
                    call_id.clone(),
                    None, // No parent for root calls - this could be enhanced later
                    depth,
                    prompt_tokens,
                )
                .await?;
        }

        let chat_request = crate::types::ChatCompletionRequest {
            model: self.config.llm.model.clone(),
            messages: vec![crate::types::ChatMessage {
                role: crate::types::ChatRole::User,
                content: prompt.clone(),
                name: None,
            }],
            max_tokens: Some(self.config.llm.max_tokens),
            temperature: Some(self.config.llm.temperature),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        // Emit recursive call initiated event
        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::RecursiveCall {
                    call_id: call_id.clone(),
                    depth,
                    status: CallStatus::InProgress,
                    prompt_tokens,
                    completion_tokens: 0,
                },
            })
            .await?;

        debug!("Executing LLM call at depth {}", depth);

        // Update progress tracker to show LLM call in progress
        if let Some(ref tracker) = self.progress_tracker {
            tracker
                .update_call_progress(&call_id, 50.0, "Executing LLM request".to_string())
                .await?;
        }

        let response = match self.execute_llm_call(&chat_request).await {
            Ok(response) => response,
            Err(e) => {
                // Mark call as failed in progress tracker
                if let Some(ref tracker) = self.progress_tracker {
                    if let Err(track_err) = tracker.fail_call(&call_id, e.to_string()).await {
                        warn!(
                            "Failed to update progress tracker for failed call: {}",
                            track_err
                        );
                    }
                }
                return Err(e);
            }
        };

        let answer = response
            .choices
            .first()
            .ok_or_else(|| RlmError::Other("No response from LLM".to_string()))?
            .message
            .content
            .clone();

        // Add token usage
        total_tokens += response.usage.total_tokens;

        // Complete call tracking with actual token usage
        if let Some(ref tracker) = self.progress_tracker {
            tracker
                .complete_call(&call_id, response.usage.completion_tokens)
                .await?;
        }

        // Emit recursive call completed event
        self.event_sink
            .emit(RlmEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.to_string(),
                timestamp: SystemTime::now(),
                data: RlmEventData::RecursiveCall {
                    call_id: call_id.clone(),
                    depth,
                    status: CallStatus::Completed,
                    prompt_tokens: response.usage.prompt_tokens,
                    completion_tokens: response.usage.completion_tokens,
                },
            })
            .await?;

        // Emit the response as chunks for streaming compatibility
        self.emit_response_chunks(&answer, request_id).await?;

        if iterations > request.max_iterations {
            return Err(RlmError::MaxIterations(request.max_iterations));
        }

        Ok((answer, iterations, recursive_calls, total_tokens))
    }

    /// Emit response content as chunks for streaming compatibility.
    async fn emit_response_chunks(&self, content: &str, request_id: &str) -> RlmResult<()> {
        // Split response into chunks for streaming
        let chunk_size = 100; // 100 characters per chunk
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for word in words {
            if current_chunk.len() + word.len() + 1 > chunk_size && !current_chunk.is_empty() {
                chunks.push(current_chunk.clone());
                current_chunk.clear();
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(word);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        // Emit each chunk as a streaming event
        for (i, chunk) in chunks.iter().enumerate() {
            let is_final = i == chunks.len() - 1;

            self.event_sink
                .emit(RlmEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    request_id: request_id.to_string(),
                    timestamp: SystemTime::now(),
                    data: RlmEventData::Chunk {
                        content: chunk.to_string(),
                        chunk_index: i as u32,
                        is_final,
                    },
                })
                .await?;

            debug!(
                "Emitted chunk {} of {}: '{}...'",
                i + 1,
                chunks.len(),
                &chunk.chars().take(20).collect::<String>()
            );
        }

        Ok(())
    }

    /// Execute an LLM call using the backend router if available, or the direct LLM provider otherwise.
    async fn execute_llm_call(
        &self,
        request: &crate::types::ChatCompletionRequest,
    ) -> RlmResult<crate::types::ChatCompletionResponse> {
        if let Some(ref router) = self.backend_router {
            debug!("Using backend router for LLM call");
            router.route_request(request).await
        } else {
            debug!("Using direct LLM provider for call");
            self.llm.complete(request).await
        }
    }

    fn build_prompt(&self, query: &str) -> String {
        format!(
            r#"You have access to a Rhai REPL environment with the following variables and functions:

## Available Variables
- `context`: The full context for this task (string)

## Rhai Syntax Examples

### String Operations
```rhai
let text = context;
let length = text.len();                    // Get string length
let upper = text.to_upper();                // Convert to uppercase
let contains = text.contains("needle");     // Check if contains substring
let lines = text.split('\n');               // Split into array
let slice = text.sub_string(0, 100);        // Get substring
```

### Array Operations
```rhai
let items = [1, 2, 3, 4, 5];
let sum = items.reduce(|a, b| a + b, 0);    // Sum all elements
let filtered = items.filter(|x| x > 2);      // Filter elements
let mapped = items.map(|x| x * 2);           // Transform elements
let first = items[0];                        // Access by index
```

### Recursive Decomposition
```rhai
// For complex tasks, decompose into sub-queries
let result = llm_query("Sub-question about part of the context");
```

## Your Task
{query}

Analyze the context and provide your answer. Use REPL operations when helpful for processing large data."#,
            query = query
        )
    }
}
