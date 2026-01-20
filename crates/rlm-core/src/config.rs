//! Configuration for RLM execution.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// RLM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    /// REPL configuration.
    pub repl: ReplConfig,

    /// LLM configuration.
    pub llm: LlmConfig,

    /// Execution limits.
    pub limits: ExecutionLimits,

    /// Streaming configuration.
    pub streaming: StreamingConfig,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            repl: ReplConfig::default(),
            llm: LlmConfig::default(),
            limits: ExecutionLimits::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

/// REPL backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplConfig {
    /// Backend type ("rhai" or "python").
    pub backend: String,

    /// Enable sandboxing.
    pub sandbox: bool,

    /// Memory limit in MB.
    pub max_memory_mb: usize,

    /// Timeout per REPL operation.
    #[serde(with = "duration_millis")]
    pub operation_timeout: Duration,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            backend: "rhai".to_string(),
            sandbox: true,
            max_memory_mb: 512,
            operation_timeout: Duration::from_secs(30),
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider name ("openai", "anthropic", etc.).
    pub provider: String,

    /// Model name.
    pub model: String,

    /// Temperature (0.0 = deterministic, per paper).
    pub temperature: f32,

    /// Max tokens per response.
    pub max_tokens: u32,

    /// API key (or environment variable name).
    pub api_key: Option<String>,

    /// Base URL (optional).
    pub base_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4-turbo".to_string(),
            temperature: 0.0,
            max_tokens: 4096,
            api_key: None,
            base_url: None,
        }
    }
}

/// Execution limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLimits {
    /// Maximum REPL iterations (default: 50, per paper).
    pub max_iterations: u32,

    /// Maximum recursion depth (default: 1, per paper).
    pub max_recursion_depth: u32,

    /// Overall execution timeout.
    #[serde(with = "duration_millis")]
    pub timeout: Duration,

    /// Chunk size for context splitting (in tokens).
    pub chunk_size_tokens: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_recursion_depth: 1,
            timeout: Duration::from_secs(300),
            chunk_size_tokens: 4096,
        }
    }
}

/// Streaming configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Enable event streaming.
    pub enabled: bool,

    /// Buffer size for event channel.
    pub buffer_size: usize,

    /// Include REPL state in events.
    pub include_repl_state: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 100,
            include_repl_state: false,
        }
    }
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub(super) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let millis = u64::try_from(duration.as_millis())
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_u64(millis)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}
