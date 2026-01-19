//! Error types for RLM.

use thiserror::Error;

/// RLM error type.
#[derive(Debug, Error)]
pub enum RlmError {
    /// REPL execution error.
    #[error("REPL error: {0}")]
    Repl(String),

    /// LLM provider error.
    #[error("LLM provider error: {0}")]
    Llm(String),

    /// Maximum iterations exceeded.
    #[error("Maximum iterations ({0}) exceeded")]
    MaxIterations(u32),

    /// Maximum recursion depth exceeded.
    #[error("Maximum recursion depth ({0}) exceeded")]
    MaxRecursion(u32),

    /// Timeout error.
    #[error("Execution timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error.
    #[error("{0}")]
    Other(String),
}

/// RLM result type.
pub type RlmResult<T> = Result<T, RlmError>;
