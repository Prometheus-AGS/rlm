//! Azure OpenAI LLM provider adapter for RLM.
//!
//! This module implements the LlmProvider trait for Azure OpenAI Service,
//! enabling the RLM executor to make recursive LLM calls through Azure's
//! OpenAI-compatible chat completions endpoint with Azure-specific
//! authentication, URL formatting, and API versioning.

use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, StatusCode};
use rlm_core::ports::{LlmProvider, ModelInfo, ProviderMetadata};
use rlm_core::types::{ChatCompletionRequest, ChatCompletionResponse};
use rlm_core::{RlmError, RlmResult, BackendConfig, ProviderType};
use serde::Deserialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;
use tracing::{debug, instrument, warn};

/// Configuration for the Azure OpenAI LLM provider.
#[derive(Debug, Clone)]
pub struct AzureOpenAiConfig {
    /// Azure resource name.
    pub resource_name: String,
    /// Azure deployment name (maps to model).
    pub deployment_name: String,
    /// Azure OpenAI API key.
    pub api_key: String,
    /// API version for Azure OpenAI.
    pub api_version: String,
    /// Base URL for Azure OpenAI (computed from resource and deployment).
    pub base_url: String,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum retries for failed requests.
    pub max_retries: u32,
    /// Enable request/response logging.
    pub enable_logging: bool,
    /// Model mappings (logical name -> deployment name).
    pub model_mappings: HashMap<String, String>,
}

impl AzureOpenAiConfig {
    /// Create a new Azure OpenAI configuration.
    pub fn new(
        resource_name: String,
        deployment_name: String,
        api_key: String,
    ) -> Self {
        let base_url = format!(
            "https://{}.openai.azure.com/openai/deployments/{}/",
            resource_name, deployment_name
        );

        let mut model_mappings = HashMap::new();
        model_mappings.insert("gpt-4".to_string(), deployment_name.clone());

        Self {
            resource_name,
            deployment_name: deployment_name.clone(),
            api_key,
            api_version: "2024-02-01".to_string(),
            base_url,
            timeout_seconds: 45, // Longer timeout for Azure
            max_retries: 5, // More aggressive retries
            enable_logging: false,
            model_mappings,
        }
    }

    /// Create configuration from a BackendConfig.
    pub fn from_backend_config(config: &BackendConfig) -> RlmResult<Self> {
        if config.provider_type != ProviderType::AzureOpenAi {
            return Err(RlmError::Config(
                "Backend config is not for Azure OpenAI".to_string(),
            ));
        }

        // Extract resource and deployment from URL
        // Expected format: https://{resource}.openai.azure.com/openai/deployments/{deployment}/
        let url_parts: Vec<&str> = config.base_url.split('/').collect();
        if url_parts.len() < 6 || !url_parts[2].contains(".openai.azure.com") {
            return Err(RlmError::Config(
                "Invalid Azure OpenAI URL format".to_string(),
            ));
        }

        let resource_name = url_parts[2]
            .split('.')
            .next()
            .ok_or_else(|| RlmError::Config("Cannot extract resource name from URL".to_string()))?
            .to_string();

        let deployment_name = url_parts
            .get(5)
            .ok_or_else(|| RlmError::Config("Cannot extract deployment name from URL".to_string()))?
            .trim_end_matches('/')
            .to_string();

        let api_version = config
            .provider_options
            .get("api_version")
            .unwrap_or(&"2024-02-01".to_string())
            .clone();

        let mut azure_config = Self::new(resource_name, deployment_name, config.api_key.clone());
        azure_config.api_version = api_version;
        azure_config.timeout_seconds = config.timeout_ms / 1000;
        azure_config.max_retries = config.retry_policy.max_attempts;
        azure_config.model_mappings = config.model_mapping.clone();

        Ok(azure_config)
    }

    /// Get the physical deployment name for a logical model name.
    pub fn get_deployment_name(&self, logical_model: &str) -> Option<&String> {
        self.model_mappings.get(logical_model)
    }

    /// Add a model mapping.
    pub fn add_model_mapping(&mut self, logical_name: String, deployment_name: String) {
        self.model_mappings.insert(logical_name, deployment_name);
    }
}

/// Azure OpenAI API error response (same structure as OpenAI).
#[derive(Debug, Deserialize)]
struct AzureOpenAiError {
    error: AzureOpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct AzureOpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: Option<String>,
    #[allow(dead_code)]
    param: Option<String>,
    #[allow(dead_code)]
    code: Option<String>,
}

