//! Builder patterns for RLM configuration and client setup.
//!
//! This module provides fluent builder APIs for configuring and creating
//! RLM clients, executors, and related components with sensible defaults
//! and validation.

use crate::{
    RlmConfig, RlmError, RlmResult,
    config::{ReplConfig, LlmConfig, ExecutionLimits, StreamingConfig},
    ContextAnalysisConfig,
    BackendConfig, ProviderType, RateLimits, RetryPolicy,
};
use std::time::Duration;

/// Builder for RLM core configuration.
#[derive(Debug, Clone)]
pub struct RlmConfigBuilder {
    repl: Option<ReplConfig>,
    llm: Option<LlmConfig>,
    limits: Option<ExecutionLimits>,
    streaming: Option<StreamingConfig>,
}

impl Default for RlmConfigBuilder {
    fn default() -> Self {
        Self {
            repl: None,
            llm: None,
            limits: None,
            streaming: None,
        }
    }
}

impl RlmConfigBuilder {
    /// Create a new RLM config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set REPL configuration.
    pub fn repl_config(mut self, config: ReplConfig) -> Self {
        self.repl = Some(config);
        self
    }

    /// Set LLM configuration.
    pub fn llm_config(mut self, config: LlmConfig) -> Self {
        self.llm = Some(config);
        self
    }

    /// Set execution limits.
    pub fn execution_limits(mut self, limits: ExecutionLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Set streaming configuration.
    pub fn streaming_config(mut self, config: StreamingConfig) -> Self {
        self.streaming = Some(config);
        self
    }

    /// Configure for performance optimization.
    pub fn performance_optimized(mut self) -> Self {
        self.limits = Some(ExecutionLimits {
            max_iterations: 100,
            max_recursion_depth: 5,
            timeout: Duration::from_secs(600),
            chunk_size_tokens: 8192,
        });
        self.streaming = Some(StreamingConfig {
            enabled: true,
            buffer_size: 1000,
            include_repl_state: false,
        });
        self
    }

    /// Configure for memory efficiency.
    pub fn memory_efficient(mut self) -> Self {
        self.repl = Some(ReplConfig {
            backend: "rhai".to_string(),
            sandbox: true,
            max_memory_mb: 128,
            operation_timeout: Duration::from_secs(15),
        });
        self.limits = Some(ExecutionLimits {
            max_iterations: 25,
            max_recursion_depth: 2,
            timeout: Duration::from_secs(300),
            chunk_size_tokens: 2048,
        });
        self.streaming = Some(StreamingConfig {
            enabled: false,
            buffer_size: 50,
            include_repl_state: false,
        });
        self
    }

    /// Configure for development/debugging.
    pub fn development(mut self) -> Self {
        self.streaming = Some(StreamingConfig {
            enabled: true,
            buffer_size: 100,
            include_repl_state: true,
        });
        self.limits = Some(ExecutionLimits {
            max_iterations: 20,
            max_recursion_depth: 3,
            timeout: Duration::from_secs(60),
            chunk_size_tokens: 1024,
        });
        self
    }

    /// Build the RLM configuration.
    pub fn build(self) -> RlmResult<RlmConfig> {
        Ok(RlmConfig {
            repl: self.repl.unwrap_or_default(),
            llm: self.llm.unwrap_or_default(),
            limits: self.limits.unwrap_or_default(),
            streaming: self.streaming.unwrap_or_default(),
        })
    }
}


/// Builder for backend configuration.
#[derive(Debug, Clone)]
pub struct BackendConfigBuilder {
    provider_type: Option<ProviderType>,
    api_key: Option<String>,
    base_url: Option<String>,
    model_mappings: std::collections::HashMap<String, String>,
    timeout: Option<Duration>,
    rate_limits: Option<RateLimits>,
    retry_policy: Option<RetryPolicy>,
    health_check_url: Option<String>,
    provider_options: std::collections::HashMap<String, String>,
}

impl Default for BackendConfigBuilder {
    fn default() -> Self {
        Self {
            provider_type: None,
            api_key: None,
            base_url: None,
            model_mappings: std::collections::HashMap::new(),
            timeout: None,
            rate_limits: None,
            retry_policy: None,
            health_check_url: None,
            provider_options: std::collections::HashMap::new(),
        }
    }
}

impl BackendConfigBuilder {
    /// Create a new backend config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the provider type.
    pub fn provider_type(mut self, provider: ProviderType) -> Self {
        self.provider_type = Some(provider);
        self
    }

