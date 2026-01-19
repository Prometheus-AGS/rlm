# Claude Skills & GitHub Speckit Analysis for RLM Project

## Executive Summary

Based on extensive research into Claude Code capabilities, GitHub Speckit, and the Model Context Protocol (MCP) ecosystem, this document identifies the most valuable tools and workflows for completing the RLM codebase generation using AI coding assistants.

**Primary Recommendation**: Use GitHub Speckit with Claude Code to implement a spec-driven development workflow that ensures the RLM implementation stays aligned with the MIT paper's architecture while maintaining Rust best practices.

---

## 1. GitHub Speckit: The Foundation

### What is GitHub Speckit?

GitHub Speckit is an **open-source toolkit** (introduced September 2024) that implements **Spec-Driven Development (SDD)** workflows. It transforms vague ideas into structured specifications that AI coding agents can execute reliably.

### Core Workflow (4 Phases)

```
Phase 1: Constitution → Define project principles
Phase 2: Specify      → What to build (requirements)
Phase 3: Plan         → How to build (technical design)
Phase 4: Tasks        → Breakdown into actionable steps
Phase 5: Implement    → Execute with AI agent
```

### Key Commands for RLM

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `/speckit.constitution` | Define Rust coding standards, RLM architecture rules | **Once at project start** |
| `/speckit.specify` | Create spec for each crate/module | **Per crate (5 crates)** |
| `/speckit.clarify` | Ask AI to clarify ambiguities | **Before plan phase** |
| `/speckit.plan` | Generate technical design (traits, types, dependencies) | **Per crate** |
| `/speckit.tasks` | Break plan into implementable tasks | **Per crate** |
| `/speckit.implement` | Execute tasks with Claude Code | **Per task** |
| `/speckit.analyze` | Verify spec ↔ implementation alignment | **After implementation** |
| `/speckit.checklist` | Quality validation ("unit tests for requirements") | **Before commit** |

### Why Speckit is Perfect for RLM

1. **Paper Alignment**: The constitution can encode MIT paper constraints (depth=1, iter=50, REPL decomposition)
2. **Rust Safety**: Enforce "no unsafe code", "all types Send+Sync", "thiserror for errors"
3. **Ports & Adapters**: Clarify phase surfaces trait design decisions early
4. **Multi-Crate Management**: Each crate gets its own spec/plan/tasks workflow
5. **Audit Trail**: All specs are git-tracked, reviewable artifacts

---

## 2. Claude Code: The AI Agent

### Core Capabilities (2024-2025)

Claude Code is Anthropic's **agentic coding assistant** with:

- **200K token context window** (can hold entire RLM codebase)
- **Tool use**: bash, file operations, web search, MCP servers
- **Plan mode**: Creates detailed plans before coding (Shift+Tab)
- **Subagents**: Specialized instances (e.g., test writer, code reviewer)
- **Skills**: Reusable knowledge modules (e.g., Rust best practices)
- **Hooks**: Automated actions (e.g., run `cargo fmt` before file write)

### Latest Features (Claude Code 2.1+)

