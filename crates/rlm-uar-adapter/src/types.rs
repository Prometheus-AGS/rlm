//! Vendored types from UAR for wire compatibility.
//!
//! These types MUST be kept in sync with `universal-agent-runtime/src/normalized.rs`.
//! We vendor them to avoid path dependencies and allow independently versioned builds,
//! while maintaining JSON-level compatibility via Serde.

use serde::{Deserialize, Serialize};

/// Citation reference - Copied from UAR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Citation {
    pub index: usize,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// Normalized streaming events - Copied from UAR
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum NormalizedEvent {
    // Stream Lifecycle
    #[serde(rename = "stream.start")]
    StreamStart { request_id: String },

    // Message Content
    #[serde(rename = "message.delta")]
    MessageDelta { text: String },

    // Extended Model Capabilities
    #[serde(rename = "thinking.delta")]
    ThinkingDelta { text: String },

    #[serde(rename = "reasoning.delta")]
    ReasoningDelta { text: String },

    #[serde(rename = "citation.added")]
    CitationAdded(Citation),

    #[serde(rename = "memory.update")]
    MemoryUpdate {
        key: String,
        value: String,
        #[serde(default = "default_memory_operation")]
        operation: String,
    },

    // Tool Calls
    #[serde(rename = "tool_call.delta")]
    ToolCallDelta {
        call_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        arguments_delta: Option<String>,
    },

    #[serde(rename = "tool_call.complete")]
    ToolCallComplete {
        call_index: usize,
        id: String,
        name: String,
        arguments_json: String,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        id: String,
        name: String,
        content: String,
        #[serde(default = "default_true")]
        success: bool,
    },

    // Errors and Completion
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    },

    #[serde(rename = "usage")]
    Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    },

    #[serde(rename = "done")]
    Done,
}

fn default_memory_operation() -> String {
    "set".to_string()
}

fn default_true() -> bool {
    true
}
