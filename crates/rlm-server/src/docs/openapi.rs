//! OpenAPI Specification for RLM Server
//!
//! This module generates the complete OpenAPI 3.0 specification for the RLM server,
//! providing OpenAI-compatible endpoints with RLM-specific extensions.

use serde_json::{json, Value};
use std::collections::HashMap;

/// Generate the complete OpenAPI specification
pub fn generate_openapi_spec() -> Value {
    json!({
        "openapi": "3.0.0",
        "info": {
            "title": "RLM OpenAI-Compatible Server",
            "description": "RLM (Recursive Language Model) server providing OpenAI-compatible chat completions API with support for arbitrarily long contexts through recursive decomposition",
            "version": "1.0.0",
            "contact": {
                "name": "RLM Development Team",
                "url": "https://github.com/rlm-project/rlm"
            },
            "license": {
                "name": "MIT",
                "url": "https://opensource.org/licenses/MIT"
            }
        },
        "servers": [
            {
                "url": "http://localhost:8080",
                "description": "Local development server"
            },
            {
                "url": "https://api.rlm.example.com",
                "description": "Production server"
            }
        ],
        "paths": generate_api_paths(),
        "components": {
            "schemas": generate_schemas(),
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "API Key"
                },
                "apiKeyAuth": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "Authorization"
                }
            }
        },
        "security": [
            {
                "bearerAuth": []
            }
        ],
        "tags": [
            {
                "name": "Chat Completions",
                "description": "OpenAI-compatible chat completion endpoints with RLM enhancements"
            },
            {
                "name": "Models",
                "description": "Available model information and capabilities"
            },
            {
                "name": "Health",
                "description": "Server health and monitoring endpoints"
            },
            {
                "name": "Metrics",
                "description": "Performance metrics and observability"
            },
            {
                "name": "RLM Extensions",
                "description": "RLM-specific endpoints and features"
            }
        ]
    })
}

