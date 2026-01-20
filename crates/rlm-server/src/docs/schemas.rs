//! Schema Definitions for API Documentation
//!
//! This module provides schema definitions, validation rules, and documentation
//! for all API request and response types used in the RLM server.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Generate JSON Schema definitions for all API types
pub fn generate_json_schemas() -> HashMap<String, Value> {
    let mut schemas = HashMap::new();

    // Core request/response schemas
    schemas.insert("ChatCompletionRequest".to_string(), chat_completion_request_schema());
    schemas.insert("ChatCompletionResponse".to_string(), chat_completion_response_schema());
    schemas.insert("ChatMessage".to_string(), chat_message_schema());
    schemas.insert("RlmConfig".to_string(), rlm_config_schema());
    schemas.insert("Usage".to_string(), usage_schema());
    schemas.insert("RlmMetadata".to_string(), rlm_metadata_schema());

    // Streaming schemas
    schemas.insert("StreamResponse".to_string(), stream_response_schema());
    schemas.insert("StreamChoice".to_string(), stream_choice_schema());
    schemas.insert("StreamDelta".to_string(), stream_delta_schema());

    // Model schemas
    schemas.insert("ModelList".to_string(), model_list_schema());
    schemas.insert("Model".to_string(), model_schema());

    // Health and monitoring schemas
    schemas.insert("HealthStatus".to_string(), health_status_schema());
    schemas.insert("ComponentHealth".to_string(), component_health_schema());
    schemas.insert("Metrics".to_string(), metrics_schema());

    // RLM-specific schemas
    schemas.insert("RlmSession".to_string(), rlm_session_schema());
    schemas.insert("SessionList".to_string(), session_list_schema());
    schemas.insert("RecursiveCall".to_string(), recursive_call_schema());

    // Error schemas
    schemas.insert("Error".to_string(), error_schema());
    schemas.insert("ValidationError".to_string(), validation_error_schema());

    schemas
}

/// Chat completion request schema
fn chat_completion_request_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Chat Completion Request",
        "description": "Request for creating a chat completion with optional RLM configuration",
        "required": ["model", "messages"],
        "properties": {
            "model": {
                "type": "string",
                "description": "ID of the model to use",
                "examples": ["gpt-4", "gpt-3.5-turbo", "claude-3-sonnet"],
                "minLength": 1,
                "maxLength": 100
            },
            "messages": {
                "type": "array",
                "description": "A list of messages comprising the conversation",
                "items": {
                    "$ref": "#/definitions/ChatMessage"
                },
                "minItems": 1,
                "maxItems": 100
            },
            "max_tokens": {
                "type": "integer",
                "description": "The maximum number of tokens to generate",
                "minimum": 1,
                "maximum": 32768,
                "default": 1024
            },
            "temperature": {
                "type": "number",
                "description": "Sampling temperature between 0 and 2",
                "minimum": 0,
                "maximum": 2,
                "default": 1.0,
                "examples": [0.7, 0.9, 1.2]
            },
            "top_p": {
                "type": "number",
                "description": "Nucleus sampling parameter",
                "minimum": 0,
                "maximum": 1,
                "default": 1.0
            },
            "n": {
                "type": "integer",
                "description": "Number of completions to generate",
                "minimum": 1,
                "maximum": 10,
                "default": 1
            },
            "stream": {
                "type": "boolean",
                "description": "Whether to stream partial results via SSE",
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
                "description": "Up to 4 sequences where the API will stop generating"
            },
            "presence_penalty": {
                "type": "number",
                "description": "Penalty for new tokens based on their presence",
                "minimum": -2.0,
                "maximum": 2.0,
                "default": 0.0
            },
            "frequency_penalty": {
                "type": "number",
                "description": "Penalty for new tokens based on their frequency",
                "minimum": -2.0,
                "maximum": 2.0,
                "default": 0.0
            },
            "logit_bias": {
                "type": "object",
                "description": "Bias values for specific tokens",
                "additionalProperties": {
                    "type": "number",
                    "minimum": -100,
                    "maximum": 100
                }
            },
            "user": {
                "type": "string",
                "description": "Unique identifier for the end user",
                "maxLength": 100
            },
            "rlm_config": {
                "$ref": "#/definitions/RlmConfig",
                "description": "RLM-specific configuration options"
            }
        },
        "additionalProperties": false,
        "examples": [
            {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a helpful assistant."
                    },
                    {
                        "role": "user",
                        "content": "Hello!"
                    }
                ],
                "max_tokens": 100,
                "temperature": 0.7
            },
            {
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "user",
                        "content": "Analyze this large document: [CONTENT...]"
                    }
                ],
                "max_tokens": 2000,
                "rlm_config": {
                    "max_recursion_depth": 5,
                    "chunk_strategy": "semantic"
                }
            }
        ]
    })
}

