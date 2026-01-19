# RLM Implementation Summary

Quick reference guide for AI coding assistants.

## 🎯 Project Goal

Build a production Rust implementation of RLM (Recursive Language Model) from the MIT paper, integrated with UAR and Cherry Studio.

## 📚 Technology Stack

- **Language**: Rust 1.75+ (edition 2021)
- **Async**: Tokio
- **REPL**: Rhai (safe, sandboxed)
- **HTTP**: Axum + SSE
- **WASM**: wasm-bindgen
- **Errors**: thiserror (libraries) + anyhow (binaries)
- **Logging**: tracing
- **Testing**: mockall, wiremock, proptest

## 🏗️ Crate Structure

```
rlm/
├── crates/
│   ├── rlm-core/           # Core types, executor, ports (no HTTP/REPL deps)
│   ├── rlm-repl-rhai/      # Rhai REPL adapter
│   ├── rlm-server/         # Axum HTTP + SSE server
│   ├── rlm-ffi/            # WASM bindings (Cherry Studio)
│   └── rlm-uar-adapter/    # UAR integration
├── tests/fixtures/         # Golden test data (paper benchmarks)
├── docs/                   # Documentation
└── Cargo.toml             # Workspace config
```

## 📋 Implementation Phases (4 weeks)

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **0: Init** | Day 1 | Workspace, CI/CD, tooling |
| **1: Types** | Days 2-3 | Core types, errors, config, events |
| **2: Ports** | Day 4 | Trait definitions (ReplBackend, LlmProvider, EventSink) |
| **3: Executor** | Days 5-7 | Main RLM pipeline (3 stages) |
| **4: Rhai** | Week 2, Days 1-3 | Rhai REPL backend |
| **5: Server** | Week 2, Days 4-5 | Axum + SSE streaming |
| **6: WASM** | Week 3, Days 1-2 | JavaScript FFI |
| **7: UAR** | Week 3, Days 3-4 | UAR adapter |
| **8: Tests** | Week 3 Day 5 - Week 4 Day 2 | Unit, integration, golden |
| **9: Bench** | Week 4, Days 3-5 | Benchmarks vs paper |

## 🔧 Quick Start Commands

```bash
# Initialize workspace
cd /Users/gqadonis/Projects/prometheus/rlm
cargo check --workspace

# Run all tests
cargo test --workspace

# Run golden tests
cargo test --package rlm-core --test golden_tests

# Format + lint
cargo fmt --all
cargo clippy --workspace -- -D warnings

# Build server
cargo build --release --bin rlm-server

# Build WASM
cd crates/rlm-ffi && wasm-pack build --target web

# Run server
cargo run --bin rlm-server
```

## 📐 Architecture Pattern: Ports & Adapters

```
Core (rlm-core)
    ↓
  Ports (traits)
    ↓
Adapters (other crates)
```

### Port Traits

```rust
// rlm-core/src/ports.rs
#[async_trait]
pub trait ReplBackend: Send + Sync {
    async fn init(&mut self) -> RlmResult<()>;
    async fn execute(&mut self, code: &str) -> RlmResult<String>;
    async fn set_variable(&mut self, name: &str, value: &str) -> RlmResult<()>;
    async fn get_variable(&mut self, name: &str) -> RlmResult<String>;
    async fn get_state(&self) -> RlmResult<HashMap<String, String>>;
    async fn reset(&mut self) -> RlmResult<()>;
    fn supports(&self, feature: &str) -> bool;
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn call(&self, prompt: &str, max_tokens: u32) -> RlmResult<String>;
    async fn stream(&self, prompt: &str, max_tokens: u32) 
        -> RlmResult<Box<dyn Stream<Item = RlmResult<String>> + Send + Unpin>>;
    fn model_name(&self) -> &str;
    fn count_tokens(&self, text: &str) -> usize;
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn emit(&self, event: RlmEvent) -> RlmResult<()>;
    async fn flush(&self) -> RlmResult<()>;
}
```

