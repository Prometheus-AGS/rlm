//! Integration tests for Azure OpenAI adapter functionality.
//!
//! This module tests the Azure OpenAI provider adapter with Azure-specific
//! configurations, URL formatting, API version handling, and authentication.
//! Tests cover both unit-level functionality and integration with the core
//! RLM executor and backends.

use rlm_core::{
    types::{ChatCompletionRequest, ChatMessage, ChatRole},
    ports::{LlmProvider, ModelInfo, ProviderMetadata},
    config::RlmConfig,
    BackendConfig, ProviderType, RetryPolicy, RateLimits,
};
use rlm_server::adapters::{OpenAiProvider, OpenAiConfigBuilder};
use std::collections::HashMap;
use std::time::Duration;

/// Test configuration for Azure OpenAI provider.
#[derive(Debug, Clone)]
pub struct AzureOpenAiTestConfig {
    pub resource_name: String,
    pub deployment_name: String,
    pub api_key: String,
    pub api_version: String,
    pub base_url: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub model_mapping: HashMap<String, String>,
}

impl AzureOpenAiTestConfig {
    /// Create a test Azure OpenAI configuration.
    pub fn new(resource_name: &str, deployment_name: &str) -> Self {
        let mut model_mapping = HashMap::new();
        model_mapping.insert("gpt-4".to_string(), deployment_name.to_string());
        model_mapping.insert("gpt-35-turbo".to_string(), format!("{}-35", deployment_name));

        Self {
            resource_name: resource_name.to_string(),
            deployment_name: deployment_name.to_string(),
            api_key: "test-azure-api-key".to_string(),
            api_version: "2024-02-01".to_string(),
            base_url: format!(
                "https://{}.openai.azure.com/openai/deployments/{}/",
                resource_name, deployment_name
            ),
            timeout_ms: 45000,
            max_retries: 5,
            model_mapping,
        }
    }

    /// Create an Azure OpenAI provider using this configuration.
    pub fn create_provider(&self) -> anyhow::Result<OpenAiProvider> {
        // Create Azure-compatible OpenAI config
        let openai_config = OpenAiConfigBuilder::new(self.api_key.clone())
            .base_url(self.base_url.clone())
            .default_model(self.deployment_name.clone())
            .timeout_seconds(self.timeout_ms / 1000)
            .max_retries(self.max_retries)
            .custom_header("api-version".to_string(), self.api_version.clone())
            .build();

        OpenAiProvider::new(openai_config)
            .map_err(|e| anyhow::anyhow!("Failed to create Azure OpenAI provider: {}", e))
    }

    /// Convert to BackendConfig for testing backend configuration.
    pub fn to_backend_config(&self) -> BackendConfig {
        BackendConfig::azure_openai(
            self.resource_name.clone(),
            self.deployment_name.clone(),
            self.api_key.clone(),
            Some(self.api_version.clone()),
        )
    }
}

#[tokio::test]
async fn test_azure_openai_configuration_creation() {
    // Test that Azure OpenAI configuration can be created with correct parameters
    let config = AzureOpenAiTestConfig::new("test-resource", "gpt-4-deployment");

    assert_eq!(config.resource_name, "test-resource");
    assert_eq!(config.deployment_name, "gpt-4-deployment");
    assert!(config.base_url.contains("test-resource.openai.azure.com"));
    assert!(config.base_url.contains("gpt-4-deployment"));
    assert_eq!(config.api_version, "2024-02-01");
    assert_eq!(config.timeout_ms, 45000); // Azure-specific longer timeout
    assert_eq!(config.max_retries, 5); // Azure-specific more aggressive retry
}

#[tokio::test]
async fn test_azure_openai_url_formatting() {
    // Test that Azure OpenAI URLs are correctly formatted
    let test_cases = vec![
        ("my-resource", "gpt-4-turbo", "https://my-resource.openai.azure.com/openai/deployments/gpt-4-turbo/"),
        ("prod-east", "gpt-35-turbo-16k", "https://prod-east.openai.azure.com/openai/deployments/gpt-35-turbo-16k/"),
        ("test-west2", "ada-002", "https://test-west2.openai.azure.com/openai/deployments/ada-002/"),
    ];

    for (resource, deployment, expected_url) in test_cases {
        let config = AzureOpenAiTestConfig::new(resource, deployment);
        assert_eq!(config.base_url, expected_url);
    }
}

#[tokio::test]
async fn test_azure_openai_model_mapping() {
    // Test that logical model names map correctly to deployment names
    let config = AzureOpenAiTestConfig::new("test-resource", "production-gpt4");

    assert_eq!(
        config.model_mapping.get("gpt-4"),
        Some(&"production-gpt4".to_string())
    );
    assert_eq!(
        config.model_mapping.get("gpt-35-turbo"),
        Some(&"production-gpt4-35".to_string())
    );
}

