//! LLM provider port definition.
//!
//! This module defines the `LlmProvider` trait that enables RLM to work
//! with different language model providers (OpenAI, Anthropic, local models, etc.)

use crate::error::RlmResult;
use crate::types::{ChatCompletionRequest, ChatCompletionResponse};
use async_trait::async_trait;
use std::pin::Pin;
use futures::Stream;

/// Provider for language model inference.
///
/// This trait abstracts different LLM providers, allowing the core executor
/// to remain agnostic to the specific model or API being used.
///
/// # Design Notes
///
/// - Supports both streaming and non-streaming responses
/// - Uses OpenAI-compatible request/response types for consistency
/// - All operations are async to support network I/O
/// - Implementations should handle rate limiting and retries
///
/// # Example
///
/// ```rust,no_run
/// use rlm_core::ports::LlmProvider;
/// use rlm_core::types::{ChatCompletionRequest, ChatMessage, ChatRole};
///
/// async fn ask_question<P: LlmProvider>(
///     provider: &P,
///     question: &str
/// ) -> Result<String, Box<dyn std::error::Error>> {
///     let request = ChatCompletionRequest {
///         model: "gpt-4".to_string(),
///         messages: vec![ChatMessage {
///             role: ChatRole::User,
///             content: question.to_string(),
///             name: None,
///         }],
///         max_tokens: Some(1000),
///         temperature: Some(0.7),
///         stream: false,
///         stop: None,
///         top_p: None,
///         rlm_config: None,
///     };
///
///     let response = provider.complete(&request).await?;
///     Ok(response.choices[0].message.content.clone())
/// }
/// ```
#[async_trait]
pub trait LlmProvider: Send + Sync + std::fmt::Debug {
    /// Complete a chat conversation (non-streaming).
    ///
    /// # Arguments
    ///
    /// * `request` - The chat completion request
    ///
    /// # Returns
    ///
    /// * `Ok(ChatCompletionResponse)` - The complete response
    /// * `Err(RlmError)` - Request failed with details
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The API request fails (network, authentication, etc.)
    /// - The model returns an error response
    /// - Request parameters are invalid
    /// - Rate limits are exceeded
    async fn complete(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse>;

    /// Complete a chat conversation with streaming.
    ///
    /// Returns a stream of partial responses that can be processed
    /// incrementally for real-time UX.
    ///
    /// # Arguments
    ///
    /// * `request` - The chat completion request (stream should be true)
    ///
    /// # Returns
    ///
    /// A stream of `ChatCompletionResponse` objects with incremental content.
    ///
    /// # Errors
    ///
    /// The stream yields errors if:
    /// - Connection is lost during streaming
    /// - Invalid server-sent events are received
    /// - The model encounters an error mid-stream
    async fn complete_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> RlmResult<Pin<Box<dyn Stream<Item = RlmResult<ChatCompletionResponse>> + Send>>>;

    /// Get information about available models.
    ///
    /// Returns a list of model identifiers that can be used
    /// in chat completion requests.
    async fn list_models(&self) -> RlmResult<Vec<ModelInfo>>;

    /// Validate a chat completion request.
    ///
    /// Checks request parameters against provider capabilities
    /// before sending to the API. Useful for early error detection.
    ///
    /// # Arguments
    ///
    /// * `request` - The request to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Request is valid
    /// * `Err(RlmError)` - Request has validation errors
    fn validate_request(&self, request: &ChatCompletionRequest) -> RlmResult<()>;

    /// Get provider metadata and capabilities.
    fn get_metadata(&self) -> ProviderMetadata;

    /// Health check for the provider.
    ///
    /// Verifies that the provider is accessible and properly configured.
    /// This should be a lightweight operation (e.g., API key validation).
    async fn health_check(&self) -> RlmResult<bool>;
}

/// Information about a language model.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    pub id: String,
    /// Human-readable model name
    pub name: String,
    /// Maximum context length in tokens
    pub max_context_tokens: u32,
    /// Maximum output tokens per request
    pub max_output_tokens: u32,
    /// Whether the model supports streaming
    pub supports_streaming: bool,
    /// Whether the model supports function calling
    pub supports_functions: bool,
    /// Cost per input token (if available)
    pub cost_per_input_token: Option<f64>,
    /// Cost per output token (if available)
    pub cost_per_output_token: Option<f64>,
}

impl ModelInfo {
    /// Create new model information.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        max_context_tokens: u32,
        max_output_tokens: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            max_context_tokens,
            max_output_tokens,
            supports_streaming: false,
            supports_functions: false,
            cost_per_input_token: None,
            cost_per_output_token: None,
        }
    }

    /// Builder pattern for optional fields.
    pub fn with_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    /// Builder pattern for function support.
    pub fn with_functions(mut self, supports_functions: bool) -> Self {
        self.supports_functions = supports_functions;
        self
    }

    /// Builder pattern for cost information.
    pub fn with_costs(mut self, input_cost: f64, output_cost: f64) -> Self {
        self.cost_per_input_token = Some(input_cost);
        self.cost_per_output_token = Some(output_cost);
        self
    }
}

/// Metadata about an LLM provider implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderMetadata {
    /// Provider name (e.g., "OpenAI", "Anthropic", "Local")
    pub name: String,
    /// Provider version or API version
    pub version: String,
    /// Base URL for API requests
    pub base_url: String,
    /// Whether authentication is required
    pub requires_auth: bool,
    /// Maximum requests per minute (if known)
    pub rate_limit_rpm: Option<u32>,
    /// Maximum tokens per minute (if known)
    pub rate_limit_tpm: Option<u32>,
    /// List of supported features
    pub capabilities: Vec<String>,
}

impl ProviderMetadata {
    /// Create new provider metadata.
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        base_url: impl Into<String>,
        requires_auth: bool,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            base_url: base_url.into(),
            requires_auth,
            rate_limit_rpm: None,
            rate_limit_tpm: None,
            capabilities: Vec::new(),
        }
    }

    /// Check if the provider supports a specific capability.
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    /// Add rate limiting information.
    pub fn with_rate_limits(mut self, rpm: u32, tpm: u32) -> Self {
        self.rate_limit_rpm = Some(rpm);
        self.rate_limit_tpm = Some(tpm);
        self
    }

    /// Add supported capabilities.
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_info_builder() {
        let model = ModelInfo::new("test-model", "Test Model", 4096, 1000)
            .with_streaming(true)
            .with_functions(false)
            .with_costs(0.01, 0.02);

        assert_eq!(model.id, "test-model");
        assert_eq!(model.name, "Test Model");
        assert_eq!(model.max_context_tokens, 4096);
        assert_eq!(model.max_output_tokens, 1000);
        assert!(model.supports_streaming);
        assert!(!model.supports_functions);
        assert_eq!(model.cost_per_input_token, Some(0.01));
        assert_eq!(model.cost_per_output_token, Some(0.02));
    }

    #[test]
    fn test_provider_metadata() {
        let metadata = ProviderMetadata::new("TestAI", "v1.0", "https://api.test.com", true)
            .with_rate_limits(1000, 50000)
            .with_capabilities(vec!["streaming".to_string(), "functions".to_string()]);

        assert_eq!(metadata.name, "TestAI");
        assert!(metadata.requires_auth);
        assert!(metadata.supports("streaming"));
        assert!(!metadata.supports("multimodal"));
        assert_eq!(metadata.rate_limit_rpm, Some(1000));
        assert_eq!(metadata.rate_limit_tpm, Some(50000));
    }
}