//! LLM providers for integration testing.
//!
//! This module provides test-specific implementations of LLM providers
//! that integrate with real APIs for comprehensive testing while
//! maintaining security through vault-managed API keys.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use reqwest::Client;

use crate::error::{TestError, TestResult};
use crate::vault::KeyVaultProvider;

/// Supported LLM provider types for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestProviderType {
    /// OpenAI GPT models.
    OpenAi,
    /// Anthropic Claude models.
    Anthropic,
    /// Mock provider for unit testing.
    Mock,
    /// Custom/local model provider.
    Custom,
}

/// Configuration for LLM test providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTestProviderConfig {
    /// Provider type.
    pub provider_type: TestProviderType,
    /// Model name.
    pub model: String,
    /// API key vault key.
    pub api_key_vault_key: String,
    /// Base URL (optional).
    pub base_url: Option<String>,
    /// Request timeout in seconds.
    pub timeout_seconds: u64,
    /// Rate limit (requests per minute).
    pub rate_limit_rpm: u32,
    /// Additional provider-specific options.
    pub options: HashMap<String, String>,
}

/// Test-specific LLM provider trait.
#[async_trait]
pub trait LlmTestProvider: Send + Sync {
    /// Send a completion request.
    async fn complete(&self, request: &CompletionRequest) -> TestResult<CompletionResponse>;

    /// Test provider connectivity.
    async fn health_check(&self) -> TestResult<HealthStatus>;

    /// Get provider metadata.
    fn metadata(&self) -> ProviderMetadata;

    /// Get current rate limit status.
    async fn rate_limit_status(&self) -> TestResult<RateLimitStatus>;
}

/// Completion request for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The prompt/messages.
    pub messages: Vec<ChatMessage>,
    /// Model to use.
    pub model: String,
    /// Temperature (0.0-2.0).
    pub temperature: f32,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Additional parameters.
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Chat message for completion requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role.
    pub role: String,
    /// Message content.
    pub content: String,
}

/// Completion response from LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content.
    pub content: String,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Response metadata.
    pub metadata: ResponseMetadata,
    /// Raw response (for debugging).
    pub raw_response: serde_json::Value,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt tokens consumed.
    pub prompt_tokens: u32,
    /// Completion tokens generated.
    pub completion_tokens: u32,
    /// Total tokens used.
    pub total_tokens: u32,
}

/// Response metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Model used for the request.
    pub model: String,
    /// Request ID (if available).
    pub request_id: Option<String>,
    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,
}

/// Health status of a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Whether the provider is healthy.
    pub is_healthy: bool,
    /// Status message.
    pub message: String,
    /// Response time in milliseconds.
    pub response_time_ms: u64,
    /// Additional status details.
    pub details: HashMap<String, String>,
}

/// Provider metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    /// Provider type.
    pub provider_type: TestProviderType,
    /// Provider name.
    pub name: String,
    /// Supported models.
    pub supported_models: Vec<String>,
    /// API endpoint.
    pub endpoint: String,
    /// Additional metadata.
    pub extra: HashMap<String, String>,
}

/// Rate limit status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    /// Requests remaining in current window.
    pub requests_remaining: u32,
    /// Time until reset in seconds.
    pub reset_time_seconds: u64,
    /// Request limit per minute.
    pub limit_per_minute: u32,
}

/// OpenAI provider for testing.
#[derive(Debug)]
pub struct OpenAiTestProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: LlmTestProviderConfig,
}

impl OpenAiTestProvider {
    /// Create a new OpenAI test provider.
    pub async fn new(config: LlmTestProviderConfig, vault: Arc<dyn KeyVaultProvider>) -> TestResult<Self> {
        let api_key = vault.get_secret(&config.api_key_vault_key).await
            .map_err(|e| TestError::provider(format!("Failed to get OpenAI API key: {}", e)))?;

        if !api_key.starts_with("sk-") {
            return Err(TestError::provider("Invalid OpenAI API key format"));
        }

        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| TestError::provider(format!("Failed to create HTTP client: {}", e)))?;

        info!("Initialized OpenAI test provider for model: {}", config.model);

        Ok(Self {
            client,
            api_key,
            base_url,
            config,
        })
    }
}

