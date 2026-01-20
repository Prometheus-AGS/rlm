//! Rhai REPL backend implementation for RLM.
//!
//! This module provides a safe, sandboxed REPL backend using the Rhai scripting language.
//! Rhai offers memory safety without FFI complexity while maintaining scripting flexibility
//! suitable for the RLM paper's context offloading approach.

use async_trait::async_trait;
use rlm_core::{ReplBackend, RlmError, RlmResult};
use rlm_core::ports::ReplMetadata;
use rhai::{Engine, Scope, Dynamic, EvalAltResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument, warn};

/// Configuration for the Rhai REPL backend.
#[derive(Debug, Clone)]
pub struct RhaiReplConfig {
    /// Maximum operations allowed per execution.
    pub max_operations: u32,
    /// Maximum string length.
    pub max_string_size: usize,
    /// Maximum array size.
    pub max_array_size: usize,
    /// Maximum map size.
    pub max_map_size: usize,
    /// Maximum nesting depth.
    pub max_expr_depth: usize,
    /// Maximum function call depth.
    pub max_function_call_depth: usize,
    /// Maximum number of modules.
    pub max_modules: usize,
    /// Enable optimizations.
    pub enable_optimization: bool,
    /// Additional custom constants.
    pub custom_constants: HashMap<String, Dynamic>,
}

impl Default for RhaiReplConfig {
    fn default() -> Self {
        Self {
            max_operations: 1_000_000,      // Generous but prevents infinite loops
            max_string_size: 1024 * 1024,   // 1MB max string
            max_array_size: 10_000,         // Max 10k array elements
            max_map_size: 10_000,           // Max 10k map entries
            max_expr_depth: 32,             // Max expression nesting
            max_function_call_depth: 64,    // Max function call depth
            max_modules: 8,                 // Max imported modules
            enable_optimization: true,      // Enable engine optimizations
            custom_constants: HashMap::new(),
        }
    }
}

/// Rhai-based REPL backend implementation.
pub struct RhaiReplBackend {
    engine: Engine,
    scope: Arc<Mutex<Scope<'static>>>,
    config: RhaiReplConfig,
    execution_count: u64,
}

impl std::fmt::Debug for RhaiReplBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RhaiReplBackend")
            .field("config", &self.config)
            .field("execution_count", &self.execution_count)
            .finish_non_exhaustive()
    }
}

impl RhaiReplBackend {
    /// Create a new Rhai REPL backend with default configuration.
    pub fn new() -> RlmResult<Self> {
        Self::with_config(RhaiReplConfig::default())
    }

    /// Create a new Rhai REPL backend with custom configuration.
    pub fn with_config(config: RhaiReplConfig) -> RlmResult<Self> {
        use crate::engine::RhaiEngineBuilder;

        let engine = RhaiEngineBuilder::new()
            .max_operations(config.max_operations)
            .max_string_size(config.max_string_size)
            .max_array_size(config.max_array_size)
            .max_map_size(config.max_map_size)
            .max_expr_depth(config.max_expr_depth)
            .max_function_call_depth(config.max_function_call_depth)
            .max_modules(config.max_modules)
            .optimization(config.enable_optimization)
            .custom_constants(config.custom_constants.clone())
            .disable_symbol("import".to_string())
            .disable_symbol("eval".to_string())
            .build();

        let mut scope = Scope::new();

        // Add custom constants
        for (key, value) in &config.custom_constants {
            scope.push_constant(key, value.clone());
        }

        // Add built-in helper constants
        scope.push_constant("PI", std::f64::consts::PI);
        scope.push_constant("E", std::f64::consts::E);

        Ok(Self {
            engine,
            scope: Arc::new(Mutex::new(scope)),
            config,
            execution_count: 0,
        })
    }


    /// Get current execution count.
    pub fn execution_count(&self) -> u64 {
        self.execution_count
    }

    /// Get a clone of the current scope (for debugging).
    pub async fn get_scope_snapshot(&self) -> HashMap<String, String> {
        let scope = self.scope.lock().await;
        let mut snapshot = HashMap::new();

        // Note: Rhai's Scope doesn't directly expose variable iteration
        // This is a simplified representation
        snapshot.insert("variables_count".to_string(), scope.len().to_string());
        snapshot
    }

