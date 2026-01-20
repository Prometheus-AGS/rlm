# Implementation Plan: RLM OpenAI-Compatible Server

**Branch**: `001-rlm-openai-server` | **Date**: 2026-01-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-rlm-openai-server/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Implement a production-ready Axum HTTP server that provides OpenAI-compatible chat completions API with RLM (Recursive Language Model) capabilities for handling arbitrarily long contexts. The server implements MIT's RLM paper (arXiv:2512.24601) three-stage pipeline: context offloading to Rhai REPL environment, recursive LLM execution with sub-queries, and result aggregation. Core features include streaming/non-streaming responses, multi-backend configuration (OpenAI, Azure, local models), comprehensive observability, and library-first architecture enabling integration into other Rust applications.

## Technical Context

**Language/Version**: Rust 1.75+ with #![forbid(unsafe_code)]
**Primary Dependencies**: Axum (HTTP server), Tokio (async runtime), Rhai (REPL backend), Serde (serialization), Tracing (structured logging)
**Storage**: In-memory REPL state management with Rhai, no persistent storage required for core functionality
**Testing**: Cargo test with integration tests, golden test fixtures from MIT paper benchmarks
**Target Platform**: Linux/macOS server deployment, WASM compilation support for FFI bindings
**Project Type**: Library-first architecture with HTTP server frontend
**Performance Goals**: Handle 10M+ token contexts, 100 concurrent requests, streaming responses <5s first chunk, <2s intervals
**Constraints**: 99.9% uptime, 150% token usage efficiency vs baseline, 3x processing time for <100K contexts, memory safety
**Scale/Scope**: Production-ready server supporting arbitrary context lengths, OpenAI API compatibility, multi-backend configuration

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ Library-First Architecture
**Requirement**: Core RLM functionality must be library-first with HTTP server as adapter
**Status**: PASS - `rlm-core` crate provides core executor, `rlm-server` crate wraps with Axum HTTP interface
**Justification**: Enables integration into other applications, WASM bindings, and UAR adapter patterns

### ✅ Memory Safety & Security
**Requirement**: Rust #![forbid(unsafe_code)] and secure REPL sandboxing
**Status**: PASS - Rhai provides memory-safe REPL environment, no unsafe code allowed
**Justification**: Production deployment requires security guarantees for arbitrary user context processing

### ✅ Hexagonal Architecture
**Requirement**: Ports-and-adapters pattern with pluggable backends
**Status**: PASS - Core defines traits (ReplBackend, LlmProvider, EventSink), adapters implement them
**Justification**: Support for multiple LLM backends (OpenAI, Azure, local models) and REPL environments

### ✅ Comprehensive Testing
**Requirement**: Integration tests with golden fixtures matching paper benchmarks
**Status**: PASS - Test fixtures from MIT paper datasets (S-NIAH, OOLONG) with performance validation
**Justification**: Must prove implementation achieves paper's claimed performance improvements

### ✅ Observability & Production Readiness
**Requirement**: Structured logging, metrics, error handling for production deployment
**Status**: PASS - Tracing spans, event streaming, graceful error handling, configurable backends
**Justification**: Long-context processing requires visibility into recursive call patterns and performance

