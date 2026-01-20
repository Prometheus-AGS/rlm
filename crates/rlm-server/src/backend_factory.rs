//! Backend factory for creating LLM providers from configuration.
//!
//! This module implements a factory pattern for creating LLM provider instances
//! based on backend configuration. It supports multiple provider types including
//! OpenAI, Azure OpenAI, and local models, with automatic configuration mapping
//! and validation.

use rlm_core::{
    ports::LlmProvider,
    BackendConfig, ProviderType, RlmError, RlmResult,
};
use crate::adapters::{
    OpenAiProvider, OpenAiConfigBuilder,
    AzureOpenAiProvider,
};
use std::sync::Arc;
use tracing::{debug, instrument};

/// Factory for creating LLM provider instances from backend configurations.
///
/// This factory encapsulates the logic for creating appropriate provider
/// instances based on the provider type and configuration parameters.
/// It handles provider-specific configuration mapping and validation.
#[derive(Debug)]
pub struct BackendFactory;

impl BackendFactory {
    /// Create a new LLM provider from backend configuration.
    ///
    /// # Arguments
    /// * `config` - Backend configuration specifying provider type and parameters
    ///
    /// # Returns
    /// * `Ok(Arc<dyn LlmProvider>)` - Successfully created provider instance
    /// * `Err(RlmError)` - Configuration or provider creation error
    ///
    /// # Examples
    /// ```rust
    /// use rlm_core::BackendConfig;
    /// use rlm_server::backend_factory::BackendFactory;
    ///
    /// let openai_config = BackendConfig::openai("your-api-key".to_string());
    /// let provider = BackendFactory::create_provider(&openai_config).unwrap();
    /// ```
    #[instrument(skip(config), fields(provider_type = ?config.provider_type))]
    pub fn create_provider(config: &BackendConfig) -> RlmResult<Arc<dyn LlmProvider + Send + Sync>> {
        debug!("Creating provider for type: {:?}", config.provider_type);

        // Validate configuration before creating provider
        config.validate().map_err(|e| RlmError::Config(e.to_string()))?;

        match config.provider_type {
            ProviderType::OpenAi => Self::create_openai_provider(config),
            ProviderType::AzureOpenAi => Self::create_azure_openai_provider(config),
            ProviderType::LocalModel => Self::create_local_model_provider(config),
            ProviderType::Custom => Self::create_custom_provider(config),
        }
    }

    /// Create multiple providers from a list of backend configurations.
    ///
    /// This method processes multiple configurations and returns a map of
    /// provider names to provider instances. Failed configurations are
    /// logged but don't prevent other providers from being created.
    ///
    /// # Arguments
    /// * `configs` - Map of provider names to backend configurations
    ///
    /// # Returns
    /// * Map of provider names to successfully created provider instances
    ///
    /// # Examples
    /// ```rust
    /// use std::collections::HashMap;
    /// use rlm_core::BackendConfig;
    /// use rlm_server::backend_factory::BackendFactory;
    ///
    /// let mut configs = HashMap::new();
    /// configs.insert("primary".to_string(), BackendConfig::openai("key1".to_string()));
    /// configs.insert("fallback".to_string(), BackendConfig::azure_openai(
    ///     "resource".to_string(),
    ///     "deployment".to_string(),
    ///     "key2".to_string(),
    ///     None
    /// ));
    ///
    /// let providers = BackendFactory::create_providers(configs);
    /// ```
    #[instrument(skip(configs))]
    pub fn create_providers(
        configs: std::collections::HashMap<String, BackendConfig>,
    ) -> std::collections::HashMap<String, Arc<dyn LlmProvider + Send + Sync>> {
        let mut providers = std::collections::HashMap::new();

        for (name, config) in configs {
            match Self::create_provider(&config) {
                Ok(provider) => {
                    debug!("Successfully created provider: {}", name);
                    providers.insert(name, provider);
                }
                Err(e) => {
                    tracing::warn!("Failed to create provider {}: {}", name, e);
                }
            }
        }

        debug!("Created {} providers out of {} configurations", providers.len(), providers.len());
        providers
    }

    /// Create an OpenAI provider from backend configuration.
    fn create_openai_provider(config: &BackendConfig) -> RlmResult<Arc<dyn LlmProvider + Send + Sync>> {
        debug!("Creating OpenAI provider");

        let openai_config = OpenAiConfigBuilder::new(config.api_key.clone())
            .base_url(config.base_url.clone())
            .timeout_seconds(config.timeout_ms / 1000)
            .max_retries(config.retry_policy.max_attempts)
            .build();

        let provider = OpenAiProvider::new(openai_config)?;

        // Note: OpenAiProvider doesn't currently support runtime model mapping
        // This would need to be added to the provider if needed

        Ok(Arc::new(provider))
    }