#[tokio::test]
async fn test_azure_openai_provider_creation() {
    // Test that Azure OpenAI provider can be created from configuration
    let config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let provider_result = config.create_provider();

    assert!(provider_result.is_ok());
    let provider = provider_result.unwrap();

    // Test provider metadata
    let metadata = provider.get_metadata();
    assert_eq!(metadata.name, "OpenAI"); // Uses OpenAI provider but with Azure config
    assert!(metadata.base_url.contains("azure.com"));
    assert!(metadata.requires_auth);
}

#[tokio::test]
async fn test_azure_openai_backend_config_integration() {
    // Test that Azure configuration integrates with BackendConfig system
    let test_config = AzureOpenAiTestConfig::new("prod-resource", "gpt-4-deployment");
    let backend_config = test_config.to_backend_config();

    assert_eq!(backend_config.provider_type, ProviderType::AzureOpenAi);
    assert!(backend_config.base_url.contains("azure.com"));
    assert!(backend_config.base_url.contains("prod-resource"));
    assert!(backend_config.base_url.contains("gpt-4-deployment"));

    // Azure-specific settings
    assert_eq!(backend_config.timeout_ms, 45000);
    assert_eq!(backend_config.retry_policy.max_attempts, 5);
    assert_eq!(backend_config.rate_limits.requests_per_minute, 240);

    // Provider options should contain API version
    assert!(backend_config.provider_options.contains_key("api_version"));
    assert_eq!(
        backend_config.provider_options.get("api_version"),
        Some(&"2024-02-01".to_string())
    );
}

#[tokio::test]
async fn test_azure_openai_request_validation() {
    // Test that Azure provider properly validates requests
    let config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let provider = config.create_provider().unwrap();

    // Valid request
    let valid_request = ChatCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Hello Azure OpenAI!".to_string(),
            name: None,
        }],
        max_tokens: Some(150),
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
        max_tokens: Some(150),
        temperature: Some(0.7),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    assert!(provider.validate_request(&invalid_request).is_err());

    // Invalid request - temperature out of range
    let mut invalid_temp_request = valid_request.clone();
    invalid_temp_request.temperature = Some(3.0);
    assert!(provider.validate_request(&invalid_temp_request).is_err());
}

#[tokio::test]
async fn test_azure_openai_deployment_name_handling() {
    // Test that deployment names are correctly used as physical model names
    let config = AzureOpenAiTestConfig::new("test-resource", "my-custom-gpt4");
    let backend_config = config.to_backend_config();

    // Logical "gpt-4" should map to deployment name
    assert_eq!(
        backend_config.get_physical_model("gpt-4"),
        Some(&"my-custom-gpt4".to_string())
    );

    // Non-existent logical model should return None
    assert_eq!(backend_config.get_physical_model("non-existent"), None);
}

#[tokio::test]
async fn test_azure_openai_api_version_handling() {
    // Test different API versions
    let api_versions = vec!["2023-12-01-preview", "2024-02-01", "2024-03-01-preview"];

    for api_version in api_versions {
        let mut config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
        config.api_version = api_version.to_string();

        let backend_config = config.to_backend_config();
        assert_eq!(
            backend_config.provider_options.get("api_version"),
            Some(&api_version.to_string())
        );
    }
}

#[tokio::test]
async fn test_azure_openai_timeout_configuration() {
    // Test that Azure OpenAI uses longer timeouts than regular OpenAI
    let azure_config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let backend_config = azure_config.to_backend_config();

    // Azure should have longer timeout than default OpenAI (30 seconds)
    assert!(backend_config.timeout_ms > 30000);
    assert_eq!(backend_config.timeout_ms, 45000);
}

#[tokio::test]
async fn test_azure_openai_retry_policy() {
    // Test that Azure OpenAI uses more aggressive retry policy
    let azure_config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let backend_config = azure_config.to_backend_config();

    // Azure should have more retry attempts than default
    assert_eq!(backend_config.retry_policy.max_attempts, 5);
    assert_eq!(backend_config.retry_policy.initial_backoff_ms, 2000);
    assert_eq!(backend_config.retry_policy.max_backoff_ms, 30000);
}

#[tokio::test]
async fn test_azure_openai_rate_limits() {
    // Test that Azure OpenAI has different rate limits than OpenAI
    let azure_config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let backend_config = azure_config.to_backend_config();

    // Azure typically has lower rate limits than OpenAI
    assert_eq!(backend_config.rate_limits.requests_per_minute, 240);
    assert_eq!(backend_config.rate_limits.tokens_per_minute, 40000);

    // Should be lower than OpenAI defaults (3500 RPM, 90k TPM)
    let openai_config = BackendConfig::openai("test-key".to_string());
    assert!(backend_config.rate_limits.requests_per_minute < openai_config.rate_limits.requests_per_minute);
    assert!(backend_config.rate_limits.tokens_per_minute < openai_config.rate_limits.tokens_per_minute);
}