### ✅ Complexity Justification Validated
**Requirement**: RLM introduces significant complexity vs simple OpenAI proxy
**Status**: PASS - Justified by unique value proposition and validated implementation approach
**Justification**: Enables 10M+ token processing with better accuracy and cost efficiency than baseline models. Complexity tracking table provides clear rationale for each architectural decision.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
# Cargo workspace with hexagonal architecture
crates/
├── rlm-core/                    # Core domain logic and ports
│   ├── src/
│   │   ├── lib.rs
│   │   ├── executor.rs          # Main RLM execution engine
│   │   ├── types.rs             # Core types (RlmRequest, RlmEvent, etc.)
│   │   ├── ports/               # Trait definitions
│   │   │   ├── repl_backend.rs  # ReplBackend trait
│   │   │   ├── llm_provider.rs  # LlmProvider trait
│   │   │   └── event_sink.rs    # EventSink trait
│   │   └── error.rs             # Error types with thiserror
│   └── tests/                   # Unit tests
├── rlm-repl-rhai/              # Rhai REPL adapter implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── backend.rs           # RhaiReplBackend impl
│   │   └── engine.rs            # Rhai engine configuration
│   └── tests/
└── rlm-server/                  # Axum HTTP server
    ├── src/
    │   ├── lib.rs               # Library interface
    │   ├── main.rs              # Binary entry point
    │   ├── server/              # HTTP server implementation
    │   │   ├── mod.rs
    │   │   ├── routes.rs        # OpenAI API endpoints
    │   │   ├── streaming.rs     # SSE streaming handler
    │   │   └── middleware.rs    # Auth, logging, CORS
    │   ├── config/              # Configuration management
    │   │   ├── mod.rs
    │   │   ├── env.rs           # Environment variables
    │   │   ├── yaml.rs          # YAML config loading
    │   │   └── cli.rs           # CLI argument parsing
    │   ├── adapters/            # External service adapters
    │   │   ├── openai.rs        # OpenAI LLM provider
    │   │   ├── azure.rs         # Azure OpenAI provider
    │   │   └── sse_sink.rs      # SSE event sink
    │   └── error.rs
    └── tests/
        ├── integration/         # Full HTTP integration tests
        └── contract/            # OpenAI API contract tests

tests/                           # Workspace-level tests
├── fixtures/                    # Golden test data from MIT paper
│   ├── s_niah/                 # Simple needle-in-haystack
│   ├── oolong/                 # OOLONG aggregation dataset
│   └── browsecomp/             # Multi-hop QA dataset
└── integration/                 # End-to-end integration tests

examples/                        # Usage examples
├── basic_usage.rs
├── streaming_client.rs
└── multi_backend.rs
```

**Structure Decision**: Cargo workspace with hexagonal architecture following the existing RLM project structure. Core business logic in `rlm-core` with adapter implementations in separate crates. HTTP server in `rlm-server` with both library and binary interfaces for flexibility.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| RLM Recursive Processing | Handle 10M+ token contexts that exceed all existing model windows | Simple OpenAI proxy cannot process contexts beyond model limits (4M tokens max) |
| Rhai REPL Backend | Memory-safe context manipulation and intermediate state storage | Python REPL creates FFI complexity and security risks in production |
| Three-Stage Pipeline | MIT paper's proven approach for O(1) vs O(n²) complexity reduction | Direct context passing hits quadratic attention complexity limits |
| Hexagonal Architecture | Support multiple LLM backends and future REPL implementations | Tight coupling to single backend prevents enterprise deployment flexibility |
| Event Streaming System | Real-time progress for long-running recursive context processing | Batch processing provides no feedback for multi-minute operations |

## Post-Design Constitution Re-evaluation

### ✅ Final Constitution Check - All Gates Passed

**Re-evaluation Date**: 2026-01-19 Post-Phase 1 Design

1. **✅ Library-First Architecture**: Validated through data model showing clear separation between `rlm-core` library and `rlm-server` HTTP adapter
2. **✅ Memory Safety & Security**: Confirmed through Rhai REPL configuration with operation limits, sandboxing, and timeout controls
3. **✅ Hexagonal Architecture**: Validated through port definitions (ReplBackend, LlmProvider, EventSink) with concrete adapter implementations
4. **✅ Comprehensive Testing**: Confirmed through integration test strategy with golden fixtures matching MIT paper benchmarks
5. **✅ Observability & Production Readiness**: Validated through comprehensive event system, metrics endpoints, and structured logging design
6. **✅ Complexity Justification**: All complexity decisions documented with clear rationale and rejected alternatives

**Design Quality Assessment**: The detailed data model, API contracts, and implementation research demonstrate that architectural decisions are well-founded and implementable. The OpenAI API compatibility ensures drop-in replacement capability while RLM extensions provide unique long-context value.

**Ready for Implementation**: All constitution gates passed, complexity justified, and implementation approach validated through research.
