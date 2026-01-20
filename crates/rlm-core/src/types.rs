//! Core types for RLM execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Request to execute RLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmRequest {
    /// User query to answer.
    pub query: String,

    /// Long context to offload to REPL.
    pub context: String,

    /// Maximum REPL iterations (default: 50, per paper).
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,

    /// Maximum recursive call depth (default: 1, per paper).
    #[serde(default = "default_recursion_depth")]
    pub recursion_depth: u32,

    /// Optional metadata for tracking.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_max_iterations() -> u32 {
    50
}

fn default_recursion_depth() -> u32 {
    1
}

/// Response from RLM execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmResponse {
    /// Final answer to the query.
    pub answer: String,

    /// Execution metadata.
    pub metadata: ExecutionMetadata,

    /// REPL final state (optional, for debugging).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repl_state: Option<String>,
}

/// Metadata about execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Number of REPL iterations used.
    pub iterations: u32,

    /// Number of recursive LLM calls made.
    pub recursive_calls: u32,

    /// Total tokens consumed.
    pub total_tokens: u32,

    /// Execution duration.
    #[serde(with = "duration_serde")]
    pub duration: Duration,

    /// Start time.
    #[serde(with = "systemtime_serde")]
    pub started_at: SystemTime,

    /// Whether execution completed successfully.
    pub success: bool,
}

/// Result from a REPL operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReplResult {
    /// Execution succeeded.
    Success {
        /// Output value as string.
        value: String,
    },
    /// Execution failed.
    Error {
        /// Error message.
        message: String,
    },
}

impl ReplResult {
    /// Check if result is success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Get value or error message.
    #[must_use]
    pub fn value_or_error(&self) -> &str {
        match self {
            Self::Success { value } => value,
            Self::Error { message } => message,
        }
    }
}

/// Serde serialization/deserialization helpers for Duration types.
pub mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    /// Serialize Duration as milliseconds.
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = u64::try_from(duration.as_millis())
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(millis)
    }

    /// Deserialize Duration from milliseconds.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Serde serialization/deserialization helpers for SystemTime types.
pub mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// Serialize SystemTime as Unix timestamp seconds.
    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match time.duration_since(UNIX_EPOCH) {
            Ok(duration) => serializer.serialize_u64(duration.as_secs()),
            Err(error) => Err(serde::ser::Error::custom(format!(
                "system time before Unix epoch: {error}"
            ))),
        }
    }

    /// Deserialize SystemTime from Unix timestamp seconds.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

/// OpenAI-compatible chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// ID of the model to use.
    pub model: String,

    /// A list of messages comprising the conversation so far.
    pub messages: Vec<ChatMessage>,

    /// The maximum number of tokens to generate in the chat completion.
    pub max_tokens: Option<u32>,

    /// What sampling temperature to use, between 0 and 2.
    pub temperature: Option<f32>,

    /// Whether to stream back partial progress.
    #[serde(default)]
    pub stream: bool,

    /// Up to 4 sequences where the API will stop generating further tokens.
    pub stop: Option<Vec<String>>,

    /// An alternative to sampling with temperature.
    pub top_p: Option<f32>,

    /// RLM-specific configuration parameters.
    #[serde(default)]
    pub rlm_config: Option<RlmConfig>,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author.
    pub role: ChatRole,

    /// The contents of the message.
    pub content: String,

    /// An optional name for the participant.
    pub name: Option<String>,
}

/// Role of a message author.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// Messages from the system that set behavior.
    System,
    /// Messages from the human user.
    User,
    /// Messages from the AI assistant.
    Assistant,
}

/// OpenAI-compatible chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// A unique identifier for the chat completion.
    pub id: String,

    /// The object type (always "chat.completion").
    pub object: String,

    /// The Unix timestamp (in seconds) of when the chat completion was created.
    pub created: u64,

    /// The model used for the chat completion.
    pub model: String,

    /// A list of chat completion choices.
    pub choices: Vec<ChatChoice>,

    /// Usage statistics for the completion request.
    pub usage: TokenUsage,
}

/// A single choice in a chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    /// The index of the choice in the list of choices.
    pub index: u32,

    /// A chat completion message generated by the model.
    pub message: ChatMessage,

    /// The reason the model stopped generating tokens.
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,

    /// Number of tokens in the generated completion.
    pub completion_tokens: u32,

    /// Total number of tokens used in the request.
    pub total_tokens: u32,

    /// RLM-specific: Number of recursive calls made.
    #[serde(default)]
    pub recursive_calls: u32,
}

/// RLM-specific configuration for advanced processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    /// Maximum depth of recursive calls.
    pub max_recursive_depth: Option<u32>,

    /// Size of context chunks for processing.
    pub context_chunk_size: Option<u32>,

    /// Whether to enable progress events in streaming.
    #[serde(default = "default_true")]
    pub enable_progress_events: bool,

    /// Aggregation strategy for combining results.
    pub aggregation_strategy: Option<AggregationStrategy>,
}

/// Strategy for aggregating results from recursive calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationStrategy {
    /// Execute recursive calls in parallel.
    Parallel,
    /// Execute recursive calls sequentially.
    Sequential,
}


/// Status of a recursive call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallStatus {
    /// Call is pending execution.
    Pending,
    /// Call is in progress.
    InProgress,
    /// Call has started.
    Started,
    /// Call completed successfully.
    Completed,
    /// Call failed with error.
    Failed,
}

fn default_true() -> bool {
    true
}

/// Events emitted during RLM processing for real-time monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmEvent {
    /// Event identifier.
    pub event_id: String,

    /// Associated request ID for correlation.
    pub request_id: String,

    /// Event timestamp.
    #[serde(with = "systemtime_serde")]
    pub timestamp: SystemTime,

    /// Event data.
    pub data: RlmEventData,
}

/// Different types of events that can occur during RLM processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RlmEventData {
    /// REPL code execution result.
    ReplOp {
        /// Iteration number.
        iteration: u32,
        /// Code that was executed.
        code: String,
        /// Execution result.
        result: ReplResult,
    },
    /// Recursive LLM call initiated.
    RecursiveCall {
        /// Unique call identifier.
        call_id: String,
        /// Recursion depth.
        depth: u32,
        /// Status of the call.
        status: CallStatus,
        /// Prompt tokens used.
        prompt_tokens: u32,
        /// Completion tokens generated.
        completion_tokens: u32,
    },
    /// Streaming response chunk.
    Chunk {
        /// Content of the chunk.
        content: String,
        /// Index of this chunk.
        chunk_index: u32,
        /// Whether this is the final chunk.
        is_final: bool,
    },
    /// Context processing chunk.
    ContextChunk {
        /// Chunk identifier.
        chunk_id: String,
        /// Size in bytes.
        size_bytes: usize,
        /// Whether processing is complete.
        processed: bool,
    },
    /// Processing completed successfully.
    Done {
        /// Total tokens consumed.
        total_tokens: u32,
        /// Total processing duration in milliseconds.
        total_duration_ms: u64,
    },
    /// Error occurred during processing.
    Error {
        /// Type of error.
        error_type: String,
        /// Error message.
        message: String,
        /// Whether the error is recoverable.
        recoverable: bool,
    },
}