/// Chat completion response schema
fn chat_completion_response_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Chat Completion Response",
        "description": "Response from chat completion API",
        "required": ["id", "object", "created", "model", "choices"],
        "properties": {
            "id": {
                "type": "string",
                "description": "Unique identifier for the completion",
                "pattern": "^chatcmpl-[a-zA-Z0-9]+$",
                "examples": ["chatcmpl-123abc"]
            },
            "object": {
                "type": "string",
                "enum": ["chat.completion"],
                "description": "Object type identifier"
            },
            "created": {
                "type": "integer",
                "description": "Unix timestamp of completion creation",
                "minimum": 1609459200
            },
            "model": {
                "type": "string",
                "description": "Model used for the completion",
                "examples": ["gpt-4", "gpt-3.5-turbo"]
            },
            "system_fingerprint": {
                "type": "string",
                "description": "System fingerprint for reproducibility"
            },
            "choices": {
                "type": "array",
                "description": "List of completion choices",
                "items": {
                    "type": "object",
                    "required": ["index", "message", "finish_reason"],
                    "properties": {
                        "index": {
                            "type": "integer",
                            "description": "Choice index",
                            "minimum": 0
                        },
                        "message": {
                            "$ref": "#/definitions/ChatMessage"
                        },
                        "finish_reason": {
                            "type": "string",
                            "enum": ["stop", "length", "content_filter", "tool_calls"],
                            "description": "Reason for completion finish"
                        },
                        "logprobs": {
                            "type": "object",
                            "nullable": true,
                            "description": "Log probability information"
                        }
                    }
                }
            },
            "usage": {
                "$ref": "#/definitions/Usage"
            },
            "rlm_metadata": {
                "$ref": "#/definitions/RlmMetadata",
                "description": "RLM-specific processing metadata"
            }
        },
        "additionalProperties": false
    })
}

/// Chat message schema
fn chat_message_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Chat Message",
        "description": "A single message in a conversation",
        "required": ["role", "content"],
        "properties": {
            "role": {
                "type": "string",
                "enum": ["system", "user", "assistant", "tool"],
                "description": "The role of the message sender"
            },
            "content": {
                "oneOf": [
                    {
                        "type": "string",
                        "description": "Text content of the message"
                    },
                    {
                        "type": "array",
                        "description": "Array of content parts (for multimodal)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["text", "image_url"]
                                },
                                "text": {
                                    "type": "string"
                                },
                                "image_url": {
                                    "type": "object",
                                    "properties": {
                                        "url": {
                                            "type": "string",
                                            "format": "uri"
                                        },
                                        "detail": {
                                            "type": "string",
                                            "enum": ["auto", "low", "high"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                ]
            },
            "name": {
                "type": "string",
                "description": "Optional name for the message sender",
                "pattern": "^[a-zA-Z0-9_-]+$",
                "maxLength": 64
            },
            "tool_call_id": {
                "type": "string",
                "description": "Tool call identifier for tool messages"
            },
            "tool_calls": {
                "type": "array",
                "description": "Tool calls made by the assistant",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "type": {"type": "string", "enum": ["function"]},
                        "function": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "arguments": {"type": "string"}
                            }
                        }
                    }
                }
            }
        },
        "additionalProperties": false,
        "examples": [
            {
                "role": "user",
                "content": "Hello, how are you?"
            },
            {
                "role": "assistant",
                "content": "I'm doing well, thank you for asking!"
            },
            {
                "role": "system",
                "content": "You are a helpful assistant specialized in data analysis."
            }
        ]
    })
}

