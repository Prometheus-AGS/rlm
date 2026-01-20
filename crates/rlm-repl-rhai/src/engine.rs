//! Rhai engine configuration and safety limits for RLM.
//!
//! This module provides safe, sandboxed Rhai engine configuration with comprehensive
//! security limits suitable for the RLM paper's context offloading approach.

use rhai::{Dynamic, Engine, OptimizationLevel};
use std::collections::HashMap;

/// Configuration for the Rhai engine safety limits.
#[derive(Debug, Clone)]
pub struct RhaiEngineConfig {
    /// Maximum operations allowed per execution.
    pub max_operations: u32,
    /// Maximum string length.
    pub max_string_size: usize,
    /// Maximum array size.
    pub max_array_size: usize,
    /// Maximum map size.
    pub max_map_size: usize,
    /// Maximum expression nesting depth.
    pub max_expr_depth: usize,
    /// Maximum function call stack depth.
    pub max_function_call_depth: usize,
    /// Maximum number of modules that can be imported.
    pub max_modules: usize,
    /// Enable AST optimizations.
    pub enable_optimization: bool,
    /// Additional custom constants to add to the engine.
    pub custom_constants: HashMap<String, Dynamic>,
    /// Disabled symbols for security.
    pub disabled_symbols: Vec<String>,
}

impl Default for RhaiEngineConfig {
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
            disabled_symbols: vec![
                "import".to_string(),      // Disable module imports
                "eval".to_string(),        // Disable eval for security
            ],
        }
    }
}

/// Engine configuration builder for Rhai.
#[derive(Debug)]
pub struct RhaiEngineBuilder {
    config: RhaiEngineConfig,
}

impl RhaiEngineBuilder {
    /// Create a new engine builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: RhaiEngineConfig::default(),
        }
    }

    /// Create a new engine builder with custom configuration.
    pub fn with_config(config: RhaiEngineConfig) -> Self {
        Self { config }
    }

    /// Set maximum operations limit.
    pub fn max_operations(mut self, max_ops: u32) -> Self {
        self.config.max_operations = max_ops;
        self
    }

    /// Set maximum string size.
    pub fn max_string_size(mut self, size: usize) -> Self {
        self.config.max_string_size = size;
        self
    }

    /// Set maximum array size.
    pub fn max_array_size(mut self, size: usize) -> Self {
        self.config.max_array_size = size;
        self
    }

    /// Set maximum map size.
    pub fn max_map_size(mut self, size: usize) -> Self {
        self.config.max_map_size = size;
        self
    }

    /// Set maximum expression depth.
    pub fn max_expr_depth(mut self, depth: usize) -> Self {
        self.config.max_expr_depth = depth;
        self
    }

    /// Set maximum function call depth.
    pub fn max_function_call_depth(mut self, depth: usize) -> Self {
        self.config.max_function_call_depth = depth;
        self
    }

    /// Set maximum number of modules.
    pub fn max_modules(mut self, modules: usize) -> Self {
        self.config.max_modules = modules;
        self
    }

    /// Enable or disable optimizations.
    pub fn optimization(mut self, enabled: bool) -> Self {
        self.config.enable_optimization = enabled;
        self
    }

    /// Add a custom constant.
    pub fn custom_constant<T: Into<Dynamic>>(mut self, name: String, value: T) -> Self {
        self.config.custom_constants.insert(name, value.into());
        self
    }

    /// Add multiple custom constants.
    pub fn custom_constants(mut self, constants: HashMap<String, Dynamic>) -> Self {
        self.config.custom_constants.extend(constants);
        self
    }

    /// Disable a symbol for security.
    pub fn disable_symbol(mut self, symbol: String) -> Self {
        if !self.config.disabled_symbols.contains(&symbol) {
            self.config.disabled_symbols.push(symbol);
        }
        self
    }

    /// Disable multiple symbols for security.
    pub fn disable_symbols(mut self, symbols: Vec<String>) -> Self {
        for symbol in symbols {
            if !self.config.disabled_symbols.contains(&symbol) {
                self.config.disabled_symbols.push(symbol);
            }
        }
        self
    }

    /// Build the configured Rhai engine.
    pub fn build(self) -> Engine {
        let mut engine = Engine::new();

        // Apply safety limits
        engine.set_max_operations(self.config.max_operations as u64);
        engine.set_max_string_size(self.config.max_string_size);
        engine.set_max_array_size(self.config.max_array_size);
        engine.set_max_map_size(self.config.max_map_size);
        engine.set_max_expr_depths(self.config.max_expr_depth, 0);
        engine.set_max_modules(self.config.max_modules);

        // Configure optimizations
        if self.config.enable_optimization {
            engine.set_optimization_level(OptimizationLevel::Full);
        } else {
            engine.set_optimization_level(OptimizationLevel::None);
        }

        // Disable dangerous symbols for security
        for symbol in &self.config.disabled_symbols {
            engine.disable_symbol(symbol);
        }

        engine
    }

    /// Get the current configuration (for inspection).
    pub fn config(&self) -> &RhaiEngineConfig {
        &self.config
    }
}