/// Azure OpenAI LLM provider implementation.
pub struct AzureOpenAiProvider {
    config: AzureOpenAiConfig,
    client: Client,
}

impl std::fmt::Debug for AzureOpenAiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureOpenAiProvider")
            .field("config", &self.config)
            .field("client", &"<reqwest::Client>")
            .finish()
    }
}

impl AzureOpenAiProvider {
    /// Create a new Azure OpenAI provider with the given configuration.
    pub fn new(config: AzureOpenAiConfig) -> RlmResult<Self> {
        if config.api_key.is_empty() {
            return Err(RlmError::Config(
                "Azure OpenAI API key is required".to_string(),
            ));
        }

        if config.resource_name.is_empty() {
            return Err(RlmError::Config(
                "Azure resource name is required".to_string(),
            ));
        }

        if config.deployment_name.is_empty() {
            return Err(RlmError::Config(
                "Azure deployment name is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| RlmError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create a new Azure OpenAI provider from environment variables.
    pub fn from_env() -> RlmResult<Self> {
        let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
            .map_err(|_| RlmError::Config("AZURE_OPENAI_RESOURCE environment variable not set".to_string()))?;

        let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
            .map_err(|_| RlmError::Config("AZURE_OPENAI_DEPLOYMENT environment variable not set".to_string()))?;

        let api_key = std::env::var("AZURE_OPENAI_API_KEY")
            .map_err(|_| RlmError::Config("AZURE_OPENAI_API_KEY environment variable not set".to_string()))?;

        let api_version = std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-01".to_string());

        let mut config = AzureOpenAiConfig::new(resource_name, deployment_name, api_key);
        config.api_version = api_version;

        Self::new(config)
    }

    /// Create from a BackendConfig.
    pub fn from_backend_config(backend_config: &BackendConfig) -> RlmResult<Self> {
        let azure_config = AzureOpenAiConfig::from_backend_config(backend_config)?;
        Self::new(azure_config)
    }

    /// Convert Azure OpenAI API error to RLM error.
    fn convert_error(&self, status: StatusCode, body: &str) -> RlmError {
        match serde_json::from_str::<AzureOpenAiError>(body) {
            Ok(error) => {
                let message = format!(
                    "Azure OpenAI API error ({}): {}",
                    status,
                    error.error.message
                );
                RlmError::Llm(message)
            }
            Err(_) => {
                RlmError::Llm(format!("Azure OpenAI API error ({}): {}", status, body))
            }
        }
    }

    /// Make a request to the Azure OpenAI API with retries.
    async fn make_request(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        // For Azure, we need to use the deployment-specific URL
        let deployment_name = self.config
            .get_deployment_name(&request.model)
            .unwrap_or(&self.config.deployment_name);

        let url = format!(
            "https://{}.openai.azure.com/openai/deployments/{}/chat/completions?api-version={}",
            self.config.resource_name,
            deployment_name,
            self.config.api_version
        );

        for attempt in 0..=self.config.max_retries {
            if self.config.enable_logging {
                debug!(
                    "Making Azure OpenAI API request (attempt {}) to deployment: {}",
                    attempt + 1,
                    deployment_name
                );
            }

            // Azure uses api-key header instead of Authorization Bearer
            let response = self.client
                .post(&url)
                .header("api-key", &self.config.api_key)
                .header("Content-Type", "application/json")
                .json(request)
                .send()
                .await
                .map_err(|e| RlmError::Llm(format!("Azure OpenAI request failed: {}", e)))?;

            let status = response.status();
            let body = response.text().await
                .map_err(|e| RlmError::Llm(format!("Failed to read response body: {}", e)))?;

            if status.is_success() {
                let completion_response: ChatCompletionResponse = serde_json::from_str(&body)
                    .map_err(|e| RlmError::Llm(format!("Failed to parse Azure OpenAI response: {}", e)))?;

                if self.config.enable_logging {
                    debug!("Azure OpenAI API request successful");
                }

                return Ok(completion_response);
            } else if status == StatusCode::TOO_MANY_REQUESTS && attempt < self.config.max_retries {
                // Retry rate limit errors with exponential backoff (more aggressive for Azure)
                let delay = Duration::from_millis(2000 * (2_u64.pow(attempt)));
                warn!("Azure OpenAI rate limited, retrying in {:?}", delay);
                tokio::time::sleep(delay).await;
                continue;
            } else {
                return Err(self.convert_error(status, &body));
            }
        }

        Err(RlmError::Llm("Azure OpenAI max retries exceeded".to_string()))
    }
}

#[async_trait]
impl LlmProvider for AzureOpenAiProvider {
    #[instrument(skip(self, request), fields(model = %request.model, deployment = %self.config.deployment_name))]
    async fn complete(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        debug!("Starting Azure OpenAI chat completion");

        // Use the model from the request or fall back to default deployment
        let mut request = request.clone();
        if request.model.is_empty() {
            request.model = self.config.deployment_name.clone();
        }

        // Ensure stream is false for non-streaming completion
        request.stream = false;

        self.make_request(&request).await
    }

    async fn complete_stream(
        &self,
        _request: &ChatCompletionRequest,
    ) -> RlmResult<Pin<Box<dyn Stream<Item = RlmResult<ChatCompletionResponse>> + Send>>> {
        // TODO: Implement streaming support for Azure OpenAI
        Err(RlmError::Other("Streaming not yet implemented for Azure OpenAI provider".to_string()))
    }

    async fn list_models(&self) -> RlmResult<Vec<ModelInfo>> {
        debug!("Fetching Azure OpenAI deployments (models)");

        // For Azure, we need to list deployments rather than models
        // This requires the management API which needs different authentication
        // For now, we'll return the configured deployments
        let models = vec![
            ModelInfo::new(
                self.config.deployment_name.clone(),
                format!("Azure Deployment: {}", self.config.deployment_name),
                128_000, // Assume GPT-4 context length
                4_096,   // Assume GPT-4 output length
            )
            .with_streaming(false) // Not implemented yet
            .with_functions(true),
        ];

        // Add additional mappings if configured
        let mut all_models = models;
        for (logical_name, deployment_name) in &self.config.model_mappings {
            if *deployment_name != self.config.deployment_name {
                all_models.push(
                    ModelInfo::new(
                        logical_name.clone(),
                        format!("Azure Deployment: {}", deployment_name),
                        128_000,
                        4_096,
                    )
                    .with_streaming(false)
                    .with_functions(true),
                );
            }
        }

        debug!("Retrieved {} Azure deployments", all_models.len());
        Ok(all_models)
    }

    fn validate_request(&self, request: &ChatCompletionRequest) -> RlmResult<()> {
        if request.messages.is_empty() {
            return Err(RlmError::Other("Request must contain at least one message".to_string()));
        }

        if request.model.is_empty() && self.config.deployment_name.is_empty() {
            return Err(RlmError::Config("No model specified and no default deployment configured".to_string()));
        }

        // Check token limits (Azure has similar limits to OpenAI)
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens == 0 {
                return Err(RlmError::Other("max_tokens must be greater than 0".to_string()));
            }
            if max_tokens > 4096 {
                return Err(RlmError::Other("max_tokens exceeds Azure OpenAI limits".to_string()));
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
            "Azure OpenAI",
            "1.0.0",
            format!("https://{}.openai.azure.com", self.config.resource_name),
            true, // requires_auth
        )
        .with_rate_limits(240, 40_000) // Typical Azure limits: 240 RPM, 40k TPM
        .with_capabilities(vec![
            "chat_completions".to_string(),
            "azure_deployments".to_string(),
            "function_calling".to_string(),
        ])
    }

    async fn health_check(&self) -> RlmResult<bool> {
        debug!("Performing Azure OpenAI provider health check");

        // Simple health check - try to make a minimal completion request
        let test_request = ChatCompletionRequest {
            model: self.config.deployment_name.clone(),
            messages: vec![rlm_core::types::ChatMessage {
                role: rlm_core::types::ChatRole::User,
                content: "Hi".to_string(),
                name: None,
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        match self.complete(&test_request).await {
            Ok(_) => {
                debug!("Azure OpenAI provider health check passed");
                Ok(true)
            }
            Err(e) => {
                warn!("Azure OpenAI provider health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

/// Builder for Azure OpenAI configuration.
#[derive(Debug)]
pub struct AzureOpenAiConfigBuilder {
    config: AzureOpenAiConfig,
}

impl AzureOpenAiConfigBuilder {
    /// Create a new config builder with required Azure parameters.
    pub fn new(
        resource_name: impl Into<String>,
        deployment_name: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            config: AzureOpenAiConfig::new(
                resource_name.into(),
                deployment_name.into(),
                api_key.into(),
            ),
        }
    }

    /// Set the API version.
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.config.api_version = version.into();
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

    /// Add a model mapping (logical name -> deployment name).
    pub fn model_mapping(mut self, logical_name: impl Into<String>, deployment_name: impl Into<String>) -> Self {
        self.config.model_mappings.insert(logical_name.into(), deployment_name.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> AzureOpenAiConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::types::{ChatMessage, ChatRole};

    #[test]
    fn test_azure_config_creation() {
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "gpt-4-deployment".to_string(),
            "test-key".to_string(),
        );

        assert_eq!(config.resource_name, "test-resource");
        assert_eq!(config.deployment_name, "gpt-4-deployment");
        assert_eq!(config.api_key, "test-key");
        assert!(config.base_url.contains("test-resource.openai.azure.com"));
        assert!(config.base_url.contains("gpt-4-deployment"));
        assert_eq!(config.api_version, "2024-02-01");
    }

    #[test]
    fn test_azure_config_builder() {
        let config = AzureOpenAiConfigBuilder::new("resource", "deployment", "key")
            .api_version("2023-12-01-preview")
            .timeout_seconds(60)
            .max_retries(3)
            .enable_logging()
            .model_mapping("gpt-4", "my-gpt4-deployment")
            .build();

        assert_eq!(config.resource_name, "resource");
        assert_eq!(config.deployment_name, "deployment");
        assert_eq!(config.api_version, "2023-12-01-preview");
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_logging);
        assert_eq!(
            config.get_deployment_name("gpt-4"),
            Some(&"my-gpt4-deployment".to_string())
        );
    }

    #[test]
    fn test_azure_provider_creation() {
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "test-deployment".to_string(),
            "test-key".to_string(),
        );

        let provider = AzureOpenAiProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_azure_provider_creation_missing_fields() {
        // Missing API key
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "test-deployment".to_string(),
            "".to_string(),
        );
        assert!(AzureOpenAiProvider::new(config).is_err());

        // Missing resource name
        let config = AzureOpenAiConfig::new(
            "".to_string(),
            "test-deployment".to_string(),
            "test-key".to_string(),
        );
        assert!(AzureOpenAiProvider::new(config).is_err());

        // Missing deployment name
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "".to_string(),
            "test-key".to_string(),
        );
        assert!(AzureOpenAiProvider::new(config).is_err());
    }

    #[test]
    fn test_azure_provider_metadata() {
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "test-deployment".to_string(),
            "test-key".to_string(),
        );
        let provider = AzureOpenAiProvider::new(config).unwrap();
        let metadata = provider.get_metadata();

        assert_eq!(metadata.name, "Azure OpenAI");
        assert!(metadata.base_url.contains("test-resource.openai.azure.com"));
        assert!(metadata.requires_auth);
        assert!(metadata.supports("chat_completions"));
        assert!(metadata.supports("azure_deployments"));
    }

    #[test]
    fn test_azure_request_validation() {
        let config = AzureOpenAiConfig::new(
            "test-resource".to_string(),
            "test-deployment".to_string(),
            "test-key".to_string(),
        );
        let provider = AzureOpenAiProvider::new(config).unwrap();

        // Valid request
        let valid_request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello Azure".to_string(),
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

        // Invalid request - empty messages
        let invalid_request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };
        assert!(provider.validate_request(&invalid_request).is_err());
    }

    #[test]
    fn test_backend_config_conversion() {
        let backend_config = BackendConfig::azure_openai(
            "test-resource".to_string(),
            "gpt-4-deployment".to_string(),
            "test-key".to_string(),
            Some("2024-03-01".to_string()),
        );

        let azure_config = AzureOpenAiConfig::from_backend_config(&backend_config).unwrap();

        assert_eq!(azure_config.resource_name, "test-resource");
        assert_eq!(azure_config.deployment_name, "gpt-4-deployment");
        assert_eq!(azure_config.api_key, "test-key");
        assert_eq!(azure_config.api_version, "2024-03-01");
    }

    #[test]
    fn test_azure_provider_from_backend_config() {
        let backend_config = BackendConfig::azure_openai(
            "test-resource".to_string(),
            "gpt-4-deployment".to_string(),
            "test-key".to_string(),
            None, // Use default API version
        );

        let provider = AzureOpenAiProvider::from_backend_config(&backend_config);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        let metadata = provider.get_metadata();
        assert_eq!(metadata.name, "Azure OpenAI");
        assert!(metadata.base_url.contains("test-resource.openai.azure.com"));
    }
}