/// RLM configuration schema
fn rlm_config_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "RLM Configuration",
        "description": "Configuration options for RLM recursive processing",
        "properties": {
            "max_recursion_depth": {
                "type": "integer",
                "description": "Maximum depth for recursive calls",
                "minimum": 1,
                "maximum": 10,
                "default": 5,
                "examples": [3, 5, 7]
            },
            "chunk_strategy": {
                "type": "string",
                "enum": ["fixed_size", "semantic", "adaptive", "sliding_window"],
                "description": "Strategy for chunking large contexts",
                "default": "adaptive",
                "examples": ["semantic", "adaptive"]
            },
            "chunk_size": {
                "type": "integer",
                "description": "Size of context chunks in tokens",
                "minimum": 100,
                "maximum": 32768,
                "default": 4096
            },
            "chunk_overlap": {
                "type": "integer",
                "description": "Overlap between chunks in tokens",
                "minimum": 0,
                "maximum": 1000,
                "default": 200
            },
            "enable_streaming": {
                "type": "boolean",
                "description": "Enable streaming for recursive operations",
                "default": true
            },
            "aggregation_strategy": {
                "type": "string",
                "enum": ["simple", "weighted", "hierarchical", "consensus"],
                "description": "Strategy for aggregating recursive results",
                "default": "hierarchical"
            },
            "context_window": {
                "type": "integer",
                "description": "Context window size for the underlying model",
                "minimum": 1000,
                "maximum": 2000000,
                "examples": [4096, 8192, 32768, 128000]
            },
            "parallel_processing": {
                "type": "boolean",
                "description": "Enable parallel processing of independent chunks",
                "default": false
            },
            "complexity_optimization": {
                "type": "boolean",
                "description": "Enable complexity-aware optimization",
                "default": true
            },
            "memory_limit_mb": {
                "type": "integer",
                "description": "Memory limit for REPL processing in MB",
                "minimum": 10,
                "maximum": 10000,
                "default": 500
            },
            "timeout_seconds": {
                "type": "integer",
                "description": "Timeout for long-running operations in seconds",
                "minimum": 10,
                "maximum": 3600,
                "default": 300
            }
        },
        "additionalProperties": false,
        "examples": [
            {
                "max_recursion_depth": 5,
                "chunk_strategy": "semantic",
                "enable_streaming": true
            },
            {
                "max_recursion_depth": 3,
                "chunk_strategy": "adaptive",
                "chunk_size": 8192,
                "aggregation_strategy": "weighted",
                "parallel_processing": true
            }
        ]
    })
}

/// Usage statistics schema
fn usage_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Token Usage",
        "description": "Token usage statistics for the request",
        "required": ["prompt_tokens", "completion_tokens", "total_tokens"],
        "properties": {
            "prompt_tokens": {
                "type": "integer",
                "description": "Number of tokens in the prompt",
                "minimum": 0
            },
            "completion_tokens": {
                "type": "integer",
                "description": "Number of tokens in the completion",
                "minimum": 0
            },
            "total_tokens": {
                "type": "integer",
                "description": "Total number of tokens used",
                "minimum": 0
            },
            "prompt_tokens_details": {
                "type": "object",
                "description": "Breakdown of prompt token usage",
                "properties": {
                    "cached_tokens": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "audio_tokens": {
                        "type": "integer",
                        "minimum": 0
                    }
                }
            },
            "completion_tokens_details": {
                "type": "object",
                "description": "Breakdown of completion token usage",
                "properties": {
                    "reasoning_tokens": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "audio_tokens": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "accepted_prediction_tokens": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "rejected_prediction_tokens": {
                        "type": "integer",
                        "minimum": 0
                    }
                }
            }
        },
        "additionalProperties": false
    })
}

