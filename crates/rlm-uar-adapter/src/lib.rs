//! UAR integration adapter for RLM.
//!
//! This crate provides the adapter layer to integrate RLM as a backend
//! within the Universal Agent Runtime (UAR) ecosystem.
//! It handles the translation of RLM-specific events into UAR's
//! [`NormalizedEvent`] model.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod adapter;
pub mod types;

pub use adapter::RlmUarAdapter;
pub use types::{Citation, NormalizedEvent};
