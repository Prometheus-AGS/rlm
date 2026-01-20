//! Backend configuration types for multi-provider support.
//!
//! This module defines the configuration structure for different LLM providers
//! including OpenAI, Azure OpenAI, and local models. It provides type-safe
//! configuration management with validation and serialization support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Type of LLM provider backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// OpenAI API (api.openai.com)
    OpenAi,
    /// Azure OpenAI Service
    AzureOpenAi,
    /// Local model server (Ollama, vLLM, etc.)
    LocalModel,
    /// Other OpenAI-compatible provider
    Custom,
}

/// Retry policy configuration for backend requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts.
    pub max_attempts: u32,
    /// Initial backoff delay in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 10000,
        }
    }
}

/// Rate limiting configuration for backend requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Maximum requests per minute.
    pub requests_per_minute: u32,
    /// Maximum tokens per minute.
    pub tokens_per_minute: u32,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 3500,
            tokens_per_minute: 90000,
        }
    }
}

/// Complete backend configuration for an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Provider type.
    pub provider_type: ProviderType,
    /// API base URL.
    pub base_url: String,
    /// API authentication key.
    pub api_key: String,
    /// Mapping from logical model names to physical model names.
    pub model_mapping: HashMap<String, String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Retry policy for failed requests.
    pub retry_policy: RetryPolicy,
    /// Rate limiting configuration.
    pub rate_limits: RateLimits,
    /// Optional health check URL override.
    pub health_check_url: Option<String>,
    /// Provider-specific configuration options.
    #[serde(default)]
    pub provider_options: HashMap<String, String>,
}

impl BackendConfig {
    /// Create a new backend configuration.
    pub fn new(
        provider_type: ProviderType,
        base_url: String,
        api_key: String,
    ) -> Self {
        Self {
            provider_type,
            base_url,
            api_key,
            model_mapping: HashMap::new(),
            timeout_ms: 30000, // 30 seconds default
            retry_policy: RetryPolicy::default(),
            rate_limits: RateLimits::default(),
            health_check_url: None,
            provider_options: HashMap::new(),
        }
    }

    /// Add a model mapping from logical name to physical name.
    pub fn with_model_mapping(mut self, logical_name: String, physical_name: String) -> Self {
        self.model_mapping.insert(logical_name, physical_name);
        self
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_ms = timeout.as_millis() as u64;
        self
    }

    /// Set the retry policy.
    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Set the rate limits.
    pub fn with_rate_limits(mut self, rate_limits: RateLimits) -> Self {
        self.rate_limits = rate_limits;
        self
    }

    /// Set a custom health check URL.
    pub fn with_health_check_url(mut self, url: String) -> Self {
        self.health_check_url = Some(url);
        self
    }

    /// Add a provider-specific option.
    pub fn with_provider_option(mut self, key: String, value: String) -> Self {
        self.provider_options.insert(key, value);
        self
    }

    /// Get the health check URL, using default if not specified.
    pub fn health_check_url(&self) -> String {
        match &self.health_check_url {
            Some(url) => url.clone(),
            None => format!("{}/models", self.base_url.trim_end_matches('/')),
        }
    }

    /// Get the mapped model name for a logical model name.
    pub fn get_physical_model(&self, logical_model: &str) -> Option<&String> {
        self.model_mapping.get(logical_model)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), BackendConfigError> {
        // Validate base URL
        if self.base_url.is_empty() {
            return Err(BackendConfigError::InvalidBaseUrl("Base URL cannot be empty".to_string()));
        }

        // Validate URL format
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(BackendConfigError::InvalidBaseUrl("Base URL must start with http:// or https://".to_string()));
        }

        // Validate API key (except for local models where it might not be required)
        if matches!(self.provider_type, ProviderType::OpenAi | ProviderType::AzureOpenAi) {
            if self.api_key.is_empty() {
                return Err(BackendConfigError::InvalidApiKey("API key is required for this provider".to_string()));
            }
        }

