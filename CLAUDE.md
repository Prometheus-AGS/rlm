# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RLM (Recursive Language Model) is a Rust implementation of MIT's groundbreaking paper "RLM: A Recursive Language Model for Long Contexts" (arXiv:2512.24601). The project implements a novel inference strategy that enables language models to handle arbitrarily long input contexts by offloading context to an external REPL environment and using recursive decomposition.

### Key Architecture Concepts

**Three-Stage Pipeline:**
1. **Context Offloading** - Move long context into REPL environment instead of direct prompt inclusion
2. **LLM Execution with Recursive Calls** - LLM can call `llm_query()` to spawn recursive sub-calls
3. **Aggregation** - Combine recursive results from REPL state

**Complexity Benefits:**
- Traditional LLM: O(n²) context attention
- RLM: O(1) for needle-in-haystack, O(n) for aggregation tasks, O(n²) for pairwise reasoning
- Handles 10M+ token inputs with higher accuracy and comparable/lower cost

## Workspace Structure

The project uses a Cargo workspace with crates organized under `crates/`:

```
rlm/
├── crates/
│   ├── rlm-core/              # Core executor, types, ports
│   ├── rlm-repl-rhai/         # Rhai REPL backend implementation
│   ├── rlm-server/            # HTTP server with SSE streaming
│   ├── rlm-ffi/              # WASM bindings for Cherry Studio
│   ├── rlm-repl-js/          # JavaScript REPL backend (future)
│   └── rlm-uar-adapter/      # UAR integration adapter
├── docs/
│   └── IMPLEMENTATION_PLAN.md # Detailed 9-phase implementation guide
├── tests/fixtures/           # Golden test fixtures from paper benchmarks
└── examples/                # Usage examples
```

### Crate Dependencies (Hexagonal Architecture)

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

## Development Commands

### Building and Testing
```bash
# Build entire workspace
cargo build --workspace

# Build optimized release
cargo build --release --workspace

# Run all tests including golden tests
cargo test --workspace

# Run tests for specific crate
cargo test --package rlm-core

# Build WASM module for Cherry Studio integration
cd crates/rlm-ffi && wasm-pack build --target web

# Run server locally
cargo run --bin rlm-server -- --port 8080

# Check code quality
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Documentation
```bash
# Build and open workspace documentation
cargo doc --workspace --no-deps --open

# Validate documentation examples
cargo test --doc --workspace
```

## Architecture Implementation Guide

### Core Design Patterns

**Ports and Adapters (Hexagonal Architecture):**
- `rlm-core` defines **ports** (traits): `ReplBackend`, `LlmProvider`, `EventSink`
- Adapter crates implement these ports: `RhaiReplBackend`, `OpenAiProvider`, `SseEventSink`

**Event-Driven Streaming:**
All RLM operations emit `RlmEvent`s for real-time monitoring:
- `ReplOp` - REPL code execution results
- `RecursiveCall` - Spawned recursive LLM calls
- `Chunk` - LLM streaming response chunks
- `ContextChunk` - Context processing progress
- `Done`/`Error` - Terminal states

**Async-First Design:**
- All I/O operations are async with Tokio runtime
- REPL backends wrapped in `Arc<Mutex<_>>` for shared access
- Event sinks use `tokio::sync::mpsc` channels for streaming

### Implementation Phases

The project follows the detailed 9-phase plan in `docs/IMPLEMENTATION_PLAN.md`:

1. **Phase 0**: Project initialization, CI/CD, tooling
2. **Phase 1**: Core types & domain model (`rlm-core`)
3. **Phase 2**: Port definitions (traits)
4. **Phase 3**: Core executor implementation
5. **Phase 4**: Rhai REPL backend (`rlm-repl-rhai`)
6. **Phase 5**: HTTP server with SSE (`rlm-server`)
7. **Phase 6**: WASM FFI bindings (`rlm-ffi`)
8. **Phase 7**: UAR integration adapter
9. **Phase 8**: Comprehensive testing & golden fixtures
10. **Phase 9**: Performance benchmarking vs paper results

### Coding Standards Compliance

The workspace enforces strict quality standards:
- `#![forbid(unsafe_code)]` - No unsafe code allowed
- All errors use `thiserror` for structured error types
- Structured logging with `tracing` (no `println!` in library code)
- All public types must be `Send + Sync + Debug`
- Comprehensive documentation required (`#![warn(missing_docs)]`)
- Clippy pedantic lints enabled with minimal exceptions

## Testing Strategy

### Test Pyramid Structure
- **Unit Tests**: Per-module correctness in each crate
- **Integration Tests**: Multi-crate workflow validation
- **Golden Tests**: Fixtures from paper benchmarks in `tests/fixtures/`

