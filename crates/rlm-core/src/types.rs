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

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = u64::try_from(duration.as_millis())
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(millis)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}
