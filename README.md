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

### Unit and Integration Tests

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

### 🔐 Integration Testing with Real LLM Providers

The `rlm-test-utils` crate provides comprehensive integration testing capabilities with **real LLM providers** and **secure key vault management**. This enables 100% integration test coverage with actual OpenAI GPT-4/GPT-5 models.

#### Supported Vault Providers

| Provider | Use Case | Features |
|----------|----------|----------|
| **Local Files** | Development & CI | File-based secrets, fast setup |
| **Supabase Vault** | Production | Database-backed, RLS policies, self-hosted/cloud |
| **AWS Secrets Manager** | Enterprise | AWS native, fine-grained IAM |
| **Azure Key Vault** | Enterprise | Azure native, AD integration |

#### Quick Setup for Local Development

1. **Create local secrets file:**
```bash
mkdir -p ~/.rlm/secrets
cat > ~/.rlm/secrets/test-secrets.json << EOF
{
  "openai_api_key": "sk-proj-your-openai-key-here",
  "anthropic_api_key": "sk-ant-your-anthropic-key-here"
}
EOF
```

2. **Set environment variable:**
```bash
export RLM_LOCAL_SECRETS_FILE="~/.rlm/secrets/test-secrets.json"
```

3. **Run integration tests:**
```bash
# Run with local vault (default)
cargo test --package rlm-test-utils

# Run with all vault providers
cargo test --package rlm-test-utils --features all-providers
```

#### Supabase Vault Setup (Recommended)

Supabase provides the most comprehensive vault solution with database-backed storage, Row Level Security (RLS), and support for both self-hosted and cloud installations.

1. **Environment variables:**
```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_ROLE_KEY="your-service-role-key"
export SUPABASE_SECRETS_TABLE="vault_secrets"  # optional
```

2. **Initialize and store secrets:**
```rust
use rlm_test_utils::SupabaseVaultProvider;

#[tokio::test]
async fn setup_supabase_vault() {
    let vault = SupabaseVaultProvider::from_env().await.unwrap();

    // Initialize the secrets table with RLS policies
    vault.init_table().await.unwrap();

    // Store your OpenAI API key securely
    vault.store_secret("openai_api_key", "sk-proj-your-key").await.unwrap();
    vault.store_secret("anthropic_api_key", "sk-ant-your-key").await.unwrap();
}
```

3. **Run tests with Supabase:**
```bash
cargo test --package rlm-test-utils --features supabase-vault
```

#### AWS Secrets Manager Setup

1. **Configure AWS credentials:**
```bash
export AWS_REGION="us-west-2"
# Use AWS CLI: aws configure
# Or use environment variables: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY
```

2. **Create secrets in AWS:**
```bash
aws secretsmanager create-secret \
    --name "openai_api_key" \
    --secret-string "sk-proj-your-openai-key"

aws secretsmanager create-secret \
    --name "anthropic_api_key" \
    --secret-string "sk-ant-your-anthropic-key"
```

3. **Run tests:**
```bash
cargo test --package rlm-test-utils --features aws-secrets
```

#### Azure Key Vault Setup

1. **Environment variables:**
```bash
export AZURE_KEYVAULT_URL="https://your-vault.vault.azure.net/"
# Use Azure CLI: az login
# Or set: AZURE_CLIENT_ID, AZURE_CLIENT_SECRET, AZURE_TENANT_ID
```

2. **Create secrets in Azure:**
```bash
az keyvault secret set --vault-name "your-vault" \
    --name "openai-api-key" --value "sk-proj-your-openai-key"

az keyvault secret set --vault-name "your-vault" \
    --name "anthropic-api-key" --value "sk-ant-your-anthropic-key"
```

3. **Run tests:**
```bash
cargo test --package rlm-test-utils --features azure-keyvault
```

#### Comprehensive Integration Testing Example

```rust
use rlm_test_utils::{
    TestFramework, LlmTestFramework, VaultFactory,
    OpenAiTestProvider, AccuracyEvaluator, PerformanceEvaluator,
    MarkdownReportGenerator, LlmTestProviderConfig,
};

#[tokio::test]
async fn comprehensive_rlm_integration_test() {
    // 1. Initialize secure vault (auto-detects provider from environment)
    let vault = VaultFactory::from_env().await.unwrap();

    // 2. Create OpenAI test provider with real API
    let config = LlmTestProviderConfig {
        model: "gpt-4".to_string(),
        api_key_vault_key: "openai_api_key".to_string(),
        max_tokens: Some(500),
        temperature: Some(0.1),
        rate_limit_rps: 1.0, // Respect rate limits
        max_retries: 3,
    };

    let provider = OpenAiTestProvider::new(config, vault.clone()).await.unwrap();
    let framework = LlmTestFramework::new(Arc::new(provider), vault);

    // 3. Execute RLM request with real LLM
    let request = RlmRequest {
        query: "What is the main theme of the provided context?".to_string(),
        context: "Long document content here...".to_string(),
        max_iterations: 10,
        recursion_depth: 1,
        metadata: HashMap::new(),
    };

    let response = framework.execute_test(&request).await.unwrap();

    // 4. Evaluate response quality
    let accuracy = AccuracyEvaluator::fuzzy(0.8, vec![
        "theme".to_string(),
        "main idea".to_string(),
    ]);
    let result = accuracy.evaluate_response(&request, &response).await.unwrap();
    assert!(result.quality_score > 0.8);

    // 5. Generate comprehensive report
    let report = framework.generate_report("RLM Integration Test").await.unwrap();
    let generator = MarkdownReportGenerator::new();
    let markdown = generator.generate_report(&report).await.unwrap();

    tokio::fs::write("integration-test-report.md", markdown).await.unwrap();
}
```

#### GitHub Actions CI/CD Setup

The framework includes automated testing workflows:

```yaml
# .github/workflows/integration-tests.yml
name: RLM Integration Tests
on: [push, pull_request, schedule]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        vault: [local, supabase, aws]
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run Integration Tests
        env:
          # Local vault
          RLM_LOCAL_SECRETS_FILE: ${{ secrets.LOCAL_SECRETS_FILE }}

          # Supabase vault
          SUPABASE_URL: ${{ secrets.SUPABASE_URL }}
          SUPABASE_SERVICE_ROLE_KEY: ${{ secrets.SUPABASE_SERVICE_ROLE_KEY }}

          # AWS vault
          AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          AWS_REGION: us-west-2

          # API Keys (stored in your vault)
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          cargo test --package rlm-test-utils --features ${{ matrix.vault }}-vault
```

#### Cost Management and Safety

The testing framework includes built-in cost controls:

```rust
use rlm_test_utils::CostEvaluator;

let cost_evaluator = CostEvaluator::openai_gpt4()
    .with_daily_budget(25.0)     // $25 daily limit
    .with_max_cost_per_test(0.10); // $0.10 per test limit

// Automatic cost tracking and budget enforcement
let cost_result = cost_evaluator.evaluate_response(&request, &response).await?;
println!("Test cost: ${:.4}", cost_result.metadata.get("total_cost").unwrap());
```

#### Security Features

- 🔐 **API keys never exposed** in logs, error messages, or console output
- 🚀 **Encrypted at rest** (vault provider dependent)
- 🔒 **Encrypted in transit** via TLS
- 📝 **Audit logging** for key access
- 🛡️ **Row-level security** for database vaults (Supabase)
- 🔄 **Key rotation** support

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
