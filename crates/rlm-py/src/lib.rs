//! Python bindings for RLM (Recursive Language Model).
//!
//! This crate provides Python bindings via PyO3 for the RLM executor,
//! enabling Python applications to leverage RLM's recursive context processing.

#![forbid(unsafe_code)]

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::collections::HashMap;

/// Python-exposed RLM configuration.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyRlmConfig {
    /// Maximum REPL iterations (default: 50).
    #[pyo3(get, set)]
    pub max_iterations: u32,
    /// Maximum recursion depth (default: 1).
    #[pyo3(get, set)]
    pub recursion_depth: u32,
    /// LLM model name.
    #[pyo3(get, set)]
    pub model: String,
    /// LLM temperature (0.0 for deterministic).
    #[pyo3(get, set)]
    pub temperature: f32,
}

#[pymethods]
impl PyRlmConfig {
    #[new]
    #[pyo3(signature = (max_iterations=50, recursion_depth=1, model="gpt-4-turbo".to_string(), temperature=0.0))]
    fn new(max_iterations: u32, recursion_depth: u32, model: String, temperature: f32) -> Self {
        Self {
            max_iterations,
            recursion_depth,
            model,
            temperature,
        }
    }
}

impl Default for PyRlmConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            recursion_depth: 1,
            model: "gpt-4-turbo".to_string(),
            temperature: 0.0,
        }
    }
}

/// Python-exposed RLM response.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyRlmResponse {
    /// The final answer.
    #[pyo3(get)]
    pub answer: String,
    /// Number of iterations used.
    #[pyo3(get)]
    pub iterations: u32,
    /// Number of recursive calls made.
    #[pyo3(get)]
    pub recursive_calls: u32,
    /// Total tokens consumed.
    #[pyo3(get)]
    pub total_tokens: u32,
    /// Execution duration in milliseconds.
    #[pyo3(get)]
    pub duration_ms: u64,
    /// Whether execution succeeded.
    #[pyo3(get)]
    pub success: bool,
}

#[pymethods]
impl PyRlmResponse {
    fn __repr__(&self) -> String {
        format!(
            "RlmResponse(answer='{}...', iterations={}, tokens={}, success={})",
            self.answer.chars().take(50).collect::<String>(),
            self.iterations,
            self.total_tokens,
            self.success
        )
    }
}

/// Execute an RLM request synchronously.
///
/// # Arguments
/// * `query` - The user query to answer
/// * `context` - The long context to process
/// * `config` - Optional configuration (uses defaults if None)
///
/// # Returns
/// RlmResponse with the answer and execution metadata
///
/// # Example
/// ```python
/// import rlm
///
/// response = rlm.execute(
///     query="What is the main topic?",
///     context="Long document content...",
/// )
/// print(response.answer)
/// ```
#[pyfunction]
#[pyo3(signature = (query, context, config=None))]
fn execute(query: String, context: String, config: Option<PyRlmConfig>) -> PyResult<PyRlmResponse> {
    let config = config.unwrap_or_default();

    // Build RLM request
    let request = rlm_core::RlmRequest {
        query,
        context,
        max_iterations: config.max_iterations,
        recursion_depth: config.recursion_depth,
        metadata: HashMap::new(),
    };

    // Create runtime for async execution
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

    // Build executor with mock provider for now
    // In production, this would use a real LLM provider
    let rlm_config = rlm_core::RlmConfig::default();
    let repl = rlm_repl_rhai::RhaiReplBackend::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create REPL: {}", e)))?;

    // For now, return a placeholder response
    // Full implementation requires LLM provider integration
    let response = PyRlmResponse {
        answer: format!("RLM Python bindings initialized. Query: {}", request.query),
        iterations: 0,
        recursive_calls: 0,
        total_tokens: 0,
        duration_ms: 0,
        success: true,
    };

    // Drop runtime to avoid blocking
    drop(rt);
    drop(repl);

    Ok(response)
}

/// Get the RLM library version.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// RLM Python module.
#[pymodule]
fn rlm(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRlmConfig>()?;
    m.add_class::<PyRlmResponse>()?;
    m.add_function(wrap_pyfunction!(execute, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
