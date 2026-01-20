//! RLM-specific functions for the Rhai REPL environment.
//!
//! This module provides the `llm_query()` function and other RLM-specific
//! functions that can be called from within the Rhai scripting environment
//! to enable recursive language model queries as described in the RLM paper.

use rhai::{Dynamic, Engine, EvalAltResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Placeholder for LLM provider interface within REPL functions.
///
/// Note: This will be replaced with a proper LLM provider trait in T032.
/// For now, this provides a simple mock interface for testing.
pub trait LlmQueryProvider: Send + Sync + std::fmt::Debug {
    /// Execute an LLM query and return the response.
    fn query(&self, prompt: &str) -> Result<String, String>;
}

/// Mock LLM provider for testing purposes.
///
/// This will be replaced with a real OpenAI provider in T032.
#[derive(Debug, Clone)]
pub struct MockLlmProvider {
    responses: Vec<String>,
    call_count: Arc<Mutex<usize>>,
}

impl MockLlmProvider {
    /// Create a new mock LLM provider with predefined responses.
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a mock provider with a single response.
    pub fn with_response(response: impl Into<String>) -> Self {
        Self::new(vec![response.into()])
    }
}

impl LlmQueryProvider for MockLlmProvider {
    fn query(&self, _prompt: &str) -> Result<String, String> {
        // In a real implementation, this would call the actual LLM
        // For now, return a mock response
        let count = {
            let mut count = self.call_count.try_lock()
                .map_err(|_| "Failed to acquire lock".to_string())?;
            let current = *count;
            *count += 1;
            current
        };

        if count < self.responses.len() {
            Ok(self.responses[count].clone())
        } else {
            Ok(format!("Mock response {}", count + 1))
        }
    }
}

/// RLM function registry that manages custom functions for the Rhai engine.
pub struct RlmFunctionRegistry {
    llm_provider: Arc<dyn LlmQueryProvider>,
}

impl std::fmt::Debug for RlmFunctionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RlmFunctionRegistry")
            .field("llm_provider", &"<dyn LlmQueryProvider>")
            .finish()
    }
}

impl RlmFunctionRegistry {
    /// Create a new function registry with an LLM provider.
    pub fn new(llm_provider: Arc<dyn LlmQueryProvider>) -> Self {
        Self { llm_provider }
    }

    /// Register all RLM functions with the given Rhai engine.
    pub fn register_functions(&self, engine: &mut Engine) {
        let provider = Arc::clone(&self.llm_provider);

        // Register the llm_query function
        engine.register_fn("llm_query", move |query: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            let provider = Arc::clone(&provider);
            Self::llm_query_impl(provider, query)
        });

        // Register additional utility functions
        self.register_utility_functions(engine);
    }

    /// Implementation of the llm_query function.
    fn llm_query_impl(
        provider: Arc<dyn LlmQueryProvider>,
        query: &str,
    ) -> Result<Dynamic, Box<EvalAltResult>> {
        debug!("Executing llm_query with prompt: {}", query);

        if query.trim().is_empty() {
            return Err("llm_query requires a non-empty query string".into());
        }

        match provider.query(query) {
            Ok(response) => {
                debug!("llm_query completed successfully, response length: {}", response.len());
                Ok(Dynamic::from(response))
            }
            Err(error) => {
                warn!("llm_query failed: {}", error);
                Err(format!("LLM query failed: {}", error).into())
            }
        }
    }

    /// Register additional utility functions for RLM.
    fn register_utility_functions(&self, engine: &mut Engine) {
        // Function to get the current context (placeholder for now)
        engine.register_fn("get_context", || -> Result<Dynamic, Box<EvalAltResult>> {
            // This would access the context variable set by the RLM executor
            // For now, return a placeholder
            Ok(Dynamic::from("context placeholder"))
        });

        // Function to log debug information
        engine.register_fn("rlm_debug", |message: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            debug!("RLM Debug: {}", message);
            Ok(Dynamic::from(()))
        });

        // Function to format results for aggregation
        engine.register_fn("format_result", |result: &str, format_type: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            match format_type {
                "json" => {
                    // Simple JSON formatting - in a real implementation this would be more robust
                    let formatted = format!("{{\"result\": \"{}\"}}", result.replace('"', "\\\""));
                    Ok(Dynamic::from(formatted))
                }
                "text" => {
                    Ok(Dynamic::from(result.to_string()))
                }
                _ => {
                    Err(format!("Unknown format type: {}", format_type).into())
                }
            }
        });
    }
}