### Golden Test Coverage
Located in `tests/fixtures/` with datasets matching paper evaluation:
- `s_niah/` - Simple needle-in-haystack (O(1) complexity)
- `oolong/` - OOLONG dataset (O(n) aggregation)
- `oolong_pairs/` - Pairwise reasoning (O(n²) complexity)
- `browsecomp/` - Multi-hop QA over documents
- `code_repo/` - Code understanding tasks

### Performance Benchmarks
Target metrics from paper Table 1:
- S-NIAH: 95.0% → 97.5% accuracy improvement
- OOLONG: 72.3% → 89.4% accuracy improvement
- Cost efficiency: comparable or lower token usage vs baseline

## Integration Points

### UAR (Universal Agent Runtime) Integration
- `rlm-uar-adapter` maps RLM events → UAR protocol events
- Exposes RLM via UAR surfaces: OpenAI REST, MCP, A2A, AG-UI
- Enables workflow triggers for long-context tasks

### Cherry Studio Integration
- `rlm-ffi` provides WASM bindings via `wasm-bindgen`
- Simple FFI: `rlm_execute(request, event_callback)`
- Real-time event streaming for UI visualization
- Cherry Studio settings integration for configuration

### LLM Provider Integration
Implement `LlmProvider` trait for different providers:
- OpenAI (GPT-4, GPT-5)
- Anthropic (Claude)
- Local models via API compatibility

## Common Development Patterns

### Error Handling
```rust
// Always use Result types, never unwrap() in library code
async fn execute_code(&mut self, code: &str) -> RlmResult<String> {
    self.engine.eval_with_scope(&mut self.scope, code)
        .map_err(|e| RlmError::Repl(format!("Failed to execute: {}", e)))
}
```

### Async Traits
```rust
#[async_trait]
impl ReplBackend for RhaiReplBackend {
    #[instrument(skip(self, code))]
    async fn execute(&mut self, code: &str) -> RlmResult<String> {
        // Implementation
    }
}
```

### Structured Logging
```rust
use tracing::{debug, info, warn, error, instrument};

#[instrument(skip(self, sensitive_data))]
async fn process_request(&self, id: &str) -> Result<()> {
    info!("Processing request: {}", id);
    debug!("Intermediate step completed");
    Ok(())
}
```

### Event Streaming
```rust
// Emit events for real-time monitoring
self.event_sink.emit(RlmEvent::ReplOp {
    iteration: 1,
    code: code.to_string(),
    result: ReplResult::Success { value: "42".to_string() },
    timestamp: SystemTime::now(),
}).await?;
```

## Paper Validation

The implementation plan has been validated against the MIT paper (arXiv:2512.24601):
- ✅ Context offloading to Python REPL (adapted to Rhai for safety)
- ✅ Recursive `llm_query()` function design
- ✅ Three-stage pipeline architecture
- ✅ Streaming events for intermediate results
- ✅ Complexity analysis and benchmarking targets
- ✅ Multi-task evaluation framework (S-NIAH, OOLONG, etc.)

## Development Notes

- **Phase-by-Phase Implementation**: Follow `docs/IMPLEMENTATION_PLAN.md` sequentially for guaranteed consistency
- **Safety-First**: Rhai REPL provides memory safety vs Python while maintaining scripting flexibility
- **Streaming Priority**: All operations should emit events for real-time UX
- **Testing Excellence**: Golden fixtures ensure paper result reproducibility
- **Documentation Quality**: Every public API must be documented with examples

## Architecture Decisions

**Why Rhai over Python REPL?**
- Memory safety without FFI complexity
- Sandboxed execution environment
- Native Rust integration
- Suitable for WASM compilation

**Why Hexagonal Architecture?**
- Clean separation of core logic from adapters
- Testability via mock implementations
- Support for multiple integration targets (UAR, Cherry Studio, etc.)

**Why SSE over WebSockets?**
- Simpler unidirectional streaming model
- Browser-native support without additional protocols
- Better alignment with HTTP/REST semantics

## Active Technologies
- Rust 1.75+ with #![forbid(unsafe_code)] + Axum (HTTP server), Tokio (async runtime), Rhai (REPL backend), Serde (serialization), Tracing (structured logging) (001-rlm-openai-server)
- In-memory REPL state management with Rhai, no persistent storage required for core functionality (001-rlm-openai-server)

## Recent Changes
- 001-rlm-openai-server: Added Rust 1.75+ with #![forbid(unsafe_code)] + Axum (HTTP server), Tokio (async runtime), Rhai (REPL backend), Serde (serialization), Tracing (structured logging)
