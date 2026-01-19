# RLM (Recursive Language Model)

A production-quality Rust implementation of the Recursive Language Model architecture from the MIT paper ["RLM: A Recursive Language Model for Long Contexts"](https://arxiv.org/abs/2410.01855).

## 📄 Paper Summary

RLM is a novel prompting technique that enables language models to process inputs far exceeding their native context window by:

1. **Context Offloading** — Moving context into an interactive REPL environment instead of the prompt
2. **Recursive Decomposition** — Breaking complex tasks into subtasks via recursive `llm_query()` calls
3. **Minimal Overhead** — Achieving linear or sub-linear complexity compared to quadratic costs of full-context processing

### Benchmark Results (from paper)

| Task | Context Size | Baseline Acc | RLM Acc | Complexity |
|------|--------------|--------------|---------|------------|
| S-NIAH | 8K-128K | 95.0% | 97.5% | O(1) |
| OOLONG | 32K-131K | 72.3% | 89.4% | O(n) |
| OOLONG-Pairs | 8K-32K | 45.2% | 78.6% | O(n²) |
| BrowseComp | 100 docs | 61.8% | 83.2% | O(n log n) |
| Code Repos | ~50K tokens | 68.5% | 84.1% | O(n) |

## 🎯 Project Goals

This implementation has three primary objectives:

### 1. UAR Integration
Integrate RLM as an execution strategy within the **Prometheus Universal Agent Runtime (UAR)**:
- Expose RLM as a UAR adapter that implements UAR's execution ports
- Stream RLM events (REPL operations, recursive calls, context chunks) as UAR-normalized events
- Support all UAR protocol surfaces: OpenAI REST, MCP, A2A, AG-UI
- Enable workflows to trigger RLM execution for long-context tasks

### 2. Cherry Studio FFI
Provide JavaScript/WASM bindings for direct integration into **Cherry Studio**:
- Compile `rlm-core` and `rlm-repl-rhai` to WASM
- Export simple FFI: `rlm_execute(request, event_callback)`
- Stream events to Cherry Studio UI for real-time visualization
- Support configuration via Cherry Studio settings

### 3. Production Rust Implementation
Build a reference implementation following Rust best practices:
- Safety-first design (no panics, proper error handling)
- Async-first with Tokio
- Minimal dependencies in core
- Comprehensive testing (unit, integration, golden fixtures)
- Full compliance with Rust API Guidelines

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     UAR / Cherry Studio                      │
│                  (OpenAI, MCP, A2A, AG-UI)                   │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
         ┌─────────────────────────────┐
         │    rlm-uar-adapter          │  ← UAR integration
         │  (maps UAR → RLM events)    │
         └─────────────┬───────────────┘
                       │
         ┌─────────────▼───────────────┐
         │       rlm-server            │  ← HTTP + SSE streaming
         │    (Axum, SSE events)       │
         └─────────────┬───────────────┘
                       │
         ┌─────────────▼───────────────┐
         │       rlm-core              │  ← Core executor
         │  (RlmExecutor + ports)      │
         └─────────────┬───────────────┘
                       │
         ┌─────────────▼───────────────┐
         │    rlm-repl-rhai            │  ← Rhai REPL backend
         │  (sandboxed execution)      │
         └─────────────────────────────┘
                       │
         ┌─────────────▼───────────────┐
         │       rlm-ffi               │  ← WASM bindings
         │  (wasm-bindgen exports)     │
         └─────────────────────────────┘
```

### Crate Breakdown

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `rlm-core` | Core types, executor, ports | `serde`, `thiserror`, `tracing` (minimal) |
| `rlm-repl-rhai` | Rhai-based REPL backend | `rhai`, `tokio` |
| `rlm-server` | HTTP server with SSE streaming | `axum`, `tokio`, `tower` |
| `rlm-ffi` | WASM FFI for Cherry Studio | `wasm-bindgen`, `serde-wasm-bindgen` |
| `rlm-uar-adapter` | UAR integration adapter | `rlm-core`, UAR types |

## 🚀 Installation

### Rust Native

```bash
# Clone the repository
git clone https://github.com/prometheus/rlm.git
cd rlm

# Build the workspace
cargo build --release

# Run the server
cargo run --bin rlm-server -- --port 8080
```

### JavaScript/WASM (Cherry Studio)

```bash
# Build WASM module
cd rlm-ffi
wasm-pack build --target web --out-dir pkg

# Use in Cherry Studio
import init, { rlm_execute } from './rlm-ffi/pkg/rlm_ffi.js';

await init();
const events = [];
await rlm_execute(request, (event) => {
  events.push(event);
  console.log('RLM Event:', event);
});
```

## 📖 Usage

### Basic Rust API

```rust
use rlm_core::{RlmExecutor, RlmRequest, RlmConfig};
use rlm_repl_rhai::RhaiReplBackend;

#[tokio::main]
async fn main() {
    let config = RlmConfig::default();
    let repl = RhaiReplBackend::new();
    let llm = MyLlmProvider::new();
    
    let executor = RlmExecutor::new(config, repl, llm);
    
    let request = RlmRequest {
        query: "Summarize key points from these 100 documents".to_string(),
        context: load_documents(),
        max_iterations: 50,
        recursion_depth: 1,
    };
    
    let response = executor.execute(request).await?;
    println!("Answer: {}", response.answer);
}
```

### HTTP Server with SSE

```bash
# Start server
cargo run --bin rlm-server

# Make request
curl -X POST http://localhost:8080/v1/rlm/execute \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "query": "Find the needle in the haystack",
    "context": "...",
    "max_iterations": 50
  }'