/// Helper function to create an engine with RLM functions registered.
pub fn create_rlm_engine(llm_provider: Arc<dyn LlmQueryProvider>) -> Engine {
    let mut engine = crate::engine::create_secure_engine();

    let registry = RlmFunctionRegistry::new(llm_provider);
    registry.register_functions(&mut engine);

    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_llm_provider() {
        let provider = MockLlmProvider::with_response("Test response");

        let result = provider.query("test prompt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test response");
    }

    #[test]
    fn test_mock_llm_provider_multiple_responses() {
        let responses = vec![
            "First response".to_string(),
            "Second response".to_string(),
            "Third response".to_string(),
        ];
        let provider = MockLlmProvider::new(responses);

        assert_eq!(provider.query("query 1").unwrap(), "First response");
        assert_eq!(provider.query("query 2").unwrap(), "Second response");
        assert_eq!(provider.query("query 3").unwrap(), "Third response");
        assert_eq!(provider.query("query 4").unwrap(), "Mock response 4");
    }

    #[test]
    fn test_llm_query_function_registration() {
        let provider = Arc::new(MockLlmProvider::with_response("Hello from LLM"));
        let engine = create_rlm_engine(provider);

        // Test that the llm_query function works
        let result: String = engine.eval(r#"llm_query("What is 2+2?")"#).unwrap();
        assert_eq!(result, "Hello from LLM");
    }

    #[test]
    fn test_llm_query_empty_string() {
        let provider = Arc::new(MockLlmProvider::with_response("Should not reach here"));
        let engine = create_rlm_engine(provider);

        // Test that empty queries are rejected
        let result = engine.eval::<String>(r#"llm_query("")"#);
        assert!(result.is_err());

        let result = engine.eval::<String>(r#"llm_query("   ")"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_utility_functions() {
        let provider = Arc::new(MockLlmProvider::with_response("test"));
        let engine = create_rlm_engine(provider);

        // Test rlm_debug function
        let result: () = engine.eval(r#"rlm_debug("Test debug message")"#).unwrap();
        assert_eq!(result, ());

        // Test format_result function
        let result: String = engine.eval(r#"format_result("hello", "text")"#).unwrap();
        assert_eq!(result, "hello");

        let result: String = engine.eval(r#"format_result("hello", "json")"#).unwrap();
        assert_eq!(result, r#"{"result": "hello"}"#);

        // Test invalid format
        let result = engine.eval::<String>(r#"format_result("hello", "invalid")"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_recursive_llm_calls() {
        let responses = vec![
            "Step 1: I need more information".to_string(),
            "Step 2: Based on previous step".to_string(),
            "Final answer: 42".to_string(),
        ];
        let provider = Arc::new(MockLlmProvider::new(responses));
        let engine = create_rlm_engine(provider);

        // Test a simple recursive pattern
        let script = r#"
            let step1 = llm_query("What is the first step?");
            let step2 = llm_query("Based on: " + step1 + ", what next?");
            let final = llm_query("Final question: " + step2);
            final
        "#;

        let result: String = engine.eval(script).unwrap();
        assert_eq!(result, "Final answer: 42");
    }

    #[test]
    fn test_context_and_formatting_integration() {
        let provider = Arc::new(MockLlmProvider::with_response("Processed context successfully"));
        let engine = create_rlm_engine(provider);

        // Test integration of context access and result formatting
        let script = r#"
            let context = get_context();
            rlm_debug("Got context: " + context);
            let response = llm_query("Process this context: " + context);
            format_result(response, "json")
        "#;

        let result: String = engine.eval(script).unwrap();
        assert!(result.contains("Processed context successfully"));
        assert!(result.starts_with("{\"result\":"));
    }

    #[test]
    fn test_function_registry_creation() {
        let provider = Arc::new(MockLlmProvider::with_response("test"));
        let registry = RlmFunctionRegistry::new(provider);

        let mut engine = Engine::new();
        registry.register_functions(&mut engine);

        // Test that functions were registered
        let result: String = engine.eval(r#"llm_query("test")"#).unwrap();
        assert_eq!(result, "test");
    }
}