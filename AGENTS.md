# AGENTS.md

## Purpose
This file is the shared source of truth for Codex CLI, Cursor, and Antigravity IDE agents working in this repo. Keep instructions concise, follow the phase plan, and prefer small, verifiable changes.

## Source of Truth
- `CLAUDE.md` for architecture overview and expectations.
- `docs/IMPLEMENTATION_PLAN.md` for the phase-by-phase build order and templates.
- `docs/coding-standards/README.md` for mandatory Rust standards and linting rules.
- `docs/Context.2512.24601v1.pdf` for paper rationale and RLM methodology.

## RLM Summary (from the paper)
- Treat the long prompt as **environment state**, not model input.
- Load the full context into a REPL variable (named `context`) and programmatically inspect it.
- Use **recursive `llm_query()` calls** to decompose long-context tasks.
- Three-stage pipeline: context offload → recursive LLM execution → aggregation.

## Repository Layout
- `crates/rlm-core`: core types, ports, executor (no HTTP/REPL deps).
- `crates/rlm-repl-rhai`: Rhai REPL backend.
- `crates/rlm-server`: Axum + SSE server.
- `crates/rlm-ffi`: WASM bindings.
- `crates/rlm-uar-adapter`: UAR integration.
- `tests/fixtures`: golden test datasets from the paper.

## Non-Negotiable Standards
- `#![forbid(unsafe_code)]` in all crates.
- Use `thiserror` for library errors; avoid `unwrap()`/`expect()` in library code.
- Use `tracing` for logging (no `println!`).
- All public types must be `Send + Sync + Debug` and documented.
- Keep APIs minimal and idiomatic; prefer strong types over primitives.

## Phase Workflow
- Implement phases **sequentially** as written in `docs/IMPLEMENTATION_PLAN.md`.
- Use the provided templates when they exist; keep deltas minimal.
- Do not skip verification steps listed in each phase.
- If a phase requires new dependencies, confirm they align with the plan.

## Default Parameters (paper-aligned)
- Max iterations: **50**.
- Max recursion depth: **1**.
- Deterministic temperature: **0.0**.

## Verification Commands
- `cargo check --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- Golden tests: `cargo test --package rlm-core --test golden_tests`

## Tooling Notes
- Codex CLI reads `AGENTS.md`/`AGENTS.override.md` from repo root downward; keep this file succinct.
- Cursor users can mirror key rules into `.cursor/rules/` or `.cursorrules` if desired, but this file is canonical.
- For reusable Codex workflows, create skills under `.codex/skills/` (keep skills small and explicit).

## Assistant Behavior
- Prefer small, reviewable changes and ask when requirements are unclear.
- Avoid touching unrelated files; do not modify `tests/fixtures` unless a phase requires it.
- Report which checks were run in your final summary.
