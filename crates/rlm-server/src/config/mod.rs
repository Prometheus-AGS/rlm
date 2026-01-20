//! Server configuration management.

use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Serialized, Yaml},
    Figment,
};
// Note: BackendConfig and ProviderType will be used in future implementation phases
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod cli;
pub mod env;
pub mod yaml;

/// Complete server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server configuration.
    pub server: HttpServerConfig,
    /// RLM core configuration.
    pub rlm: rlm_core::RlmConfig,
    /// Single LLM provider configuration (legacy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmProviderConfig>,
    /// Multi-backend configuration (new).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backends: Option<BackendsConfig>,
    /// Logging configuration.
    pub logging: LoggingConfig,
}

/// HTTP server specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
    /// Server host.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes.
    pub max_request_size_bytes: usize,
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Provider type (openai, azure, etc.).
    pub provider: String,
    /// API key for the provider.
    pub api_key: String,
    /// Base URL for the API.
    pub base_url: String,
    /// Model name to use.
    pub model: String,
    /// Azure-specific deployment name.
    pub deployment_name: Option<String>,
    /// Azure-specific API version.
    pub api_version: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

/// Multi-backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendsConfig {
    /// Default backend to use when none is specified.
    pub default: String,
    /// Map of backend names to configurations.
    pub providers: HashMap<String, BackendProviderConfig>,
    /// Backend routing rules (optional).
    #[serde(default)]
    pub routing: BackendRoutingConfig,
}

/// Backend provider configuration that can be converted to BackendConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendProviderConfig {
    /// Provider type (openai, azure_openai, local_model, custom).
    pub provider_type: String,
    /// API base URL.
    pub base_url: String,
    /// API authentication key.
    pub api_key: String,
    /// Request timeout in milliseconds.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Model mappings (logical -> physical).
    #[serde(default)]
    pub model_mapping: HashMap<String, String>,
    /// Provider-specific options.
    #[serde(default)]
    pub provider_options: HashMap<String, String>,
    /// Retry policy configuration.
    #[serde(default)]
    pub retry: RetryPolicyConfig,
    /// Rate limiting configuration.
    #[serde(default)]
    pub rate_limits: RateLimitsConfig,
}

/// Backend routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendRoutingConfig {
    /// Route requests based on model name patterns.
    #[serde(default)]
    pub model_patterns: HashMap<String, String>,
    /// Route requests based on request size thresholds.
    #[serde(default)]
    pub size_thresholds: Vec<SizeThreshold>,
    /// Fallback routing strategy.
    #[serde(default = "default_fallback_strategy")]
    pub fallback_strategy: String,
}

/// Size-based routing threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeThreshold {
    /// Maximum tokens for this backend.
    pub max_tokens: u32,
    /// Backend to use for requests under this threshold.
    pub backend: String,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyConfig {
    /// Maximum number of retry attempts.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// Initial backoff delay in milliseconds.
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds.
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitsConfig {
    /// Maximum requests per minute.
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
    /// Maximum tokens per minute.
    #[serde(default = "default_tokens_per_minute")]
    pub tokens_per_minute: u32,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level.
    pub level: String,
    /// Log format (json or pretty).
    pub format: String,
}

/// Configuration source information for debugging and validation.
#[derive(Debug, Clone)]
pub struct ConfigSourceInfo {
    /// Configuration file that was used (if any).
    pub config_file_used: Option<String>,
    /// Whether the configuration file exists.
    pub config_file_exists: bool,
    /// Environment variables that affect configuration.
    pub environment_vars: Vec<EnvVarInfo>,
    /// Default configuration files that were checked.
    pub default_files_checked: Vec<DefaultFileInfo>,
    /// Whether legacy LLM configuration is present.
    pub has_legacy_config: bool,
    /// Whether new backend configuration is present.
    pub has_backend_config: bool,
}

/// Information about an environment variable.
#[derive(Debug, Clone)]
pub struct EnvVarInfo {
    /// Environment variable key.
    pub key: String,
    /// Environment variable value.
    pub value: String,
    /// Whether the value is considered valid.
    pub is_valid: bool,
}

/// Information about a default configuration file check.
#[derive(Debug, Clone)]
pub struct DefaultFileInfo {
    /// File path that was checked.
    pub path: String,
    /// Whether the file exists.
    pub exists: bool,
}

// Default value functions for serde
fn default_timeout_ms() -> u64 {
    30000
}

fn default_max_attempts() -> u32 {
    3
}

fn default_initial_backoff() -> u64 {
    1000
}

fn default_max_backoff() -> u64 {
    10000
}

fn default_requests_per_minute() -> u32 {
    60
}

fn default_tokens_per_minute() -> u32 {
    10000
}

fn default_fallback_strategy() -> String {
    "round_robin".to_string()
}

impl Default for RetryPolicyConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff_ms: default_initial_backoff(),
            max_backoff_ms: default_max_backoff(),
        }
    }
}

