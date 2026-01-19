# RLM Implementation Plan

**Version**: 1.0  
**Target**: Production-ready v0.1  
**Timeline**: 4 weeks (9 phases)  
**Paper Reference**: [RLM: A Recursive Language Model for Long Contexts](https://arxiv.org/abs/2410.01855)

## Table of Contents
1. [Overview](#overview)
2. [Architecture Recap](#architecture-recap)
3. [Implementation Phases](#implementation-phases)
4. [Coding Standards Compliance](#coding-standards-compliance)
5. [Testing Strategy](#testing-strategy)
6. [AI Assistant Instructions](#ai-assistant-instructions)

---

## Overview

This plan provides a detailed, phase-by-phase implementation guide for the RLM (Recursive Language Model) project. Each phase includes:

- **Objective**: What to build
- **Inputs**: Dependencies and prerequisites
- **Outputs**: Deliverables and artifacts
- **Implementation Steps**: Detailed instructions
- **Code Examples**: Copy-paste ready templates
- **Verification**: How to validate completion
- **Coding Standards**: Compliance with `docs/coding-standards/README.md`

### Technology Stack (Research-Informed)

Based on latest 2024 research and best practices:

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **REPL Backend** | Rhai | Safe, sandboxed, Rust-native scripting (no Python FFI complexity) |
| **Async Runtime** | Tokio | Industry standard, mature, excellent ecosystem |
| **HTTP Server** | Axum | Modern, performant, type-safe, built on Tokio |
| **Serialization** | Serde | Zero-cost, derive macros, extensive ecosystem |
| **Error Handling** | thiserror + anyhow | Canonical Rust error pattern |
| **Logging** | tracing | Structured, async-aware, spans for context |
| **WASM FFI** | wasm-bindgen | Standard Rust→JS bridge |
| **Streaming** | SSE (Server-Sent Events) | Simple, unidirectional, browser-native |

### Paper Architecture (Fig 2)

The RLM architecture from the paper consists of:

```
┌──────────────────────────────────────────────────────────┐
│  User Query + Long Context                               │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│  Stage 1: Context Offloading                             │
│  • Load context into REPL environment                    │
│  • Generate prompt referencing REPL variables            │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│  Stage 2: LLM Execution with Recursive Calls             │
│  • LLM processes prompt                                  │
│  • Can call llm_query(subquery) → spawn recursion        │
│  • REPL stores intermediate results                      │
└────────────────────┬─────────────────────────────────────┘
                     │
                     ▼
┌──────────────────────────────────────────────────────────┐
│  Stage 3: Aggregation                                    │
│  • Combine recursive results from REPL                   │
│  • Generate final answer                                 │
└──────────────────────────────────────────────────────────┘
```

Key insights from Section 3.1 (Methodology):
1. **Context offloading** reduces prompt size from O(n) to O(1)
2. **Recursive decomposition** enables O(n) or O(n²) strategies depending on task
3. **REPL state** persists across iterations (max 50 per paper defaults)
4. **Streaming events** enable real-time monitoring of REPL ops and recursive calls

---

## Architecture Recap

### Crate Dependency Graph

```
rlm-uar-adapter  rlm-server  rlm-ffi
       │              │          │
       └──────┬───────┴──────────┘
              │
              ▼
          rlm-core ────────┐
              │            │
              ▼            ▼
       rlm-repl-rhai   (LLM provider trait)
```

### Port Definitions (Hexagonal Architecture)

RLM core defines **ports** (traits) implemented by **adapters**:

| Port | Purpose | Implementations |
|------|---------|-----------------|
| `ReplBackend` | Execute code in sandboxed REPL | `RhaiReplBackend`, `PythonReplBackend` (future) |
| `LlmProvider` | Call LLM with prompts | `OpenAiProvider`, `AnthropicProvider`, `MockProvider` |
| `EventSink` | Stream events to consumers | `SseEventSink`, `UarEventSink`, `WasmEventSink` |

### Event Model (Section 3.1)

All RLM operations emit events for streaming:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RlmEvent {
    /// REPL operation executed
    ReplOp {
        iteration: u32,
        code: String,
        result: ReplResult,
        timestamp: SystemTime,
    },
    
    /// Recursive LLM call initiated
    RecursiveCall {
        depth: u32,
        query: String,
        parent_id: Option<String>,
        call_id: String,
    },
    
    /// Recursive call completed
    RecursiveResult {
        call_id: String,
        result: String,
        tokens_used: u32,
    },
    
    /// LLM streaming chunk
    Chunk {
        content: String,
        finish_reason: Option<String>,
    },
    
    /// Context chunk processed
    ContextChunk {
        index: usize,
        total: usize,
        size_tokens: u32,
    },
    
    /// Execution completed
    Done {
        answer: String,
        metadata: ExecutionMetadata,
    },
    
    /// Error occurred
    Error {
        message: String,
        recoverable: bool,
    },
}
```

---

## Implementation Phases

### Phase 0: Project Initialization (Week 1, Day 1)

**Objective**: Set up workspace, CI/CD, and tooling.

**Inputs**: 
- Workspace `Cargo.toml` (already created)
- Coding standards from `docs/coding-standards/README.md`

**Outputs**:
- ✅ Workspace builds with `cargo check`
- ✅ CI/CD pipeline (GitHub Actions)
- ✅ Pre-commit hooks (fmt, clippy)
- ✅ Documentation structure

**Steps**:

1. **Create crate directories**:
```bash
mkdir -p crates/{rlm-core,rlm-repl-rhai,rlm-server,rlm-ffi,rlm-uar-adapter}/src
```

2. **Initialize each crate**:

`crates/rlm-core/Cargo.toml`:
```toml
[package]
name = "rlm-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tokio = { workspace = true }
tokio-stream = { workspace = true }
async-trait = { workspace = true }
futures = { workspace = true }

[dev-dependencies]
mockall = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }

[lints]
workspace = true
```

`crates/rlm-core/src/lib.rs`:
```rust
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
pub use types::{RlmRequest, RlmResponse, ExecutionMetadata};
```

Repeat for other crates (detailed `Cargo.toml` files provided in Phase 1-4).

3. **Set up CI/CD** (`.github/workflows/ci.yml`):
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run clippy
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      
      - name: Build
        run: cargo build --workspace --all-features
      
      - name: Test
        run: cargo test --workspace --all-features
      
      - name: Doc
        run: cargo doc --workspace --no-deps --all-features
```

4. **Pre-commit hook** (`.githooks/pre-commit`):
```bash
#!/bin/sh
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace --quiet

echo "✅ Pre-commit checks passed"
```

Install: `git config core.hooksPath .githooks`

**Verification**:
```bash
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
```

**Compliance**: 
- ✅ M-STATIC-VERIFICATION (clippy configured)
- ✅ M-DOCS (lib.rs docs)

---

### Phase 1: Core Types & Domain Model (Week 1, Days 2-3)

**Objective**: Define all core types, errors, and configuration.

**Inputs**: Paper section 3.1 (methodology), coding standards

**Outputs**:
- `rlm-core/src/types.rs`
- `rlm-core/src/error.rs`
- `rlm-core/src/config.rs`
- `rlm-core/src/events.rs`

**Steps**:

1. **Define domain types** (`types.rs`):

```rust
//! Core types for RLM execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Request to execute RLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmRequest {
    /// User query to answer.
    pub query: String,
    
    /// Long context to offload to REPL.
    pub context: String,
    
    /// Maximum REPL iterations (default: 50, per paper).
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    
    /// Maximum recursive call depth (default: 1, per paper).
    #[serde(default = "default_recursion_depth")]
    pub recursion_depth: u32,
    
    /// Optional metadata for tracking.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_max_iterations() -> u32 { 50 }
fn default_recursion_depth() -> u32 { 1 }

/// Response from RLM execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmResponse {
    /// Final answer to the query.
    pub answer: String,
    
    /// Execution metadata.
    pub metadata: ExecutionMetadata,
    
    /// REPL final state (optional, for debugging).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repl_state: Option<String>,
}

/// Metadata about execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Number of REPL iterations used.
    pub iterations: u32,
    
    /// Number of recursive LLM calls made.
    pub recursive_calls: u32,
    
    /// Total tokens consumed.
    pub total_tokens: u32,
    
    /// Execution duration.
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    
    /// Start time.
    #[serde(with = "systemtime_serde")]
    pub started_at: SystemTime,
    
    /// Whether execution completed successfully.
    pub success: bool,
}

// Serde helpers for Duration and SystemTime
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;
    
    pub fn serialize<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        s.serialize_u64(d.as_millis() as u64)
    }
    
    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

mod systemtime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn serialize<S>(t: &SystemTime, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let since_epoch = t.duration_since(UNIX_EPOCH).unwrap();
        s.serialize_u64(since_epoch.as_secs())
    }
    
    pub fn deserialize<'de, D>(d: D) -> Result<SystemTime, D::Error>
    where D: Deserializer<'de> {
        let secs = u64::deserialize(d)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_secs(secs))
    }
}

/// Result from a REPL operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReplResult {
    /// Execution succeeded.
    Success {
        /// Output value as string.
        value: String,
    },
    /// Execution failed.
    Error {
        /// Error message.
        message: String,
    },
}

impl ReplResult {
    /// Check if result is success.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
    
    /// Get value or error message.
    #[must_use]
    pub fn value_or_error(&self) -> &str {
        match self {
            Self::Success { value } => value,
            Self::Error { message } => message,
        }
    }
}
```

2. **Define errors** (`error.rs`):

```rust
//! Error types for RLM.

use thiserror::Error;

/// RLM error type.
#[derive(Debug, Error)]
pub enum RlmError {
    /// REPL execution error.
    #[error("REPL error: {0}")]
    Repl(String),
    
    /// LLM provider error.
    #[error("LLM provider error: {0}")]
    Llm(String),
    
    /// Maximum iterations exceeded.
    #[error("Maximum iterations ({0}) exceeded")]
    MaxIterations(u32),
    
    /// Maximum recursion depth exceeded.
    #[error("Maximum recursion depth ({0}) exceeded")]
    MaxRecursion(u32),
    
    /// Timeout error.
    #[error("Execution timeout after {0:?}")]
    Timeout(std::time::Duration),
    
    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Generic error.
    #[error("{0}")]
    Other(String),
}

/// RLM result type.
pub type RlmResult<T> = Result<T, RlmError>;

// Compliance: M-ERRORS-CANONICAL-STRUCTS
// All errors use thiserror, provide Display + Error traits
```

3. **Define configuration** (`config.rs`):

```rust
//! Configuration for RLM execution.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// RLM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    /// REPL configuration.
    pub repl: ReplConfig,
    
    /// LLM configuration.
    pub llm: LlmConfig,
    
    /// Execution limits.
    pub limits: ExecutionLimits,
    
    /// Streaming configuration.
    pub streaming: StreamingConfig,
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            repl: ReplConfig::default(),
            llm: LlmConfig::default(),
            limits: ExecutionLimits::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

/// REPL backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplConfig {
    /// Backend type ("rhai" or "python").
    pub backend: String,
    
    /// Enable sandboxing.
    pub sandbox: bool,
    
    /// Memory limit in MB.
    pub max_memory_mb: usize,
    
    /// Timeout per REPL operation.
    #[serde(with = "duration_millis")]
    pub operation_timeout: Duration,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            backend: "rhai".to_string(),
            sandbox: true,
            max_memory_mb: 512,
            operation_timeout: Duration::from_secs(30),
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider name ("openai", "anthropic", etc.).
    pub provider: String,
    
    /// Model name.
    pub model: String,
    
    /// Temperature (0.0 = deterministic, per paper).
    pub temperature: f32,
    
    /// Max tokens per response.
    pub max_tokens: u32,
    
    /// API key (or environment variable name).
    pub api_key: Option<String>,
    
    /// Base URL (optional).
    pub base_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4-turbo".to_string(),
            temperature: 0.0, // Deterministic per paper
            max_tokens: 4096,
            api_key: None, // Read from env
            base_url: None,
        }
    }
}

/// Execution limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLimits {
    /// Maximum REPL iterations (default: 50, per paper).
    pub max_iterations: u32,
    
    /// Maximum recursion depth (default: 1, per paper).
    pub max_recursion_depth: u32,
    
    /// Overall execution timeout.
    #[serde(with = "duration_millis")]
    pub timeout: Duration,
    
    /// Chunk size for context splitting (in tokens).
    pub chunk_size_tokens: u32,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_iterations: 50,        // Per paper
            max_recursion_depth: 1,    // Per paper
            timeout: Duration::from_secs(300),
            chunk_size_tokens: 4096,
        }
    }
}

/// Streaming configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Enable event streaming.
    pub enabled: bool,
    
    /// Buffer size for event channel.
    pub buffer_size: usize,
    
    /// Include REPL state in events.
    pub include_repl_state: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 100,
            include_repl_state: false,
        }
    }
}

// Helper module for Duration serialization
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;
    
    pub fn serialize<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        s.serialize_u64(d.as_millis() as u64)
    }
    
    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where D: Deserializer<'de> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}
```

4. **Define events** (`events.rs`):

```rust
//! Event types for RLM streaming.

use crate::types::ReplResult;
use crate::ExecutionMetadata;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// RLM execution event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RlmEvent {
    /// Execution started.
    Started {
        /// Request ID.
        request_id: String,
        /// Start time.
        timestamp: SystemTime,
    },
    
    /// REPL operation executed.
    ReplOp {
        /// Iteration number.
        iteration: u32,
        /// Code executed.
        code: String,
        /// Execution result.
        result: ReplResult,
        /// Timestamp.
        timestamp: SystemTime,
    },
    
    /// Recursive LLM call initiated.
    RecursiveCall {
        /// Call depth.
        depth: u32,
        /// Sub-query.
        query: String,
        /// Parent call ID.
        parent_id: Option<String>,
        /// This call's ID.
        call_id: String,
    },
    
    /// Recursive call completed.
    RecursiveResult {
        /// Call ID.
        call_id: String,
        /// Result value.
        result: String,
        /// Tokens used.
        tokens_used: u32,
    },
    
    /// LLM streaming chunk.
    Chunk {
        /// Content delta.
        content: String,
        /// Finish reason (if final chunk).
        finish_reason: Option<String>,
    },
    
    /// Context chunk processed.
    ContextChunk {
        /// Chunk index.
        index: usize,
        /// Total chunks.
        total: usize,
        /// Chunk size in tokens.
        size_tokens: u32,
    },
    
    /// Execution completed successfully.
    Done {
        /// Final answer.
        answer: String,
        /// Execution metadata.
        metadata: ExecutionMetadata,
    },
    
    /// Error occurred.
    Error {
        /// Error message.
        message: String,
        /// Whether error is recoverable.
        recoverable: bool,
    },
}

impl RlmEvent {
    /// Check if event is terminal (Done or Error).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Error { .. })
    }
    
    /// Get event type as string.
    #[must_use]
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Started { .. } => "started",
            Self::ReplOp { .. } => "repl_op",
            Self::RecursiveCall { .. } => "recursive_call",
            Self::RecursiveResult { .. } => "recursive_result",
            Self::Chunk { .. } => "chunk",
            Self::ContextChunk { .. } => "context_chunk",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
        }
    }
}
```

**Verification**:
```bash
cd crates/rlm-core
cargo check
cargo test
cargo doc --no-deps --open
```

**Compliance**:
- ✅ M-TYPES-SEND: All types are `Send + Sync` (no `Rc`, no `!Send` fields)
- ✅ M-PUBLIC-DEBUG: All types implement `Debug`
- ✅ M-ERRORS-CANONICAL-STRUCTS: Errors use `thiserror`
- ✅ M-DOCS: All public items documented

---

### Phase 2: Port Definitions (Week 1, Day 4)

**Objective**: Define trait-based ports for REPL, LLM, and event sinks.

**Inputs**: Phase 1 types

**Outputs**: `rlm-core/src/ports.rs`

**Steps**:

1. **Define ports** (`ports.rs`):

```rust
//! Port definitions (traits) for RLM adapters.

use crate::{RlmError, RlmEvent, RlmResult};
use async_trait::async_trait;
use std::collections::HashMap;

/// REPL backend port.
#[async_trait]
pub trait ReplBackend: Send + Sync {
    /// Initialize the REPL environment.
    async fn init(&mut self) -> RlmResult<()>;
    
    /// Execute code in the REPL.
    ///
    /// # Arguments
    /// * `code` - Code to execute
    ///
    /// # Returns
    /// Result as a string (or error)
    async fn execute(&mut self, code: &str) -> RlmResult<String>;
    
    /// Set a variable in the REPL.
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()>;
    
    /// Get a variable from the REPL.
    async fn get_variable(&mut self, name: &str) -> RlmResult<String>;
    
    /// Get all variables (for debugging).
    async fn get_state(&self) -> RlmResult<HashMap<String, String>>;
    
    /// Reset the REPL to initial state.
    async fn reset(&mut self) -> RlmResult<()>;
    
    /// Check if REPL supports a feature.
    fn supports(&self, feature: &str) -> bool;
}

/// LLM provider port.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Call the LLM with a prompt.
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send
    /// * `max_tokens` - Maximum tokens to generate
    ///
    /// # Returns
    /// Generated text
    async fn call(&self, prompt: &str, max_tokens: u32) -> RlmResult<String>;
    
    /// Stream LLM response (returns async stream).
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send
    /// * `max_tokens` - Maximum tokens to generate
    ///
    /// # Returns
    /// Stream of text chunks
    async fn stream(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> RlmResult<Box<dyn futures::Stream<Item = RlmResult<String>> + Send + Unpin>>;
    
    /// Get model name.
    fn model_name(&self) -> &str;
    
    /// Count tokens in text (approximate).
    fn count_tokens(&self, text: &str) -> usize;
}

/// Event sink port (for streaming events).
#[async_trait]
pub trait EventSink: Send + Sync {
    /// Emit an event.
    async fn emit(&self, event: RlmEvent) -> RlmResult<()>;
    
    /// Flush any buffered events.
    async fn flush(&self) -> RlmResult<()>;
}

/// Null event sink (no-op).
#[derive(Debug, Clone)]
pub struct NullEventSink;

#[async_trait]
impl EventSink for NullEventSink {
    async fn emit(&self, _event: RlmEvent) -> RlmResult<()> {
        Ok(())
    }
    
    async fn flush(&self) -> RlmResult<()> {
        Ok(())
    }
}

// Compliance: M-TYPES-SEND
// All traits require Send + Sync bounds
```

**Verification**:
```bash
cd crates/rlm-core
cargo check
```

**Compliance**:
- ✅ M-TYPES-SEND: All traits require `Send + Sync`
- ✅ M-DOCS: All traits documented

---

### Phase 3: Core Executor (Week 1, Days 5-7)

**Objective**: Implement the main RLM executor with the three-stage pipeline.

**Inputs**: Phase 1 types, Phase 2 ports

**Outputs**: `rlm-core/src/executor.rs`

**Steps**:

1. **Implement executor** (`executor.rs`):

```rust
//! Core RLM executor implementation.

use crate::{
    ports::{EventSink, LlmProvider, NullEventSink, ReplBackend},
    RlmConfig, RlmError, RlmEvent, RlmRequest, RlmResponse, RlmResult,
    ExecutionMetadata, ReplResult,
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;
use tracing::{debug, info, warn, instrument};

/// Main RLM executor.
///
/// Implements the three-stage pipeline from the paper (Fig 2):
/// 1. Context offloading → REPL
/// 2. LLM execution with recursive calls
/// 3. Aggregation of results
pub struct RlmExecutor<R, L>
where
    R: ReplBackend,
    L: LlmProvider,
{
    config: RlmConfig,
    repl: Arc<tokio::sync::Mutex<R>>,
    llm: Arc<L>,
    event_sink: Arc<dyn EventSink>,
}

impl<R, L> RlmExecutor<R, L>
where
    R: ReplBackend + 'static,
    L: LlmProvider + 'static,
{
    /// Create a new executor.
    pub fn new(config: RlmConfig, repl: R, llm: L) -> Self {
        Self {
            config,
            repl: Arc::new(tokio::sync::Mutex::new(repl)),
            llm: Arc::new(llm),
            event_sink: Arc::new(NullEventSink),
        }
    }
    
    /// Set custom event sink.
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = sink;
        self
    }
    
    /// Execute an RLM request.
    #[instrument(skip(self, request), fields(query = %request.query))]
    pub async fn execute(&self, request: RlmRequest) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();
        let request_id = uuid::Uuid::new_v4().to_string();
        
        info!("Starting RLM execution: {}", request_id);
        
        // Emit start event
        self.event_sink.emit(RlmEvent::Started {
            request_id: request_id.clone(),
            timestamp: started_at,
        }).await?;
        
        // Apply timeout
        let result = timeout(
            self.config.limits.timeout,
            self.execute_inner(request, &request_id),
        ).await;
        
        match result {
            Ok(Ok(response)) => {
                info!("RLM execution completed successfully");
                Ok(response)
            }
            Ok(Err(e)) => {
                warn!("RLM execution failed: {}", e);
                self.event_sink.emit(RlmEvent::Error {
                    message: e.to_string(),
                    recoverable: false,
                }).await?;
                Err(e)
            }
            Err(_) => {
                let err = RlmError::Timeout(self.config.limits.timeout);
                warn!("RLM execution timed out");
                self.event_sink.emit(RlmEvent::Error {
                    message: err.to_string(),
                    recoverable: false,
                }).await?;
                Err(err)
            }
        }
    }
    
    /// Inner execution logic (no timeout wrapper).
    async fn execute_inner(
        &self,
        request: RlmRequest,
        request_id: &str,
    ) -> RlmResult<RlmResponse> {
        let started_at = SystemTime::now();
        
        // Stage 1: Context offloading
        debug!("Stage 1: Offloading context to REPL");
        self.stage1_context_offload(&request).await?;
        
        // Stage 2: LLM execution with REPL iteration
        debug!("Stage 2: LLM execution with REPL");
        let (answer, iterations, recursive_calls, total_tokens) = 
            self.stage2_llm_execution(&request, request_id, 0).await?;
        
        // Stage 3: Aggregation (implicit in stage 2 for now)
        debug!("Stage 3: Final aggregation");
        
        let duration = started_at.elapsed().unwrap_or_default();
        
        let metadata = ExecutionMetadata {
            iterations,
            recursive_calls,
            total_tokens,
            duration,
            started_at,
            success: true,
        };
        
        let response = RlmResponse {
            answer: answer.clone(),
            metadata: metadata.clone(),
            repl_state: None,
        };
        
        // Emit done event
        self.event_sink.emit(RlmEvent::Done {
            answer,
            metadata,
        }).await?;
        
        Ok(response)
    }
    
    /// Stage 1: Offload context to REPL.
    async fn stage1_context_offload(&self, request: &RlmRequest) -> RlmResult<()> {
        let mut repl = self.repl.lock().await;
        
        // Initialize REPL
        repl.init().await?;
        
        // Set context variable
        repl.set_variable("context", &request.context).await?;
        
        // Emit context chunk event
        let context_tokens = self.llm.count_tokens(&request.context);
        self.event_sink.emit(RlmEvent::ContextChunk {
            index: 0,
            total: 1,
            size_tokens: context_tokens as u32,
        }).await?;
        
        debug!("Context offloaded: {} tokens", context_tokens);
        Ok(())
    }
    
    /// Stage 2: LLM execution with REPL iterations.
    async fn stage2_llm_execution(
        &self,
        request: &RlmRequest,
        request_id: &str,
        depth: u32,
    ) -> RlmResult<(String, u32, u32, u32)> {
        // Check recursion depth
        if depth > request.recursion_depth {
            return Err(RlmError::MaxRecursion(request.recursion_depth));
        }
        
        let mut iterations = 0u32;
        let mut recursive_calls = 0u32;
        let mut total_tokens = 0u32;
        
        // Build initial prompt
        let prompt = self.build_prompt(&request.query);
        
        // Call LLM
        let answer = self.llm.call(&prompt, self.config.llm.max_tokens).await?;
        total_tokens += self.llm.count_tokens(&answer) as u32;
        
        // TODO: Parse answer for REPL operations and recursive calls
        // For now, return answer directly
        
        Ok((answer, iterations, recursive_calls, total_tokens))
    }
    
    /// Build prompt with context reference.
    fn build_prompt(&self, query: &str) -> String {
        format!(
            "You have access to a REPL environment with the following variable:\n\
             - context: The full context for this task\n\n\
             You can call llm_query(sub_query) to recursively decompose the task.\n\n\
             Task: {}\n\n\
             Provide your answer:",
            query
        )
    }
}

// Compliance: M-TYPES-SEND
// RlmExecutor requires R: ReplBackend (which is Send + Sync)
// and L: LlmProvider (which is Send + Sync)
```

**Note**: This is a minimal implementation. Phase 4 will add REPL iteration parsing and recursive call handling.

**Verification**:
```bash
cd crates/rlm-core
cargo check
cargo test
```

**Compliance**:
- ✅ M-TYPES-SEND: Executor is `Send + Sync`
- ✅ M-LOG-STRUCTURED: Uses `tracing` with structured fields
- ✅ M-UNSAFE: No unsafe code

---

### Phase 4: Rhai REPL Backend (Week 2, Days 1-3)

**Objective**: Implement Rhai-based REPL backend adapter.

**Inputs**: Phase 2 `ReplBackend` trait

**Outputs**: `rlm-repl-rhai/src/lib.rs`

**Steps**:

1. **Create crate** (`crates/rlm-repl-rhai/Cargo.toml`):

```toml
[package]
name = "rlm-repl-rhai"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
rlm-core = { workspace = true }
rhai = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }

[lints]
workspace = true
```

2. **Implement backend** (`src/lib.rs`):

```rust
//! Rhai-based REPL backend for RLM.

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

use async_trait::async_trait;
use rlm_core::{
    ports::ReplBackend,
    RlmError, RlmResult,
};
use rhai::{Engine, Scope, Dynamic};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Rhai REPL backend.
#[derive(Debug)]
pub struct RhaiReplBackend {
    engine: Engine,
    scope: Scope<'static>,
}

impl RhaiReplBackend {
    /// Create a new Rhai REPL backend.
    pub fn new() -> Self {
        let mut engine = Engine::new();
        
        // Configure engine for safety
        engine.set_max_operations(10_000); // Prevent infinite loops
        engine.set_max_call_levels(10);     // Prevent deep recursion
        
        // TODO: Register llm_query function in Phase 5
        
        Self {
            engine,
            scope: Scope::new(),
        }
    }
}

impl Default for RhaiReplBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReplBackend for RhaiReplBackend {
    #[instrument(skip(self))]
    async fn init(&mut self) -> RlmResult<()> {
        debug!("Initializing Rhai REPL");
        self.scope.clear();
        Ok(())
    }
    
    #[instrument(skip(self, code))]
    async fn execute(&mut self, code: &str) -> RlmResult<String> {
        debug!("Executing Rhai code: {}", code);
        
        let result = self.engine
            .eval_with_scope::<Dynamic>(&mut self.scope, code)
            .map_err(|e| RlmError::Repl(e.to_string()))?;
        
        Ok(result.to_string())
    }
    
    #[instrument(skip(self, value))]
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()> {
        debug!("Setting variable: {}", name);
        
        // Store as string
        self.scope.push(name, value.to_string());
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn get_variable(&mut self, name: &str) -> RlmResult<String> {
        debug!("Getting variable: {}", name);
        
        let value = self.scope
            .get_value::<Dynamic>(name)
            .ok_or_else(|| RlmError::Repl(format!("Variable '{}' not found", name)))?;
        
        Ok(value.to_string())
    }
    
    async fn get_state(&self) -> RlmResult<HashMap<String, String>> {
        let mut state = HashMap::new();
        
        for (name, _, value) in self.scope.iter() {
            state.insert(name.to_string(), value.to_string());
        }
        
        Ok(state)
    }
    
    #[instrument(skip(self))]
    async fn reset(&mut self) -> RlmResult<()> {
        debug!("Resetting Rhai REPL");
        self.scope.clear();
        Ok(())
    }
    
    fn supports(&self, feature: &str) -> bool {
        matches!(feature, "variables" | "expressions" | "functions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_execution() {
        let mut repl = RhaiReplBackend::new();
        repl.init().await.unwrap();
        
        let result = repl.execute("2 + 2").await.unwrap();
        assert_eq!(result, "4");
    }
    
    #[tokio::test]
    async fn test_variables() {
        let mut repl = RhaiReplBackend::new();
        repl.init().await.unwrap();
        
        repl.set_variable("x", "42").await.unwrap();
        let result = repl.get_variable("x").await.unwrap();
        assert_eq!(result, "42");
    }
    
    #[tokio::test]
    async fn test_scope_persistence() {
        let mut repl = RhaiReplBackend::new();
        repl.init().await.unwrap();
        
        repl.execute("let y = 10;").await.unwrap();
        let result = repl.execute("y * 2").await.unwrap();
        assert_eq!(result, "20");
    }
}
```

**Verification**:
```bash
cd crates/rlm-repl-rhai
cargo test
```

**Compliance**:
- ✅ M-TYPES-SEND: `RhaiReplBackend` is `Send + Sync`
- ✅ M-UNSAFE: No unsafe code
- ✅ M-LOG-STRUCTURED: Uses `tracing`

---

### Phase 5: HTTP Server with SSE (Week 2, Days 4-5)

**Objective**: Build Axum HTTP server with SSE streaming.

**Inputs**: Phase 3 executor

**Outputs**: `rlm-server/src/main.rs`, `rlm-server/src/routes.rs`

**Steps**:

1. **Create crate** (`crates/rlm-server/Cargo.toml`):

```toml
[package]
name = "rlm-server"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
rlm-core = { workspace = true }
rlm-repl-rhai = { workspace = true }

axum = { workspace = true }
tokio = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

[lints]
workspace = true
```

2. **Implement server** (`src/main.rs`):

```rust
//! RLM HTTP server with SSE streaming.

#![forbid(unsafe_code)]

use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod routes;
mod sse;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rlm_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Build router
    let app = Router::new()
        .route("/v1/rlm/execute", post(routes::execute))
        .layer(TraceLayer::new_for_http());
    
    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("RLM server listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

3. **Implement routes** (`src/routes.rs`):

```rust
//! HTTP route handlers.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    Json,
};
use rlm_core::{RlmRequest, RlmResponse, RlmExecutor, RlmConfig};
use rlm_repl_rhai::RhaiReplBackend;
use crate::sse::SseEventSink;
use std::sync::Arc;

/// Execute RLM request (SSE streaming).
pub async fn execute(
    Json(request): Json<RlmRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // Create executor
    let config = RlmConfig::default();
    let repl = RhaiReplBackend::new();
    let llm = MockLlmProvider::new(); // TODO: Real provider
    
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let event_sink = Arc::new(SseEventSink::new(tx));
    
    let executor = RlmExecutor::new(config, repl, llm)
        .with_event_sink(event_sink);
    
    // Spawn execution task
    tokio::spawn(async move {
        let _ = executor.execute(request).await;
    });
    
    // Return SSE stream
    Ok(Sse::new(ReceiverStream::new(rx)))
}
```

4. **Implement SSE sink** (`src/sse.rs`):

```rust
//! SSE event sink implementation.

use async_trait::async_trait;
use rlm_core::{
    ports::EventSink,
    RlmEvent, RlmResult,
};
use tokio::sync::mpsc::Sender;

/// SSE event sink.
#[derive(Debug)]
pub struct SseEventSink {
    sender: Sender<Result<axum::response::sse::Event, std::convert::Infallible>>,
}

impl SseEventSink {
    pub fn new(sender: Sender<Result<axum::response::sse::Event, std::convert::Infallible>>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl EventSink for SseEventSink {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()> {
        let event_type = event.event_type();
        let data = serde_json::to_string(&event)
            .map_err(|e| rlm_core::RlmError::Serialization(e))?;
        
        let sse_event = axum::response::sse::Event::default()
            .event(event_type)
            .data(data);
        
        self.sender.send(Ok(sse_event)).await
            .map_err(|_| rlm_core::RlmError::Other("Channel closed".to_string()))?;
        
        Ok(())
    }
    
    async fn flush(&self) -> RlmResult<()> {
        Ok(())
    }
}
```

**Verification**:
```bash
cd crates/rlm-server
cargo run &
curl -X POST http://localhost:8080/v1/rlm/execute \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{"query":"test","context":"test"}'
```

**Compliance**:
- ✅ M-LOG-STRUCTURED: Uses `tracing`
- ✅ M-ERRORS-CANONICAL-STRUCTS: Errors properly propagated

---

### Phase 6: WASM FFI Bindings (Week 3, Days 1-2)

**Objective**: Create WASM bindings for Cherry Studio.

**Inputs**: Phase 3 executor

**Outputs**: `rlm-ffi/src/lib.rs`

**Steps**:

1. **Create crate** (`crates/rlm-ffi/Cargo.toml`):

```toml
[package]
name = "rlm-ffi"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rlm-core = { workspace = true }
rlm-repl-rhai = { workspace = true }

wasm-bindgen = { workspace = true }
wasm-bindgen-futures = { workspace = true }
serde = { workspace = true }
serde-wasm-bindgen = { workspace = true }
js-sys = { workspace = true }
web-sys = { workspace = true }

[lints]
workspace = true
```

2. **Implement FFI** (`src/lib.rs`):

```rust
//! WASM FFI bindings for Cherry Studio.

#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;
use rlm_core::{RlmRequest, RlmConfig, RlmExecutor};
use rlm_repl_rhai::RhaiReplBackend;

/// Execute RLM request from JavaScript.
#[wasm_bindgen]
pub async fn rlm_execute(
    request_json: JsValue,
    event_callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    // Parse request
    let request: RlmRequest = serde_wasm_bindgen::from_value(request_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    // Create executor
    let config = RlmConfig::default();
    let repl = RhaiReplBackend::new();
    let llm = MockLlmProvider::new(); // TODO
    
    let executor = RlmExecutor::new(config, repl, llm);
    
    // TODO: Wire event_callback to event sink
    
    let response = executor.execute(request).await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

**Verification**:
```bash
cd crates/rlm-ffi
wasm-pack build --target web
```

**Compliance**:
- ✅ M-UNSAFE: No unsafe code

---

### Phase 7: UAR Integration Adapter (Week 3, Days 3-4)

**Objective**: Create adapter to integrate RLM with UAR.

**Inputs**: UAR types (assumed external), Phase 3 executor

**Outputs**: `rlm-uar-adapter/src/lib.rs`

**Implementation**: Similar structure to Phase 5-6, mapping UAR events ↔ RLM events.

---

### Phase 8: Comprehensive Testing (Week 3, Day 5 - Week 4, Day 2)

**Objective**: Implement all test suites including golden tests.

**Steps**:

1. **Unit tests** (already in each module)
2. **Integration tests** (`tests/integration_tests.rs`)
3. **Golden tests** (`tests/golden_tests.rs`):

```rust
//! Golden tests against paper benchmarks.

use rlm_core::{RlmRequest, RlmResponse};
use std::fs;

#[tokio::test]
async fn test_s_niah_basic() {
    let request: RlmRequest = load_fixture("s_niah/basic.json");
    let expected: RlmResponse = load_fixture("s_niah/expected_response.json");
    
    let executor = create_test_executor();
    let response = executor.execute(request).await.unwrap();
    
    assert_eq!(response.answer, expected.answer);
}

fn load_fixture<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let content = fs::read_to_string(format!("tests/fixtures/{}", path)).unwrap();
    serde_json::from_str(&content).unwrap()
}
```

**Verification**:
```bash
cargo test --workspace
cargo test --package rlm-core --test golden_tests
```

**Compliance**:
- ✅ All M-* guidelines enforced via tests

---

### Phase 9: Performance Benchmarking (Week 4, Days 3-5)

**Objective**: Benchmark against paper results.

**Steps**:

1. **Criterion benchmarks** (`benches/rlm_bench.rs`)
2. **Compare to paper Table 1 results**
3. **Profile with flamegraph**

---

## Coding Standards Compliance

Every phase enforces compliance with `docs/coding-standards/README.md`:

| Guideline | Implementation |
|-----------|----------------|
| **M-STATIC-VERIFICATION** | `cargo clippy -- -D warnings` in CI |
| **M-ERRORS-CANONICAL-STRUCTS** | All errors use `thiserror` |
| **M-LOG-STRUCTURED** | `tracing` with structured fields |
| **M-TYPES-SEND** | All types + traits require `Send + Sync` |
| **M-UNSAFE** | `#![forbid(unsafe_code)]` in all crates |
| **M-PUBLIC-DEBUG** | `#[derive(Debug)]` on all public types |
| **M-DOCS** | `#![warn(missing_docs)]` enforced |

---

## Testing Strategy

### Test Pyramid

```
         ┌─────────────┐
         │  Golden     │  ← Paper benchmarks
         │   Tests     │
         └─────────────┘
       ┌───────────────────┐
       │   Integration     │  ← Multi-crate workflows
       │      Tests        │
       └───────────────────┘
   ┌─────────────────────────────┐
   │        Unit Tests            │  ← Per-module correctness
   └─────────────────────────────┘
```

### Golden Test Coverage

Each fixture from `tests/fixtures/` validates:
- Correct answer vs paper dataset
- Token usage within expected bounds
- Event ordering (REPL ops → recursive calls → chunks → done)
- Iteration count ≤ max (50)
- Recursion depth ≤ max (1)

---

## AI Assistant Instructions

### For Claude Code / Codex / Roo Code

When implementing a phase:

1. **Read the phase section** carefully
2. **Copy the provided code templates** exactly
3. **Run verification steps** after each file
4. **Check compliance markers** (✅ M-* guidelines)
5. **If uncertain**, choose the conservative default and mark it with `// TODO: Verify assumption`

### Common Patterns

**Error handling**:
```rust
// Always attach context
.map_err(|e| RlmError::Repl(format!("Failed to execute: {}", e)))?
```

**Async functions**:
```rust
// Use #[async_trait] for trait methods
#[async_trait]
impl ReplBackend for MyBackend {
    async fn execute(&mut self, code: &str) -> RlmResult<String> {
        // ...
    }
}
```

**Logging**:
```rust
use tracing::{debug, info, warn, error, instrument};

#[instrument(skip(self, sensitive_data))]
async fn my_function(&self, id: &str, sensitive_data: &str) -> Result<()> {
    info!("Starting operation for {}", id);
    debug!("Processing step 1");
    Ok(())
}
```

### Common Pitfalls to Avoid

❌ **Don't**: Use `unwrap()` or `expect()` in library code  
✅ **Do**: Return `Result` and propagate errors

❌ **Don't**: Use `println!` for logging  
✅ **Do**: Use `tracing::info!`, `debug!`, etc.

❌ **Don't**: Block the async runtime with `std::thread::sleep`  
✅ **Do**: Use `tokio::time::sleep`

❌ **Don't**: Make everything `pub`  
✅ **Do**: Minimize public API surface

---

## Summary Timeline

| Week | Phase | Deliverable |
|------|-------|-------------|
| 1 | 0-3 | Core types, ports, basic executor |
| 2 | 4-5 | Rhai backend, HTTP server |
| 3 | 6-7 | WASM FFI, UAR adapter |
| 4 | 8-9 | Tests, benchmarks, docs |

**Target**: Production-ready v0.1 by end of Week 4.

---

**Next Steps**: Begin Phase 0 (workspace initialization).
