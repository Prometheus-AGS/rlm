//! Backend routing logic for RLM executor.
//!
//! This module provides the backend routing functionality that allows the RLM executor
//! to select and route requests to different LLM providers (OpenAI, Azure, local models)
//! based on configuration, load balancing, health status, and routing policies.

use crate::{
    ports::{LlmProvider, ProviderMetadata},
    types::{ChatCompletionRequest, ChatCompletionResponse},
    BackendConfig, RlmError, RlmResult,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

/// Backend routing strategies for selecting which provider to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Always use the first available backend.
    FirstAvailable,
    /// Round-robin between available backends.
    RoundRobin,
    /// Route based on load (least busy backend).
    LeastLoad,
    /// Route based on provider capabilities (model support, etc.).
    ModelBased,
    /// Route based on cost optimization.
    CostOptimized,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::FirstAvailable
    }
}

/// Health status of a backend provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendHealth {
    /// Backend is healthy and can accept requests.
    Healthy,
    /// Backend is degraded but still accepting requests.
    Degraded,
    /// Backend is unhealthy and should not receive new requests.
    Unhealthy,
    /// Backend health is unknown (not yet checked).
    Unknown,
}

impl Default for BackendHealth {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Backend routing statistics for monitoring and decisions.
#[derive(Debug, Clone, Default)]
pub struct BackendStats {
    /// Total number of requests sent to this backend.
    pub total_requests: u64,
    /// Number of successful requests.
    pub successful_requests: u64,
    /// Number of failed requests.
    pub failed_requests: u64,
    /// Average response time in milliseconds.
    pub avg_response_time_ms: u64,
    /// Current number of active requests.
    pub active_requests: u64,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

/// A backend provider with metadata and statistics.
#[derive(Debug)]
pub struct RoutedBackend {
    /// The backend identifier.
    pub backend_id: String,
    /// The LLM provider implementation.
    pub provider: Arc<dyn LlmProvider>,
    /// Backend configuration.
    pub config: BackendConfig,
    /// Current health status.
    pub health: BackendHealth,
    /// Backend statistics.
    pub stats: BackendStats,
    /// Whether this backend is enabled for routing.
    pub enabled: bool,
}

impl RoutedBackend {
    /// Create a new routed backend.
    pub fn new(
        backend_id: String,
        provider: Arc<dyn LlmProvider>,
        config: BackendConfig,
    ) -> Self {
        Self {
            backend_id,
            provider,
            config,
            health: BackendHealth::Unknown,
            stats: BackendStats::default(),
            enabled: true,
        }
    }

    /// Check if this backend can handle a specific model.
    pub fn supports_model(&self, model: &str) -> bool {
        // Check if the model is mapped in the configuration
        self.config.model_mapping.contains_key(model) ||
        // Or if it's a direct model match (for providers that support the model directly)
        self.config.model_mapping.values().any(|mapped| mapped == model)
    }

    /// Get the physical model name for a logical model.
    pub fn get_physical_model(&self, logical_model: &str) -> Option<&String> {
        self.config.get_physical_model(logical_model)
    }

    /// Check if this backend is available for routing.
    pub fn is_available(&self) -> bool {
        self.enabled && matches!(self.health, BackendHealth::Healthy | BackendHealth::Degraded)
    }

