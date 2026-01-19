# Skills Configuration Guide for RLM Project

## Executive Summary

**✅ GOOD NEWS**: The rust-skills are **already automatically detected and working** in Claude Code!

You **DO NOT need to manually update** CLAUDE.md or AGENTS.md to make the skills work. Claude Code's skill system uses **automatic discovery and activation** based on:

1. **Content matching** (keywords in your conversation)
2. **File pattern matching** (working with `*.rs` or `Cargo.toml` files)
3. **Error code detection** (E0382, E0597, etc.)
4. **Domain keyword detection** (web, CLI, embedded, fintech, etc.)

---

## How Claude Code Skills Work (The Architecture)

### Discovery Phase (Startup)
When Claude Code starts, it:
1. Scans `~/.claude/plugins/cache/` for installed skills
2. Loads **only the name and description** (50 tokens per skill)
3. Keeps the full skill content **lazy-loaded** (not in context yet)

**For your installation:**
```
✅ rust-skills installed at:
   /Users/gqadonis/.claude/plugins/cache/rust-skills-local/rust-skills/1.0.0/
   
✅ Includes skills:
   - rust-router (main entry point skill)
   - rust-learner
   - rust-daily
   - rust-skill-creator
   - m01-m15 (ownership, concurrency, error handling, etc.)
   - domain-* (web, CLI, fintech, cloud-native, etc.)
```

### Activation Phase (During Conversation)
When you mention:
- **Rust-related keywords**: "cargo", "rustc", "borrow", "lifetime", "async"
- **Error codes**: "E0382", "E0597", "cannot borrow"
- **File patterns**: Working in files matching `**/*.rs` or `**/Cargo.toml`
- **Domain keywords**: "web server", "CLI", "trading", "microservice"

Claude Code:
1. **Matches** these triggers to the skill's `description` field
2. **Loads** the full skill content (2-5K tokens)
3. **Applies** the skill's instructions and patterns
4. **Routes** to additional skills if needed (via the Meta-Cognition Framework)

---

## What You Already Have Working

### 1. Global Skills (User-Level)
Location: `~/.claude/plugins/cache/rust-skills-local/rust-skills/1.0.0/skills/`

**Automatically activated for ALL Rust work across ALL projects:**

#### Layer 1 Skills (Language Mechanics)
- `m01-ownership` - Move semantics, borrowing, lifetimes
- `m02-resource` - Box, Arc, Rc, RefCell
- `m03-mutability` - Interior mutability patterns
- `m04-zero-cost` - Generics, traits, monomorphization
- `m05-type-driven` - Type state, newtypes, phantom types
- `m06-error-handling` - Result, Error, thiserror, anyhow
- `m07-concurrency` - Send, Sync, async, tokio

#### Layer 2 Skills (Design Patterns)
- `m09-domain` - Domain modeling
- `m10-performance` - Optimization patterns
- `m11-ecosystem` - Crate integration
- `m12-lifecycle` - RAII, Drop, resource management
- `m13-domain-error` - Domain-specific error strategies
- `m14-mental-model` - How to think in Rust
- `m15-anti-pattern` - Common mistakes to avoid

#### Layer 3 Skills (Domain-Specific)
- `domain-web` - Axum, HTTP, REST APIs
- `domain-cli` - Clap, terminal applications
- `domain-fintech` - Trading systems, financial data
- `domain-cloud-native` - Kubernetes, gRPC, microservices
- `domain-iot` - Embedded systems, MQTT
- `domain-ml` - Machine learning, tensors

### 2. Project-Level Commands (RLM-Specific)
Location: `/Users/gqadonis/Projects/prometheus/rlm/.claude/commands/`

**GitHub Speckit commands available in this project:**
- `/speckit.constitution` - Define project principles
- `/speckit.specify` - Create feature specs
- `/speckit.clarify` - Ask AI to clarify ambiguities
- `/speckit.plan` - Generate technical design
- `/speckit.tasks` - Break plan into tasks
- `/speckit.implement` - Execute tasks
- `/speckit.analyze` - Verify spec alignment
- `/speckit.checklist` - Quality validation

### 3. Project Context Files (Already Configured)
- ✅ `/Users/gqadonis/Projects/prometheus/rlm/CLAUDE.md` - Project overview, architecture, commands
- ✅ `/Users/gqadonis/Projects/prometheus/rlm/AGENTS.md` - Shared rules for all AI agents

---

## How Skills Are Automatically Triggered