/// RLM metadata schema
fn rlm_metadata_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "RLM Metadata",
        "description": "RLM-specific processing metadata and statistics",
        "properties": {
            "session_id": {
                "type": "string",
                "description": "RLM session identifier",
                "format": "uuid"
            },
            "recursive_calls": {
                "type": "integer",
                "description": "Number of recursive sub-calls made",
                "minimum": 0
            },
            "context_chunks": {
                "type": "integer",
                "description": "Number of context chunks processed",
                "minimum": 0
            },
            "processing_time_ms": {
                "type": "integer",
                "description": "Total processing time in milliseconds",
                "minimum": 0
            },
            "memory_usage_mb": {
                "type": "number",
                "description": "Peak memory usage in megabytes",
                "minimum": 0
            },
            "complexity_class": {
                "type": "string",
                "enum": ["O(1)", "O(log n)", "O(n)", "O(n log n)", "O(n^2)", "O(2^n)"],
                "description": "Estimated computational complexity class"
            },
            "chunk_strategy_used": {
                "type": "string",
                "description": "Actual chunking strategy applied"
            },
            "aggregation_strategy_used": {
                "type": "string",
                "description": "Actual aggregation strategy applied"
            },
            "repl_operations": {
                "type": "integer",
                "description": "Number of REPL operations executed",
                "minimum": 0
            },
            "cache_hits": {
                "type": "integer",
                "description": "Number of cache hits during processing",
                "minimum": 0
            },
            "cache_misses": {
                "type": "integer",
                "description": "Number of cache misses during processing",
                "minimum": 0
            },
            "error_count": {
                "type": "integer",
                "description": "Number of recoverable errors encountered",
                "minimum": 0
            },
            "optimization_applied": {
                "type": "array",
                "description": "List of optimizations applied",
                "items": {
                    "type": "string"
                }
            }
        },
        "additionalProperties": false
    })
}

/// Stream response schema
fn stream_response_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Stream Response Chunk",
        "description": "Individual chunk in a streaming response",
        "required": ["id", "object", "created", "model", "choices"],
        "properties": {
            "id": {
                "type": "string",
                "description": "Unique identifier for the completion"
            },
            "object": {
                "type": "string",
                "enum": ["chat.completion.chunk"],
                "description": "Object type identifier"
            },
            "created": {
                "type": "integer",
                "description": "Unix timestamp of chunk creation"
            },
            "model": {
                "type": "string",
                "description": "Model used for the completion"
            },
            "system_fingerprint": {
                "type": "string",
                "description": "System fingerprint"
            },
            "choices": {
                "type": "array",
                "items": {
                    "$ref": "#/definitions/StreamChoice"
                }
            },
            "usage": {
                "$ref": "#/definitions/Usage",
                "description": "Token usage (only in final chunk)"
            }
        }
    })
}

/// Stream choice schema
fn stream_choice_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Stream Choice",
        "description": "Choice in a streaming response chunk",
        "required": ["index", "delta"],
        "properties": {
            "index": {
                "type": "integer",
                "description": "Choice index"
            },
            "delta": {
                "$ref": "#/definitions/StreamDelta"
            },
            "logprobs": {
                "type": "object",
                "nullable": true,
                "description": "Log probability information"
            },
            "finish_reason": {
                "type": "string",
                "enum": ["stop", "length", "content_filter", "tool_calls"],
                "nullable": true,
                "description": "Reason for completion finish"
            }
        }
    })
}

/// Stream delta schema
fn stream_delta_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Stream Delta",
        "description": "Delta content in a streaming chunk",
        "properties": {
            "role": {
                "type": "string",
                "enum": ["assistant"],
                "description": "Role (only in first chunk)"
            },
            "content": {
                "type": "string",
                "nullable": true,
                "description": "Content delta"
            },
            "tool_calls": {
                "type": "array",
                "description": "Tool call deltas",
                "items": {
                    "type": "object"
                }
            }
        }
    })
}