#[tokio::test]
async fn test_azure_openai_health_check_url() {
    // Test that health check URL is correctly constructed for Azure
    let azure_config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let backend_config = azure_config.to_backend_config();

    let health_url = backend_config.health_check_url();

    // Should be based on the deployment URL + /models
    assert!(health_url.contains("test-resource.openai.azure.com"));
    assert!(health_url.contains("test-deployment"));
    assert!(health_url.ends_with("/models"));
}

#[tokio::test]
async fn test_azure_openai_configuration_validation() {
    // Test configuration validation for Azure-specific requirements
    let azure_config = AzureOpenAiTestConfig::new("test-resource", "test-deployment");
    let backend_config = azure_config.to_backend_config();

    // Should pass validation
    assert!(backend_config.validate().is_ok());

    // Test invalid configurations
    let mut invalid_config = backend_config.clone();
    invalid_config.api_key = "".to_string();
    assert!(invalid_config.validate().is_err()); // Azure requires API key

    let mut invalid_url_config = backend_config.clone();
    invalid_url_config.base_url = "not-a-url".to_string();
    assert!(invalid_url_config.validate().is_err()); // Invalid URL format
}

/// Integration test for Azure OpenAI end-to-end functionality.
///
/// This test simulates a complete flow from configuration to request processing
/// using Azure OpenAI provider with the RLM system.
#[tokio::test]
async fn test_azure_openai_end_to_end_integration() {
    // This test will be expanded as the full Azure provider implementation is completed

    // Step 1: Create Azure configuration
    let azure_config = AzureOpenAiTestConfig::new("production-east", "gpt-4-turbo-deployment");

    // Step 2: Validate configuration
    let backend_config = azure_config.to_backend_config();
    assert!(backend_config.validate().is_ok());

    // Step 3: Create provider (mock for now)
    let provider_result = azure_config.create_provider();
    assert!(provider_result.is_ok());

    // Step 4: Create test request
    let request = ChatCompletionRequest {
        model: "gpt-4".to_string(), // Logical model name
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Test Azure OpenAI integration with RLM".to_string(),
            name: None,
        }],
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: false,
        stop: None,
        top_p: None,
        rlm_config: None,
    };

    // Step 5: Validate request
    let provider = provider_result.unwrap();
    assert!(provider.validate_request(&request).is_ok());

    // Step 6: Verify model mapping would work correctly
    // The logical "gpt-4" should map to "gpt-4-turbo-deployment"
    assert_eq!(
        backend_config.get_physical_model("gpt-4"),
        Some(&"gpt-4-turbo-deployment".to_string())
    );

    // Step 7: Verify provider metadata
    let metadata = provider.get_metadata();
    assert_eq!(metadata.name, "OpenAI");
    assert!(metadata.base_url.contains("azure.com"));
    assert!(metadata.requires_auth);
}

/// Test for Azure OpenAI multi-region configuration.
#[tokio::test]
async fn test_azure_openai_multi_region_configuration() {
    // Test different Azure regions and resource configurations
    let regions = vec![
        ("eastus-resource", "East US"),
        ("westeurope-resource", "West Europe"),
        ("southeastasia-resource", "Southeast Asia"),
    ];

    for (resource_name, _region_name) in regions {
        let config = AzureOpenAiTestConfig::new(resource_name, "gpt-4-deployment");
        let backend_config = config.to_backend_config();

        // Each region should have valid configuration
        assert!(backend_config.validate().is_ok());
        assert!(backend_config.base_url.contains(resource_name));
        assert!(backend_config.base_url.contains("azure.com"));

        // Provider creation should work for all regions
        let provider_result = config.create_provider();
        assert!(provider_result.is_ok());
    }
}

/// Test Azure OpenAI specific error handling scenarios.
#[tokio::test]
async fn test_azure_openai_error_scenarios() {
    // Test configuration errors specific to Azure OpenAI

    // Empty resource name
    let config_with_empty_resource = AzureOpenAiTestConfig {
        resource_name: "".to_string(),
        deployment_name: "test-deployment".to_string(),
        api_key: "test-key".to_string(),
        api_version: "2024-02-01".to_string(),
        base_url: "https://.openai.azure.com/openai/deployments/test-deployment/".to_string(),
        timeout_ms: 45000,
        max_retries: 5,
        model_mapping: HashMap::new(),
    };

    // This should result in an invalid URL
    let backend_config = config_with_empty_resource.to_backend_config();
    // URL validation should catch the malformed URL
    assert!(backend_config.validate().is_err());

    // Empty deployment name
    let config_with_empty_deployment = AzureOpenAiTestConfig {
        resource_name: "test-resource".to_string(),
        deployment_name: "".to_string(),
        api_key: "test-key".to_string(),
        api_version: "2024-02-01".to_string(),
        base_url: "https://test-resource.openai.azure.com/openai/deployments//".to_string(),
        timeout_ms: 45000,
        max_retries: 5,
        model_mapping: HashMap::new(),
    };

    let backend_config_empty_deployment = config_with_empty_deployment.to_backend_config();
    // URL should still be valid but deployment would be empty
    assert!(backend_config_empty_deployment.validate().is_ok());
}