    /// Calculate the backend's current load score (lower is better).
    pub fn load_score(&self) -> f64 {
        // Simple load calculation based on active requests and error rate
        let error_rate = if self.stats.total_requests > 0 {
            self.stats.failed_requests as f64 / self.stats.total_requests as f64
        } else {
            0.0
        };

        // Load score combines active requests, error rate, and response time
        self.stats.active_requests as f64 +
        (error_rate * 10.0) +
        (self.stats.avg_response_time_ms as f64 / 1000.0)
    }
}

/// Backend router that manages multiple LLM providers and routes requests.
#[derive(Debug)]
pub struct BackendRouter {
    /// All registered backends indexed by backend ID.
    backends: Arc<RwLock<HashMap<String, RoutedBackend>>>,
    /// Current routing strategy.
    routing_strategy: RoutingStrategy,
    /// Round-robin counter for round-robin routing.
    round_robin_counter: Arc<RwLock<usize>>,
}

impl BackendRouter {
    /// Create a new backend router.
    pub fn new(routing_strategy: RoutingStrategy) -> Self {
        Self {
            backends: Arc::new(RwLock::new(HashMap::new())),
            routing_strategy,
            round_robin_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a backend to the router.
    pub async fn add_backend(
        &self,
        backend_id: String,
        provider: Arc<dyn LlmProvider>,
        config: BackendConfig,
    ) -> RlmResult<()> {
        debug!("Adding backend '{}' to router", backend_id);

        // Validate the backend configuration
        config.validate()
            .map_err(|e| RlmError::Config(format!("Invalid backend config for '{}': {}", backend_id, e)))?;

        let routed_backend = RoutedBackend::new(backend_id.clone(), provider, config);

        let mut backends = self.backends.write().await;
        backends.insert(backend_id, routed_backend);

        Ok(())
    }

    /// Remove a backend from the router.
    pub async fn remove_backend(&self, backend_id: &str) -> RlmResult<()> {
        debug!("Removing backend '{}' from router", backend_id);

        let mut backends = self.backends.write().await;
        backends.remove(backend_id);

        Ok(())
    }

    /// Enable or disable a backend.
    pub async fn set_backend_enabled(&self, backend_id: &str, enabled: bool) -> RlmResult<()> {
        let mut backends = self.backends.write().await;

        if let Some(backend) = backends.get_mut(backend_id) {
            backend.enabled = enabled;
            debug!("Backend '{}' {}", backend_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(RlmError::Config(format!("Backend '{}' not found", backend_id)))
        }
    }

    /// Update the health status of a backend.
    pub async fn update_backend_health(&self, backend_id: &str, health: BackendHealth) -> RlmResult<()> {
        let mut backends = self.backends.write().await;

        if let Some(backend) = backends.get_mut(backend_id) {
            backend.health = health;
            debug!("Updated health for backend '{}': {:?}", backend_id, backend.health);
            Ok(())
        } else {
            Err(RlmError::Config(format!("Backend '{}' not found", backend_id)))
        }
    }

    /// Route a chat completion request to an appropriate backend.
    #[instrument(skip(self, request))]
    pub async fn route_request(&self, request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
        debug!("Routing request for model: {}", request.model);

        // Select the best backend for this request
        let backend_id = self.select_backend(&request.model).await?;

        debug!("Selected backend '{}' for request", backend_id);

        // Execute the request with the selected backend
        let result = self.execute_with_backend(&backend_id, request).await;

        // Update backend statistics based on the result
        self.update_backend_stats(&backend_id, &result).await;

        result
    }

    /// Select the best backend for a given model.
    async fn select_backend(&self, model: &str) -> RlmResult<String> {
        let backends = self.backends.read().await;

        // Filter available backends that support the model
        let available_backends: Vec<_> = backends
            .iter()
            .filter(|(_, backend)| backend.is_available() && backend.supports_model(model))
            .collect();

        if available_backends.is_empty() {
            return Err(RlmError::Config(format!(
                "No available backends support model '{}'", model
            )));
        }

        // Apply routing strategy
        let selected_backend_id = match self.routing_strategy {
            RoutingStrategy::FirstAvailable => {
                available_backends[0].0.clone()
            }
            RoutingStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.write().await;
                let index = *counter % available_backends.len();
                *counter += 1;
                available_backends[index].0.clone()
            }
            RoutingStrategy::LeastLoad => {
                available_backends
                    .iter()
                    .min_by(|a, b| a.1.load_score().partial_cmp(&b.1.load_score()).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(id, _)| (*id).clone())
                    .unwrap()
            }
            RoutingStrategy::ModelBased => {
                // For now, just pick the first one that supports the model
                // TODO: Implement more sophisticated model-based routing
                available_backends[0].0.clone()
            }
            RoutingStrategy::CostOptimized => {
                // For now, just pick the first one
                // TODO: Implement cost-based routing based on provider pricing
                available_backends[0].0.clone()
            }
        };

        Ok(selected_backend_id)
    }

    /// Execute a request with a specific backend.
    async fn execute_with_backend(
        &self,
        backend_id: &str,
        request: &ChatCompletionRequest,
    ) -> RlmResult<ChatCompletionResponse> {
        let backend = {
            let backends = self.backends.read().await;
            backends.get(backend_id)
                .ok_or_else(|| RlmError::Config(format!("Backend '{}' not found", backend_id)))?
                .provider.clone()
        };

        // Increment active request counter
        {
            let mut backends = self.backends.write().await;
            if let Some(backend) = backends.get_mut(backend_id) {
                backend.stats.active_requests += 1;
            }
        }

        // Execute the request
        let start_time = std::time::Instant::now();
        let result = backend.complete(request).await;
        let response_time_ms = start_time.elapsed().as_millis() as u64;

        // Decrement active request counter and update response time
        {
            let mut backends = self.backends.write().await;
            if let Some(backend) = backends.get_mut(backend_id) {
                backend.stats.active_requests = backend.stats.active_requests.saturating_sub(1);

                // Update average response time (simple moving average)
                let total_requests = backend.stats.total_requests + 1;
                backend.stats.avg_response_time_ms =
                    (backend.stats.avg_response_time_ms * backend.stats.total_requests + response_time_ms) / total_requests;
            }
        }

        result
    }

    /// Update backend statistics based on request result.
    async fn update_backend_stats(&self, backend_id: &str, result: &RlmResult<ChatCompletionResponse>) {
        let mut backends = self.backends.write().await;

        if let Some(backend) = backends.get_mut(backend_id) {
            backend.stats.total_requests += 1;

            match result {
                Ok(_) => {
                    backend.stats.successful_requests += 1;
                }
                Err(e) => {
                    backend.stats.failed_requests += 1;
                    backend.stats.last_error = Some(e.to_string());

                    // If we have too many failures, mark as unhealthy
                    let error_rate = backend.stats.failed_requests as f64 / backend.stats.total_requests as f64;
                    if error_rate > 0.5 && backend.stats.total_requests > 5 {
                        warn!("Backend '{}' has high error rate: {:.2}%, marking as degraded", backend_id, error_rate * 100.0);
                        backend.health = BackendHealth::Degraded;
                    }
                }
            }
        }
    }

    /// Get statistics for all backends.
    pub async fn get_backend_stats(&self) -> HashMap<String, (BackendHealth, BackendStats)> {
        let backends = self.backends.read().await;

        backends
            .iter()
            .map(|(id, backend)| (id.clone(), (backend.health.clone(), backend.stats.clone())))
            .collect()
    }

    /// Get the list of available backends.
    pub async fn get_available_backends(&self) -> Vec<String> {
        let backends = self.backends.read().await;

        backends
            .iter()
            .filter(|(_, backend)| backend.is_available())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check if any backend supports a specific model.
    pub async fn supports_model(&self, model: &str) -> bool {
        let backends = self.backends.read().await;

        backends.values().any(|backend| backend.supports_model(model))
    }

    /// Get metadata for all registered providers.
    pub async fn get_provider_metadata(&self) -> Vec<ProviderMetadata> {
        let backends = self.backends.read().await;
        let mut metadata = Vec::new();

        for backend in backends.values() {
            metadata.push(backend.provider.get_metadata());
        }

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{LlmProvider, ProviderMetadata};
    use crate::types::{ChatChoice, ChatMessage, ChatRole, TokenUsage};
    use async_trait::async_trait;

    // Mock LLM provider for testing
    #[derive(Debug)]
    struct MockProvider {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn complete(&self, _request: &ChatCompletionRequest) -> RlmResult<ChatCompletionResponse> {
            if self.should_fail {
                return Err(RlmError::Other("Mock failure".to_string()));
            }

            Ok(ChatCompletionResponse {
                id: "mock-response".to_string(),
                object: "chat.completion".to_string(),
                created: 1234567890,
                model: "mock-model".to_string(),
                choices: vec![ChatChoice {
                    message: ChatMessage {
                        role: ChatRole::Assistant,
                        content: "Mock response".to_string(),
                        name: None,
                    },
                    index: 0,
                    finish_reason: Some("stop".to_string()),
                }],
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                    recursive_calls: 0,
                },
            })
        }

        async fn complete_stream(
            &self,
            _request: &ChatCompletionRequest,
        ) -> RlmResult<std::pin::Pin<Box<dyn futures::Stream<Item = RlmResult<ChatCompletionResponse>> + Send>>> {
            Err(RlmError::Other("Streaming not implemented in mock".to_string()))
        }

        async fn list_models(&self) -> RlmResult<Vec<crate::ports::ModelInfo>> {
            Ok(vec![crate::ports::ModelInfo::new("gpt-4", "Mock GPT-4", 4096, 2048)])
        }

        fn validate_request(&self, _request: &ChatCompletionRequest) -> RlmResult<()> {
            Ok(())
        }

        fn get_metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                base_url: "https://mock.api".to_string(),
                requires_auth: true,
                rate_limit_rpm: Some(60),
                rate_limit_tpm: Some(10000),
                capabilities: vec!["chat".to_string()],
            }
        }

        async fn health_check(&self) -> RlmResult<bool> {
            Ok(!self.should_fail)
        }
    }

    #[tokio::test]
    async fn test_backend_router_creation() {
        let router = BackendRouter::new(RoutingStrategy::RoundRobin);
        assert!(matches!(router.routing_strategy, RoutingStrategy::RoundRobin));
    }

    #[tokio::test]
    async fn test_add_backend() {
        let router = BackendRouter::new(RoutingStrategy::FirstAvailable);
        let provider = Arc::new(MockProvider {
            name: "test-provider".to_string(),
            should_fail: false,
        });
        let config = BackendConfig::openai("test-key".to_string());

        let result = router.add_backend("test-backend".to_string(), provider, config).await;
        assert!(result.is_ok());

        let available = router.get_available_backends().await;
        assert_eq!(available.len(), 1);
    }

    #[tokio::test]
    async fn test_backend_routing() {
        let router = BackendRouter::new(RoutingStrategy::FirstAvailable);
        let provider = Arc::new(MockProvider {
            name: "test-provider".to_string(),
            should_fail: false,
        });
        let config = BackendConfig::openai("test-key".to_string());

        router.add_backend("test-backend".to_string(), provider, config).await.unwrap();

        // Set backend as healthy
        router.update_backend_health("test-backend", BackendHealth::Healthy).await.unwrap();

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Test message".to_string(),
                name: None,
            }],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = router.route_request(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_no_available_backends() {
        let router = BackendRouter::new(RoutingStrategy::FirstAvailable);

        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = router.route_request(&request).await;
        assert!(result.is_err());
    }
}