/// Generate API path definitions
fn generate_api_paths() -> Value {
    json!({
        "/v1/chat/completions": {
            "post": {
                "tags": ["Chat Completions"],
                "summary": "Create chat completion",
                "description": "Creates a chat completion with RLM recursive processing for long contexts. Compatible with OpenAI ChatGPT API.",
                "operationId": "createChatCompletion",
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": "#/components/schemas/ChatCompletionRequest"
                            },
                            "examples": {
                                "basic_completion": {
                                    "summary": "Basic completion request",
                                    "value": {
                                        "model": "gpt-4",
                                        "messages": [
                                            {
                                                "role": "user",
                                                "content": "What is the capital of France?"
                                            }
                                        ],
                                        "max_tokens": 100,
                                        "temperature": 0.7
                                    }
                                },
                                "long_context_completion": {
                                    "summary": "Long context completion with RLM",
                                    "value": {
                                        "model": "gpt-4",
                                        "messages": [
                                            {
                                                "role": "user",
                                                "content": "[Very long context document...] Please summarize the key findings from this 500-page research document."
                                            }
                                        ],
                                        "max_tokens": 2000,
                                        "temperature": 0.3,
                                        "rlm_config": {
                                            "max_recursion_depth": 5,
                                            "chunk_strategy": "semantic",
                                            "enable_streaming": true
                                        }
                                    }
                                },
                                "streaming_completion": {
                                    "summary": "Streaming completion request",
                                    "value": {
                                        "model": "gpt-4",
                                        "messages": [
                                            {
                                                "role": "user",
                                                "content": "Write a detailed explanation of quantum computing."
                                            }
                                        ],
                                        "stream": true,
                                        "max_tokens": 1500
                                    }
                                }
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Successful completion response",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChatCompletionResponse"
                                }
                            },
                            "text/event-stream": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChatCompletionStreamResponse"
                                }
                            }
                        }
                    },
                    "400": {
                        "description": "Bad request - invalid parameters",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    },
                    "401": {
                        "description": "Unauthorized - invalid API key",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    },
                    "429": {
                        "description": "Rate limit exceeded",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    },
                    "500": {
                        "description": "Internal server error",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/v1/models": {
            "get": {
                "tags": ["Models"],
                "summary": "List available models",
                "description": "Lists all models available through the configured backends",
                "operationId": "listModels",
                "responses": {
                    "200": {
                        "description": "List of available models",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ModelList"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/v1/models/{model}": {
            "get": {
                "tags": ["Models"],
                "summary": "Get model information",
                "description": "Retrieves detailed information about a specific model",
                "operationId": "getModel",
                "parameters": [
                    {
                        "name": "model",
                        "in": "path",
                        "required": true,
                        "description": "Model identifier",
                        "schema": {
                            "type": "string",
                            "example": "gpt-4"
                        }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Model information",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Model"
                                }
                            }
                        }
                    },
                    "404": {
                        "description": "Model not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/health": {
            "get": {
                "tags": ["Health"],
                "summary": "Health check",
                "description": "Returns server health status and component availability",
                "operationId": "healthCheck",
                "responses": {
                    "200": {
                        "description": "Server is healthy",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/HealthStatus"
                                }
                            }
                        }
                    },
                    "503": {
                        "description": "Server is unhealthy",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/HealthStatus"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/metrics": {
            "get": {
                "tags": ["Metrics"],
                "summary": "Prometheus metrics",
                "description": "Returns Prometheus-compatible metrics for monitoring",
                "operationId": "getMetrics",
                "responses": {
                    "200": {
                        "description": "Metrics in Prometheus format",
                        "content": {
                            "text/plain": {
                                "schema": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/v1/rlm/sessions": {
            "get": {
                "tags": ["RLM Extensions"],
                "summary": "List active RLM sessions",
                "description": "Lists all active RLM processing sessions with their status",
                "operationId": "listRlmSessions",
                "responses": {
                    "200": {
                        "description": "List of active sessions",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/SessionList"
                                }
                            }
                        }
                    }
                }
            }
        },
        "/v1/rlm/sessions/{sessionId}": {
            "get": {
                "tags": ["RLM Extensions"],
                "summary": "Get RLM session details",
                "description": "Returns detailed information about a specific RLM session",
                "operationId": "getRlmSession",
                "parameters": [
                    {
                        "name": "sessionId",
                        "in": "path",
                        "required": true,
                        "description": "Session identifier",
                        "schema": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Session details",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/RlmSession"
                                }
                            }
                        }
                    },
                    "404": {
                        "description": "Session not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    }
                }
            },
            "delete": {
                "tags": ["RLM Extensions"],
                "summary": "Cancel RLM session",
                "description": "Cancels an active RLM processing session",
                "operationId": "cancelRlmSession",
                "parameters": [
                    {
                        "name": "sessionId",
                        "in": "path",
                        "required": true,
                        "description": "Session identifier",
                        "schema": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }
                ],
                "responses": {
                    "204": {
                        "description": "Session cancelled successfully"
                    },
                    "404": {
                        "description": "Session not found",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Error"
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Generate schema definitions
fn generate_schemas() -> Value {
    json!({
        "ChatCompletionRequest": {
            "type": "object",
            "required": ["model", "messages"],
            "properties": {
                "model": {
                    "type": "string",
                    "description": "Model to use for completion",
                    "example": "gpt-4"
                },
                "messages": {
                    "type": "array",
                    "description": "List of messages in the conversation",
                    "items": {
                        "$ref": "#/components/schemas/ChatMessage"
                    }
                },
                "max_tokens": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 4096,
                    "description": "Maximum number of tokens to generate",
                    "example": 100
                },
                "temperature": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 2,
                    "description": "Sampling temperature",
                    "example": 0.7
                },
                "top_p": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "Nucleus sampling parameter",
                    "example": 1.0
                },
                "stream": {
                    "type": "boolean",
                    "description": "Whether to stream responses via Server-Sent Events",
                    "default": false
                },
                "stop": {
                    "oneOf": [
                        {"type": "string"},
                        {
                            "type": "array",
                            "items": {"type": "string"},
                            "maxItems": 4
                        }
                    ],
                    "description": "Stop sequences for response termination"
                },
                "presence_penalty": {
                    "type": "number",
                    "minimum": -2,
                    "maximum": 2,
                    "description": "Penalty for token presence"
                },
                "frequency_penalty": {
                    "type": "number",
                    "minimum": -2,
                    "maximum": 2,
                    "description": "Penalty for token frequency"
                },
                "rlm_config": {
                    "$ref": "#/components/schemas/RlmConfig"
                }
            }
        },
        "ChatMessage": {
            "type": "object",
            "required": ["role", "content"],
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["system", "user", "assistant"],
                    "description": "Message role"
                },
                "content": {
                    "type": "string",
                    "description": "Message content"
                },
                "name": {
                    "type": "string",
                    "description": "Optional message name"
                }
            }
        },
        "RlmConfig": {
            "type": "object",
            "description": "RLM-specific configuration options",
            "properties": {
                "max_recursion_depth": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10,
                    "description": "Maximum depth for recursive calls",
                    "default": 5
                },
                "chunk_strategy": {
                    "type": "string",
                    "enum": ["fixed_size", "semantic", "adaptive"],
                    "description": "Context chunking strategy",
                    "default": "adaptive"
                },
                "enable_streaming": {
                    "type": "boolean",
                    "description": "Enable streaming for long-running operations",
                    "default": true
                },
                "context_window": {
                    "type": "integer",
                    "minimum": 1000,
                    "description": "Context window size in tokens"
                },
                "aggregation_strategy": {
                    "type": "string",
                    "enum": ["simple", "weighted", "hierarchical"],
                    "description": "Result aggregation strategy",
                    "default": "hierarchical"
                }
            }
        },
        "ChatCompletionResponse": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Unique response identifier"
                },
                "object": {
                    "type": "string",
                    "enum": ["chat.completion"],
                    "description": "Object type"
                },
                "created": {
                    "type": "integer",
                    "description": "Response creation timestamp"
                },
                "model": {
                    "type": "string",
                    "description": "Model used for completion"
                },
                "choices": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/ChatChoice"
                    }
                },
                "usage": {
                    "$ref": "#/components/schemas/Usage"
                },
                "rlm_metadata": {
                    "$ref": "#/components/schemas/RlmMetadata"
                }
            }
        },
        "ChatChoice": {
            "type": "object",
            "properties": {
                "index": {
                    "type": "integer",
                    "description": "Choice index"
                },
                "message": {
                    "$ref": "#/components/schemas/ChatMessage"
                },
                "finish_reason": {
                    "type": "string",
                    "enum": ["stop", "length", "content_filter", "null"],
                    "description": "Reason for completion finish"
                }
            }
        },
        "ChatCompletionStreamResponse": {
            "type": "object",
            "description": "Server-Sent Event response chunk",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Response identifier"
                },
                "object": {
                    "type": "string",
                    "enum": ["chat.completion.chunk"],
                    "description": "Object type"
                },
                "created": {
                    "type": "integer",
                    "description": "Chunk creation timestamp"
                },
                "model": {
                    "type": "string",
                    "description": "Model used"
                },
                "choices": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/StreamChoice"
                    }
                }
            }
        },
        "StreamChoice": {
            "type": "object",
            "properties": {
                "index": {
                    "type": "integer"
                },
                "delta": {
                    "type": "object",
                    "properties": {
                        "role": {
                            "type": "string"
                        },
                        "content": {
                            "type": "string"
                        }
                    }
                },
                "finish_reason": {
                    "type": "string",
                    "nullable": true
                }
            }
        },
        "Usage": {
            "type": "object",
            "properties": {
                "prompt_tokens": {
                    "type": "integer",
                    "description": "Tokens used in the prompt"
                },
                "completion_tokens": {
                    "type": "integer",
                    "description": "Tokens used in the completion"
                },
                "total_tokens": {
                    "type": "integer",
                    "description": "Total tokens used"
                }
            }
        },
        "RlmMetadata": {
            "type": "object",
            "description": "RLM-specific response metadata",
            "properties": {
                "recursive_calls": {
                    "type": "integer",
                    "description": "Number of recursive sub-calls made"
                },
                "context_chunks": {
                    "type": "integer",
                    "description": "Number of context chunks processed"
                },
                "processing_time_ms": {
                    "type": "integer",
                    "description": "Total processing time in milliseconds"
                },
                "memory_usage_mb": {
                    "type": "number",
                    "description": "Peak memory usage in megabytes"
                },
                "complexity_class": {
                    "type": "string",
                    "enum": ["O(1)", "O(log n)", "O(n)", "O(n log n)", "O(n^2)"],
                    "description": "Estimated computational complexity"
                }
            }
        },
        "ModelList": {
            "type": "object",
            "properties": {
                "object": {
                    "type": "string",
                    "enum": ["list"]
                },
                "data": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/Model"
                    }
                }
            }
        },
        "Model": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Model identifier"
                },
                "object": {
                    "type": "string",
                    "enum": ["model"]
                },
                "created": {
                    "type": "integer",
                    "description": "Model creation timestamp"
                },
                "owned_by": {
                    "type": "string",
                    "description": "Organization owning the model"
                },
                "permission": {
                    "type": "array",
                    "items": {
                        "type": "object"
                    }
                },
                "context_length": {
                    "type": "integer",
                    "description": "Maximum context length supported"
                },
                "rlm_capable": {
                    "type": "boolean",
                    "description": "Whether model supports RLM recursive processing"
                }
            }
        },
        "HealthStatus": {
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["healthy", "unhealthy", "degraded"]
                },
                "timestamp": {
                    "type": "string",
                    "format": "date-time"
                },
                "version": {
                    "type": "string",
                    "description": "Server version"
                },
                "uptime_seconds": {
                    "type": "integer",
                    "description": "Server uptime in seconds"
                },
                "components": {
                    "type": "object",
                    "additionalProperties": {
                        "$ref": "#/components/schemas/ComponentHealth"
                    }
                }
            }
        },
        "ComponentHealth": {
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["healthy", "unhealthy", "unknown"]
                },
                "last_check": {
                    "type": "string",
                    "format": "date-time"
                },
                "details": {
                    "type": "object",
                    "additionalProperties": true
                }
            }
        },
        "SessionList": {
            "type": "object",
            "properties": {
                "sessions": {
                    "type": "array",
                    "items": {
                        "$ref": "#/components/schemas/RlmSession"
                    }
                },
                "total": {
                    "type": "integer"
                }
            }
        },
        "RlmSession": {
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "format": "uuid"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "processing", "completed", "failed", "cancelled"]
                },
                "created_at": {
                    "type": "string",
                    "format": "date-time"
                },
                "updated_at": {
                    "type": "string",
                    "format": "date-time"
                },
                "model": {
                    "type": "string"
                },
                "context_size_tokens": {
                    "type": "integer"
                },
                "recursive_calls": {
                    "type": "integer"
                },
                "processing_time_ms": {
                    "type": "integer"
                },
                "memory_usage_mb": {
                    "type": "number"
                }
            }
        },
        "Error": {
            "type": "object",
            "properties": {
                "error": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Human-readable error message"
                        },
                        "type": {
                            "type": "string",
                            "description": "Error type identifier"
                        },
                        "code": {
                            "type": "string",
                            "description": "Error code for programmatic handling"
                        },
                        "param": {
                            "type": "string",
                            "description": "Parameter that caused the error"
                        }
                    }
                }
            }
        }
    })
}

