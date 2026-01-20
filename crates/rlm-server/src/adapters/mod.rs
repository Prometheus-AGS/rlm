//! RLM server adapter implementations.
//!
//! This module contains adapters that implement the core RLM ports
//! for specific external services and providers.

pub mod azure_openai;
pub mod openai;
pub mod sse_sink;

pub use azure_openai::{AzureOpenAiConfig, AzureOpenAiConfigBuilder, AzureOpenAiProvider};
pub use openai::{OpenAiConfig, OpenAiConfigBuilder, OpenAiProvider};
pub use sse_sink::{SseEventSink, SseEventSinkBuilder, create_sse_stream};