/// Model list schema
fn model_list_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Model List",
        "description": "List of available models",
        "required": ["object", "data"],
        "properties": {
            "object": {
                "type": "string",
                "enum": ["list"]
            },
            "data": {
                "type": "array",
                "items": {
                    "$ref": "#/definitions/Model"
                }
            }
        }
    })
}

/// Model schema
fn model_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Model",
        "description": "Information about a specific model",
        "required": ["id", "object", "created", "owned_by"],
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
                "description": "Unix timestamp of model creation"
            },
            "owned_by": {
                "type": "string",
                "description": "Organization that owns the model"
            },
            "permission": {
                "type": "array",
                "items": {
                    "type": "object"
                }
            },
            "context_length": {
                "type": "integer",
                "description": "Maximum context length in tokens",
                "minimum": 1
            },
            "max_output_tokens": {
                "type": "integer",
                "description": "Maximum output tokens",
                "minimum": 1
            },
            "rlm_capable": {
                "type": "boolean",
                "description": "Whether model supports RLM processing",
                "default": true
            },
            "supported_formats": {
                "type": "array",
                "items": {
                    "type": "string",
                    "enum": ["text", "json", "function_call"]
                }
            }
        }
    })
}

/// Health status schema
fn health_status_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Health Status",
        "description": "Server health status information",
        "required": ["status", "timestamp"],
        "properties": {
            "status": {
                "type": "string",
                "enum": ["healthy", "unhealthy", "degraded"],
                "description": "Overall server health status"
            },
            "timestamp": {
                "type": "string",
                "format": "date-time",
                "description": "Health check timestamp"
            },
            "version": {
                "type": "string",
                "description": "Server version"
            },
            "uptime_seconds": {
                "type": "integer",
                "description": "Server uptime in seconds",
                "minimum": 0
            },
            "components": {
                "type": "object",
                "description": "Component health statuses",
                "additionalProperties": {
                    "$ref": "#/definitions/ComponentHealth"
                }
            }
        }
    })
}

/// Component health schema
fn component_health_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Component Health",
        "description": "Health status of a system component",
        "required": ["status"],
        "properties": {
            "status": {
                "type": "string",
                "enum": ["healthy", "unhealthy", "unknown"],
                "description": "Component health status"
            },
            "last_check": {
                "type": "string",
                "format": "date-time",
                "description": "Last health check timestamp"
            },
            "response_time_ms": {
                "type": "number",
                "description": "Response time in milliseconds",
                "minimum": 0
            },
            "error_message": {
                "type": "string",
                "description": "Error message if unhealthy"
            },
            "details": {
                "type": "object",
                "description": "Additional component-specific details",
                "additionalProperties": true
            }
        }
    })
}

/// Metrics schema
fn metrics_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Metrics",
        "description": "Server performance and operational metrics",
        "properties": {
            "requests_total": {
                "type": "integer",
                "description": "Total number of requests processed",
                "minimum": 0
            },
            "requests_per_second": {
                "type": "number",
                "description": "Current requests per second rate",
                "minimum": 0
            },
            "average_response_time_ms": {
                "type": "number",
                "description": "Average response time in milliseconds",
                "minimum": 0
            },
            "memory_usage_mb": {
                "type": "number",
                "description": "Current memory usage in MB",
                "minimum": 0
            },
            "active_sessions": {
                "type": "integer",
                "description": "Number of active RLM sessions",
                "minimum": 0
            },
            "recursive_calls_total": {
                "type": "integer",
                "description": "Total recursive calls made",
                "minimum": 0
            },
            "tokens_processed_total": {
                "type": "integer",
                "description": "Total tokens processed",
                "minimum": 0
            },
            "errors_total": {
                "type": "integer",
                "description": "Total errors encountered",
                "minimum": 0
            }
        }
    })
}

