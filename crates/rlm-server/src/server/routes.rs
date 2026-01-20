//! HTTP route handlers for OpenAI-compatible API.
//!
//! This module implements the core chat completions endpoint that provides
//! OpenAI API compatibility while using the RLM recursive execution engine.
//!
//! Currently provides a basic structure for T033 implementation.
//! Full integration with OpenAI provider will be completed in T034-T035.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use rlm_core::types::{ChatCompletionRequest, RlmRequest};
use rlm_core::config::RlmConfig;
use rlm_core::ports::LlmProvider;
use rlm_core::executor::RlmExecutor;
use rlm_repl_rhai::RhaiReplBackend;
use crate::adapters::{OpenAiProvider, OpenAiConfigBuilder};
use crate::health::{BackendHealthChecker, HealthCheckConfig};
use crate::server::{streaming, connection_manager::SseConnectionManager};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error};
use uuid::Uuid;

/// Application state containing shared resources.
#[derive(Debug, Clone)]
pub struct AppState {
    /// OpenAI provider for LLM completions.
    pub openai_provider: Arc<OpenAiProvider>,
    /// RLM executor for recursive language model processing.
    pub rlm_executor: Arc<RlmExecutor<RhaiReplBackend, OpenAiProvider>>,
    /// All configured backend providers for multi-backend support.
    pub backend_providers: Arc<HashMap<String, Arc<dyn LlmProvider + Send + Sync>>>,
    /// Backend health checker for system monitoring.
    pub health_checker: Arc<BackendHealthChecker>,
    /// SSE connection manager for streaming connections.
    pub connection_manager: Arc<SseConnectionManager>,
}

impl AppState {
    /// Create new app state with OpenAI provider and RLM executor.
    pub fn new() -> anyhow::Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "mock-key".to_string());

        // Create OpenAI provider configuration
        let openai_config = OpenAiConfigBuilder::new(api_key.clone())
            .build();

        // Create two separate provider instances (one for direct use, one for RLM)
        let direct_provider = OpenAiProvider::new(openai_config.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI provider: {}", e))?;

        let executor_provider = OpenAiProvider::new(openai_config)
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI provider for executor: {}", e))?;

        // Create Rhai REPL backend
        let repl_backend = RhaiReplBackend::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Rhai REPL backend: {}", e))?;

        // Create RLM configuration with reasonable defaults
        let rlm_config = RlmConfig::default();

        // Create RLM executor
        let rlm_executor = RlmExecutor::new(rlm_config, repl_backend, executor_provider);

        // Create Arc for the direct provider to share it
        let direct_provider_arc = Arc::new(direct_provider);

        // Create backend providers map for health checking
        let mut backend_providers: HashMap<String, Arc<dyn LlmProvider + Send + Sync>> = HashMap::new();
        backend_providers.insert("openai".to_string(), direct_provider_arc.clone());

        // Create health checker
        let health_config = HealthCheckConfig::default();
        let health_checker = BackendHealthChecker::new(health_config);

        // Create SSE connection manager
        let connection_manager = SseConnectionManager::with_defaults();

        Ok(Self {
            openai_provider: direct_provider_arc,
            rlm_executor: Arc::new(rlm_executor),
            backend_providers: Arc::new(backend_providers),
            health_checker: Arc::new(health_checker),
            connection_manager: Arc::new(connection_manager),
        })
    }
}

/// Create the v1 API router with all endpoints.
///
/// Now integrated with OpenAI provider for T034 and streaming support for T042.
pub fn create_v1_router() -> Router<AppState> {
    Router::new()
        // Chat completions endpoint (main RLM functionality)
        .route("/chat/completions", post(chat_completions_handler))
        // Models endpoint for OpenAI compatibility
        .route("/models", get(list_models))
        .route("/models/:model_id", get(get_model))
        // Health check specific to RLM functionality
        .route("/rlm/health", get(rlm_health_check))
}

/// Create system-wide health endpoint at root level.
pub fn create_health_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(system_health_check))
}

