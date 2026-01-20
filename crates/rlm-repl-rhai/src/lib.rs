//! Rhai-based REPL backend for RLM.
//!
//! This crate provides a safe, sandboxed REPL backend using the Rhai scripting language.
//! Rhai offers memory safety without FFI complexity while maintaining scripting flexibility
//! suitable for the RLM paper's context offloading approach.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod backend;
pub mod engine;
pub mod functions;

pub use backend::{RhaiReplBackend, RhaiReplConfig};
pub use engine::{RhaiEngineBuilder, RhaiEngineConfig, create_secure_engine};
pub use functions::{LlmQueryProvider, MockLlmProvider, RlmFunctionRegistry, create_rlm_engine};
