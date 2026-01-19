# Quick Start Guide: RLM Development with Claude Code + Speckit

This guide gets you from zero to productive in 60 minutes.

---

## Prerequisites

- macOS, Linux, or Windows with WSL
- Node.js 18+ (for Speckit)
- Git
- Anthropic Claude subscription (Pro or Team)

---

## Step 1: Install Tools (15 min)

### A. Install Claude Code

```bash
# macOS
brew install claude-code

# Or download from:
# https://claude.ai/download
```

### B. Install GitHub Speckit

```bash
npm install -g @github/spec-kit
```

### C. Verify Installation

```bash
claude --version
# Expected: claude 2.1.0 or higher

specify --version
# Expected: specify 0.3.0 or higher
```

---

## Step 2: Initialize RLM Project with Speckit (10 min)

```bash
cd /Users/gqadonis/Projects/prometheus/rlm

# Initialize Speckit
specify init . --here --ai claude

# This creates:
# .specify/
# ├── constitution.md
# ├── templates/
# │   ├── spec.md
# │   ├── plan.md
# │   └── tasks.md
# └── .specifyrc
```

---

## Step 3: Configure Constitution (10 min)

Replace `.specify/constitution.md` with:

```markdown
# RLM Project Constitution

## Core Principles

1. **Paper Alignment** (arXiv:2410.01855)
   - Default recursion depth = 1
   - Default max iterations = 50
   - REPL-based context offloading

2. **Rust Safety First**
   - Forbid unsafe code
   - No unwrap/expect in library code
   - All public types Send + Sync

3. **Ports & Adapters Architecture**
   - rlm-core has NO HTTP/REPL deps
   - Protocol adapters in separate crates

4. **Event-Driven Streaming**
   - All operations emit RlmEvent
   - Events are ordered (seq field)
   - Filterable by client

## Tech Stack

- Rust 2021 edition
- Tokio (async runtime)
- Rhai (REPL backend)
- Axum (HTTP server)
- wasm-bindgen (FFI)
- thiserror (errors)
- tracing (logging)

## Coding Standards

- cargo fmt before every commit
- cargo clippy --deny warnings
- All pub items have doc comments
- TDD: write test first

## Testing Requirements

- Unit tests in src/
- Integration tests in tests/
- Golden fixtures for benchmarks
- 80%+ coverage on rlm-core
```

Save and commit:

```bash
git add .specify/constitution.md
git commit -m "chore: add RLM constitution"
```

---

## Step 4: Create Project Memory (CLAUDE.md) (10 min)

Create `CLAUDE.md` at project root:

```markdown
# RLM Project Context

## What is RLM?

Recursive Language Model implementation in Rust, based on MIT paper arXiv:2410.01855.

## Architecture

```
rlm-server  rlm-ffi  rlm-uar-adapter
     │         │           │
     └─────────┼───────────┘
               │
               ▼
           rlm-core ────────────┐
               │                │
               ▼                ▼
        rlm-repl-rhai    (LLM provider)
```

## Tech Decisions

### Why Rhai over Python?
- Safety: Sandboxed, no FFI
- Portability: Compiles to WASM
- Speed: <10ms overhead

### Why Axum over warp?
- Tokio integration
- Type safety
- Native SSE support

## Code Patterns

**Event Emission:**
```rust
self.tx.send(RlmEvent::LlmCallStart { seq, tokens }).await?;
```

**Error Handling:**
```rust
// Library: use thiserror
#[derive(Error, Debug)]
pub enum RlmError {
    #[error("REPL failed: {0}")]
    ReplError(String),
}

// Binary: use anyhow
executor.execute(req).await.context("Execution failed")?;
```

## Guidelines

❌ **Never:**
- Use unwrap/expect in libs
- Put HTTP in rlm-core
- Block tokio runtime

✅ **Always:**
- Emit events
- Write tests first
- Update this file when making decisions
```

Commit:

```bash
git add CLAUDE.md
git commit -m "docs: add project memory file"
```

---

## Step 5: Configure MCP Servers (10 min)

Create `~/.claude/mcp.json`:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": [
        "-y",
        "@modelcontextprotocol/server-filesystem",
        "/Users/gqadonis/Projects/prometheus/rlm"
      ]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_YOUR_TOKEN_HERE"
      }
    }
  }
}
```

Replace `ghp_YOUR_TOKEN_HERE` with your GitHub token (create at https://github.com/settings/tokens).

Test MCP:

```bash
claude --mcp-list
# Expected: filesystem, github
```

---

## Step 6: First Spec (rlm-core) (5 min)

Start Claude Code:

```bash
cd /Users/gqadonis/Projects/prometheus/rlm
claude
```

In Claude Code, run:

```
/speckit.specify Implement rlm-core crate with:
- Core types: RlmRequest, RlmResponse, RlmConfig, RlmEvent
- Main executor with orchestration loop
- Port traits: LlmProvider, ReplBackend, KnowledgeBase, MemoryStore
- Event streaming via tokio channels
- NO HTTP or REPL implementation (ports only)
- Must compile on first attempt
- All types Send + Sync
- Zero unsafe code
```

Claude will:
1. Read CLAUDE.md and constitution.md
2. Create a feature branch (e.g., `001-rlm-core`)
3. Generate `specs/001-rlm-core/spec.md`
4. Ask clarifying questions

---

## Step 7: Clarify Spec (if needed) (5 min)

If Claude asks questions, answer them. Example:

**Claude**: "Should RlmEvent variants be exhaustive or allow for future extensions?"

**You**: "Exhaustive for v0.1. We'll use semantic versioning for breaking changes."

Run:

```
/speckit.clarify
```

Claude will update the spec.

---

## Step 8: Generate Plan (5 min)

```
/speckit.plan
```

Claude will create:
- `specs/001-rlm-core/plan/research.md` (crate choices, patterns)
- `specs/001-rlm-core/plan/data-model.md` (types, traits)
- `specs/001-rlm-core/plan/contracts/` (API contracts)
- `specs/001-rlm-core/plan/quickstart.md` (how to test)

Review the plan carefully. This is your last gate before coding.

---

## Step 9: Break into Tasks (2 min)

```
/speckit.tasks
```

Claude generates `specs/001-rlm-core/tasks.md`:

```markdown
# Tasks for rlm-core