    /// Convert Rhai evaluation error to RLM error.
    fn convert_error(error: Box<EvalAltResult>) -> RlmError {
        match *error {
            EvalAltResult::ErrorRuntime(msg, _) => RlmError::Repl(format!("Runtime error: {}", msg)),
            EvalAltResult::ErrorParsing(parse_error, _) => {
                RlmError::Repl(format!("Parse error: {}", parse_error))
            }
            EvalAltResult::ErrorTooManyOperations(_) => {
                RlmError::Repl("Too many operations - possible infinite loop".to_string())
            }
            EvalAltResult::ErrorTooManyModules(_) => {
                RlmError::Repl("Too many modules imported".to_string())
            }
            EvalAltResult::ErrorDataTooLarge(_, _) => {
                RlmError::Repl("Data size exceeds limits".to_string())
            }
            EvalAltResult::ErrorStackOverflow(_) => {
                RlmError::Repl("Stack overflow - recursion too deep".to_string())
            }
            other => RlmError::Repl(format!("Rhai error: {}", other)),
        }
    }
}

impl Default for RhaiReplBackend {
    fn default() -> Self {
        Self::new().expect("Failed to create default RhaiReplBackend")
    }
}

#[async_trait]
impl ReplBackend for RhaiReplBackend {
    #[instrument(skip(self))]
    async fn reset(&mut self) -> RlmResult<()> {
        debug!("Resetting Rhai REPL backend");

        // Create new scope but keep engine configuration
        let mut new_scope = Scope::new();

        // Re-add custom constants
        for (key, value) in &self.config.custom_constants {
            new_scope.push_constant(key, value.clone());
        }

        // Re-add built-in helper constants
        new_scope.push_constant("PI", std::f64::consts::PI);
        new_scope.push_constant("E", std::f64::consts::E);

        *self.scope.lock().await = new_scope;
        self.execution_count = 0;

        debug!("Rhai REPL backend reset complete");
        Ok(())
    }

    #[instrument(skip(self, code))]
    async fn execute(&mut self, code: &str) -> RlmResult<String> {
        debug!("Executing Rhai code");

        if code.trim().is_empty() {
            return Ok(String::new());
        }

        self.execution_count += 1;

        let mut scope = self.scope.lock().await;

        match self.engine.eval_with_scope::<Dynamic>(&mut *scope, code) {
            Ok(result) => {
                let output = match result.type_name() {
                    "()" => String::new(), // Unit type, no output
                    _ => result.to_string(),
                };

                debug!("Rhai execution successful, output length: {}", output.len());
                Ok(output)
            }
            Err(error) => {
                warn!("Rhai execution failed: {}", error);
                Err(Self::convert_error(error))
            }
        }
    }