/// Handle chat completions requests.
///
/// Now integrated with RLM executor for T035.
/// Supports both direct OpenAI completions and RLM recursive processing.
async fn chat_completions(
    State(state): State<AppState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Received chat completion request: {:?}", request);

    // Validate the request
    validate_chat_request(&request)?;

    // Check if this should use RLM processing
    // RLM is triggered by having rlm_config in the request
    if let Some(_rlm_config) = &request.rlm_config {
        debug!("Using RLM executor for recursive processing");
        return handle_rlm_request(state, request).await;
    }

    // Otherwise, use direct OpenAI provider
    debug!("Using direct OpenAI completion");
    match state.openai_provider.complete(&request).await {
        Ok(response) => {
            debug!("OpenAI completion successful");

            // Convert response to JSON for compatibility
            let json_response = json!({
                "id": response.id,
                "object": "chat.completion",
                "created": response.created,
                "model": response.model,
                "choices": response.choices.iter().map(|choice| json!({
                    "index": choice.index,
                    "message": {
                        "role": format!("{:?}", choice.message.role).to_lowercase(),
                        "content": choice.message.content
                    },
                    "finish_reason": choice.finish_reason
                })).collect::<Vec<_>>(),
                "usage": {
                    "prompt_tokens": response.usage.prompt_tokens,
                    "completion_tokens": response.usage.completion_tokens,
                    "total_tokens": response.usage.total_tokens
                }
            });

            Ok(Json(json_response))
        }
        Err(e) => {
            error!("OpenAI completion failed: {:?}", e);
            match e {
                rlm_core::error::RlmError::Config(msg) => {
                    Err(AppError::BadRequest(format!("Configuration error: {}", msg)))
                }
                rlm_core::error::RlmError::Timeout(_) => {
                    Err(AppError::Internal("Request timeout".to_string()))
                }
                _ => {
                    Err(AppError::Internal(format!("OpenAI API error: {}", e)))
                }
            }
        }
    }
}

/// Chat completions handler that routes between streaming and non-streaming responses.
///
/// This function determines whether to return an SSE stream or a JSON response
/// based on the `stream` parameter in the request.
async fn chat_completions_handler(
    state: State<AppState>,
    request: Json<ChatCompletionRequest>,
) -> Result<Response, AppError> {
    // Check if streaming is requested
    if request.stream {
        // Route to streaming handler
        let sse_response = streaming::stream_chat_completions(state, request).await?;
        Ok(sse_response.into_response())
    } else {
        // Route to non-streaming handler
        let json_response = chat_completions(state, request).await?;
        Ok(json_response.into_response())
    }
}

/// Handle RLM-specific chat completion requests.
///
/// Converts ChatCompletionRequest to RlmRequest and uses the RLM executor.
async fn handle_rlm_request(
    state: AppState,
    request: ChatCompletionRequest,
) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Processing RLM request with recursive execution");

    // Convert ChatCompletionRequest to RlmRequest
    // For now, we'll use the last user message as the query and previous messages as context
    let mut query = String::new();
    let mut context = String::new();

    for message in &request.messages {
        match message.role {
            rlm_core::types::ChatRole::User => {
                if query.is_empty() {
                    query = message.content.clone();
                } else {
                    context.push_str(&format!("User: {}\n", message.content));
                }
            }
            rlm_core::types::ChatRole::Assistant => {
                context.push_str(&format!("Assistant: {}\n", message.content));
            }
            rlm_core::types::ChatRole::System => {
                context.push_str(&format!("System: {}\n", message.content));
            }
        }
    }

    // Use the last user message as query, everything else as context
    if query.is_empty() {
        return Err(AppError::BadRequest("No user message found for RLM processing".to_string()));
    }

    let rlm_request = RlmRequest {
        query,
        context,
        max_iterations: 50, // Default from paper
        recursion_depth: request.rlm_config.as_ref()
            .and_then(|c| c.max_recursive_depth)
            .unwrap_or(1),
        metadata: HashMap::new(),
    };

    // Execute with RLM
    match state.rlm_executor.execute(rlm_request).await {
        Ok(rlm_response) => {
            debug!("RLM execution successful");

            // Convert RlmResponse back to OpenAI format
            let response = json!({
                "id": format!("rlm-{}", Uuid::new_v4()),
                "object": "chat.completion",
                "created": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                "model": request.model,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": rlm_response.answer
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": rlm_response.metadata.total_tokens / 2, // Rough estimate
                    "completion_tokens": rlm_response.metadata.total_tokens / 2, // Rough estimate
                    "total_tokens": rlm_response.metadata.total_tokens
                }
            });

            Ok(Json(response))
        }
        Err(e) => {
            error!("RLM execution failed: {:?}", e);
            match e {
                rlm_core::error::RlmError::Config(msg) => {
                    Err(AppError::BadRequest(format!("RLM configuration error: {}", msg)))
                }
                rlm_core::error::RlmError::Timeout(_) => {
                    Err(AppError::Internal("RLM execution timeout".to_string()))
                }
                _ => {
                    Err(AppError::Internal(format!("RLM execution error: {}", e)))
                }
            }
        }
    }
}