        // Validate timeout
        if self.timeout_ms == 0 {
            return Err(BackendConfigError::InvalidTimeout("Timeout must be greater than 0".to_string()));
        }

        if self.timeout_ms > 300000 { // 5 minutes max
            return Err(BackendConfigError::InvalidTimeout("Timeout cannot exceed 5 minutes".to_string()));
        }

        // Validate retry policy
        if self.retry_policy.max_attempts == 0 {
            return Err(BackendConfigError::InvalidRetryPolicy("Max attempts must be greater than 0".to_string()));
        }

        if self.retry_policy.initial_backoff_ms == 0 {
            return Err(BackendConfigError::InvalidRetryPolicy("Initial backoff must be greater than 0".to_string()));
        }

        if self.retry_policy.max_backoff_ms < self.retry_policy.initial_backoff_ms {
            return Err(BackendConfigError::InvalidRetryPolicy("Max backoff must be >= initial backoff".to_string()));
        }

        // Validate rate limits
        if self.rate_limits.requests_per_minute == 0 {
            return Err(BackendConfigError::InvalidRateLimits("Requests per minute must be greater than 0".to_string()));
        }

        if self.rate_limits.tokens_per_minute == 0 {
            return Err(BackendConfigError::InvalidRateLimits("Tokens per minute must be greater than 0".to_string()));
        }

        Ok(())
    }
}

/// Errors that can occur during backend configuration validation.
#[derive(Debug, thiserror::Error)]
pub enum BackendConfigError {
    /// Invalid base URL format or content.
    #[error("Invalid base URL: {0}")]
    InvalidBaseUrl(String),

    /// Invalid or missing API key.
    #[error("Invalid API key: {0}")]
    InvalidApiKey(String),

    /// Invalid timeout value.
    #[error("Invalid timeout: {0}")]
    InvalidTimeout(String),

    /// Invalid retry policy configuration.
    #[error("Invalid retry policy: {0}")]
    InvalidRetryPolicy(String),

    /// Invalid rate limiting configuration.
    #[error("Invalid rate limits: {0}")]
    InvalidRateLimits(String),
}

/// Predefined configurations for common providers.
impl BackendConfig {
    /// Create OpenAI configuration with reasonable defaults.
    pub fn openai(api_key: String) -> Self {
        let mut config = Self::new(
            ProviderType::OpenAi,
            "https://api.openai.com/v1".to_string(),
            api_key,
        );

        // Add common model mappings
        config = config
            .with_model_mapping("gpt-4".to_string(), "gpt-4".to_string())
            .with_model_mapping("gpt-3.5-turbo".to_string(), "gpt-3.5-turbo".to_string())
            .with_model_mapping("gpt-4-turbo".to_string(), "gpt-4-turbo-preview".to_string());

        // OpenAI rate limits (Tier 2)
        config = config.with_rate_limits(RateLimits {
            requests_per_minute: 3500,
            tokens_per_minute: 90000,
        });

        config
    }

    /// Create Azure OpenAI configuration.
    pub fn azure_openai(
        resource_name: String,
        deployment_name: String,
        api_key: String,
        api_version: Option<String>,
    ) -> Self {
        let api_version = api_version.unwrap_or_else(|| "2024-02-01".to_string());
        let base_url = format!(
            "https://{}.openai.azure.com/openai/deployments/{}/",
            resource_name, deployment_name
        );

        let mut config = Self::new(
            ProviderType::AzureOpenAi,
            base_url,
            api_key,
        );

        // For Azure, the deployment name is the physical model
        config = config.with_model_mapping("gpt-4".to_string(), deployment_name.clone());

        // Add API version to provider options
        config = config.with_provider_option("api_version".to_string(), api_version);

        // Azure typically has lower rate limits
        config = config.with_rate_limits(RateLimits {
            requests_per_minute: 240,
            tokens_per_minute: 40000,
        });

        // Longer timeout for Azure
        config = config.with_timeout(Duration::from_secs(45));

        // More aggressive retry policy
        config = config.with_retry_policy(RetryPolicy {
            max_attempts: 5,
            initial_backoff_ms: 2000,
            max_backoff_ms: 30000,
        });

        config
    }

