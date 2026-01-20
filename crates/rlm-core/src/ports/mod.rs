//! Port definitions for the hexagonal architecture.
//!
//! This module contains trait definitions that define the boundaries
//! between the core RLM logic and external systems.
//!
//! ## Hexagonal Architecture Pattern
//!
//! The RLM system follows the hexagonal architecture (ports and adapters) pattern:
//!
//! - **Ports** (this module): Abstract interfaces defining what the core needs
//! - **Adapters** (in separate crates): Concrete implementations of the ports
//! - **Core** (`rlm-core`): Business logic that depends only on ports
//!
//! This design enables:
//! - Easy testing via mock implementations
//! - Multiple implementation strategies (Rhai vs Python REPL, different LLM providers)
//! - Clean separation of concerns
//! - Framework-agnostic core logic

pub mod repl_backend;
pub mod llm_provider;
pub mod event_sink;

pub use repl_backend::{ReplBackend, ReplMetadata};
pub use llm_provider::{LlmProvider, ModelInfo, ProviderMetadata};
pub use event_sink::{EventSink, SinkStats, SinkMetadata, EventFilter, EventSeverity, NullEventSink};