    /// Create an Azure OpenAI provider from backend configuration.
    fn create_azure_openai_provider(config: &BackendConfig) -> RlmResult<Arc<dyn LlmProvider + Send + Sync>> {
        debug!("Creating Azure OpenAI provider");

        let provider = AzureOpenAiProvider::from_backend_config(config)?;
        Ok(Arc::new(provider))
    }

    /// Create a local model provider from backend configuration.
    fn create_local_model_provider(config: &BackendConfig) -> RlmResult<Arc<dyn LlmProvider + Send + Sync>> {
        debug!("Creating local model provider");

        // For local models, we can use OpenAI-compatible API with custom base URL
        let local_config = OpenAiConfigBuilder::new(
            if config.api_key.is_empty() {
                "not-required".to_string()
            } else {
                config.api_key.clone()
            }
        )
        .base_url(config.base_url.clone())
        .timeout_seconds(config.timeout_ms / 1000)
        .max_retries(config.retry_policy.max_attempts)
        .build();

        let provider = OpenAiProvider::new(local_config)?;
        Ok(Arc::new(provider))
    }

    /// Create a custom provider from backend configuration.
    fn create_custom_provider(config: &BackendConfig) -> RlmResult<Arc<dyn LlmProvider + Send + Sync>> {
        debug!("Creating custom provider");

        // For custom providers, we treat them as OpenAI-compatible
        // This assumes the custom provider implements OpenAI's chat completions API
        let custom_config = OpenAiConfigBuilder::new(config.api_key.clone())
            .base_url(config.base_url.clone())
            .timeout_seconds(config.timeout_ms / 1000)
            .max_retries(config.retry_policy.max_attempts)
            .build();

        let provider = OpenAiProvider::new(custom_config)?;
        Ok(Arc::new(provider))
    }

    /// Validate that a provider type is supported by this factory.
    pub fn is_provider_supported(provider_type: &ProviderType) -> bool {
        matches!(
            provider_type,
            ProviderType::OpenAi
            | ProviderType::AzureOpenAi
            | ProviderType::LocalModel
            | ProviderType::Custom
        )
    }

    /// Get a list of all supported provider types.
    pub fn supported_provider_types() -> Vec<ProviderType> {
        vec![
            ProviderType::OpenAi,
            ProviderType::AzureOpenAi,
            ProviderType::LocalModel,
            ProviderType::Custom,
        ]
    }

    /// Test connectivity to all providers in a configuration set.
    ///
    /// This method performs health checks on all providers to verify
    /// they can be reached and are properly configured. Useful for
    /// startup validation or configuration testing.
    ///
    /// # Arguments
    /// * `providers` - Map of provider names to provider instances
    ///
    /// # Returns
    /// * Map of provider names to health check results
    pub async fn health_check_providers(
        providers: &std::collections::HashMap<String, Arc<dyn LlmProvider + Send + Sync>>,
    ) -> std::collections::HashMap<String, bool> {
        let mut results = std::collections::HashMap::new();

        for (name, provider) in providers {
            match provider.health_check().await {
                Ok(healthy) => {
                    debug!("Health check for provider {}: {}", name, healthy);
                    results.insert(name.clone(), healthy);
                }
                Err(e) => {
                    tracing::warn!("Health check failed for provider {}: {}", name, e);
                    results.insert(name.clone(), false);
                }
            }
        }

        results
    }
}

/// Builder for creating backend factory configurations.
///
/// This builder provides a fluent interface for constructing backend
/// configurations that can be used with the BackendFactory.
#[derive(Debug)]
pub struct BackendFactoryBuilder {
    configs: std::collections::HashMap<String, BackendConfig>,
}

impl BackendFactoryBuilder {
    /// Create a new backend factory builder.
    pub fn new() -> Self {
        Self {
            configs: std::collections::HashMap::new(),
        }
    }

    /// Add an OpenAI backend configuration.
    pub fn add_openai(mut self, name: String, api_key: String) -> Self {
        let config = BackendConfig::openai(api_key);
        self.configs.insert(name, config);
        self
    }

    /// Add an Azure OpenAI backend configuration.
    pub fn add_azure_openai(
        mut self,
        name: String,
        resource_name: String,
        deployment_name: String,
        api_key: String,
        api_version: Option<String>,
    ) -> Self {
        let config = BackendConfig::azure_openai(resource_name, deployment_name, api_key, api_version);
        self.configs.insert(name, config);
        self
    }

