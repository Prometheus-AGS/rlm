//! Environment variable configuration support.

use std::env;

/// Environment variable keys used by RLM server.
#[derive(Debug)]
pub struct EnvKeys;

impl EnvKeys {
    /// Server host
    pub const SERVER_HOST: &'static str = "RLM_SERVER_HOST";
    /// Server port
    pub const SERVER_PORT: &'static str = "RLM_SERVER_PORT";
    /// LLM provider type
    pub const LLM_PROVIDER: &'static str = "RLM_LLM_PROVIDER";
    /// LLM API key
    pub const LLM_API_KEY: &'static str = "RLM_LLM_API_KEY";
    /// LLM base URL
    pub const LLM_BASE_URL: &'static str = "RLM_LLM_BASE_URL";
    /// LLM model
    pub const LLM_MODEL: &'static str = "RLM_LLM_MODEL";
    /// Azure deployment name
    pub const LLM_DEPLOYMENT_NAME: &'static str = "RLM_LLM_DEPLOYMENT_NAME";
    /// Azure API version
    pub const LLM_API_VERSION: &'static str = "RLM_LLM_API_VERSION";
    /// Log level
    pub const LOG_LEVEL: &'static str = "RLM_LOG_LEVEL";
    /// Log format
    pub const LOG_FORMAT: &'static str = "RLM_LOG_FORMAT";
    /// Max context size
    pub const MAX_CONTEXT_SIZE: &'static str = "RLM_MAX_CONTEXT_SIZE";
    /// Max recursive depth
    pub const MAX_RECURSIVE_DEPTH: &'static str = "RLM_MAX_RECURSIVE_DEPTH";
    /// Max iterations
    pub const MAX_ITERATIONS: &'static str = "RLM_MAX_ITERATIONS";
}

/// Get environment variable as string.
pub fn get_env_string(key: &str) -> Option<String> {
    env::var(key).ok().filter(|s| !s.is_empty())
}

/// Get environment variable as u16.
pub fn get_env_u16(key: &str) -> Option<u16> {
    get_env_string(key)?.parse().ok()
}

/// Get environment variable as u32.
pub fn get_env_u32(key: &str) -> Option<u32> {
    get_env_string(key)?.parse().ok()
}

/// Get environment variable as bool.
pub fn get_env_bool(key: &str) -> Option<bool> {
    match get_env_string(key)?.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}