```

### UAR Integration

```rust
use rlm_uar_adapter::RlmUarAdapter;
use uar::{UarRuntime, ExecutionRequest};

let runtime = UarRuntime::new();
runtime.register_adapter("rlm", RlmUarAdapter::new());

let request = ExecutionRequest {
    strategy: "rlm".to_string(),
    messages: vec![...],
    tools: vec![...],
};

let mut stream = runtime.execute(request).await?;
while let Some(event) = stream.next().await {
    match event {
        UarEvent::RlmReplOp(op) => { /* handle REPL operation */ },
        UarEvent::RlmRecursiveCall(call) => { /* handle recursive call */ },
        UarEvent::Chunk(chunk) => { /* handle LLM chunk */ },
        _ => {}
    }
}
```

## 🧩 How It Works

RLM implements the three-stage process from the paper:

### Stage 1: Context Offloading
```python
# Instead of:
prompt = f"{long_context}\n\nQuestion: {query}"

# RLM does:
repl.set_variable("context", long_context)
prompt = f"Context is in REPL variable 'context'. Question: {query}"
```

### Stage 2: Recursive Decomposition
The LLM can call `llm_query(subquery)` to spawn recursive sub-calls:
```python
# REPL available function
def llm_query(q: str) -> str:
    """Recursively query the LLM with reduced context"""
    return rlm_executor.execute_recursive(q, current_depth + 1)
```

### Stage 3: Aggregation
Sub-call results are stored in REPL and aggregated:
```python
results = []
for item in context:
    results.append(llm_query(f"Process {item}"))
final_answer = aggregate(results)
```

## ⚙️ Configuration

```toml
[rlm]
max_iterations = 50          # Max REPL iterations per execution
recursion_depth = 1          # Max recursive call depth
chunk_size = 4096            # Context chunk size (tokens)
timeout_seconds = 300        # Execution timeout
enable_code_exec = true      # Allow REPL code execution
streaming = true             # Enable SSE streaming

[rlm.llm]
provider = "openai"
model = "gpt-4-turbo"
temperature = 0.0
max_tokens = 4096

[rlm.repl]
backend = "rhai"             # "rhai" or "python" (future)
sandbox = true               # Enable sandboxing
max_memory_mb = 512          # Memory limit
```

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run golden tests (paper benchmarks)
cargo test --package rlm-core --test golden_tests

# Run with coverage
cargo tarpaulin --workspace --out Html

# Benchmark against paper results
cargo bench --package rlm-core
```