## 🎨 Event Model (Streaming)

All RLM operations emit events:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RlmEvent {
    Started { request_id: String, timestamp: SystemTime },
    ReplOp { iteration: u32, code: String, result: ReplResult, timestamp: SystemTime },
    RecursiveCall { depth: u32, query: String, parent_id: Option<String>, call_id: String },
    RecursiveResult { call_id: String, result: String, tokens_used: u32 },
    Chunk { content: String, finish_reason: Option<String> },
    ContextChunk { index: usize, total: usize, size_tokens: u32 },
    Done { answer: String, metadata: ExecutionMetadata },
    Error { message: String, recoverable: bool },
}
```

## ✅ Coding Standards (Mandatory)

Every file must comply with `docs/coding-standards/README.md`:

| Guideline | Implementation |
|-----------|----------------|
| **M-STATIC-VERIFICATION** | `cargo clippy -- -D warnings` in CI |
| **M-ERRORS-CANONICAL-STRUCTS** | Use `thiserror` for all error types |
| **M-LOG-STRUCTURED** | Use `tracing` with structured fields |
| **M-TYPES-SEND** | All types/traits must be `Send + Sync` |
| **M-UNSAFE** | `#![forbid(unsafe_code)]` in all crates |
| **M-PUBLIC-DEBUG** | `#[derive(Debug)]` on all public types |
| **M-DOCS** | `#![warn(missing_docs)]` + doc comments on public items |

### Lints (Workspace-level)

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
missing_docs = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
```

## 🧪 Testing Strategy

### Test Pyramid

1. **Unit tests** — Per-module (in each `src/` file)
2. **Integration tests** — Multi-crate workflows (`tests/`)
3. **Golden tests** — Paper benchmarks (`tests/fixtures/`)

### Golden Test Fixtures

Located in `tests/fixtures/`:

- `s_niah/` — Simple needle-in-haystack (O(1))
- `oolong/` — Aggregation tasks (O(n))
- `oolong_pairs/` — Pairwise reasoning (O(n²))
- `browsecomp/` — Multi-hop QA
- `code_repo/` — Code understanding
- `streaming/` — Event ordering validation

Each fixture includes:
- `*.json` — Request/response/context data
- `expected_response.json` — Expected answer + metadata

### Test Template

```rust
#[tokio::test]
async fn test_fixture_name() {
    let request: RlmRequest = load_fixture("dataset/request.json");
    let expected: RlmResponse = load_fixture("dataset/expected_response.json");
    
    let executor = create_test_executor();
    let response = executor.execute(request).await.unwrap();
    
    assert_eq!(response.answer, expected.answer);
    assert!(response.metadata.iterations <= 50);
    assert!(response.metadata.recursive_calls <= expected.metadata.recursive_calls);
}
```

## 📦 Dependency Policy

### Core (`rlm-core`)
**Minimal dependencies only**:
- `serde`, `serde_json` (serialization)
- `thiserror` (errors)
- `tracing` (logging)
- `tokio`, `async-trait`, `futures` (async)

**No**:
- HTTP libraries (axum, reqwest)
- REPL implementations (rhai, Python)
- Framework-specific deps

### Server (`rlm-server`)
Can depend on:
- `axum`, `tower`, `hyper` (HTTP)
- `reqwest` (LLM client)

### FFI (`rlm-ffi`)
Can depend on:
- `wasm-bindgen`, `js-sys`, `web-sys`

## 🚀 AI Assistant Workflow

### For Each Phase

1. **Read** the phase section in `IMPLEMENTATION_PLAN.md`
2. **Copy** the provided code templates
3. **Write** the file to disk
4. **Verify** with the checklist:
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --package <crate-name>
   ```
5. **Check** compliance markers (✅ M-* guidelines)
6. **Commit** with message: `feat(phase-N): <description>`

### Common Patterns