### Example 1: Compile Error Scenario
```
You: "I'm getting E0382 in rlm-core/src/executor.rs"

Claude Code:
1. Detects "E0382" keyword
2. Matches to rust-router skill description
3. Loads rust-router (routes to m01-ownership)
4. Loads m01-ownership skill (ownership/move errors)
5. Applies the skill's diagnostic patterns
6. Checks if working in `**/*.rs` file
7. Answers using ownership best practices
```

### Example 2: Design Question Scenario
```
You: "How should I design async REPL execution for RLM?"

Claude Code:
1. Detects "async" and "design" keywords
2. Loads rust-router (routes to Layer 2)
3. Loads m07-concurrency (async patterns)
4. Loads m09-domain (domain modeling)
5. Checks CLAUDE.md for project context
6. Answers with tokio patterns + RLM constraints
```

### Example 3: Domain-Specific Scenario
```
You: "Building HTTP server with Axum for rlm-server"

Claude Code:
1. Detects "HTTP server" and "Axum"
2. Loads rust-router → domain-web skill
3. Loads m07-concurrency (async handlers)
4. Reads CLAUDE.md (sees rlm-server uses Axum)
5. Applies domain-web patterns (state, extractors, SSE)
6. Generates code following RLM architecture
```

---

## The Meta-Cognition Framework (Advanced)

The rust-skills use a **3-layer cognitive model** that automatically traces through problem space:

```
Layer 3: Domain Constraints (WHY)
├── Business rules, regulatory requirements
├── domain-fintech, domain-web, domain-cli, etc.
└── "Why is it designed this way?"

Layer 2: Design Choices (WHAT)
├── Architecture patterns, DDD concepts
├── m09-m15 skills
└── "What pattern should I use?"

Layer 1: Language Mechanics (HOW)
├── Ownership, borrowing, lifetimes, traits
├── m01-m07 skills
└── "How do I implement this in Rust?"
```

### Automatic Routing Rules

| Your Question Type | Entry Layer | Routing Direction | Skills Loaded |
|--------------------|-------------|-------------------|---------------|
| "E0382 error" | Layer 1 | Trace UP ↑ | m01 → check m09/domain |
| "How to design..." | Layer 2 | Check L3, DOWN ↓ | m09 → domain → m01-m07 |
| "Building web app" | Layer 3 | Trace DOWN ↓ | domain-web → m07 → m01 |
| "Best practice..." | Layer 2 | Both directions | m09-m15 + context |

### Example of Multi-Skill Activation

```
You: "My web API handler reports 'Rc<State> cannot be sent between threads'"

Automatic Skill Cascade:
1. rust-router detects: "web API" + "cannot be sent"
2. Loads domain-web (Layer 3 - web state management)
3. Loads m07-concurrency (Layer 1 - Send/Sync traits)
4. Traces up: Web handlers must be Send (HTTP is concurrent)
5. Traces down: Replace Rc with Arc (thread-safe)
6. Answer includes both layers + rationale
```

---

## What You DON'T Need to Do

### ❌ Don't Update CLAUDE.md for Skill Detection
The skills are **automatically detected**. Your CLAUDE.md is already perfect for:
- ✅ Project overview
- ✅ Architecture documentation
- ✅ Development commands
- ✅ Common patterns

**Don't add:**
- Skill activation rules (automatic)
- Skill trigger keywords (handled by rust-router)
- Skill routing logic (Meta-Cognition handles it)

### ❌ Don't Update AGENTS.md for Skill Detection
Your AGENTS.md is already correctly focused on:
- ✅ Phase workflow rules
- ✅ Non-negotiable standards
- ✅ Verification commands
- ✅ Paper constraints

**Don't add:**
- Skill references (automatic)
- Domain knowledge (skills provide it)
- Rust patterns (m01-m15 provide them)

### ❌ Don't Create Project-Specific Rust Skills
The global rust-skills already cover:
- ✅ All Rust language mechanics (m01-m07)
- ✅ All design patterns (m09-m15)
- ✅ All common domains (web, CLI, fintech, etc.)

**Only create project skills for:**
- ✅ RLM-specific workflows (already have Speckit commands)
- ✅ Paper-specific constraints (already in CLAUDE.md)
- ✅ Hooks/automation (e.g., pre-commit checks)

---

## What You SHOULD Consider Adding

### Optional Enhancement 1: Project-Specific Hooks

Create `.claude/hooks/` for automated quality gates:

**`.claude/hooks/before_write.md`** (Auto-format before file writes)
```markdown
---
name: before-write
description: Run before writing any Rust file
---

Before writing any .rs file:

1. Run `cargo fmt` on the file
2. Check for:
   - Missing doc comments on pub items
   - Use of `unwrap()` or `expect()`
   - Missing Send + Sync bounds
3. If errors found, warn user or auto-fix
```

**`.claude/hooks/after_implement.md`** (Post-implementation checks)
```markdown
---
name: after-implement
description: Run after implementing a task
---

After implementing any Rust code:

1. Run `cargo test` for the modified crate
2. Run `cargo clippy -- -D warnings`
3. Update CLAUDE.md if architectural decision made
4. Emit summary of changes
```

### Optional Enhancement 2: RLM-Specific Skill

If you find yourself **repeatedly explaining RLM concepts**, create:

**`.claude/skills/rlm-patterns/SKILL.md`**
```markdown
---
name: rlm-patterns
description: RLM-specific architecture patterns. Use when implementing RLM executor, REPL integration, or recursive LLM calls.
globs: ["**/rlm-*/src/**/*.rs"]
---

# RLM Architecture Patterns

## When to Use
- Implementing RLM executor components
- Writing REPL backend adapters
- Working with recursive LLM calls
- Event streaming for RLM operations