/// RLM session schema
fn rlm_session_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "RLM Session",
        "description": "Active RLM processing session",
        "required": ["id", "status", "created_at"],
        "properties": {
            "id": {
                "type": "string",
                "format": "uuid",
                "description": "Unique session identifier"
            },
            "status": {
                "type": "string",
                "enum": ["pending", "processing", "completed", "failed", "cancelled"],
                "description": "Current session status"
            },
            "created_at": {
                "type": "string",
                "format": "date-time",
                "description": "Session creation timestamp"
            },
            "updated_at": {
                "type": "string",
                "format": "date-time",
                "description": "Last update timestamp"
            },
            "model": {
                "type": "string",
                "description": "Model being used"
            },
            "context_size_tokens": {
                "type": "integer",
                "description": "Size of input context in tokens",
                "minimum": 0
            },
            "recursive_calls": {
                "type": "integer",
                "description": "Number of recursive calls made",
                "minimum": 0
            },
            "processing_time_ms": {
                "type": "integer",
                "description": "Processing time in milliseconds",
                "minimum": 0
            },
            "memory_usage_mb": {
                "type": "number",
                "description": "Memory usage in MB",
                "minimum": 0
            },
            "progress": {
                "type": "number",
                "description": "Completion progress (0-1)",
                "minimum": 0,
                "maximum": 1
            }
        }
    })
}

/// Session list schema
fn session_list_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Session List",
        "description": "List of RLM sessions",
        "required": ["sessions", "total"],
        "properties": {
            "sessions": {
                "type": "array",
                "items": {
                    "$ref": "#/definitions/RlmSession"
                }
            },
            "total": {
                "type": "integer",
                "description": "Total number of sessions",
                "minimum": 0
            },
            "page": {
                "type": "integer",
                "description": "Current page number",
                "minimum": 1
            },
            "page_size": {
                "type": "integer",
                "description": "Number of sessions per page",
                "minimum": 1
            }
        }
    })
}

/// Recursive call schema
fn recursive_call_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Recursive Call",
        "description": "Information about a recursive sub-call",
        "required": ["id", "depth", "status"],
        "properties": {
            "id": {
                "type": "string",
                "description": "Unique call identifier"
            },
            "parent_id": {
                "type": "string",
                "nullable": true,
                "description": "Parent call identifier"
            },
            "depth": {
                "type": "integer",
                "description": "Recursion depth",
                "minimum": 0
            },
            "status": {
                "type": "string",
                "enum": ["pending", "processing", "completed", "failed"],
                "description": "Call status"
            },
            "prompt": {
                "type": "string",
                "description": "Prompt for this call"
            },
            "response": {
                "type": "string",
                "description": "Response from this call"
            },
            "tokens_used": {
                "type": "integer",
                "description": "Tokens used in this call",
                "minimum": 0
            },
            "duration_ms": {
                "type": "integer",
                "description": "Call duration in milliseconds",
                "minimum": 0
            }
        }
    })
}

/// Error schema
fn error_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Error",
        "description": "API error response",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["message", "type"],
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Human-readable error message"
                    },
                    "type": {
                        "type": "string",
                        "description": "Error type classification",
                        "examples": ["invalid_request_error", "authentication_error", "rate_limit_error", "server_error"]
                    },
                    "code": {
                        "type": "string",
                        "description": "Specific error code for programmatic handling",
                        "examples": ["missing_required_parameter", "invalid_api_key", "rate_limit_exceeded"]
                    },
                    "param": {
                        "type": "string",
                        "nullable": true,
                        "description": "Parameter that caused the error"
                    },
                    "details": {
                        "type": "object",
                        "description": "Additional error details",
                        "additionalProperties": true
                    }
                }
            }
        }
    })
}