#[async_trait]
impl LlmTestProvider for OpenAiTestProvider {
    #[instrument(skip(self, request))]
    async fn complete(&self, request: &CompletionRequest) -> TestResult<CompletionResponse> {
        let start_time = Instant::now();

        let openai_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens
        });

        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| TestError::provider(format!("OpenAI API request failed: {}", e)))?;

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(TestError::provider(format!("OpenAI API error {}: {}", status, error_text)));
        }

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| TestError::provider(format!("Failed to parse OpenAI response: {}", e)))?;

        // Extract response data
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: response_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response_json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let metadata = ResponseMetadata {
            model: response_json["model"].as_str().unwrap_or(&request.model).to_string(),
            request_id: response_json["id"].as_str().map(|s| s.to_string()),
            processing_time_ms,
            estimated_cost_usd: Self::estimate_cost(&usage, &request.model),
        };

        Ok(CompletionResponse {
            content,
            usage,
            metadata,
            raw_response: response_json,
        })
    }

    async fn health_check(&self) -> TestResult<HealthStatus> {
        let start_time = Instant::now();

        let test_request = CompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Say 'OK' if you can hear me.".to_string(),
            }],
            model: self.config.model.clone(),
            temperature: 0.0,
            max_tokens: 5,
            parameters: HashMap::new(),
        };

        match self.complete(&test_request).await {
            Ok(_) => {
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(HealthStatus {
                    is_healthy: true,
                    message: "OpenAI API is accessible".to_string(),
                    response_time_ms,
                    details: HashMap::new(),
                })
            }
            Err(e) => {
                let response_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(HealthStatus {
                    is_healthy: false,
                    message: format!("OpenAI API health check failed: {}", e),
                    response_time_ms,
                    details: HashMap::new(),
                })
            }
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        let mut extra = HashMap::new();
        extra.insert("api_version".to_string(), "v1".to_string());
        extra.insert("timeout_seconds".to_string(), self.config.timeout_seconds.to_string());

        ProviderMetadata {
            provider_type: TestProviderType::OpenAi,
            name: "OpenAI".to_string(),
            supported_models: vec![
                "gpt-4".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4o".to_string(),
                "gpt-5".to_string(),
            ],
            endpoint: self.base_url.clone(),
            extra,
        }
    }

    async fn rate_limit_status(&self) -> TestResult<RateLimitStatus> {
        // OpenAI doesn't provide a rate limit status endpoint
        // Return estimated status based on configuration
        Ok(RateLimitStatus {
            requests_remaining: self.config.rate_limit_rpm,
            reset_time_seconds: 60,
            limit_per_minute: self.config.rate_limit_rpm,
        })
    }
}

impl OpenAiTestProvider {
    /// Estimate API cost for a request.
    fn estimate_cost(usage: &TokenUsage, model: &str) -> f64 {
        // Rough cost estimates (USD per 1K tokens) - these would need regular updates
        let (prompt_cost_per_1k, completion_cost_per_1k) = match model {
            "gpt-4" => (0.03, 0.06),
            "gpt-4-turbo" => (0.01, 0.03),
            "gpt-4o" => (0.005, 0.015),
            "gpt-5" => (0.01, 0.03), // Estimated
            _ => (0.01, 0.03), // Default estimate
        };

        let prompt_cost = (usage.prompt_tokens as f64 / 1000.0) * prompt_cost_per_1k;
        let completion_cost = (usage.completion_tokens as f64 / 1000.0) * completion_cost_per_1k;

        prompt_cost + completion_cost
    }
}

/// Mock LLM provider for unit testing.
#[derive(Debug)]
pub struct MockLlmProvider {
    config: LlmTestProviderConfig,
    responses: Vec<String>,
    response_index: std::sync::atomic::AtomicUsize,
}

impl MockLlmProvider {
    /// Create a new mock provider with predefined responses.
    pub fn new(config: LlmTestProviderConfig, responses: Vec<String>) -> Self {
        Self {
            config,
            responses,
            response_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create a mock provider with a single response.
    pub fn with_response(response: String) -> Self {
        let config = LlmTestProviderConfig {
            provider_type: TestProviderType::Mock,
            model: "mock-model".to_string(),
            api_key_vault_key: "mock-key".to_string(),
            base_url: None,
            timeout_seconds: 30,
            rate_limit_rpm: 1000,
            options: HashMap::new(),
        };

        Self::new(config, vec![response])
    }
}

#[async_trait]
impl LlmTestProvider for MockLlmProvider {
    async fn complete(&self, request: &CompletionRequest) -> TestResult<CompletionResponse> {
        use std::sync::atomic::Ordering;

        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(100)).await;

        let index = self.response_index.fetch_add(1, Ordering::SeqCst) % self.responses.len();
        let content = self.responses[index].clone();

        let usage = TokenUsage {
            prompt_tokens: request.messages.iter()
                .map(|m| m.content.len() / 4) // Rough token estimate
                .sum::<usize>() as u32,
            completion_tokens: content.len() as u32 / 4,
            total_tokens: 0,
        };
        let total_tokens = usage.prompt_tokens + usage.completion_tokens;

        Ok(CompletionResponse {
            content,
            usage: TokenUsage { total_tokens, ..usage },
            metadata: ResponseMetadata {
                model: request.model.clone(),
                request_id: Some(uuid::Uuid::new_v4().to_string()),
                processing_time_ms: 100,
                estimated_cost_usd: 0.001,
            },
            raw_response: serde_json::json!({
                "mock": true,
                "model": request.model
            }),
        })
    }

    async fn health_check(&self) -> TestResult<HealthStatus> {
        Ok(HealthStatus {
            is_healthy: true,
            message: "Mock provider is always healthy".to_string(),
            response_time_ms: 1,
            details: HashMap::new(),
        })
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            provider_type: TestProviderType::Mock,
            name: "Mock Provider".to_string(),
            supported_models: vec!["mock-model".to_string()],
            endpoint: "mock://localhost".to_string(),
            extra: HashMap::new(),
        }
    }

