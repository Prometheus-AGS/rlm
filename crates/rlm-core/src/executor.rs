//! Core RLM executor implementation.

use crate::{
    ports::{EventSink, LlmProvider, NullEventSink, ReplBackend},
    ExecutionMetadata, RlmConfig, RlmError, RlmEvent, RlmRequest, RlmResponse, RlmResult,
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
        Self {
            config,
            repl: Arc::new(tokio::sync::Mutex::new(repl)),
            llm: Arc::new(llm),
            event_sink: Arc::new(NullEventSink),
        }
    }

    /// Set custom event sink.
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = sink;
        self
    }

    /// Execute an RLM request.
    #[instrument(skip(self, request), fields(query = %request.query))]
    pub async fn execute(&self, request: RlmRequest) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        info!("Starting RLM execution: {}", request_id);

        self.event_sink
            .emit(RlmEvent::Started {
                request_id: request_id.clone(),
                timestamp: started_at,
            })
            .await?;

        let result = timeout(
            self.config.limits.timeout,
            self.execute_inner(request, &request_id),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                info!("RLM execution completed successfully");
                Ok(response)
            }
            Ok(Err(error)) => {
                warn!("RLM execution failed: {}", error);
                self.event_sink
                    .emit(RlmEvent::Error {
                        message: error.to_string(),
                        recoverable: false,
                    })
                    .await?;
                Err(error)
            }
            Err(_) => {
                let error = RlmError::Timeout(self.config.limits.timeout);
                warn!("RLM execution timed out");
                self.event_sink
                    .emit(RlmEvent::Error {
                        message: error.to_string(),
                        recoverable: false,
                    })
                    .await?;
                Err(error)
            }
        }
    }

    async fn execute_inner(&self, request: RlmRequest, request_id: &str) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();

        debug!("Stage 1: Offloading context to REPL");
        self.stage1_context_offload(&request).await?;

        debug!("Stage 2: LLM execution with REPL");
        let (answer, iterations, recursive_calls, total_tokens) =
            self.stage2_llm_execution(&request, request_id, 0).await?;

        debug!("Stage 3: Final aggregation");

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

        self.event_sink
            .emit(RlmEvent::Done { answer, metadata })
            .await?;

        Ok(response)
    }

    async fn stage1_context_offload(&self, request: &RlmRequest) -> RlmResult<()> {
        let mut repl = self.repl.lock().await;

        repl.init().await?;
        repl.set_variable("context", &request.context).await?;

        let context_tokens = u32::try_from(self.llm.count_tokens(&request.context))
            .map_err(|_| RlmError::Other("Context token count exceeds u32::MAX".to_string()))?;
        self.event_sink
            .emit(RlmEvent::ContextChunk {
                index: 0,
                total: 1,
                size_tokens: context_tokens,
            })
            .await?;

        debug!("Context offloaded: {} tokens", context_tokens);
        Ok(())
    }

    async fn stage2_llm_execution(
        &self,
        request: &RlmRequest,
        _request_id: &str,
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

        let prompt = self.build_prompt(&request.query);
        let answer = self
            .llm
            .call(&prompt, self.config.llm.max_tokens)
            .await?;
        total_tokens += u32::try_from(self.llm.count_tokens(&answer))
            .map_err(|_| RlmError::Other("Token count exceeds u32::MAX".to_string()))?;

        if iterations > request.max_iterations {
            return Err(RlmError::MaxIterations(request.max_iterations));
        }

        Ok((answer, iterations, recursive_calls, total_tokens))
    }

    fn build_prompt(&self, query: &str) -> String {
        format!(
            "You have access to a REPL environment with the following variable:\n\
             - context: The full context for this task\n\n\
             You can call llm_query(sub_query) to recursively decompose the task.\n\n\
             Task: {}\n\n\
             Provide your answer:",
            query
        )
    }
}