impl Default for RhaiEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a secure Rhai engine with RLM-specific safety configuration.
pub fn create_secure_engine() -> Engine {
    RhaiEngineBuilder::new()
        .max_operations(1_000_000)      // Prevent infinite loops
        .max_string_size(1024 * 1024)   // 1MB string limit
        .max_array_size(10_000)         // Reasonable array size
        .max_map_size(10_000)           // Reasonable map size
        .max_expr_depth(32)             // Prevent deep nesting attacks
        .max_function_call_depth(64)    // Prevent stack overflow
        .max_modules(8)                 // Limited module imports
        .optimization(true)             // Enable performance optimizations
        .disable_symbol("import".to_string())  // Block file system access
        .disable_symbol("eval".to_string())     // Block dynamic evaluation
        .build()
}

/// Create a development Rhai engine with relaxed limits for testing.
#[cfg(test)]
pub fn create_development_engine() -> Engine {
    RhaiEngineBuilder::new()
        .max_operations(10_000)         // Lower limit for quick tests
        .max_string_size(1024)          // Smaller strings for tests
        .max_array_size(100)            // Smaller arrays for tests
        .max_map_size(100)              // Smaller maps for tests
        .max_expr_depth(16)             // Shallower nesting for tests
        .max_function_call_depth(32)    // Smaller call stack for tests
        .max_modules(4)                 // Fewer modules for tests
        .optimization(false)            // Disable optimizations for predictable tests
        .disable_symbol("import".to_string())
        .disable_symbol("eval".to_string())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_builder_default() {
        let builder = RhaiEngineBuilder::new();
        let config = builder.config();

        assert_eq!(config.max_operations, 1_000_000);
        assert_eq!(config.max_string_size, 1024 * 1024);
        assert_eq!(config.max_array_size, 10_000);
        assert_eq!(config.max_map_size, 10_000);
        assert_eq!(config.max_expr_depth, 32);
        assert_eq!(config.max_function_call_depth, 64);
        assert_eq!(config.max_modules, 8);
        assert!(config.enable_optimization);
        assert!(config.disabled_symbols.contains(&"import".to_string()));
        assert!(config.disabled_symbols.contains(&"eval".to_string()));
    }

    #[test]
    fn test_engine_builder_customization() {
        let builder = RhaiEngineBuilder::new()
            .max_operations(500_000)
            .max_string_size(512 * 1024)
            .optimization(false)
            .disable_symbol("println".to_string());

        let config = builder.config();
        assert_eq!(config.max_operations, 500_000);
        assert_eq!(config.max_string_size, 512 * 1024);
        assert!(!config.enable_optimization);
        assert!(config.disabled_symbols.contains(&"println".to_string()));
    }

    #[test]
    fn test_custom_constants() {
        let mut constants = HashMap::new();
        constants.insert("TEST_VALUE".to_string(), Dynamic::from(42));
        constants.insert("TEST_STRING".to_string(), Dynamic::from("hello"));

        let builder = RhaiEngineBuilder::new()
            .custom_constants(constants)
            .custom_constant("ANOTHER_VALUE".to_string(), 100);

        let config = builder.config();
        assert!(config.custom_constants.contains_key("TEST_VALUE"));
        assert!(config.custom_constants.contains_key("TEST_STRING"));
        assert!(config.custom_constants.contains_key("ANOTHER_VALUE"));
    }

    #[test]
    fn test_secure_engine_creation() {
        let engine = create_secure_engine();

        // Test that basic operations work
        let result: i64 = engine.eval("1 + 1").expect("Basic math should work");
        assert_eq!(result, 2);

        // Test that dangerous operations are disabled
        let result = engine.eval::<()>("import \"std\"");
        assert!(result.is_err(), "Import should be disabled");

        let result = engine.eval::<()>("eval(\"1 + 1\")");
        assert!(result.is_err(), "Eval should be disabled");
    }

    #[test]
    fn test_development_engine_creation() {
        let engine = create_development_engine();

        // Test that basic operations work with development engine too
        let result: i64 = engine.eval("2 * 3").expect("Basic math should work");
        assert_eq!(result, 6);
    }

    #[test]
    fn test_engine_safety_limits() {
        let engine = RhaiEngineBuilder::new()
            .max_operations(10)  // Very low limit for testing
            .build();

        // This should hit the operations limit
        let result = engine.eval::<()>("let mut i = 0; loop { i += 1; if i > 100 { break; } }");
        assert!(result.is_err(), "Should hit operations limit");
    }

    #[test]
    fn test_duplicate_symbol_disable() {
        let builder = RhaiEngineBuilder::new()
            .disable_symbol("test".to_string())
            .disable_symbol("test".to_string()); // Duplicate

        let config = builder.config();
        let test_count = config.disabled_symbols.iter()
            .filter(|&s| s == "test")
            .count();

        assert_eq!(test_count, 1, "Should not have duplicate disabled symbols");
    }
}