impl Default for RateLimitsConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            tokens_per_minute: default_tokens_per_minute(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: HttpServerConfig::default(),
            rlm: rlm_core::RlmConfig::default(),
            llm: Some(LlmProviderConfig::default()),
            backends: None,
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 300, // 5 minutes for long contexts
            max_request_size_bytes: 100 * 1024 * 1024, // 100MB
        }
    }
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            deployment_name: None,
            api_version: None,
            timeout_secs: 300,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
        }
    }
}

impl ServerConfig {
    /// Load configuration from various sources.
    ///
    /// Priority (highest to lowest):
    /// 1. Environment variables (RLM_*)
    /// 2. YAML config file (if provided)
    /// 3. Default values
    pub fn load(config_file: Option<&str>) -> Result<Self> {
        Self::load_with_validation(config_file, true)
    }

    /// Load configuration with optional validation.
    ///
    /// This method provides detailed configuration loading with hierarchy validation
    /// and source tracking for better error reporting.
    ///
    /// # Arguments
    /// * `config_file` - Optional path to YAML configuration file
    /// * `validate_hierarchy` - Whether to perform detailed hierarchy validation
    ///
    /// # Returns
    /// * `Ok(ServerConfig)` - Successfully loaded and validated configuration
    /// * `Err(anyhow::Error)` - Configuration loading or validation error with context
    pub fn load_with_validation(config_file: Option<&str>, validate_hierarchy: bool) -> Result<Self> {
        let mut figment = Figment::from(Serialized::defaults(Self::default()));
        let mut config_sources = Vec::<String>::new();
        let mut used_config_file: Option<String> = None;

        // Track default values source
        config_sources.push("Default values".to_string());

        // Load from YAML file if provided
        if let Some(path) = config_file {
            if Path::new(path).exists() {
                figment = figment.merge(Yaml::file(path));
                config_sources.push(format!("YAML file: {}", path));
                used_config_file = Some(path.to_string());
            } else {
                anyhow::bail!("Configuration file not found: {}", path);
            }
        } else {
            // Try default config file locations
            for default_path in &["rlm-config.yaml", "rlm.yaml", ".rlm.yaml", "config.default.yaml"] {
                if Path::new(default_path).exists() {
                    figment = figment.merge(Yaml::file(default_path));
                    config_sources.push(format!("Default YAML file: {}", default_path));
                    used_config_file = Some(default_path.to_string());
                    break;
                }
            }
        }

        // Check for environment variables before adding them
        let env_vars = std::env::vars()
            .filter(|(key, _)| key.starts_with("RLM_"))
            .collect::<Vec<_>>();

        if !env_vars.is_empty() {
            config_sources.push(format!("Environment variables: {} vars", env_vars.len()));
            figment = figment.merge(Env::prefixed("RLM_").split("_"));
        }

        // Extract configuration with detailed error context
        let config = figment
            .extract()
            .with_context(|| {
                format!(
                    "Failed to load configuration from sources: {}",
                    config_sources.join(" → ")
                )
            })?;

        // Perform hierarchy validation if requested
        if validate_hierarchy {
            Self::validate_configuration_hierarchy(&config, &config_sources, used_config_file.as_deref())?;
        }

        Ok(config)
    }