- **Lazy loading of MCP tools** (85% context savings)
- **Hot reload** (no restart needed when config changes)
- **Extended thinking mode** (deep reasoning for complex problems)
- **Code-simplifier plugin** (Anthropic's internal refactoring agent)

### Claude Code + Speckit Integration

Claude Code has **native Speckit plugin** (PR #1451):

```bash
# Install Speckit plugin for Claude Code
claude --plugin-dir ./path/to/spec-kit/plugin

# Now all /speckit:* commands available
/speckit:constitution
/speckit:specify
/speckit:plan
# ... etc
```

---

## 3. Useful Claude Skills for RLM Development

### Recommended Skills (Install Before Starting)

#### A. Rust-Specific Skills

**1. Rust Skills** (by ZhangHanDong)
- GitHub: https://github.com/ZhangHanDong/rust-skills
- **Features**:
  - Meta-cognition framework (Layer 1: Syntax → Layer 2: Idioms → Layer 3: Domain)
  - Domain-specific constraints (e.g., `domain-cloud-native` for 12-factor, observability)
  - Ownership/borrowing explanations beyond "use .clone()"
- **Use for**: Avoiding naive solutions like excessive cloning in REPL state management

**2. LSP (Language Server Protocol) Skill**
- **Built-in to Claude Code**
- **Features**:
  - Semantic code navigation (find definitions, references)
  - Type information queries
  - Real-time error detection
- **Use for**: Understanding trait bounds, async fn signatures across crates

#### B. Development Workflow Skills

**3. Code Review Skill**
- **Purpose**: Automated pre-commit reviews
- **Checks**:
  - Code organization (modules, visibility)
  - Error handling (Result types, thiserror usage)
  - Security (no hardcoded secrets)
  - Test coverage
- **Use for**: Every PR before merging to main

**4. Project Memory Skill** (by richardhightower)
- **Purpose**: Persistent context across sessions
- **Files**: `bugs.md`, `decisions.md`, `key_facts.md`
- **Use for**: Recording why we chose Rhai over Python REPL, UAR integration decisions

**5. TDD Workflow Skill**
- **Purpose**: Test-first development
- **Workflow**: Write test → Run (fail) → Implement → Run (pass) → Refactor
- **Use for**: Golden fixture tests (OOLONG, BrowseComp benchmarks)

#### C. Spec-Driven Development Skill

**6. SDD Agent Skill** (built-in to Speckit plugin)
- **Purpose**: Auto-activates when discussing SDD methodology
- **Features**:
  - Enforces gated workflow (no code before spec)
  - Suggests clarifying questions
  - Validates spec completeness
- **Use for**: Ensuring we don't skip spec phase for "quick fixes"

---

## 4. Model Context Protocol (MCP) Servers for RLM

### What is MCP?

MCP is Anthropic's **open standard** (Nov 2024) for connecting AI agents to external tools/data. Think of it as "USB for AI" — a universal interface.

### Recommended MCP Servers for RLM

#### A. Development Tools

**1. Filesystem MCP** (built-in)
- Read/write/search files
- **Use for**: All file operations in `rlm-*` crates

**2. GitHub MCP**
- List issues/PRs
- Create branches from tickets
- **Use for**: Two-PR workflow (spec PR → impl PR)

**3. Git MCP**
- Commit, branch, diff operations
- **Use for**: Automated commits per task (`/speckit.tasks`)

#### B. Documentation & Research

**4. Web Search MCP** (Tavily)
- Real-time web search
- **Use for**: Finding latest Rhai/wasm-bindgen examples

**5. Documentation MCP**
- Access docs.rs, crates.io
- **Use for**: Verifying API usage (Axum, Tokio, serde)

#### C. Testing & Quality

**6. Code Execution MCP** (sandbox)
- Run Rust code in isolated environment
- **Use for**: Testing REPL executor before integration

**7. Linting MCP** (clippy)
- Run `cargo clippy` automatically
- **Use for**: Hook after file write (auto-fix warnings)

### MCP Configuration Example

```json
// ~/.claude/mcp.json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/gqadonis/Projects/prometheus/rlm"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "<your-token>"
      }
    },
    "tavily-search": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-tavily"],
      "env": {
        "TAVILY_API_KEY": "<your-key>"
      }
    }
  }
}
```

---

## 5. Recommended Setup for RLM Project

### Step 1: Install Tools

```bash
# Install Claude Code
brew install claude-code  # or download from anthropic.com

# Install GitHub Speckit
npm install -g @github/spec-kit

# Initialize Speckit in RLM project
cd /Users/gqadonis/Projects/prometheus/rlm
specify init . --here --ai claude
```

### Step 2: Create Constitution

Create `.specify/constitution.md`:

```markdown
# RLM Project Constitution

## Architecture Principles

1. **Paper Alignment**: All defaults match arXiv:2410.01855
   - Default recursion depth = 1
   - Default iterations = 50
   - Context offloading via REPL required

2. **Ports & Adapters**: Core must be protocol-agnostic
   - No HTTP in `rlm-core`
   - Traits for LLM, REPL, memory, KB
   - Adapters in separate crates

3. **Event-First**: All operations emit events
   - LlmCallStart, LlmChunkReceived, ToolCallPlanned, etc.
   - Events must be orderable (sequence numbers)
   - Filterable by client request

## Rust Standards

1. **Safety**: Forbid unsafe code (`#![forbid(unsafe_code)]`)
2. **Error Handling**: `thiserror` for library errors, `anyhow` for binaries
3. **Concurrency**: All public types must be `Send + Sync`
4. **Async**: Tokio runtime, never block executor
5. **Logging**: Structured logging with `tracing`

## Testing Standards

1. **Golden Fixtures**: All benchmarks (OOLONG, BrowseComp) as tests
2. **Unit Tests**: Every public function has at least one test
3. **Integration Tests**: Each crate has `tests/` directory
4. **Property Tests**: Use `proptest` for event ordering invariants

## Documentation Standards

1. **Every Public Item**: Has doc comment
2. **Examples**: Code examples in doc comments
3. **CLAUDE.md**: Project memory file at workspace root
4. **README.md**: Each crate has its own README

## Compliance Markers

- M-STATIC-VERIFICATION: clippy deny warnings
- M-ERRORS-CANONICAL-STRUCTS: proper thiserror types
- M-LOG-STRUCTURED: tracing macros
- M-TYPES-SEND: all Send + Sync
- M-UNSAFE: forbid unsafe
- M-PUBLIC-DEBUG: all Debug derived
```

### Step 3: Create CLAUDE.md (Project Memory)

Create `/Users/gqadonis/Projects/prometheus/rlm/CLAUDE.md`:

```markdown
# RLM Project Context

## What is RLM?

Recursive Language Model (RLM) is a Rust implementation of the MIT paper (arXiv:2410.01855) that enables LLMs to handle long-context tasks via REPL-based decomposition.

## Tech Stack

- **Language**: Rust (2021 edition)
- **Async Runtime**: Tokio
- **REPL Backend**: Rhai (safe, sandboxed)
- **HTTP Server**: Axum + SSE
- **WASM Target**: wasm-bindgen
- **Error Handling**: thiserror + anyhow
- **Logging**: tracing
- **Testing**: mockall, proptest

## Crate Structure

- `rlm-core`: Execution engine (no HTTP/REPL deps)
- `rlm-repl-rhai`: Rhai REPL adapter
- `rlm-server`: Axum HTTP server with SSE
- `rlm-ffi`: WASM bindings for Cherry Studio
- `rlm-uar-adapter`: UAR integration

## Key Decisions

### Why Rhai over Python?

- **Safety**: Sandboxed, no FFI to CPython
- **Portability**: Compiles to WASM
- **Speed**: Sufficient for our use case (< 10ms overhead)

### Why Axum over warp?

- **Tokio Integration**: First-class async support
- **Type Safety**: Extractors prevent runtime errors
- **SSE Support**: Native via `axum::response::sse`

## Code Patterns

### Event Emission

```rust
self.tx.send(RlmEvent::LlmCallStart {
    seq: self.next_seq(),
    prompt_tokens: tokens,
}).await?;
```

### Error Handling

```rust
// Library code (rlm-core)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RlmError {
    #[error("REPL execution failed: {0}")]
    ReplError(String),
}

// Binary code (rlm-server)
use anyhow::Context;

let result = executor.execute(req)
    .await
    .context("Failed to execute RLM request")?;
```

## Don't Do This

- ❌ Don't use `unwrap()` or `expect()` in library code
- ❌ Don't put HTTP logic in `rlm-core`
- ❌ Don't use global state (prefer explicit passing)
- ❌ Don't block tokio runtime (use `spawn_blocking` for CPU work)

## Always Do This

- ✅ Emit events for all state changes
- ✅ Write tests first (TDD)
- ✅ Run `cargo fmt && cargo clippy` before commit
- ✅ Update CLAUDE.md when making architectural decisions
```

### Step 4: Define Workflows

#### Workflow 1: Per-Crate Development

```bash
# Example: Implementing rlm-core

# 1. Create specification
/speckit.specify Implement rlm-core crate with:
- Core types (RlmRequest, RlmResponse, RlmConfig, RlmEvent)
- Main executor with orchestration loop
- Port traits (LlmProvider, ReplBackend, KnowledgeBase, MemoryStore)
- Event streaming via tokio channels
- No HTTP or REPL implementation (ports only)

# 2. Clarify ambiguities
/speckit.clarify

# 3. Generate technical plan
/speckit.plan

# 4. Review plan, approve
# (Claude shows: traits, types, module structure, dependencies)

# 5. Break into tasks
/speckit.tasks

# 6. Implement each task
/speckit.implement

# 7. Validate against spec
/speckit.analyze

# 8. Quality checklist
/speckit.checklist
```

#### Workflow 2: Two-PR Strategy

```bash
# Phase 1: Spec PR
/speckit.specify "Feature X"
/speckit.plan
/speckit.tasks
/speckit.spec-pr  # Creates PR with spec/plan/tasks

# Review spec PR → Merge

# Phase 2: Implementation PR
/speckit.implement
/speckit.impl-pr  # Creates PR with code

# Review impl PR → Merge
```

### Step 5: Configure Hooks (Automation)

Create `.claude/hooks/before_write.md`:

```markdown
---
name: before-write
description: Run before writing any file
---

Before writing any file:

1. Run `cargo fmt` on the file (if .rs)
2. Check for:
   - Missing doc comments on pub items
   - Use of `unwrap()` or `expect()`
   - Missing `Send + Sync` bounds on public types
3. If errors found, fix automatically or warn user
```

Create `.claude/hooks/after_implement.md`:

```markdown
---
name: after-implement
description: Run after implementing a task
---

After implementing a task:

1. Run `cargo test` for the modified crate
2. Run `cargo clippy -- -D warnings`
3. Update CLAUDE.md if architectural decision made
4. Emit summary of changes
```

---

## 6. Recommended Execution Plan

### Phase 0: Setup (1 day)

- [ ] Install Claude Code + Speckit
- [ ] Configure MCP servers (filesystem, GitHub, Tavily)
- [ ] Create constitution.md
- [ ] Create CLAUDE.md
- [ ] Install Rust Skills
- [ ] Create hooks (before_write, after_implement)

### Phase 1: rlm-core (Week 1)

- [ ] `/speckit.specify` for rlm-core
- [ ] `/speckit.clarify` event emission patterns
- [ ] `/speckit.plan` types, traits, modules
- [ ] `/speckit.tasks` breakdown (types.rs, error.rs, executor.rs, etc.)
- [ ] `/speckit.implement` each task
- [ ] `/speckit.analyze` spec alignment
- [ ] `/speckit.checklist` quality gates

### Phase 2: rlm-repl-rhai (Week 2)

- [ ] `/speckit.specify` Rhai adapter
- [ ] `/speckit.plan` trait implementation, sandboxing
- [ ] `/speckit.tasks` breakdown
- [ ] `/speckit.implement`
- [ ] Test with S-NIAH golden fixtures

### Phase 3: rlm-server (Week 2-3)

- [ ] `/speckit.specify` HTTP + SSE server
- [ ] `/speckit.plan` Axum routes, OpenAI compatibility
- [ ] `/speckit.tasks` breakdown
- [ ] `/speckit.implement`
- [ ] Test with Postman/curl

### Phase 4: rlm-ffi (Week 3)

- [ ] `/speckit.specify` WASM bindings
- [ ] `/speckit.plan` JS interop, Cherry Studio integration
- [ ] `/speckit.tasks` breakdown
- [ ] `/speckit.implement`

### Phase 5: rlm-uar-adapter (Week 4)

- [ ] `/speckit.specify` UAR integration
- [ ] `/speckit.plan` event projection, compatibility
- [ ] `/speckit.tasks` breakdown
- [ ] `/speckit.implement`

### Phase 6: Integration & Testing (Week 4)

- [ ] Run all golden fixture tests (OOLONG, BrowseComp)
- [ ] Benchmark against paper results
- [ ] Documentation review
- [ ] Final `/speckit.analyze` across all crates

---

## 7. Key Learnings from Research

### Do's (From Anthropic's Internal Usage)

1. **Use Plan Mode First** (Shift+Tab in Claude Code)
   - Let Claude explore codebase, create plan
   - Review plan before code execution
   - Saves rework from wrong assumptions

2. **Run Multiple Claude Sessions in Parallel**
   - Terminal 1: Core implementation
   - Terminal 2: Test writing
   - Terminal 3: Documentation
   - Label tabs clearly (e.g., "rlm-core", "tests", "docs")

3. **Leverage Subagents**
   - Create `/agent test-writer` with TDD focus
   - Create `/agent reviewer` with clippy/security focus
   - Subagents have isolated context (no pollution)

4. **Use Skills for Repetitive Patterns**
   - Rust error handling patterns
   - Async fn signatures
   - Trait implementation templates

5. **Let Claude Handle Git**
   - `git commit -m "feat(core): add RlmExecutor"`
   - `git branch 001-rlm-core`
   - Automatic commit message formatting

### Don'ts (Common Pitfalls)

1. **Don't Skip Spec Phase**
   - "Just generate the code" leads to rework
   - Spec clarifies assumptions before investment

2. **Don't Accept All Changes Blindly**
   - Review each file change
   - Claude can misinterpret requirements

3. **Don't Mix Concerns in One Session**
   - Separate sessions for implementation vs. refactoring
   - Prevents context rot (Claude loses focus)

4. **Don't Ignore Warnings**
   - Claude might suggest unsafe patterns if unconstrained
   - Constitution + hooks prevent this

5. **Don't Forget to Update CLAUDE.md**
   - Decisions made in session should be recorded
   - Future sessions benefit from history

---

## 8. Anthropic's Own Stats (Validation)

From "How Anthropic Teams Use Claude Code" blog post:

- **90% of Claude Code's own code** is written by Claude Code
- **Inference team** uses Claude to translate tests into Rust (for unfamiliar codebases)
- **Security team** workflow: "design doc → TDD with Claude → reliable code"
- **Data Infrastructure** saved 20 minutes during K8s outage (Claude diagnosed pod IP exhaustion)

**Key Takeaway**: Even Anthropic (the creators) use Claude Code for Rust development, proving it's production-ready for this use case.

---

## 9. Alternative Tools (For Comparison)

### Cursor

- **Pros**: VS Code fork, tight IDE integration
- **Cons**: Less agentic than Claude Code, weaker planning mode
- **Use Case**: If you prefer IDE over terminal

### GitHub Copilot

- **Pros**: Native GitHub integration
- **Cons**: Autocomplete-focused, not task-oriented
- **Use Case**: Line-by-line assistance, not full crate generation

### Codex (OpenAI)

- **Pros**: GPT-4 Turbo backing
- **Cons**: No Speckit integration, weaker Rust knowledge
- **Use Case**: If you're already OpenAI-heavy

**Recommendation**: Stick with **Claude Code + Speckit** for this project due to superior agentic capabilities and native Rust support.

---

## 10. Success Metrics

Track these metrics to validate the approach:

### Development Speed

- **Baseline**: Estimated 4 weeks for manual implementation
- **Target**: Complete in 4 weeks with AI assistance (same time, higher quality)
- **Measure**: Lines of code generated per day, tests written per day

### Code Quality

- **Baseline**: 0 unsafe code, 0 unwrap/expect in libs
- **Target**: 100% compliance with constitution
- **Measure**: `cargo clippy` warnings, unsafe code count

### Test Coverage

- **Baseline**: All golden fixtures pass
- **Target**: 80%+ coverage on rlm-core
- **Measure**: `cargo tarpaulin`

### Spec Alignment

- **Baseline**: N/A (no spec without Speckit)
- **Target**: 100% of implementation tasks traceable to spec
- **Measure**: `/speckit.analyze` pass rate

---

## 11. Contingency Plans

### If Speckit is Too Heavyweight

**Fallback**: Use CLAUDE.md + manual workflow
- Write specs in `docs/specs/`
- Use `/plan` command (built-in to Claude Code)
- Manual task tracking in GitHub Issues

### If Claude Code Context Runs Out

**Mitigation**: Use Subagents
- Each subagent has 200K context
- Main agent delegates to specialists
- Example: "Subagent A: implement rlm-core, Subagent B: write tests"

### If Rhai Integration Fails

**Fallback**: Python REPL via PyO3
- Requires FFI (complicates WASM)
- Use only if Rhai performance insufficient

---

## 12. Next Steps (Immediate)

1. **Install Tools** (30 min)
   ```bash
   brew install claude-code
   npm install -g @github/spec-kit
   cd /Users/gqadonis/Projects/prometheus/rlm
   specify init . --here --ai claude
   ```

2. **Create Constitution** (1 hour)
   - Copy template from Section 5
   - Customize for RLM specifics

3. **Configure MCP** (30 min)
   - Add filesystem, GitHub, Tavily servers
   - Test with `/mcp list`

4. **Install Skills** (30 min)
   - Clone https://github.com/ZhangHanDong/rust-skills
   - Add to `.claude/skills/`

5. **First Spec** (2 hours)
   - `/speckit.specify` for rlm-core
   - `/speckit.clarify` to test workflow
   - Review generated spec.md

6. **Begin Phase 1** (Week 1)
   - Execute rlm-core implementation
   - Validate with golden fixtures

---

## 13. Resources

### Official Documentation

- Claude Code Docs: https://code.claude.com/docs
- GitHub Speckit: https://github.com/github/spec-kit
- MCP Specification: https://modelcontextprotocol.io
- Anthropic Blog: https://www.anthropic.com/news/how-anthropic-teams-use-claude-code

### Community Resources

- Reddit /r/ClaudeAI: Speckit discussions
- X (Twitter): @bcherny (Claude Code creator)
- Medium: JP Caparas (Claude Code tutorials)
- YouTube: Eric Tech (Claude Skills tutorials)

### Rust-Specific

- Rust Skills: https://github.com/ZhangHanDong/rust-skills
- Rust Book: https://doc.rust-lang.org/book/
- Tokio Tutorial: https://tokio.rs/tokio/tutorial

---

## Conclusion

**The winning combination for RLM:**

1. **GitHub Speckit** for structured, spec-driven workflow
2. **Claude Code** as the AI agent (200K context, agentic)
3. **Rust Skills** for domain-specific expertise
4. **MCP Servers** for filesystem, GitHub, documentation access
5. **Two-PR Workflow** for design review before implementation
6. **Hooks** for automated quality gates (fmt, clippy, tests)

This setup ensures:
- ✅ Paper alignment (constitution enforces defaults)
- ✅ Rust safety (no unsafe, proper error handling)
- ✅ Testability (golden fixtures, TDD workflow)
- ✅ Portability (WASM support via feature flags)
- ✅ Maintainability (spec-driven, git-tracked decisions)

**Estimated Timeline**: 4 weeks to v0.1 (production-ready)

**Confidence Level**: High (based on Anthropic's own usage + community validation)

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-19  
**Author**: RustForge PMPO (based on Tavily research)