- [ ] Task 1: Create crates/rlm-core/Cargo.toml
- [ ] Task 2: Implement src/types.rs (RlmRequest, RlmResponse, RlmConfig)
- [ ] Task 3: Implement src/error.rs (RlmError with thiserror)
- [ ] Task 4: Implement src/events.rs (RlmEvent enum)
- [ ] Task 5: Define src/ports.rs (LlmProvider, ReplBackend traits)
- [ ] Task 6: Implement src/executor.rs (RlmExecutor orchestration)
- [ ] Task 7: Write tests/executor_test.rs
- [ ] Task 8: Write README.md
```

---

## Step 10: Implement (Automated!) (varies)

```
/speckit.implement
```

Claude will:
1. Execute each task sequentially
2. Run `cargo fmt` + `cargo clippy` after each file
3. Write tests before implementation (TDD)
4. Ask for approval before writing files (if interactive mode on)
5. Emit progress updates

**Expected output:**
```
✅ Task 1 complete: Cargo.toml created
✅ Task 2 complete: types.rs implemented (128 lines)
✅ Task 3 complete: error.rs implemented (42 lines)
...
✅ All tasks complete! rlm-core is ready.
```

---

## Step 11: Validate (2 min)

```
/speckit.analyze
```

Claude checks:
- Does implementation match spec?
- Are all tasks completed?
- Are tests passing?
- Is documentation complete?

If any issues, Claude will list them. Fix and re-run.

---

## Step 12: Quality Checklist (2 min)

```
/speckit.checklist
```

Claude verifies:
- [ ] No unsafe code
- [ ] No unwrap/expect in lib
- [ ] All pub items documented
- [ ] All tests pass
- [ ] cargo clippy clean
- [ ] Constitution compliance

---

## Step 13: Commit & PR (5 min)

```bash
git add crates/rlm-core specs/001-rlm-core
git commit -m "feat(core): implement rlm-core crate

- Add core types (RlmRequest, RlmResponse, RlmConfig)
- Add port traits (LlmProvider, ReplBackend)
- Add RlmExecutor orchestration loop
- Add event streaming via tokio channels
- 100% test coverage
- Closes #1"

git push origin 001-rlm-core
```

Create PR on GitHub. Link to spec in PR description.

---

## Next Steps

Now repeat Steps 6-13 for:
- `rlm-repl-rhai` (Week 2)
- `rlm-server` (Week 2-3)
- `rlm-ffi` (Week 3)
- `rlm-uar-adapter` (Week 4)

---

## Troubleshooting

### "Claude can't find CLAUDE.md"

```bash
# Ensure it's at project root
ls -la CLAUDE.md

# Tell Claude explicitly
In Claude Code: "Read CLAUDE.md at project root"
```

### "Speckit commands not found"

```bash
# Reload Claude Code
claude --reload

# Or restart
exit
claude
```

### "Tasks take too long"

```bash
# Use Plan Mode to review before executing
# Shift+Tab in Claude Code terminal
```

### "Tests fail after implementation"

```
# Run this in Claude:
Fix failing tests in rlm-core. Read test output, identify root cause, fix implementation.
```

---

## Tips for Success

1. **Always review plans** before `/speckit.implement`
2. **Use Plan Mode** (Shift+Tab) for complex crates
3. **Run multiple Claude sessions** for parallel work
4. **Update CLAUDE.md** when making key decisions
5. **Commit specs separately** from implementation (Two-PR workflow)

---

## Common Commands Reference

| Command | Purpose |
|---------|---------|
| `/speckit.specify <description>` | Create spec |
| `/speckit.clarify` | Ask clarifying questions |
| `/speckit.plan` | Generate technical plan |
| `/speckit.tasks` | Break into tasks |
| `/speckit.implement` | Execute tasks |
| `/speckit.analyze` | Validate implementation |
| `/speckit.checklist` | Quality gates |
| `/plan` | Built-in planning (no Speckit) |
| `/agent <name>` | Create subagent |

---

## Success Criteria

After following this guide, you should have:

- ✅ Claude Code + Speckit installed
- ✅ RLM project initialized
- ✅ Constitution defined
- ✅ CLAUDE.md created
- ✅ MCP configured
- ✅ First spec (rlm-core) generated
- ✅ First implementation complete
- ✅ All tests passing

**Time spent:** ~60 minutes  
**Lines of code generated:** ~500-1000 (for rlm-core)  
**Manual coding:** ~0 lines (Claude did it all)

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-19  
**Estimated Completion Time**: 60 minutes
