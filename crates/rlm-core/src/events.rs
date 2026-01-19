//! Event types for RLM streaming.

use crate::types::ReplResult;
use crate::ExecutionMetadata;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// RLM execution event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RlmEvent {
    /// Execution started.
    Started {
        /// Request ID.
        request_id: String,
        /// Start time.
        timestamp: SystemTime,
    },

    /// REPL operation executed.
    ReplOp {
        /// Iteration number.
        iteration: u32,
        /// Code executed.
        code: String,
        /// Execution result.
        result: ReplResult,
        /// Timestamp.
        timestamp: SystemTime,
    },

    /// Recursive LLM call initiated.
    RecursiveCall {
        /// Call depth.
        depth: u32,
        /// Sub-query.
        query: String,
        /// Parent call ID.
        parent_id: Option<String>,
        /// This call's ID.
        call_id: String,
    },

    /// Recursive call completed.
    RecursiveResult {
        /// Call ID.
        call_id: String,
        /// Result value.
        result: String,
        /// Tokens used.
        tokens_used: u32,
    },

    /// LLM streaming chunk.
    Chunk {
        /// Content delta.
        content: String,
        /// Finish reason (if final chunk).
        finish_reason: Option<String>,
    },

    /// Context chunk processed.
    ContextChunk {
        /// Chunk index.
        index: usize,
        /// Total chunks.
        total: usize,
        /// Chunk size in tokens.
        size_tokens: u32,
    },

    /// Execution completed successfully.
    Done {
        /// Final answer.
        answer: String,
        /// Execution metadata.
        metadata: ExecutionMetadata,
    },

    /// Error occurred.
    Error {
        /// Error message.
        message: String,
        /// Whether error is recoverable.
        recoverable: bool,
    },
}

impl RlmEvent {
    /// Check if event is terminal (Done or Error).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Error { .. })
    }

    /// Get event type as string.
    #[must_use]
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Started { .. } => "started",
            Self::ReplOp { .. } => "repl_op",
            Self::RecursiveCall { .. } => "recursive_call",
            Self::RecursiveResult { .. } => "recursive_result",
            Self::Chunk { .. } => "chunk",
            Self::ContextChunk { .. } => "context_chunk",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
        }
    }
}