    /// Add a local model backend configuration.
    pub fn add_local_model(
        mut self,
        name: String,
        host: String,
        port: u16,
        model_name: String,
    ) -> Self {
        let config = BackendConfig::local_model(host, port, model_name);
        self.configs.insert(name, config);
        self
    }

    /// Add a custom backend configuration.
    pub fn add_custom(mut self, name: String, config: BackendConfig) -> Self {
        self.configs.insert(name, config);
        self
    }

    /// Build the provider set from all configurations.
    pub fn build(self) -> std::collections::HashMap<String, Arc<dyn LlmProvider + Send + Sync>> {
        BackendFactory::create_providers(self.configs)
    }

    /// Build and validate all providers with health checks.
    pub async fn build_and_validate(self) -> RlmResult<std::collections::HashMap<String, Arc<dyn LlmProvider + Send + Sync>>> {
        let providers = self.build();

        if providers.is_empty() {
            return Err(RlmError::Config("No providers could be created from configurations".to_string()));
        }

        let health_results = BackendFactory::health_check_providers(&providers).await;
        let healthy_providers: Vec<_> = health_results.iter()
            .filter(|(_, &healthy)| healthy)
            .map(|(name, _)| name.clone())
            .collect();

        if healthy_providers.is_empty() {
            return Err(RlmError::Config("No providers passed health checks".to_string()));
        }

        debug!("Healthy providers: {:?}", healthy_providers);
        Ok(providers)
    }
}

impl Default for BackendFactoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::ProviderType;

    #[test]
    fn test_provider_support_checking() {
        assert!(BackendFactory::is_provider_supported(&ProviderType::OpenAi));
        assert!(BackendFactory::is_provider_supported(&ProviderType::AzureOpenAi));
        assert!(BackendFactory::is_provider_supported(&ProviderType::LocalModel));
        assert!(BackendFactory::is_provider_supported(&ProviderType::Custom));
    }

    #[test]
    fn test_supported_provider_types_list() {
        let supported = BackendFactory::supported_provider_types();
        assert_eq!(supported.len(), 4);
        assert!(supported.contains(&ProviderType::OpenAi));
        assert!(supported.contains(&ProviderType::AzureOpenAi));
        assert!(supported.contains(&ProviderType::LocalModel));
        assert!(supported.contains(&ProviderType::Custom));
    }

    #[test]
    fn test_openai_provider_creation() {
        let config = BackendConfig::openai("test-api-key".to_string());
        let result = BackendFactory::create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_azure_provider_creation() {
        let config = BackendConfig::azure_openai(
            "test-resource".to_string(),
            "test-deployment".to_string(),
            "test-api-key".to_string(),
            None,
        );
        let result = BackendFactory::create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_local_model_provider_creation() {
        let config = BackendConfig::local_model(
            "localhost".to_string(),
            8080,
            "llama-7b".to_string(),
        );
        let result = BackendFactory::create_provider(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_configuration_handling() {
        let mut config = BackendConfig::openai("test-key".to_string());
        config.base_url = "".to_string(); // Invalid empty URL

        let result = BackendFactory::create_provider(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_backend_factory_builder() {
        let providers = BackendFactoryBuilder::new()
            .add_openai("primary".to_string(), "openai-key".to_string())
            .add_azure_openai(
                "secondary".to_string(),
                "resource".to_string(),
                "deployment".to_string(),
                "azure-key".to_string(),
                None,
            )
            .add_local_model(
                "local".to_string(),
                "localhost".to_string(),
                8080,
                "llama".to_string(),
            )
            .build();

        assert_eq!(providers.len(), 3);
        assert!(providers.contains_key("primary"));
        assert!(providers.contains_key("secondary"));
        assert!(providers.contains_key("local"));
    }

    #[test]
    fn test_multiple_provider_creation_with_failures() {
        let mut configs = std::collections::HashMap::new();

        // Valid configuration
        configs.insert(
            "valid".to_string(),
            BackendConfig::openai("valid-key".to_string()),
        );

        // Invalid configuration (empty base URL)
        let mut invalid_config = BackendConfig::openai("invalid-key".to_string());
        invalid_config.base_url = "".to_string();
        configs.insert("invalid".to_string(), invalid_config);

        let providers = BackendFactory::create_providers(configs);

        // Should have successfully created one provider, skipped the invalid one
        assert_eq!(providers.len(), 1);
        assert!(providers.contains_key("valid"));
        assert!(!providers.contains_key("invalid"));
    }

    #[tokio::test]
    async fn test_health_check_providers() {
        // This test would require actual provider instances
        // For now, we'll test with an empty set
        let providers = std::collections::HashMap::new();
        let results = BackendFactory::health_check_providers(&providers).await;
        assert!(results.is_empty());
    }
}