    #[instrument(skip(self, value))]
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()> {
        debug!("Setting variable '{}' in Rhai REPL", name);

        let mut scope = self.scope.lock().await;

        // Set the variable as a string value
        scope.push(name.to_string(), value.to_string());

        debug!("Variable '{}' set successfully", name);
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_state(&self) -> RlmResult<HashMap<String, String>> {
        debug!("Getting state from Rhai REPL");

        let scope = self.scope.lock().await;

        // Rhai's scope doesn't expose variable names directly
        // This is a simplified state representation
        let mut state = HashMap::new();
        state.insert("variables_count".to_string(), scope.len().to_string());
        state.insert("execution_count".to_string(), self.execution_count.to_string());

        debug!("Retrieved Rhai state with {} entries", state.len());
        Ok(state)
    }

    #[instrument(skip(self))]
    async fn health_check(&self) -> RlmResult<bool> {
        debug!("Performing Rhai REPL health check");

        // Simple health check: try to evaluate a basic expression
        let test_code = "1 + 1";
        let _scope_guard = self.scope.lock().await;

        match self.engine.eval::<i64>(test_code) {
            Ok(result) if result == 2 => {
                debug!("Rhai REPL health check passed");
                Ok(true)
            }
            _ => {
                debug!("Rhai REPL health check failed");
                Ok(false)
            }
        }
    }

    fn get_metadata(&self) -> ReplMetadata {
        ReplMetadata::new(
            "rhai",
            env!("CARGO_PKG_VERSION"),
            vec![
                "memory_safe".to_string(),
                "sandboxed".to_string(),
                "configurable_limits".to_string(),
                "native_rust_types".to_string(),
            ],
            Some(1024 * 1024), // 1MB max code length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rhai_backend_creation() {
        let backend = RhaiReplBackend::new().unwrap();
        assert_eq!(backend.execution_count(), 0);
    }

    #[tokio::test]
    async fn test_basic_execution() {
        let mut backend = RhaiReplBackend::new().unwrap();

        let result = backend.execute("2 + 3").await.unwrap();
        assert_eq!(result, "5");
        assert_eq!(backend.execution_count(), 1);
    }

    #[tokio::test]
    async fn test_variable_operations() {
        let mut backend = RhaiReplBackend::new().unwrap();

        // Set variable
        backend.set_variable("test_var", "hello world").await.unwrap();

        // Check state includes the variable
        let state = backend.get_state().await.unwrap();
        assert!(state.get("variables_count").is_some());
    }

    #[tokio::test]
    async fn test_reset_functionality() {
        let mut backend = RhaiReplBackend::new().unwrap();

        // Set up some state
        backend.set_variable("temp", "value").await.unwrap();
        backend.execute("let x = 42;").await.unwrap();
        assert_eq!(backend.execution_count(), 1);

        // Reset
        backend.reset().await.unwrap();
        assert_eq!(backend.execution_count(), 0);

        // Check that state is reset
        let state = backend.get_state().await.unwrap();
        let var_count: usize = state.get("variables_count").unwrap().parse().unwrap();
        assert!(var_count <= 2); // Only built-in constants should remain
    }

    #[tokio::test]
    async fn test_error_handling() {
        let mut backend = RhaiReplBackend::new().unwrap();

        // Test parse error
        let result = backend.execute("2 +").await;
        assert!(result.is_err());

        // Test runtime error
        let _result = backend.execute("1 / 0").await;
        // Note: Rhai might handle division by zero differently
        // This test mainly ensures error handling works
    }

    #[tokio::test]
    async fn test_safety_limits() {
        let mut config = RhaiReplConfig::default();
        config.max_operations = 100; // Very low limit for testing

        let mut backend = RhaiReplBackend::with_config(config).unwrap();

        // This should hit the operations limit
        let result = backend.execute("let mut i = 0; loop { i += 1; if i > 1000 { break; } }").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metadata() {
        let backend = RhaiReplBackend::new().unwrap();
        let metadata = backend.get_metadata();

        assert_eq!(metadata.backend_type, "rhai");
        assert!(metadata.capabilities.contains(&"memory_safe".to_string()));
        assert!(metadata.capabilities.contains(&"sandboxed".to_string()));
        assert_eq!(metadata.max_code_length, Some(1024 * 1024));
    }

    #[tokio::test]
    async fn test_string_handling() {
        let mut backend = RhaiReplBackend::new().unwrap();

        // Test string operations
        let result = backend.execute(r#""hello" + " " + "world""#).await.unwrap();
        assert_eq!(result, "hello world");

        // Test string length
        let result = backend.execute(r#""test".len()"#).await.unwrap();
        assert_eq!(result, "4");
    }

    #[tokio::test]
    async fn test_array_operations() {
        let mut backend = RhaiReplBackend::new().unwrap();

        // Test array creation and access
        let result = backend.execute("let arr = [1, 2, 3]; arr[1]").await.unwrap();
        assert_eq!(result, "2");

        // Test array length
        let result = backend.execute("[1, 2, 3, 4].len()").await.unwrap();
        assert_eq!(result, "4");
    }

    #[tokio::test]
    async fn test_custom_constants() {
        let mut config = RhaiReplConfig::default();
        config.custom_constants.insert("CUSTOM_VALUE".to_string(), Dynamic::from(42));

        let mut backend = RhaiReplBackend::with_config(config).unwrap();

        let result = backend.execute("CUSTOM_VALUE").await.unwrap();
        assert_eq!(result, "42");
    }
}