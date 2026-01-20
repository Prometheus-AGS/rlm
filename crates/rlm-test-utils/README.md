# RLM Test Utils

Comprehensive integration testing utilities for the RLM (Recursive Language Model) project with secure key management and real LLM provider testing.

## 🚀 Features

- **🔐 Secure Key Management**: Integration with multiple vault providers
  - Supabase Vault (self-hosted and cloud)
  - HashiCorp Vault
  - AWS Secrets Manager
  - Azure Key Vault
  - Local file-based secrets for development

- **🤖 Real LLM Testing**: Test against actual language model providers
  - OpenAI GPT-4/GPT-5 models
  - Anthropic Claude models
  - Mock providers for offline testing

- **📊 Comprehensive Evaluation**: Multi-dimensional test evaluation
  - Accuracy assessment with fuzzy matching
  - Performance benchmarking (latency, throughput)
  - Cost tracking and budget management
  - Security audit capabilities

- **📈 Advanced Reporting**: Rich test reports with coverage statistics
  - Markdown reports with charts and metrics
  - JSON reports for programmatic access
  - Real-time coverage analysis with cargo-tarpaulin
  - GitHub Actions integration

## 🏗️ Architecture

The framework follows a modular, secure-by-design architecture:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Test Runner   │    │   Evaluators    │    │   Reporters     │
│                 │    │                 │    │                 │
│ • Framework     │    │ • Accuracy      │    │ • Markdown      │
│ • Orchestrator  │    │ • Performance   │    │ • JSON          │
│ • Parallel Exec │    │ • Cost          │    │ • Coverage      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
┌─────────────────────────────────┼─────────────────────────────────┐
│                    Core Framework                                  │
├─────────────────┬─────────────────┬─────────────────┬─────────────┤
│  Vault Providers │  LLM Providers  │   Test Cases    │  Security   │
│                 │                 │                 │             │
│ • Supabase      │ • OpenAI        │ • Unit Tests    │ • API Keys  │
│ • HashiCorp     │ • Anthropic     │ • Integration   │ • TLS/HTTPS │
│ • AWS Secrets   │ • Mock Provider │ • Performance   │ • Audit Log │
│ • Azure KV      │ • Custom        │ • Edge Cases    │ • RLS       │
│ • Local Files   │                 │                 │             │
└─────────────────┴─────────────────┴─────────────────┴─────────────┘
```

## 🔧 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rlm-test-utils = { path = "../rlm-test-utils", features = ["all-providers"] }

[dev-dependencies]
tokio-test = "0.4"
serial_test = "3.0"
```

### Feature Flags

- `default` - Local vault provider only
- `supabase-vault` - Supabase vault integration
- `hashicorp-vault` - HashiCorp Vault support
- `aws-secrets` - AWS Secrets Manager integration
- `azure-keyvault` - Azure Key Vault support
- `all-providers` - Enable all vault providers

## 🚀 Quick Start

### 1. Basic Setup

```rust
use rlm_test_utils::{
    TestFramework, LlmTestFramework, VaultFactory,
    AccuracyEvaluator, PerformanceEvaluator,
    MarkdownReportGenerator,
};

#[tokio::test]
async fn test_llm_integration() {
    // Initialize secure vault
    let vault = VaultFactory::from_env().await.unwrap();

    // Create test framework
    let framework = TestFramework::new(vault.clone(), Default::default());

    // Run your tests...
}
```

### 2. Supabase Vault Setup

```bash
# Environment variables
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_ROLE_KEY="your-service-role-key"
export SUPABASE_SECRETS_TABLE="vault_secrets"  # optional
```

```rust
use rlm_test_utils::SupabaseVaultProvider;

#[tokio::test]
async fn test_with_supabase_vault() {
    let vault = SupabaseVaultProvider::from_env().await.unwrap();

    // Initialize the secrets table
    vault.init_table().await.unwrap();

    // Store test secrets
    vault.store_secret("openai_api_key", "sk-proj-your-key").await.unwrap();

    // Use in tests...
}
```

### 3. Real LLM Testing

