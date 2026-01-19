# Quick Answer: Do I Need to Update CLAUDE.md or AGENTS.md?

## Short Answer

**NO.** ✅

The rust-skills you installed **automatically work** without any manual configuration.

---

## How It Works

Claude Code uses **automatic skill discovery**:

1. **At Startup**: Scans `~/.claude/plugins/cache/` and loads skill names/descriptions (50 tokens each)
2. **During Conversation**: When you mention Rust keywords, error codes, or work in `*.rs` files, skills activate automatically
3. **Progressive Loading**: Full skill content (2-5K tokens) only loads when triggered

---

## What's Already Configured

✅ **rust-skills installed globally**  
Location: `~/.claude/plugins/cache/rust-skills-local/rust-skills/1.0.0/`

✅ **Automatic activation triggers**  
- Keywords: "cargo", "async", "borrow", "lifetime", etc.
- Error codes: E0382, E0597, etc.
- File patterns: `**/*.rs`, `**/Cargo.toml`
- Domain keywords: "web", "CLI", "fintech", etc.

✅ **CLAUDE.md already perfect**  
Contains: Project overview, architecture, commands  
No need to add: Skill activation rules (automatic)

✅ **AGENTS.md already perfect**  
Contains: Phase workflow, standards, verification  
No need to add: Skill references (automatic)

✅ **Speckit commands ready**  
Location: `.claude/commands/speckit.*.md`  
Available: `/speckit.specify`, `/speckit.plan`, etc.

---

## Skills You Have

### Layer 1: Language Mechanics (m01-m07)
- Ownership, borrowing, lifetimes
- Resource management (Box, Arc, Rc)
- Async, Send, Sync, tokio
- Error handling (Result, thiserror)

### Layer 2: Design Patterns (m09-m15)
- Domain modeling
- Performance optimization
- Anti-patterns to avoid

### Layer 3: Domain-Specific
- **domain-web** (Axum, HTTP, SSE)
- **domain-cli** (Clap, terminal apps)
- **domain-cloud-native** (Kubernetes, gRPC)

---

## Example: Automatic Activation

**You ask:**
```
"I'm getting E0382 in rlm-core/src/executor.rs"
```

**Claude Code automatically:**
1. Detects "E0382" keyword
2. Loads `rust-router` skill
3. Routes to `m01-ownership` skill
4. Applies ownership patterns
5. Answers with fix

**No configuration needed!**

---

## Optional Enhancements (Not Required)

You MAY add (but don't need to):

1. **Hooks** for automation:
   - `.claude/hooks/before_write.md` (auto-format)
   - `.claude/hooks/after_implement.md` (auto-test)

2. **RLM-specific skill** (only if you keep repeating same RLM patterns):
   - `.claude/skills/rlm-patterns/SKILL.md`

---

## Test It Now

Try these prompts to verify skills work:

```
1. "Explain Box vs Arc"
   → Should activate m02-resource

2. "How to structure Axum handlers?"
   → Should activate domain-web + m07-concurrency

3. "I'm getting E0382"
   → Should activate rust-router → m01-ownership
```

---

## Read More

For detailed explanation, see:  
`docs/SKILLS_CONFIGURATION_GUIDE.md`

---

**TL;DR**: Skills work automatically. No updates needed. Start coding! 🚀