### Golden Test Fixtures

Located in `tests/fixtures/`:
- `s_niah/` — Simple needle-in-haystack (O(1) complexity)
- `oolong/` — OOLONG dataset (O(n) aggregation)
- `oolong_pairs/` — Pairwise reasoning (O(n²) complexity)
- `browsecomp/` — Multi-hop QA over documents
- `code_repo/` — Code understanding tasks
- `streaming/` — Event streaming validation

## 📊 Performance Characteristics

Based on paper results:

| Complexity Class | Traditional LLM | RLM | Improvement |
|------------------|-----------------|-----|-------------|
| O(1) — S-NIAH | O(n²) context | O(1) | ~128x |
| O(n) — OOLONG | O(n³) | O(n) | ~n² |
| O(n²) — Pairs | O(n⁴) | O(n²) | ~n² |
| O(n log n) — Browse | O(n³) | O(n log n) | ~n²/log n |

Memory usage: **Linear** in context size (vs quadratic for attention)

## 🔌 UAR Integration Details

RLM integrates with UAR via the adapter pattern:

```rust
// UAR adapter maps RLM events → UAR events
impl UarExecutionAdapter for RlmUarAdapter {
    async fn execute(&self, request: UarRequest) -> UarEventStream {
        let rlm_request = self.convert_request(request);
        let rlm_stream = self.executor.execute_stream(rlm_request).await?;
        
        rlm_stream.map(|event| match event {
            RlmEvent::ReplOp(op) => UarEvent::ToolCall {
                tool: "repl.execute",
                args: op.code,
            },
            RlmEvent::RecursiveCall(call) => UarEvent::SubCall {
                query: call.query,
                depth: call.depth,
            },
            RlmEvent::Chunk(chunk) => UarEvent::Chunk(chunk),
            // ... more mappings
        })
    }
}
```

UAR can then expose RLM via:
- OpenAI `/v1/chat/completions` with `strategy: "rlm"`
- MCP tool: `rlm.execute`
- A2A event: `rlm.stream`
- AG-UI widget: RLM visualization

## 🌐 Cherry Studio Integration

Cherry Studio uses the WASM FFI:

```typescript
import init, { RlmExecutor } from '@rlm/ffi';

await init();

const executor = new RlmExecutor({
  maxIterations: 50,
  recursionDepth: 1,
});

// Stream events to UI
executor.execute(request, (event) => {
  if (event.type === 'repl_op') {
    ui.showReplExecution(event.code, event.result);
  } else if (event.type === 'recursive_call') {
    ui.showRecursiveCall(event.query, event.depth);
  } else if (event.type === 'chunk') {
    ui.appendChunk(event.content);
  }
});
```

## 🗺️ Roadmap

### Phase 1: Core Implementation (v0.1)
- ✅ Core types and executor
- ✅ Rhai REPL backend
- ✅ HTTP server with SSE
- ✅ Golden test fixtures

### Phase 2: Integration (v0.2)
- ⬜ UAR adapter
- ⬜ WASM FFI
- ⬜ Cherry Studio plugin
- ⬜ Performance benchmarks

### Phase 3: Advanced Features (v0.3)
- ⬜ Python REPL backend
- ⬜ Workflow integration
- ⬜ Custom decomposition strategies
- ⬜ Distributed execution

## 📚 References

- [RLM Paper (arXiv:2410.01855)](https://arxiv.org/abs/2410.01855)
- [Prometheus UAR Documentation](../uar/README.md)
- [Cherry Studio](https://github.com/cherry-studio/cherry-studio)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## 📝 License

MIT

## 🤝 Contributing

Contributions welcome! Please read `docs/IMPLEMENTATION_PLAN.md` for development guidelines.

---

**Status**: 🚧 Active Development | **Target**: Production-ready v0.1 by Q2 2024