/// List available models.
///
/// Now integrated with OpenAI provider for T034.
async fn list_models(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Listing available models from OpenAI");

    match state.openai_provider.list_models().await {
        Ok(models) => {
            let data: Vec<_> = models.iter().map(|model| json!({
                "id": model.id,
                "object": "model",
                "created": 1677610602, // Fallback timestamp
                "owned_by": "openai" // Default for OpenAI models
            })).collect();

            Ok(Json(json!({
                "object": "list",
                "data": data
            })))
        }
        Err(e) => {
            error!("Failed to fetch models: {:?}", e);
            // Fallback to basic model list if OpenAI call fails
            Ok(Json(json!({
                "object": "list",
                "data": [
                    {
                        "id": "gpt-4",
                        "object": "model",
                        "created": 1677610602,
                        "owned_by": "openai"
                    },
                    {
                        "id": "gpt-3.5-turbo",
                        "object": "model",
                        "created": 1677610602,
                        "owned_by": "openai"
                    }
                ]
            })))
        }
    }
}

/// Get details for a specific model.
///
/// Returns mock model details for T033. Will be integrated with OpenAI provider in T034.
async fn get_model(Path(model_id): Path<String>) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Getting model details for: {}", model_id);

    // Mock response for supported models
    match model_id.as_str() {
        "gpt-4" | "gpt-3.5-turbo" => {
            let response = json!({
                "id": model_id,
                "object": "model",
                "created": 1677610602,
                "owned_by": "openai"
            });
            Ok(Json(response))
        }
        _ => Err(AppError::ModelNotFound(model_id)),
    }
}

/// RLM-specific health check.
///
/// Basic implementation for T033. Will be enhanced with provider checks in T034.
async fn rlm_health_check() -> Json<serde_json::Value> {
    debug!("Performing RLM health check");

    let response = json!({
        "status": "healthy",
        "services": {
            "openai_provider": "not_implemented",
            "rlm_executor": "not_implemented"
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "phase": "T033_basic_routes"
    });

    Json(response)
}

/// System-wide health check with comprehensive backend monitoring.
///
/// Implementation for T052. Provides complete health status including all
/// configured backends, system resources, and detailed diagnostics.
async fn system_health_check(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    debug!("Performing comprehensive system health check");

    // Perform health check using the health checker
    match state.health_checker.check_system_health(&state.backend_providers).await {
        Ok(health_result) => {
            debug!("System health check completed: {:?}", health_result.status);

            // Convert to JSON response
            let json_response = serde_json::to_value(health_result)
                .map_err(|e| AppError::Internal(format!("Failed to serialize health response: {}", e)))?;

            Ok(Json(json_response))
        }
        Err(e) => {
            error!("System health check failed: {}", e);

            // Return unhealthy status on error
            let error_response = json!({
                "status": "unhealthy",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION"),
                "backends": {},
                "system": {
                    "memory": {
                        "used_bytes": 0,
                        "available_bytes": null,
                        "usage_percent": null
                    },
                    "active_sessions": 0,
                    "uptime_seconds": 0
                },
                "error": e.to_string()
            });

            Ok(Json(error_response))
        }
    }
}

/// Validate a chat completion request.
pub fn validate_chat_request(request: &ChatCompletionRequest) -> Result<(), AppError> {
    if request.messages.is_empty() {
        return Err(AppError::BadRequest(
            "Messages array cannot be empty".to_string(),
        ));
    }

    if request.model.is_empty() {
        return Err(AppError::BadRequest(
            "Model field cannot be empty".to_string(),
        ));
    }

    // Validate temperature range
    if let Some(temp) = request.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return Err(AppError::BadRequest(
                "Temperature must be between 0.0 and 2.0".to_string(),
            ));
        }
    }

    // Validate max_tokens
    if let Some(max_tokens) = request.max_tokens {
        if max_tokens <= 0 {
            return Err(AppError::BadRequest(
                "max_tokens must be positive".to_string(),
            ));
        }
    }

    Ok(())
}

/// Application-specific error type for HTTP responses.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Bad request from client.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Model not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::ModelNotFound(model) => {
                (StatusCode::NOT_FOUND, format!("Model '{}' not found", model))
            }
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = json!({
            "error": {
                "message": message,
                "type": "invalid_request_error"
            }
        });

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::types::{ChatMessage, ChatRole};

    #[test]
    fn test_validate_chat_request_empty_messages() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = validate_chat_request(&request);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_validate_chat_request_empty_model() {
        let request = ChatCompletionRequest {
            model: "".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = validate_chat_request(&request);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_validate_chat_request_invalid_temperature() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: Some(3.0), // Invalid temperature > 2.0
            max_tokens: None,
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = validate_chat_request(&request);
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_validate_chat_request_valid() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(1000),
            stream: false,
            stop: None,
            top_p: None,
            rlm_config: None,
        };

        let result = validate_chat_request(&request);
        assert!(result.is_ok());
    }
}