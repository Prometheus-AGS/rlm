//! YAML configuration file support.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Example YAML configuration for documentation.
pub const EXAMPLE_CONFIG: &str = r#"
server:
  host: "0.0.0.0"
  port: 8080
  request_timeout_secs: 300
  max_request_size_bytes: 104857600  # 100MB

rlm:
  limits:
    max_iterations: 50
    max_recursive_depth: 10
    max_context_size_tokens: 10000000
    total_timeout_secs: 600
  repl:
    backend: "rhai"
    sandbox: true
    max_memory_mb: 1024
    operation_timeout_ms: 30000
  llm:
    model: "gpt-4"
    timeout_secs: 300
    max_tokens: 4096
    temperature: 0.1
  streaming:
    enable_events: true
    chunk_timeout_ms: 5000
    progress_interval_ms: 2000

llm:
  provider: "openai"
  api_key: "${RLM_LLM_API_KEY}"
  base_url: "https://api.openai.com/v1"
  model: "gpt-4"
  timeout_secs: 300

logging:
  level: "info"
  format: "pretty"

# Azure OpenAI example:
# llm:
#   provider: "azure"
#   api_key: "${RLM_LLM_API_KEY}"
#   base_url: "https://your-resource.openai.azure.com"
#   deployment_name: "gpt-4-deployment"
#   api_version: "2024-02-01"
"#;

/// Write example configuration to file.
pub fn write_example_config<P: AsRef<Path>>(path: P) -> Result<()> {
    fs::write(path, EXAMPLE_CONFIG)?;
    Ok(())
}

/// Configuration template with environment variable substitution.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigTemplate {
    /// Raw configuration data.
    pub raw: String,
}

impl ConfigTemplate {
    /// Load from file with environment variable substitution.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let raw = fs::read_to_string(path)?;
        Ok(Self { raw })
    }

    /// Substitute environment variables in the configuration.
    pub fn substitute_env_vars(&self) -> String {
        let mut result = self.raw.clone();

        // Simple environment variable substitution: ${VAR_NAME}
        let re = regex::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();

        result = re.replace_all(&result, |caps: &regex::Captures<'_>| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap_or_else(|_| {
                eprintln!("Warning: Environment variable {} not found", var_name);
                format!("${{{}}}", var_name)
            })
        }).to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_var_substitution() {
        env::set_var("TEST_KEY", "test_value");

        let template = ConfigTemplate {
            raw: "api_key: ${TEST_KEY}".to_string(),
        };

        let result = template.substitute_env_vars();
        assert_eq!(result, "api_key: test_value");

        env::remove_var("TEST_KEY");
    }
}