/// Validation error schema
fn validation_error_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "title": "Validation Error",
        "description": "Request validation error with field-specific details",
        "required": ["error"],
        "properties": {
            "error": {
                "type": "object",
                "required": ["message", "type", "validation_errors"],
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Overall validation error message"
                    },
                    "type": {
                        "type": "string",
                        "enum": ["validation_error"]
                    },
                    "validation_errors": {
                        "type": "array",
                        "description": "Field-specific validation errors",
                        "items": {
                            "type": "object",
                            "required": ["field", "message"],
                            "properties": {
                                "field": {
                                    "type": "string",
                                    "description": "Field that failed validation"
                                },
                                "message": {
                                    "type": "string",
                                    "description": "Validation error message"
                                },
                                "code": {
                                    "type": "string",
                                    "description": "Validation error code"
                                },
                                "received_value": {
                                    "description": "Value that failed validation"
                                },
                                "expected": {
                                    "type": "string",
                                    "description": "Expected value or format"
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Get validation rules for a specific schema
pub fn get_validation_rules(schema_name: &str) -> Option<Value> {
    match schema_name {
        "ChatCompletionRequest" => Some(json!({
            "required_fields": ["model", "messages"],
            "field_constraints": {
                "model": {
                    "min_length": 1,
                    "max_length": 100,
                    "pattern": "^[a-zA-Z0-9_.-]+$"
                },
                "messages": {
                    "min_items": 1,
                    "max_items": 100,
                    "item_constraints": {
                        "role": {
                            "allowed_values": ["system", "user", "assistant", "tool"]
                        },
                        "content": {
                            "max_length": 1000000
                        }
                    }
                },
                "max_tokens": {
                    "min": 1,
                    "max": 32768
                },
                "temperature": {
                    "min": 0.0,
                    "max": 2.0
                },
                "top_p": {
                    "min": 0.0,
                    "max": 1.0
                }
            }
        })),
        "RlmConfig" => Some(json!({
            "field_constraints": {
                "max_recursion_depth": {
                    "min": 1,
                    "max": 10
                },
                "chunk_strategy": {
                    "allowed_values": ["fixed_size", "semantic", "adaptive", "sliding_window"]
                },
                "chunk_size": {
                    "min": 100,
                    "max": 32768
                },
                "chunk_overlap": {
                    "min": 0,
                    "max": 1000
                },
                "aggregation_strategy": {
                    "allowed_values": ["simple", "weighted", "hierarchical", "consensus"]
                }
            }
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schemas = generate_json_schemas();

        // Verify all expected schemas are present
        let expected_schemas = vec![
            "ChatCompletionRequest",
            "ChatCompletionResponse",
            "ChatMessage",
            "RlmConfig",
            "Usage",
            "RlmMetadata",
            "Error",
            "HealthStatus"
        ];

        for schema_name in expected_schemas {
            assert!(schemas.contains_key(schema_name), "Missing schema: {}", schema_name);
        }
    }

    #[test]
    fn test_chat_completion_request_schema() {
        let schema = chat_completion_request_schema();

        // Verify required fields
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("model")));
        assert!(required.contains(&json!("messages")));

        // Verify properties exist
        assert!(schema["properties"]["model"].is_object());
        assert!(schema["properties"]["messages"].is_object());
        assert!(schema["properties"]["rlm_config"].is_object());
    }

    #[test]
    fn test_rlm_config_schema() {
        let schema = rlm_config_schema();

        // Verify chunk strategy enum
        let chunk_strategy = &schema["properties"]["chunk_strategy"];
        let allowed_values = chunk_strategy["enum"].as_array().unwrap();
        assert!(allowed_values.contains(&json!("semantic")));
        assert!(allowed_values.contains(&json!("adaptive")));
    }

    #[test]
    fn test_validation_rules() {
        let rules = get_validation_rules("ChatCompletionRequest").unwrap();

        // Verify required fields
        let required = rules["required_fields"].as_array().unwrap();
        assert!(required.contains(&json!("model")));
        assert!(required.contains(&json!("messages")));

        // Verify field constraints
        let constraints = &rules["field_constraints"];
        assert!(constraints["max_tokens"]["min"].as_u64().unwrap() == 1);
        assert!(constraints["max_tokens"]["max"].as_u64().unwrap() == 32768);
    }
}