    /// Validate configuration hierarchy and source consistency.
    ///
    /// This method performs comprehensive validation of the configuration hierarchy,
    /// checking for conflicts, missing required values, and source consistency.
    fn validate_configuration_hierarchy(
        config: &ServerConfig,
        sources: &[String],
        config_file: Option<&str>,
    ) -> Result<()> {
        // Validate that configuration sources are consistent
        if sources.is_empty() {
            anyhow::bail!("No configuration sources available");
        }

        // Check for configuration conflicts between legacy and new formats
        match (&config.llm, &config.backends) {
            (Some(_), Some(_)) => {
                anyhow::bail!(
                    "Configuration conflict: Both 'llm' (legacy) and 'backends' (new) are specified. \
                     Please use only one configuration format. \
                     Sources: {}",
                    sources.join(" → ")
                );
            },
            (None, None) => {
                anyhow::bail!(
                    "Missing provider configuration: Either 'llm' (legacy) or 'backends' (new) must be specified. \
                     Sources: {}",
                    sources.join(" → ")
                );
            },
            _ => {
                // Valid configuration - only one format is used
            }
        }

        // Validate configuration file format if one was used
        if let Some(file_path) = config_file {
            Self::validate_config_file_format(file_path)?;
        }

        // Validate environment variable format
        Self::validate_environment_variables()?;

        Ok(())
    }

    /// Validate configuration file format and structure.
    fn validate_config_file_format(file_path: &str) -> Result<()> {
        // Read and parse the configuration file to check for common issues
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read configuration file: {}", file_path))?;

        // Basic YAML syntax validation
        if content.trim().is_empty() {
            anyhow::bail!("Configuration file is empty: {}", file_path);
        }

        // Check for common YAML issues
        if content.contains('\t') {
            anyhow::bail!(
                "Configuration file contains tabs: {}. YAML files should use spaces for indentation.",
                file_path
            );
        }

        // Try to parse as raw YAML to catch syntax errors early
        serde_yml::from_str::<serde_yml::Value>(&content)
            .with_context(|| format!("Invalid YAML syntax in configuration file: {}", file_path))?;

        Ok(())
    }

    /// Validate environment variables format and values.
    fn validate_environment_variables() -> Result<()> {
        let invalid_vars: Vec<_> = std::env::vars()
            .filter(|(key, _)| key.starts_with("RLM_"))
            .filter(|(key, value)| {
                // Check for empty values
                value.trim().is_empty() ||
                // Check for malformed keys (should not end with underscore)
                key.ends_with('_') ||
                // Check for keys with consecutive underscores
                key.contains("__")
            })
            .collect();

        if !invalid_vars.is_empty() {
            let invalid_keys: Vec<_> = invalid_vars.iter().map(|(k, _)| k.as_str()).collect();
            anyhow::bail!(
                "Invalid environment variables found: {}. \
                 Environment variables should not be empty, end with underscore, or contain consecutive underscores.",
                invalid_keys.join(", ")
            );
        }

        Ok(())
    }