    async fn rate_limit_status(&self) -> TestResult<RateLimitStatus> {
        Ok(RateLimitStatus {
            requests_remaining: 1000,
            reset_time_seconds: 60,
            limit_per_minute: 1000,
        })
    }
}

/// Anthropic Claude provider for testing.
#[derive(Debug)]
pub struct AnthropicTestProvider {
    client: Client,
    api_key: String,
    base_url: String,
    config: LlmTestProviderConfig,
}

impl AnthropicTestProvider {
    /// Create a new Anthropic test provider.
    pub async fn new(config: LlmTestProviderConfig, vault: Arc<dyn KeyVaultProvider>) -> TestResult<Self> {
        let api_key = vault.get_secret(&config.api_key_vault_key).await
            .map_err(|e| TestError::provider(format!("Failed to get Anthropic API key: {}", e)))?;

        let base_url = config.base_url.clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| TestError::provider(format!("Failed to create HTTP client: {}", e)))?;

        info!("Initialized Anthropic test provider for model: {}", config.model);

        Ok(Self {
            client,
            api_key,
            base_url,
            config,
        })
    }
}

#[async_trait]
impl LlmTestProvider for AnthropicTestProvider {
    #[instrument(skip(self, _request))]
    async fn complete(&self, _request: &CompletionRequest) -> TestResult<CompletionResponse> {
        // TODO: Implement Anthropic API integration
        // This would be similar to OpenAI but using Anthropic's message format

        Err(TestError::provider("Anthropic provider not yet implemented"))
    }

    async fn health_check(&self) -> TestResult<HealthStatus> {
        Ok(HealthStatus {
            is_healthy: false,
            message: "Anthropic provider not yet implemented".to_string(),
            response_time_ms: 0,
            details: HashMap::new(),
        })
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            provider_type: TestProviderType::Anthropic,
            name: "Anthropic".to_string(),
            supported_models: vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
            ],
            endpoint: self.base_url.clone(),
            extra: HashMap::new(),
        }
    }

    async fn rate_limit_status(&self) -> TestResult<RateLimitStatus> {
        Ok(RateLimitStatus {
            requests_remaining: self.config.rate_limit_rpm,
            reset_time_seconds: 60,
            limit_per_minute: self.config.rate_limit_rpm,
        })
    }
}

/// Factory for creating LLM test providers.
pub struct LlmTestProviderFactory;

impl LlmTestProviderFactory {
    /// Create a provider from configuration.
    pub async fn create(
        config: LlmTestProviderConfig,
        vault: Arc<dyn KeyVaultProvider>,
    ) -> TestResult<Arc<dyn LlmTestProvider>> {
        match config.provider_type {
            TestProviderType::OpenAi => {
                let provider = OpenAiTestProvider::new(config, vault).await?;
                Ok(Arc::new(provider))
            }
            TestProviderType::Anthropic => {
                let provider = AnthropicTestProvider::new(config, vault).await?;
                Ok(Arc::new(provider))
            }
            TestProviderType::Mock => {
                let responses = vec!["This is a mock response".to_string()];
                let provider = MockLlmProvider::new(config, responses);
                Ok(Arc::new(provider))
            }
            TestProviderType::Custom => {
                Err(TestError::provider("Custom provider type not yet supported"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let responses = vec![
            "First response".to_string(),
            "Second response".to_string(),
        ];
        let config = LlmTestProviderConfig {
            provider_type: TestProviderType::Mock,
            model: "test-model".to_string(),
            api_key_vault_key: "test-key".to_string(),
            base_url: None,
            timeout_seconds: 30,
            rate_limit_rpm: 100,
            options: HashMap::new(),
        };

        let provider = MockLlmProvider::new(config, responses);

        let request = CompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Test message".to_string(),
            }],
            model: "test-model".to_string(),
            temperature: 0.0,
            max_tokens: 100,
            parameters: HashMap::new(),
        };

        // Test first response
        let response1 = provider.complete(&request).await.unwrap();
        assert_eq!(response1.content, "First response");

        // Test second response
        let response2 = provider.complete(&request).await.unwrap();
        assert_eq!(response2.content, "Second response");

        // Test cycling back to first
        let response3 = provider.complete(&request).await.unwrap();
        assert_eq!(response3.content, "First response");

        // Test health check
        let health = provider.health_check().await.unwrap();
        assert!(health.is_healthy);
    }

    #[test]
    fn test_openai_cost_estimation() {
        let usage = TokenUsage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };

        let cost = OpenAiTestProvider::estimate_cost(&usage, "gpt-4");
        assert!(cost > 0.0);
        assert!(cost < 1.0); // Should be reasonable for test amounts
    }
}