//! REPL backend port definition.
//!
//! This module defines the `ReplBackend` trait that enables RLM to execute
//! code in different REPL environments while maintaining the core logic.

use crate::error::RlmResult;
use async_trait::async_trait;
use std::collections::HashMap;

/// Backend for executing code in a REPL environment.
///
/// This trait abstracts different REPL implementations (Rhai, Python, etc.)
/// allowing the core executor to remain agnostic to the execution environment.
///
/// # Design Notes
///
/// - All operations are async to support I/O-bound REPL environments
/// - State management is implementation-specific
/// - Error handling follows the standard `RlmResult` pattern
/// - Implementations should be thread-safe (`Send + Sync`)
///
/// # Example
///
/// ```rust,no_run
/// use rlm_core::ports::ReplBackend;
/// use rlm_core::error::RlmResult;
///
/// async fn execute_calculation<B: ReplBackend>(backend: &mut B) -> RlmResult<String> {
///     backend.execute("let result = 2 + 2; result").await
/// }
/// ```
#[async_trait]
pub trait ReplBackend: Send + Sync + std::fmt::Debug {
    /// Execute code in the REPL environment.
    ///
    /// # Arguments
    ///
    /// * `code` - The code string to execute
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The execution result as a string
    /// * `Err(RlmError)` - Execution failed with details
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The code contains syntax errors
    /// - Runtime execution fails
    /// - The REPL environment is in an invalid state
    async fn execute(&mut self, code: &str) -> RlmResult<String>;

    /// Reset the REPL environment to a clean state.
    ///
    /// This clears all variables and state, returning the REPL to its
    /// initial configuration. Useful between different RLM executions.
    ///
    /// # Errors
    ///
    /// Returns an error if the REPL cannot be reset to a clean state.
    async fn reset(&mut self) -> RlmResult<()>;

    /// Get the current state/variables from the REPL.
    ///
    /// Returns a map of variable names to their string representations.
    /// This is primarily used for debugging and state inspection.
    ///
    /// # Returns
    ///
    /// A HashMap where keys are variable names and values are their
    /// string representations in the REPL environment.
    async fn get_state(&self) -> RlmResult<HashMap<String, String>>;

    /// Set a variable in the REPL environment.
    ///
    /// This allows the core executor to inject values into the REPL
    /// before code execution.
    ///
    /// # Arguments
    ///
    /// * `name` - Variable name
    /// * `value` - Variable value as a string (REPL will parse appropriately)
    ///
    /// # Errors
    ///
    /// Returns an error if the variable cannot be set due to naming
    /// conflicts or invalid values.
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()>;

    /// Check if the REPL environment is healthy and ready for execution.
    ///
    /// This is used for health checks and ensuring the REPL is in a
    /// valid state before critical operations.
    async fn health_check(&self) -> RlmResult<bool>;

    /// Get metadata about the REPL backend.
    ///
    /// Returns information about the backend type, version, capabilities, etc.
    /// Useful for logging, debugging, and feature detection.
    fn get_metadata(&self) -> ReplMetadata;
}

/// Metadata about a REPL backend implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplMetadata {
    /// Backend type identifier (e.g., "rhai", "python", "javascript")
    pub backend_type: String,
    /// Backend version string
    pub version: String,
    /// List of supported features/capabilities
    pub capabilities: Vec<String>,
    /// Maximum code length supported (if any)
    pub max_code_length: Option<usize>,
}

impl ReplMetadata {
    /// Create new REPL metadata.
    pub fn new(
        backend_type: impl Into<String>,
        version: impl Into<String>,
        capabilities: Vec<String>,
        max_code_length: Option<usize>,
    ) -> Self {
        Self {
            backend_type: backend_type.into(),
            version: version.into(),
            capabilities,
            max_code_length,
        }
    }

    /// Check if the backend supports a specific capability.
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_metadata() {
        let metadata = ReplMetadata::new(
            "test",
            "1.0.0",
            vec!["basic_execution".to_string(), "state_management".to_string()],
            Some(10000),
        );

        assert_eq!(metadata.backend_type, "test");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.supports("basic_execution"));
        assert!(!metadata.supports("advanced_features"));
        assert_eq!(metadata.max_code_length, Some(10000));
    }
}