    /// Create local model configuration.
    pub fn local_model(host: String, port: u16, model_name: String) -> Self {
        let base_url = format!("http://{}:{}/v1", host, port);

        let mut config = Self::new(
            ProviderType::LocalModel,
            base_url,
            "not-required".to_string(), // Local models often don't need API keys
        );

        // Map to the local model name
        config = config.with_model_mapping(model_name.clone(), model_name);

        // Local models usually need more time
        config = config.with_timeout(Duration::from_secs(60));

        // Conservative rate limits for local models
        config = config.with_rate_limits(RateLimits {
            requests_per_minute: 60,
            tokens_per_minute: 10000,
        });

        // Fewer retries for local (faster to fail)
        config = config.with_retry_policy(RetryPolicy {
            max_attempts: 2,
            initial_backoff_ms: 500,
            max_backoff_ms: 5000,
        });

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_config_creation() {
        let config = BackendConfig::new(
            ProviderType::OpenAi,
            "https://api.openai.com/v1".to_string(),
            "test-key".to_string(),
        );

        assert_eq!(config.provider_type, ProviderType::OpenAi);
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_openai_config_preset() {
        let config = BackendConfig::openai("test-key".to_string());

        assert_eq!(config.provider_type, ProviderType::OpenAi);
        assert!(config.model_mapping.contains_key("gpt-4"));
        assert_eq!(config.rate_limits.requests_per_minute, 3500);
    }

    #[test]
    fn test_azure_config_preset() {
        let config = BackendConfig::azure_openai(
            "test-resource".to_string(),
            "gpt-4-deployment".to_string(),
            "test-key".to_string(),
            None,
        );

        assert_eq!(config.provider_type, ProviderType::AzureOpenAi);
        assert!(config.base_url.contains("azure.com"));
        assert!(config.base_url.contains("gpt-4-deployment"));
        assert_eq!(config.timeout_ms, 45000);
    }

    #[test]
    fn test_local_model_config_preset() {
        let config = BackendConfig::local_model(
            "localhost".to_string(),
            8080,
            "llama-7b".to_string(),
        );

        assert_eq!(config.provider_type, ProviderType::LocalModel);
        assert_eq!(config.base_url, "http://localhost:8080/v1");
        assert!(config.model_mapping.contains_key("llama-7b"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = BackendConfig::openai("valid-key".to_string());
        assert!(config.validate().is_ok());

        // Test empty base URL
        config.base_url = "".to_string();
        assert!(config.validate().is_err());

        // Test invalid URL format
        config.base_url = "not-a-url".to_string();
        assert!(config.validate().is_err());

        // Test empty API key for OpenAI
        config.base_url = "https://api.openai.com/v1".to_string();
        config.api_key = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_health_check_url() {
        let config = BackendConfig::openai("test-key".to_string());
        assert_eq!(config.health_check_url(), "https://api.openai.com/v1/models");

        let config_with_custom = config.with_health_check_url("https://custom.health/check".to_string());
        assert_eq!(config_with_custom.health_check_url(), "https://custom.health/check");
    }

    #[test]
    fn test_model_mapping() {
        let config = BackendConfig::new(
            ProviderType::Custom,
            "https://api.example.com/v1".to_string(),
            "test-key".to_string(),
        ).with_model_mapping("gpt-4".to_string(), "custom-model-v1".to_string());

        assert_eq!(config.get_physical_model("gpt-4"), Some(&"custom-model-v1".to_string()));
        assert_eq!(config.get_physical_model("nonexistent"), None);
    }
}