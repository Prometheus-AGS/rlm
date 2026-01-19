//! Port definitions (traits) for RLM adapters.

use crate::{RlmEvent, RlmResult};
use async_trait::async_trait;
use std::collections::HashMap;

/// REPL backend port.
#[async_trait]
pub trait ReplBackend: Send + Sync {
    /// Initialize the REPL environment.
    async fn init(&mut self) -> RlmResult<()>;

    /// Execute code in the REPL.
    ///
    /// # Arguments
    /// * `code` - Code to execute
    ///
    /// # Returns
    /// Result as a string (or error)
    async fn execute(&mut self, code: &str) -> RlmResult<String>;

    /// Set a variable in the REPL.
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()>;

    /// Get a variable from the REPL.
    async fn get_variable(&mut self, name: &str) -> RlmResult<String>;

    /// Get all variables (for debugging).
    async fn get_state(&self) -> RlmResult<HashMap<String, String>>;

    /// Reset the REPL to initial state.
    async fn reset(&mut self) -> RlmResult<()>;

    /// Check if REPL supports a feature.
    fn supports(&self, feature: &str) -> bool;
}

/// LLM provider port.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Call the LLM with a prompt.
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send
    /// * `max_tokens` - Maximum tokens to generate
    ///
    /// # Returns
    /// Generated text
    async fn call(&self, prompt: &str, max_tokens: u32) -> RlmResult<String>;

    /// Stream LLM response (returns async stream).
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send
    /// * `max_tokens` - Maximum tokens to generate
    ///
    /// # Returns
    /// Stream of text chunks
    async fn stream(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> RlmResult<Box<dyn futures::Stream<Item = RlmResult<String>> + Send + Unpin>>;

    /// Get model name.
    fn model_name(&self) -> &str;

    /// Count tokens in text (approximate).
    fn count_tokens(&self, text: &str) -> usize;
}

/// Event sink port (for streaming events).
#[async_trait]
pub trait EventSink: Send + Sync {
    /// Emit an event.
    async fn emit(&self, event: RlmEvent) -> RlmResult<()>;

    /// Flush any buffered events.
    async fn flush(&self) -> RlmResult<()>;
}

/// Null event sink (no-op).
#[derive(Debug, Clone)]
pub struct NullEventSink;

#[async_trait]
impl EventSink for NullEventSink {
    async fn emit(&self, _event: RlmEvent) -> RlmResult<()> {
        Ok(())
    }

    async fn flush(&self) -> RlmResult<()> {
        Ok(())
    }
}