/// Generate OpenAPI specification as JSON string
pub fn generate_openapi_json() -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&generate_openapi_spec())
}

/// Generate OpenAPI specification as YAML string
pub fn generate_openapi_yaml() -> Result<String, Box<dyn std::error::Error>> {
    let spec = generate_openapi_spec();
    serde_yaml::to_string(&spec).map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = generate_openapi_spec();

        // Verify basic structure
        assert_eq!(spec["openapi"], "3.0.0");
        assert_eq!(spec["info"]["title"], "RLM OpenAI-Compatible Server");

        // Verify key endpoints exist
        assert!(spec["paths"]["/v1/chat/completions"].is_object());
        assert!(spec["paths"]["/v1/models"].is_object());
        assert!(spec["paths"]["/health"].is_object());

        // Verify schemas exist
        assert!(spec["components"]["schemas"]["ChatCompletionRequest"].is_object());
        assert!(spec["components"]["schemas"]["RlmConfig"].is_object());
    }

    #[test]
    fn test_openapi_json_generation() {
        let json_result = generate_openapi_json();
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("RLM OpenAI-Compatible Server"));
        assert!(json_str.contains("/v1/chat/completions"));
    }

    #[test]
    fn test_schema_completeness() {
        let spec = generate_openapi_spec();
        let schemas = &spec["components"]["schemas"];

        // Verify all required schemas are present
        let required_schemas = vec![
            "ChatCompletionRequest",
            "ChatCompletionResponse",
            "ChatMessage",
            "RlmConfig",
            "RlmMetadata",
            "Error",
            "HealthStatus"
        ];

        for schema_name in required_schemas {
            assert!(schemas[schema_name].is_object(), "Schema {} is missing", schema_name);
        }
    }
}