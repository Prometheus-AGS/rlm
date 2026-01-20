//! Error types for the RLM testing framework.

use thiserror::Error;

/// Result type for testing operations.
pub type TestResult<T> = Result<T, TestError>;

/// Errors that can occur during testing operations.
#[derive(Error, Debug)]
pub enum TestError {
    /// Vault-related errors.
    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),

    /// HTTP client errors.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Serialization/deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO errors.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// RLM core errors.
    #[error("RLM error: {0}")]
    Rlm(#[from] rlm_core::RlmError),

    /// Test configuration errors.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Test execution errors.
    #[error("Test execution failed: {0}")]
    Execution(String),

    /// LLM provider errors.
    #[error("LLM provider error: {0}")]
    Provider(String),

    /// Coverage analysis errors.
    #[error("Coverage analysis failed: {0}")]
    Coverage(String),

    /// Environment setup errors.
    #[error("Environment error: {0}")]
    Environment(String),

    /// Authentication errors.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Test timeout errors.
    #[error("Test timed out after {timeout_ms}ms: {operation}")]
    Timeout {
        /// Operation that timed out.
        operation: String,
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Rate limiting errors.
    #[error("Rate limited: {message}. Retry after {retry_after_ms}ms")]
    RateLimited {
        /// Rate limit message.
        message: String,
        /// Retry delay in milliseconds.
        retry_after_ms: u64,
    },
}

/// Vault-specific errors.
#[derive(Error, Debug)]
pub enum VaultError {
    /// Connection to vault failed.
    #[error("Failed to connect to vault: {0}")]
    Connection(String),

    /// Authentication with vault failed.
    #[error("Vault authentication failed: {0}")]
    Authentication(String),

    /// Secret not found in vault.
    #[error("Secret not found: {key}")]
    SecretNotFound {
        /// The key that was not found.
        key: String,
    },

    /// Invalid vault configuration.
    #[error("Invalid vault configuration: {0}")]
    InvalidConfig(String),

    /// Vault operation failed.
    #[error("Vault operation failed: {0}")]
    Operation(String),

    /// Local secrets file error.
    #[error("Local secrets file error: {0}")]
    LocalSecretsFile(String),

    /// Environment variable not found.
    #[error("Required environment variable not found: {var}")]
    MissingEnvVar {
        /// The missing environment variable name.
        var: String,
    },

    /// Secrets rotation error.
    #[error("Secret rotation failed for key '{key}': {reason}")]
    RotationFailed {
        /// The key that failed to rotate.
        key: String,
        /// The reason for the failure.
        reason: String,
    },
}

impl TestError {
    /// Create a new configuration error.
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new execution error.
    pub fn execution<S: Into<String>>(msg: S) -> Self {
        Self::Execution(msg.into())
    }

    /// Create a new provider error.
    pub fn provider<S: Into<String>>(msg: S) -> Self {
        Self::Provider(msg.into())
    }

    /// Create a new coverage error.
    pub fn coverage<S: Into<String>>(msg: S) -> Self {
        Self::Coverage(msg.into())
    }

    /// Create a new environment error.
    pub fn environment<S: Into<String>>(msg: S) -> Self {
        Self::Environment(msg.into())
    }

    /// Create a new authentication error.
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        Self::Authentication(msg.into())
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            TestError::Http(_) |
            TestError::RateLimited { .. } |
            TestError::Timeout { .. } |
            TestError::Provider(_) if self.to_string().contains("rate limit") ||
                                     self.to_string().contains("timeout") ||
                                     self.to_string().contains("connection")
        )
    }

    /// Get the retry delay in milliseconds for retryable errors.
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            TestError::RateLimited { retry_after_ms, .. } => Some(*retry_after_ms),
            TestError::Timeout { .. } => Some(1000), // 1 second default
            TestError::Http(_) => Some(500), // 500ms for HTTP errors
            _ => None,
        }
    }
}

impl VaultError {
    /// Create a new connection error.
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a new authentication error.
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        Self::Authentication(msg.into())
    }

    /// Create a new secret not found error.
    pub fn secret_not_found<S: Into<String>>(key: S) -> Self {
        Self::SecretNotFound { key: key.into() }
    }

    /// Create a new invalid config error.
    pub fn invalid_config<S: Into<String>>(msg: S) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a new operation error.
    pub fn operation<S: Into<String>>(msg: S) -> Self {
        Self::Operation(msg.into())
    }

    /// Create a new local secrets file error.
    pub fn local_secrets_file<S: Into<String>>(msg: S) -> Self {
        Self::LocalSecretsFile(msg.into())
    }

    /// Create a new missing environment variable error.
    pub fn missing_env_var<S: Into<String>>(var: S) -> Self {
        Self::MissingEnvVar { var: var.into() }
    }

    /// Create a new rotation failed error.
    pub fn rotation_failed<S: Into<String>>(key: S, reason: S) -> Self {
        Self::RotationFailed {
            key: key.into(),
            reason: reason.into(),
        }
    }
}