## Core Patterns

### Context Offloading Pattern
```rust
// GOOD: Load context into REPL environment
let context_var = format!("context = {}", serde_json::to_string(&context)?);
repl.execute(&context_var).await?;

// BAD: Include full context in prompt
let prompt = format!("Given this context: {}\n...", context);
```

### Recursive Call Pattern
```rust
// GOOD: Expose llm_query() to REPL
repl.register_fn("llm_query", |prompt: String| {
    executor.spawn_recursive_call(prompt)
});

// BAD: Direct LLM calls from REPL
// (loses event streaming and control)
```

### Event Emission Pattern
```rust
// ALWAYS emit events for all state changes
self.event_sink.emit(RlmEvent::ReplOp {
    iteration: self.iteration,
    code: code.to_string(),
    result: ReplResult::Success { value },
    timestamp: SystemTime::now(),
}).await?;
```

## Anti-Patterns

### ❌ Don't Block Tokio Runtime
```rust
// BAD: Blocking REPL execution
let result = repl.execute(code).await?;  // blocks executor

// GOOD: Use spawn_blocking
let result = tokio::task::spawn_blocking(move || {
    repl_blocking.execute(code)
}).await??;
```

## Integration with Rust Skills

This skill complements:
- **m07-concurrency** for async patterns
- **m06-error-handling** for RlmError types
- **domain-web** for rlm-server implementation
```

This would be stored at: `/Users/gqadonis/Projects/prometheus/rlm/.claude/skills/rlm-patterns/SKILL.md`

And would **automatically activate** when:
- Working in `rlm-*/src/**/*.rs` files
- Mentioning "RLM executor", "REPL integration", etc.

---

## Testing Skill Activation

### Verify Skills Are Working

Try these test prompts:

1. **Test rust-router activation:**
   ```
   You: "Explain the difference between Box, Rc, and Arc"
   
   Expected: Claude loads m02-resource skill and explains with examples
   ```

2. **Test domain skill activation:**
   ```
   You: "How should I structure Axum handlers for rlm-server?"
   
   Expected: Claude loads domain-web + m07-concurrency
   ```

3. **Test error code routing:**
   ```
   You: "I'm getting E0382 in rlm-core"
   
   Expected: Claude loads rust-router → m01-ownership
   ```

4. **Test Meta-Cognition tracing:**
   ```
   You: "Analyze this question: Should I use Arc or Rc for REPL state?"
   
   Expected: Claude traces Layer 1 (m02) → Layer 2 (m07 concurrency) → answer
   ```

### Debugging Skill Activation

If skills don't seem to activate, check:

1. **Skill installation:**
   ```bash
   ls -la ~/.claude/plugins/cache/rust-skills-local/rust-skills/1.0.0/skills/
   ```

2. **Claude Code version:**
   ```bash
   claude --version
   # Should be v2.1.0+ for best skill support
   ```

3. **Permissions:**
   ```bash
   cat ~/.claude/settings.json
   # Look for skill-related permissions
   ```

---

## Summary: What's Already Working

| Component | Status | Location | Purpose |
|-----------|--------|----------|---------|
| **rust-router** | ✅ Active | `~/.claude/plugins/.../rust-router/` | Entry point, routes to other skills |
| **m01-m07 skills** | ✅ Active | `~/.claude/plugins/.../skills/m0*/` | Language mechanics (ownership, async, etc.) |
| **m09-m15 skills** | ✅ Active | `~/.claude/plugins/.../skills/m*/` | Design patterns, anti-patterns |
| **domain-* skills** | ✅ Active | `~/.claude/plugins/.../skills/domain-*/` | Web, CLI, fintech, cloud-native |
| **CLAUDE.md** | ✅ Complete | `/Users/.../rlm/CLAUDE.md` | Project context |
| **AGENTS.md** | ✅ Complete | `/Users/.../rlm/AGENTS.md` | Agent rules |
| **Speckit commands** | ✅ Available | `/Users/.../rlm/.claude/commands/` | Spec-driven workflow |

---

## Key Takeaway

**🎉 You're already fully configured!**

The rust-skills you installed are **globally available** and will **automatically activate** when you:
- Work on Rust files
- Ask Rust questions
- Encounter compile errors
- Discuss architecture/design
- Mention domain keywords (web, CLI, etc.)

**No manual configuration needed in CLAUDE.md or AGENTS.md.**

The only optional additions are:
1. **Hooks** (for automation like pre-commit formatting)
2. **Project-specific skills** (only if you have RLM-specific patterns not covered by generic Rust skills)

---

## Next Steps

1. **Try the test prompts above** to verify skills are activating
2. **Start using Speckit commands** (`/speckit.specify`, `/speckit.plan`, etc.) for structured development
3. **Rely on rust-router** to automatically route your questions to the right skills
4. **Watch for skill activation messages** in Claude Code output (it may show which skills were loaded)
5. **Focus on coding** - the skills will guide you automatically!

---

**Document Version**: 1.0  
**Last Updated**: 2026-01-19  
**Author**: RustForge PMPO (based on rust-skills v2.0.0)