#### Error Handling
```rust
// ❌ DON'T
let value = some_operation().unwrap();

// ✅ DO
let value = some_operation()
    .map_err(|e| RlmError::Other(format!("Operation failed: {}", e)))?;
```

#### Async Traits
```rust
use async_trait::async_trait;

#[async_trait]
impl MyTrait for MyType {
    async fn my_method(&self) -> Result<()> {
        // ...
    }
}
```

#### Logging
```rust
use tracing::{debug, info, warn, instrument};

#[instrument(skip(self, sensitive_data))]
async fn process(&self, id: &str, sensitive_data: &str) -> Result<()> {
    info!("Starting process for {}", id);
    debug!("Step 1 complete");
    Ok(())
}
```

#### Streams
```rust
use futures::Stream;
use tokio_stream::StreamExt;

async fn consume_stream<S>(mut stream: S)
where
    S: Stream<Item = RlmEvent> + Unpin,
{
    while let Some(event) = stream.next().await {
        // Process event
    }
}
```

## 🔍 Verification Checklist

Before marking a phase complete:

- [ ] `cargo check --workspace` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --package <crate>` passes
- [ ] All M-* compliance markers present
- [ ] All public items documented
- [ ] No `unwrap()`/`expect()` in library code
- [ ] All types implement `Debug`
- [ ] All async types are `Send + Sync`

## 📊 Paper Reference (arXiv:2410.01855)

### Key Sections

- **Section 3.1**: Methodology (three-stage pipeline)
- **Section 3.2**: REPL environment design
- **Section 4**: Experimental results
- **Table 1**: Benchmark accuracy comparison
- **Figure 2**: RLM architecture diagram

### Default Config (from paper)

```rust
RlmConfig {
    limits: ExecutionLimits {
        max_iterations: 50,        // Section 3.2
        max_recursion_depth: 1,    // Section 3.2
        timeout: Duration::from_secs(300),
        chunk_size_tokens: 4096,
    },
    llm: LlmConfig {
        temperature: 0.0,          // Deterministic (Section 4.1)
        max_tokens: 4096,
        // ...
    },
    // ...
}
```

## 🐛 Common Pitfalls

| ❌ Don't | ✅ Do |
|---------|------|
| `unwrap()` in lib code | Return `Result` |
| `println!` for logs | `tracing::info!` |
| `std::thread::sleep` | `tokio::time::sleep` |
| Public `struct` fields | Private fields + getters |
| `Rc<T>` in async | `Arc<T>` |
| Blocking I/O | `tokio::fs`, `reqwest` |
| `panic!` in lib | Return error |

## 📂 File Structure Template

Every crate follows:

```
crates/my-crate/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Crate root with module docs
│   ├── types.rs        # Domain types
│   ├── error.rs        # Error types
│   ├── config.rs       # Configuration
│   └── ...             # Other modules
├── tests/
│   └── integration_test.rs
└── benches/
    └── bench.rs
```

**Every `lib.rs`**:
```rust
//! Crate description.
//!
//! # Examples
//! ```
//! // Usage example
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod types;
pub mod error;
// ...

pub use error::{MyError, MyResult};
pub use types::MyType;
```

## 🎯 Success Criteria

Phase complete when:

1. ✅ All verification checks pass
2. ✅ Golden tests validate against paper results
3. ✅ Documentation complete (`cargo doc --no-deps`)
4. ✅ Benchmarks show expected complexity (O(1), O(n), O(n²))
5. ✅ UAR integration working
6. ✅ Cherry Studio FFI functional

## 📞 Next Steps

1. Run `cargo check --workspace` to verify setup
2. Begin **Phase 1**: Implement core types (`rlm-core/src/types.rs`)
3. Follow `IMPLEMENTATION_PLAN.md` phase-by-phase
4. Use golden fixtures for TDD (test-driven development)

---

**Remember**: Safety first, explicit over implicit, composition over inheritance.