    /// Set the API key.
    pub fn api_key<S: Into<String>>(mut self, key: S) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the base URL.
    pub fn base_url<S: Into<String>>(mut self, url: S) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Add a model mapping.
    pub fn model_mapping<S1: Into<String>, S2: Into<String>>(mut self, logical: S1, physical: S2) -> Self {
        self.model_mappings.insert(logical.into(), physical.into());
        self
    }

    /// Set timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set rate limits.
    pub fn rate_limits(mut self, limits: RateLimits) -> Self {
        self.rate_limits = Some(limits);
        self
    }

    /// Set retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Set health check URL.
    pub fn health_check_url<S: Into<String>>(mut self, url: S) -> Self {
        self.health_check_url = Some(url.into());
        self
    }

    /// Add a provider option.
    pub fn provider_option<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.provider_options.insert(key.into(), value.into());
        self
    }

    /// Configure for OpenAI.
    pub fn openai(self) -> Self {
        self
            .provider_type(ProviderType::OpenAi)
            .base_url("https://api.openai.com/v1")
            .model_mapping("gpt-4", "gpt-4")
            .model_mapping("gpt-4o", "gpt-4o")
    }

    /// Configure for Azure OpenAI.
    pub fn azure_openai<S: Into<String>>(self, resource_name: S, deployment_name: S, api_version: Option<S>) -> Self {
        let resource = resource_name.into();
        let deployment = deployment_name.into();
        let version = api_version.map(|v| v.into()).unwrap_or_else(|| "2024-02-01".to_string());

        let base_url = format!("https://{}.openai.azure.com/openai/deployments/{}/", resource, deployment);

        self
            .provider_type(ProviderType::AzureOpenAi)
            .base_url(base_url)
            .model_mapping("gpt-4", deployment.clone())
            .provider_option("api_version", version)
    }

    /// Configure for local model.
    pub fn local_model<S: Into<String>>(self, host: S, port: u16, model_name: S) -> Self {
        let model = model_name.into();
        let base_url = format!("http://{}:{}/v1", host.into(), port);

        self
            .provider_type(ProviderType::LocalModel)
            .base_url(base_url)
            .model_mapping(model.clone(), model)
            .api_key("not-required")
    }

    /// Build the backend configuration.
    pub fn build(self) -> RlmResult<BackendConfig> {
        let provider_type = self.provider_type.ok_or_else(|| {
            RlmError::Config("Provider type is required".to_string())
        })?;

        let api_key = self.api_key.ok_or_else(|| {
            RlmError::Config("API key is required".to_string())
        })?;

        let base_url = self.base_url.ok_or_else(|| {
            RlmError::Config("Base URL is required".to_string())
        })?;

        let timeout_ms = self.timeout.map(|t| t.as_millis() as u64).unwrap_or(30000);

        Ok(BackendConfig {
            provider_type,
            base_url,
            api_key,
            model_mapping: self.model_mappings,
            timeout_ms,
            retry_policy: self.retry_policy.unwrap_or_default(),
            rate_limits: self.rate_limits.unwrap_or_default(),
            health_check_url: self.health_check_url,
            provider_options: self.provider_options,
        })
    }
}

/// Context analysis configuration builder.
#[derive(Debug, Clone)]
pub struct ContextAnalysisConfigBuilder {
    min_chunk_size: Option<usize>,
    max_chunk_size: Option<usize>,
    chunk_overlap: Option<usize>,
    enable_semantic_chunking: Option<bool>,
    language: Option<String>,
}

impl Default for ContextAnalysisConfigBuilder {
    fn default() -> Self {
        Self {
            min_chunk_size: None,
            max_chunk_size: None,
            chunk_overlap: None,
            enable_semantic_chunking: None,
            language: None,
        }
    }
}

impl ContextAnalysisConfigBuilder {
    /// Create a new context analysis config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum chunk size.
    pub fn min_chunk_size(mut self, size: usize) -> Self {
        self.min_chunk_size = Some(size);
        self
    }

    /// Set maximum chunk size.
    pub fn max_chunk_size(mut self, size: usize) -> Self {
        self.max_chunk_size = Some(size);
        self
    }

    /// Set chunk overlap.
    pub fn chunk_overlap(mut self, overlap: usize) -> Self {
        self.chunk_overlap = Some(overlap);
        self
    }

    /// Enable or disable semantic chunking.
    pub fn enable_semantic_chunking(mut self, enable: bool) -> Self {
        self.enable_semantic_chunking = Some(enable);
        self
    }

    /// Set language hint.
    pub fn language<S: Into<String>>(mut self, lang: S) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Configure for code analysis.
    pub fn code_analysis(self) -> Self {
        self
            .min_chunk_size(512)
            .max_chunk_size(8192)
            .chunk_overlap(256)
            .enable_semantic_chunking(true)
            .language("rust")
    }