```rust
use rlm_test_utils::{
    OpenAiTestProvider, LlmTestProviderConfig,
    AccuracyEvaluator, PerformanceEvaluator, CostEvaluator,
};

#[tokio::test]
async fn test_openai_integration() {
    let vault = VaultFactory::from_env().await.unwrap();

    let config = LlmTestProviderConfig {
        model: "gpt-4".to_string(),
        api_key_vault_key: "openai_api_key".to_string(),
        max_tokens: Some(500),
        temperature: Some(0.1),
        timeout: Some(Duration::from_secs(30)),
        max_retries: 3,
        rate_limit_rps: 1.0,
    };

    let provider = OpenAiTestProvider::new(config, vault).await.unwrap();
    let framework = LlmTestFramework::new(Arc::new(provider), vault);

    // Execute tests with real LLM
    let request = RlmRequest {
        id: Uuid::new_v4(),
        context: "You are a helpful assistant.".to_string(),
        query: "What is 2+2?".to_string(),
        config: None,
    };

    let response = framework.execute_test(&request).await.unwrap();

    // Evaluate response
    let evaluator = AccuracyEvaluator::exact(0.9, vec!["4".to_string()]);
    let result = evaluator.evaluate_response(&request, &response).await.unwrap();

    assert!(result.passed);
}
```

## 📊 Coverage Analysis

The framework includes automated coverage analysis with markdown reporting:

```rust
use rlm_test_utils::{CoverageAnalyzer, CoverageConfig, MarkdownReportGenerator};

#[tokio::test]
async fn test_with_coverage() {
    let config = CoverageConfig {
        target_directory: std::env::current_dir().unwrap(),
        include_patterns: vec!["src/**/*.rs".to_string()],
        exclude_patterns: vec!["tests/**/*.rs".to_string()],
        minimum_coverage: 85.0,
        output_format: "json".to_string(),
        timeout: Duration::from_secs(300),
    };

    let analyzer = CoverageAnalyzer::new(config);
    let coverage_report = analyzer.analyze().await.unwrap();

    println!("Coverage: {:.1}%", coverage_report.total_coverage);

    // Generate markdown report
    let report_builder = ReportBuilder::new()
        .with_coverage(coverage_report)
        .build()
        .unwrap();

    let generator = MarkdownReportGenerator::new();
    let markdown = generator.generate_report(&report_builder).await.unwrap();

    tokio::fs::write("coverage-report.md", markdown).await.unwrap();
}
```

## 🔒 Security Features

### Secure Key Management

The framework ensures API keys are never exposed:

- ✅ Keys stored in secure vaults (encrypted at rest)
- ✅ Keys transmitted over TLS (encrypted in transit)
- ✅ No keys in logs or error messages
- ✅ Key rotation support
- ✅ Row-level security for database vaults
- ✅ Audit logging for key access

### Security Audit

```rust
use rlm_test_utils::{SecurityAuditReport, VaultSecurityReport};

let security_report = SecurityAuditReport {
    vault_security: VaultSecurityReport {
        provider: "Supabase".to_string(),
        encryption_in_transit: true,
        encryption_at_rest: true,
        access_logging: true,
        row_level_security: Some(true),
        security_score: 0.95,
    },
    // ... other security assessments
};
```

## 🚀 GitHub Actions Integration

The framework includes a comprehensive GitHub Actions workflow:

```yaml
name: Integration Tests
on: [push, pull_request, schedule]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features: [default, all-providers, supabase-vault]
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        env:
          SUPABASE_URL: ${{ secrets.SUPABASE_URL }}
          SUPABASE_SERVICE_ROLE_KEY: ${{ secrets.SUPABASE_SERVICE_ROLE_KEY }}
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          cargo test --features ${{ matrix.features }}
```

## 📈 Cost Management

Track and control LLM API costs:

```rust
use rlm_test_utils::CostEvaluator;

let cost_evaluator = CostEvaluator::openai_gpt4()
    .with_daily_budget(50.0)  // $50 daily limit
    .with_max_cost_per_test(0.10);  // $0.10 per test limit

let cost_result = cost_evaluator.evaluate_response(&request, &response).await?;

println!("Cost: ${:.4}", cost_result.metrics.get("total_cost").unwrap());
println!("Budget remaining: ${:.2}",
         cost_evaluator.daily_budget_limit - cost_evaluator.current_daily_spending);
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `RLM_VAULT_PROVIDER` | Vault provider type | `supabase`, `local`, `hashicorp`, `aws`, `azure` |
| `SUPABASE_URL` | Supabase project URL | `https://abc123.supabase.co` |
| `SUPABASE_SERVICE_ROLE_KEY` | Supabase service role key | `eyJ0...` |
| `SUPABASE_SECRETS_TABLE` | Custom secrets table name | `vault_secrets` |
| `OPENAI_API_KEY` | OpenAI API key | `sk-proj-...` |
| `ANTHROPIC_API_KEY` | Anthropic API key | `sk-ant-...` |

### Local Development

For local development, create a secrets file:

```json
{
  "openai_api_key": "sk-proj-your-key-here",
  "anthropic_api_key": "sk-ant-your-key-here",
  "test_api_key": "test-value"
}
```

Set the path:
```bash
export RLM_LOCAL_SECRETS_FILE="~/.rlm/secrets/test-secrets.json"
```

## 📊 Example Reports

### Markdown Report Sample

```markdown
# RLM Integration Test Report

**Generated:** 2024-01-20 14:30:00 UTC
**Total Tests:** 15
**Passed:** 14 (93.3%)

## Coverage
🟢 **Overall Coverage:** 87.5% (1,234/1,410 lines)

## Performance
- **Average Latency:** 2.3s
- **P95 Latency:** 4.1s
- **Throughput:** 23.5 tokens/sec

## Cost Analysis
- **Total Cost:** $0.45
- **Budget Utilization:** 18.0% ($4.50/$25.00)

## Security Audit
🔐 **Overall Security Score:** 9.2/10
- ✅ Vault Security: 9.5/10
- ✅ API Key Security: 9.0/10
- ✅ Network Security: 9.1/10
```

### JSON Report Structure

```json
{
  "metadata": {
    "report_id": "550e8400-e29b-41d4-a716-446655440000",
    "generated_at": "2024-01-20T14:30:00Z",
    "commit_hash": "abc123",
    "version": "0.1.0"
  },
  "summary": {
    "total_tests": 15,
    "passed_tests": 14,
    "pass_rate": 0.933
  },
  "coverage": {
    "total_coverage": 87.5,
    "lines_covered": 1234,
    "lines_total": 1410
  }
}
```

## 🏃 Running Tests

### Unit Tests
```bash
cargo test --lib --features all-providers
```

### Integration Tests
```bash
cargo test --test integration_test --features all-providers
```

### With Coverage
```bash
cargo tarpaulin --out Html --out Json --features all-providers
```

### Comprehensive Example
```bash
cargo run --example comprehensive_integration_test --features all-providers
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass with coverage ≥85%
5. Update documentation
6. Submit a pull request

### Development Setup

```bash
# Install required tools
cargo install cargo-tarpaulin

# Run full test suite
cargo test --workspace --features all-providers

# Generate coverage report
cargo tarpaulin --workspace --features all-providers --out Html
```

## 📚 Examples

See the [`examples/`](./examples/) directory for comprehensive usage examples:

- [`comprehensive_integration_test.rs`](./examples/comprehensive_integration_test.rs) - Complete test suite demonstration
- Integration with different vault providers
- Real LLM testing with cost tracking
- Performance benchmarking
- Security auditing

## 🔗 Related Crates

- [`rlm-core`](../rlm-core/) - Core RLM types and executor
- [`rlm-server`](../rlm-server/) - HTTP server with SSE streaming
- [`rlm-repl-rhai`](../rlm-repl-rhai/) - Rhai REPL backend

## 📄 License

This project is licensed under the MIT OR Apache-2.0 license.

## 🆘 Support

- 📖 [Documentation](https://docs.rs/rlm-test-utils)
- 🐛 [Issue Tracker](https://github.com/prometheus-labs/rlm/issues)
- 💬 [Discussions](https://github.com/prometheus-labs/rlm/discussions)

---

**⚡ Built for the RLM project - enabling 100% integration test coverage with real LLM providers and secure key management.**