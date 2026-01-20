//! OpenAI LLM provider adapter for RLM.
//!
//! This module implements the LlmProvider trait for OpenAI's API,
//! enabling the RLM executor to make recursive LLM calls through OpenAI's
//! chat completions endpoint.

use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, StatusCode};
use rlm_core::ports::{LlmProvider, ModelInfo, ProviderMetadata};
use rlm_core::types::{ChatCompletionRequest, ChatCompletionResponse};
use rlm_core::{RlmError, RlmResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// Configuration for the OpenAI LLM provider.
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// OpenAI API key.
    pub api_key: String,
    /// Base URL for OpenAI API (default: https://api.openai.com/v1).
    pub base_url: String,
    /// Default model to use for completions.
    pub default_model: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests.
    pub max_retries: u32,
    /// Enable request/response logging.
    pub enable_logging: bool,
    /// Custom headers to include in requests.
    pub custom_headers: HashMap<String, String>,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            default_model: "gpt-4".to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            enable_logging: false,
            custom_headers: HashMap::new(),
        }
    }
}

/// OpenAI API error response.
#[derive(Debug, Deserialize)]
struct OpenAiError {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
    #[allow(dead_code)]
    param: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

/// OpenAI LLM provider implementation.
pub struct OpenAiProvider {
    config: OpenAiConfig,
    client: Client,
}

impl std::fmt::Debug for OpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiProvider")
            .field("config", &self.config)
            .field("client", &"<reqwest::Client>")
            .finish()
    }
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given configuration.
    pub fn new(config: OpenAiConfig) -> RlmResult<Self> {
        if config.api_key.is_empty() {
            return Err(RlmError::Config(
                "OpenAI API key is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| RlmError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create a new OpenAI provider from environment variables.
    pub fn from_env() -> RlmResult<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| RlmError::Config("OPENAI_API_KEY environment variable not set".to_string()))?;

        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let default_model = std::env::var("OPENAI_MODEL")
            .unwrap_or_else(|_| "gpt-4".to_string());

        let config = OpenAiConfig {
            api_key,
            base_url,
            default_model,
            ..Default::default()
        };

        Self::new(config)
    }

    /// Convert OpenAI API error to RLM error.
    fn convert_error(&self, status: StatusCode, body: &str) -> RlmError {
        match serde_json::from_str::<OpenAiError>(body) {
            Ok(error) => {
                let message = format!(
                    "OpenAI API error ({}): {}",
                    status,
                    error.error.message
                );
                RlmError::Llm(message)
            }
            Err(_) => {
                RlmError::Llm(format!("OpenAI API error ({}): {}", status, body))
            }
        }
    }

    /// Make a request to the OpenAI API with retries.
    async fn make_request(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        for attempt in 0..=self.config.max_retries {
            if self.config.enable_logging {
                debug!("Making OpenAI API request (attempt {})", attempt + 1);
            }

            let mut req_builder = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json");

            // Add custom headers
            for (key, value) in &self.config.custom_headers {
                req_builder = req_builder.header(key, value);
            }

            let response = req_builder
                .json(request)
                .send()
                .await
                .map_err(|e| RlmError::Llm(format!("Request failed: {}", e)))?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| RlmError::Llm(format!("Failed to read response body: {}", e)))?;

            if status.is_success() {
                let completion_response: ChatCompletionResponse = serde_json::from_str(&body)
                    .map_err(|e| RlmError::Llm(format!("Failed to parse response: {}", e)))?;

                if self.config.enable_logging {
                    debug!("OpenAI API request successful");
                }

                return Ok(completion_response);
            } else if status == StatusCode::TOO_MANY_REQUESTS && attempt < self.config.max_retries {
                // Retry rate limit errors with exponential backoff
                let delay = Duration::from_millis(1000 * (2_u64.pow(attempt)));
                warn!("Rate limited, retrying in {:?}", delay);
                tokio::time::sleep(delay).await;
                continue;
            } else {
                return Err(self.convert_error(status, &body));
            }
        }

        Err(RlmError::Llm("Max retries exceeded".to_string()))
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    #[instrument(skip(self, request), fields(model = %request.model))]
    async fn complete(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        debug!("Starting OpenAI chat completion");

        // Use the model from the request or fall back to default
        let mut request = request.clone();
        if request.model.is_empty() {
            request.model = self.config.default_model.clone();
        }

        // Ensure stream is false for non-streaming completion
        request.stream = false;

        self.make_request(&request).await
    }

    async fn complete_stream(
        &self,
        _request: &ChatCompletionRequest,
    ) -> RlmResult<Pin<Box<dyn Stream<Item = RlmResult<ChatCompletionResponse>> + Send>>> {
        // TODO: Implement streaming support for OpenAI
        Err(RlmError::Other("Streaming not yet implemented for OpenAI provider".to_string()))
    }

    async fn list_models(&self) -> RlmResult<Vec<ModelInfo>> {
        debug!("Fetching OpenAI models");

        let url = format!("{}/models", self.config.base_url);
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| RlmError::Llm(format!("Request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(self.convert_error(status, &body));
        }

        #[derive(Deserialize)]
        struct ModelsResponse {
            data: Vec<OpenAiModel>,
        }

        #[derive(Deserialize)]
        struct OpenAiModel {
            id: String,
            #[allow(dead_code)]
            owned_by: Option<String>,
        }

        let models_response: ModelsResponse = response.json().await
            .map_err(|e| RlmError::Llm(format!("Failed to parse models response: {}", e)))?;

        let models: Vec<ModelInfo> = models_response.data
            .into_iter()
            .map(|model| {
                // Set reasonable defaults based on model ID
                let (max_context, max_output) = match model.id.as_str() {
                    id if id.contains("gpt-4") => (128_000, 4_096),
                    id if id.contains("gpt-3.5") => (16_384, 4_096),
                    _ => (4_096, 2_048),
                };

                ModelInfo::new(
                    model.id.clone(),
                    model.id, // Use ID as display name
                    max_context,
                    max_output,
                )
                .with_streaming(true)
                .with_functions(true)
            })
            .collect();

        debug!("Retrieved {} models", models.len());
        Ok(models)
    }

    fn validate_request(&self, request: &ChatCompletionRequest) -> RlmResult<()> {
        if request.messages.is_empty() {
            return Err(RlmError::Other("Request must contain at least one message".to_string()));
        }

        if request.model.is_empty() && self.config.default_model.is_empty() {
            return Err(RlmError::Config("No model specified and no default model configured".to_string()));
        }

        // Check token limits
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(RlmError::Other("max_tokens must be greater than 0".to_string()));
            }
            if max_tokens > 4096 {
                return Err(RlmError::Other("max_tokens exceeds OpenAI limits".to_string()));
            }
        }

        // Check temperature range
        if let Some(temperature) = request.temperature {
            if temperature < 0.0 || temperature > 2.0 {
                return Err(RlmError::Other("temperature must be between 0.0 and 2.0".to_string()));
            }
        }

        // Check top_p range
        if let Some(top_p) = request.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err(RlmError::Other("top_p must be between 0.0 and 1.0".to_string()));
            }
        }

        Ok(())
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata::new(
            "OpenAI",
            "1.0.0",
            self.config.base_url.clone(),
            true, // requires_auth
        )
        .with_rate_limits(10_000, 300_000) // 10k RPM, 300k TPM for GPT-4
        .with_capabilities(vec![
            "chat_completions".to_string(),
            "streaming".to_string(),
            "function_calling".to_string(),
            "model_listing".to_string(),
        ])
    }

    async fn health_check(&self) -> RlmResult<bool> {
        debug!("Performing OpenAI provider health check");

        // Simple health check - try to list models
        match self.list_models().await {
            Ok(_) => {
                debug!("OpenAI provider health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("OpenAI provider health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

/// Builder for OpenAI configuration.
#[derive(Debug)]
pub struct OpenAiConfigBuilder {
    config: OpenAiConfig,
}

impl OpenAiConfigBuilder {
    /// Create a new config builder with an API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            config: OpenAiConfig {
                api_key: api_key.into(),
                ..Default::default()
            },
        }
    }

    /// Set the base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    /// Set the default model.
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.config.default_model = model.into();
        self
    }

    /// Set the request timeout.
    pub fn timeout_seconds(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    /// Set the maximum retries.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Enable request/response logging.
    pub fn enable_logging(mut self) -> Self {
        self.config.enable_logging = true;
        self
    }

    /// Add a custom header.
    pub fn custom_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.custom_headers.insert(key.into(), value.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> OpenAiConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::types::{ChatMessage, ChatRole};

    #[test]
    fn test_config_builder() {
        let config = OpenAiConfigBuilder::new("test-key")
            .base_url("https://custom-api.com/v1")
            .default_model("gpt-3.5-turbo")
            .timeout_seconds(60)
            .max_retries(5)
            .enable_logging()
            .custom_header("X-Custom", "value")
            .build();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://custom-api.com/v1");
        assert_eq!(config.default_model, "gpt-3.5-turbo");
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 5);
        assert!(config.enable_logging);
        assert_eq!(config.custom_headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_provider_creation_without_api_key() {
        let config = OpenAiConfig {
            api_key: String::new(),
            ..Default::default()
        };

        let result = OpenAiProvider::new(config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RlmError::Config(_)));
    }

    #[test]
    fn test_provider_creation_with_api_key() {
        let config = OpenAiConfigBuilder::new("test-key").build();
        let result = OpenAiProvider::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_metadata() {
        let config = OpenAiConfigBuilder::new("test-key").build();
        let provider = OpenAiProvider::new(config).unwrap();
        let metadata = provider.get_metadata();

        assert_eq!(metadata.name, "OpenAI");
        assert!(metadata.supports("chat_completions"));
        assert!(metadata.supports("streaming"));
    }

    #[test]
    fn test_request_validation() {
        let config = OpenAiConfigBuilder::new("test-key").build();
        let provider = OpenAiProvider::new(config).unwrap();

        // Test empty messages
        let empty_request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };
        assert!(provider.validate_request(&empty_request).is_err());

        // Test valid request
        let valid_request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };
        assert!(provider.validate_request(&valid_request).is_ok());

        // Test invalid temperature
        let mut invalid_temp_request = valid_request.clone();
        invalid_temp_request.temperature = Some(5.0);
        assert!(provider.validate_request(&invalid_temp_request).is_err());
    }

    #[test]
    fn test_error_conversion() {
        let config = OpenAiConfigBuilder::new("test-key").build();
        let provider = OpenAiProvider::new(config).unwrap();

        let error_json = r#"{"error": {"message": "Invalid API key", "type": "authentication_error"}}"#;
        let error = provider.convert_error(StatusCode::UNAUTHORIZED, error_json);
        assert!(matches!(error, RlmError::Llm(_)));

        let error = provider.convert_error(StatusCode::TOO_MANY_REQUESTS, error_json);
        assert!(matches!(error, RlmError::Llm(_)));
    }

    // Integration test - requires OPENAI_API_KEY environment variable
    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_real_openai_request() {
        let provider = OpenAiProvider::from_env().unwrap();

        let request = ChatCompletionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Say hello in one word".to_string(),
                name: None,
            }],
            max_tokens: Some(10),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let response = provider.complete(&request).await.unwrap();
        assert!(!response.choices.is_empty());
        assert!(!response.choices[0].message.content.is_empty());
        assert!(response.usage.total_tokens > 0);
    }

    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_list_models() {
        let provider = OpenAiProvider::from_env().unwrap();
        let models = provider.list_models().await.unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id.contains("gpt")));
    }

    #[tokio::test]
    #[ignore = "requires API key"]
    async fn test_health_check() {
        let provider = OpenAiProvider::from_env().unwrap();
        let health = provider.health_check().await.unwrap();
        assert!(health);
    }
}