    /// Configure for document analysis.
    pub fn document_analysis(self) -> Self {
        self
            .min_chunk_size(1024)
            .max_chunk_size(4096)
            .chunk_overlap(512)
            .enable_semantic_chunking(true)
    }

    /// Build the context analysis configuration.
    pub fn build(self) -> RlmResult<ContextAnalysisConfig> {
        let min_size = self.min_chunk_size.unwrap_or(512);
        let max_size = self.max_chunk_size.unwrap_or(8192);

        // Validate chunk sizes
        if min_size >= max_size {
            return Err(RlmError::Config(
                "Minimum chunk size must be smaller than maximum chunk size".to_string()
            ));
        }

        // Validate chunk overlap is reasonable
        let overlap = self.chunk_overlap.unwrap_or(256);
        if overlap >= min_size {
            return Err(RlmError::Config(
                "Chunk overlap must be smaller than minimum chunk size".to_string()
            ));
        }

        Ok(ContextAnalysisConfig {
            max_chunk_size: max_size,
            min_chunk_size: min_size,
            chunk_overlap: overlap,
            enable_semantic_chunking: self.enable_semantic_chunking.unwrap_or(true),
            language: self.language,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlm_config_builder_defaults() {
        let config = RlmConfigBuilder::new().build().unwrap();

        // Test actual config structure defaults
        assert_eq!(config.limits.max_iterations, 50);
        assert_eq!(config.limits.max_recursion_depth, 1);
        assert_eq!(config.limits.chunk_size_tokens, 4096);
        assert_eq!(config.limits.timeout, Duration::from_secs(300));
        assert!(config.streaming.enabled);
        assert_eq!(config.repl.backend, "rhai");
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.model, "gpt-4-turbo");
    }

    #[test]
    fn test_backend_config_builder() {
        let config = BackendConfigBuilder::new()
            .openai()
            .api_key("test-key")
            .build()
            .unwrap();

        assert!(matches!(config.provider_type, ProviderType::OpenAi));
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert!(config.model_mapping.contains_key("gpt-4"));
        assert!(config.model_mapping.contains_key("gpt-4o"));
    }

    #[test]
    fn test_context_analysis_config_builder() {
        let config = ContextAnalysisConfigBuilder::new()
            .code_analysis()
            .max_chunk_size(10000)
            .build()
            .unwrap();

        assert_eq!(config.max_chunk_size, 10000);
        assert_eq!(config.min_chunk_size, 512);
        assert_eq!(config.chunk_overlap, 256);
        assert!(config.enable_semantic_chunking);
        assert_eq!(config.language, Some("rust".to_string()));
    }

    #[test]
    fn test_presets() {
        // Test performance preset
        let config = RlmConfigBuilder::new()
            .performance_optimized()
            .build()
            .unwrap();

        assert_eq!(config.limits.max_iterations, 100);
        assert_eq!(config.limits.max_recursion_depth, 5);
        assert!(config.streaming.enabled);

        // Test memory efficient preset
        let config = RlmConfigBuilder::new()
            .memory_efficient()
            .build()
            .unwrap();

        assert_eq!(config.limits.max_iterations, 25);
        assert_eq!(config.limits.chunk_size_tokens, 2048);
        assert!(!config.streaming.enabled);

        // Test development preset
        let config = RlmConfigBuilder::new()
            .development()
            .build()
            .unwrap();

        assert!(config.streaming.enabled);
        assert_eq!(config.limits.timeout, Duration::from_secs(60));
        assert_eq!(config.streaming.include_repl_state, true);
    }

    #[test]
    fn test_backend_config_builder_validation() {
        // Test missing provider type
        let result = BackendConfigBuilder::new()
            .api_key("test-key")
            .build();
        assert!(result.is_err());

        // Test missing API key
        let result = BackendConfigBuilder::new()
            .openai()
            .build();
        assert!(result.is_err());

        // Test missing base URL
        let result = BackendConfigBuilder::new()
            .provider_type(ProviderType::Custom)
            .api_key("test-key")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_context_analysis_config_builder_validation() {
        // Test chunk size validation
        let result = ContextAnalysisConfigBuilder::new()
            .min_chunk_size(5000)
            .max_chunk_size(1000) // Invalid: min > max
            .build();
        assert!(result.is_err());

        // Test chunk overlap validation
        let result = ContextAnalysisConfigBuilder::new()
            .min_chunk_size(512)
            .chunk_overlap(1000) // Invalid: overlap >= min_size
            .build();
        assert!(result.is_err());
    }
}