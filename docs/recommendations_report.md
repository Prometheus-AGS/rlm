# RLM Recommendations Report

**Date**: 2026-01-21
**Target**: Prometheus RLM Engineering Team

## 1. Executive Summary

This report outlines critical recommendations for the implementation of the Recursive Language Model (RLM). Based on a comparative analysis of the Python reference (`recursive-llm`) and the Rust implementation (`rlm`), we have identified a primary risk area in **REPL Prompt Engineering** and defined strategies for **Polyglot FFI Bindings**.

## 2. Recommendation: Rhai Prompt Engineering

### Risk: The Language Gap
The Rust implementation uses **Rhai** as its scripting language for safety and sandboxability. However, Large Language Models (LLMs) like GPT-4 and Claude 3.5 are overwhelmingly trained on Python. The current generic system prompt does not provide specific Rhai syntax examples.

**Consequence**: The LLM is highly likely to hallucinate Python syntax (e.g., `import re`, `[:]` slicing) when interacting with the Rhai REPL, leading to runtime errors and task failure.

### Strategy
To mitigate this, the `rlm-core` system prompt MUST be updated to include **Rhai-specific few-shot examples**. We recommend enforcing the following patterns in the system prompt:

1.  **Explicit Standard Library Usage**:
    *   *Bad (Hallucinated)*: `text[0:100]`
    *   *Good (Rhai)*: `text.sub_string(0, 100)`
2.  **Regex Handling**:
    *   *Bad (Python)*: `import re; re.findall(...)`
    *   *Good (Rhai)*: `regex_find(text, "pattern")` (Assuming an `rlm-repl-rhai` helper is registered)
3.  **Iteration**:
    *   *Bad (Python)*: `for x in [1,2]:`
    *   *Good (Rhai)*: `for x in [1, 2] { ... }`

**Action Item**: Update `rlm-core/src/executor.rs` to inject these examples dynamically based on the selected `ReplBackend`.

## 3. Recommendation: Polyglot FFI Bindings

To support Python and TypeScript consumption of the RLM core libraries, we recommend a split-strategy approach.

### Python Bindings (PyO3)
We will use **PyO3** to create native Python extension modules. This allows `rlm-core` to be installed as a standard pip package.
*   **Tooling**: `maturin` for building and publishing.
*   **Performance**: Zero-cost abstraction over the Rust core.
*   **Concurrency**: PyO3 supports `asyncio` integration, allowing Python async/await to drive the Rust `tokio` runtime transparently.

### TypeScript Bindings (WASM & NAPI)
For TypeScript, we target two environments:
1.  **Browser (WASM)**: Use `wasm-bindgen` to compile `rlm-core` to WebAssembly. This enables RLM to run entirely client-side (e.g., in Cherry Studio).
2.  **Node.js (NAPI)**: Use `napi-rs` for high-performance Node.js bindings if specialized backend performance is needed, though WASM is often sufficient for the RLM use-case (orchestration).

**Action Item**: Create a new `crates/rlm-ffi` crate with feature flags for `python` (PyO3) and `js` (wasm-bindgen).

## 4. Recommendation: Coding Standards Compliance

The `rlm-core` implementation should be audited against the internal **Pragmatic Rust Guidelines**. Key focus areas:
*   **Error Handling**: Strict use of `thiserror` for library errors (currently implemented).
*   **Async Hygiene**: Proper `yield_now` points in long-running recursive folds to prevent validator starvation.
*   **Telemetry**: Adherence to the `M-LOG-STRUCTURED` guideline using `tracing` spans with named fields.

## 5. Next Steps

1.  **Update Implementation Plan**: Add phases for coding standards audit and FFI/Polyglot bindings.
2.  **Coding Audit**: Review `rlm-core` against `docs/coding-standards/README.md`.
3.  **Prototype FFI**: Initialize `rlm-ffi` with PyO3 and wasm-bindgen targets.