    /// Get configuration source information for debugging.
    ///
    /// This method provides detailed information about where configuration
    /// values are coming from, useful for troubleshooting configuration issues.
    pub fn get_source_info(config_file: Option<&str>) -> Result<ConfigSourceInfo> {
        let mut info = ConfigSourceInfo {
            config_file_used: None,
            config_file_exists: false,
            environment_vars: Vec::new(),
            default_files_checked: Vec::new(),
            has_legacy_config: false,
            has_backend_config: false,
        };

        // Check configuration file
        if let Some(path) = config_file {
            info.config_file_used = Some(path.to_string());
            info.config_file_exists = Path::new(path).exists();
        } else {
            // Check default locations
            for default_path in &["rlm-config.yaml", "rlm.yaml", ".rlm.yaml", "config.default.yaml"] {
                let exists = Path::new(default_path).exists();
                info.default_files_checked.push(DefaultFileInfo {
                    path: default_path.to_string(),
                    exists,
                });
                if exists && info.config_file_used.is_none() {
                    info.config_file_used = Some(default_path.to_string());
                    info.config_file_exists = true;
                }
            }
        }

        // Collect environment variables
        info.environment_vars = std::env::vars()
            .filter(|(key, _)| key.starts_with("RLM_"))
            .map(|(key, value)| {
                let is_valid = !value.trim().is_empty();
                EnvVarInfo {
                    key,
                    value,
                    is_valid,
                }
            })
            .collect();

        // Try to determine config type without loading full config
        if let Ok(config) = Self::load_with_validation(config_file, false) {
            info.has_legacy_config = config.llm.is_some();
            info.has_backend_config = config.backends.is_some();
        }

        Ok(info)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.host.is_empty() {
            anyhow::bail!("Server host cannot be empty");
        }
        if self.server.port == 0 {
            anyhow::bail!("Server port must be greater than 0");
        }

        // Validate that we have either legacy LLM config or new backends config
        match (&self.llm, &self.backends) {
            (Some(llm_config), None) => {
                // Legacy single provider validation
                if llm_config.api_key.is_empty() {
                    anyhow::bail!("LLM API key is required");
                }
                if llm_config.model.is_empty() {
                    anyhow::bail!("LLM model name is required");
                }

                // Validate Azure-specific config
                if llm_config.provider == "azure" {
                    if llm_config.deployment_name.is_none() {
                        anyhow::bail!("Azure deployment name is required for Azure provider");
                    }
                    if llm_config.api_version.is_none() {
                        anyhow::bail!("Azure API version is required for Azure provider");
                    }
                }
            },
            (None, Some(backends_config)) => {
                // Multi-backend validation
                if backends_config.providers.is_empty() {
                    anyhow::bail!("At least one backend provider must be configured");
                }
                if !backends_config.providers.contains_key(&backends_config.default) {
                    anyhow::bail!("Default backend '{}' is not configured in providers", backends_config.default);
                }

                // Validate each backend provider
                for (name, provider_config) in &backends_config.providers {
                    if provider_config.api_key.is_empty() && provider_config.provider_type != "local_model" {
                        anyhow::bail!("API key is required for backend provider '{}'", name);
                    }
                    if provider_config.base_url.is_empty() {
                        anyhow::bail!("Base URL is required for backend provider '{}'", name);
                    }
                }
            },
            (Some(_), Some(_)) => {
                anyhow::bail!("Cannot specify both 'llm' and 'backends' configuration - use only one");
            },
            (None, None) => {
                anyhow::bail!("Either 'llm' (legacy) or 'backends' (multi-backend) configuration must be provided");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod hierarchy_tests {
    use super::*;

    #[test]
    fn test_config_source_info() {
        let info = ServerConfig::get_source_info(None).unwrap();

        // Should check default files
        assert!(!info.default_files_checked.is_empty());
        assert!(info.default_files_checked.iter().any(|f| f.path == "config.default.yaml"));

        // File existence depends on actual file system state
        // Just verify the structure is correct
        assert!(info.default_files_checked.len() >= 4); // Should check at least 4 default files
    }

    #[test]
    fn test_configuration_hierarchy_validation_with_defaults() {
        // Test loading with default configuration (no file)
        let config = ServerConfig::load(None).unwrap();

        // Should have default values
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);

        // Should have legacy config with defaults
        assert!(config.llm.is_some());
        assert!(config.backends.is_none());

        // Should pass validation (even though API key is empty in defaults)
        // The validation will fail on empty API key, but that's expected behavior
        let validation_result = config.validate();
        // Default config has empty API key, so validation should fail
        assert!(validation_result.is_err());
        assert!(validation_result.unwrap_err().to_string().contains("API key"));
    }

    #[test]
    fn test_invalid_config_file_handling() {
        let result = ServerConfig::load(Some("nonexistent-config.yaml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration file not found"));
    }

    #[test]
    fn test_environment_variable_validation() {
        // This test would require setting environment variables which could affect other tests
        // For now, just verify the validation function exists and works with current env
        let result = ServerConfig::validate_environment_variables();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_with_validation_false() {
        // Test that we can load without validation using just defaults
        let result = ServerConfig::load_with_validation(None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_file_format_validation() {
        use std::fs;

        // Create a temporary valid YAML file for testing
        let temp_file = "test_temp_config.yaml";
        let valid_yaml_content = r#"
server:
  host: "127.0.0.1"
  port: 8080

llm:
  provider: "openai"
  api_key: "test-key"
  model: "gpt-4"

logging:
  level: "info"
  format: "pretty"
"#;

        fs::write(temp_file, valid_yaml_content).unwrap();

        // Test that valid YAML passes validation
        let result = ServerConfig::validate_config_file_format(temp_file);
        assert!(result.is_ok());

        // Clean up
        fs::remove_file(temp_file).ok();

        // Test that empty file fails validation
        fs::write(temp_file, "").unwrap();
        let result = ServerConfig::validate_config_file_format(temp_file);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Configuration file is empty"));

        // Clean up
        fs::remove_file(temp_file).ok();
    }
}