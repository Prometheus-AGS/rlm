//! RLM (Recursive Language Model) core implementation.
//!
//! This crate provides the core types, executor, and port definitions
//! for the RLM architecture described in the paper:
//! "RLM: A Recursive Language Model for Long Contexts" (arXiv:2410.01855).
//!
//! # Architecture
//!
//! RLM uses a ports-and-adapters (hexagonal) architecture:
//! - **Core**: Execution logic, events, types (this crate)
//! - **Ports**: Traits for REPL, LLM, event sinks
//! - **Adapters**: Concrete implementations (other crates)

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod config;
pub mod error;
pub mod events;
pub mod executor;
pub mod ports;
pub mod types;

pub use config::RlmConfig;
pub use error::{RlmError, RlmResult};
pub use events::RlmEvent;
pub use executor::RlmExecutor;
pub use types::{ExecutionMetadata, ReplResult, RlmRequest, RlmResponse};
