//! RLM Integration Testing Utilities
//!
//! This crate provides comprehensive testing utilities for the RLM project,
//! including secure key vault integration, LLM testing frameworks, and
//! coverage reporting tools.
//!
//! # Features
//!
//! - **Secure Key Management**: Integration with HashiCorp Vault, AWS Secrets Manager, Azure Key Vault
//! - **Real LLM Testing**: Test harness for OpenAI GPT-5 and other providers
//! - **Coverage Reporting**: Markdown report generation with timestamps
//! - **Local Development**: File-based secrets for development workflows
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use rlm_test_utils::{TestFramework, KeyVaultProvider};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let vault = KeyVaultProvider::from_env().await?;
//!     let framework = TestFramework::new(vault).await?;
//!
//!     let result = framework.run_integration_tests().await?;
//!     println!("Coverage: {:.1}%", result.coverage_percentage);
//!
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

pub mod error;
pub mod framework;
pub mod coverage;
pub mod providers;
pub mod vault;
pub mod evaluators;
pub mod reporting;

// Re-exports
pub use error::{TestError, TestResult};
pub use framework::{TestFramework, LlmTestFramework, ResponseEvaluator};
pub use coverage::{CoverageReport, CoverageMetrics, MarkdownReporter};
pub use providers::{OpenAiTestProvider, AnthropicTestProvider, MockLlmProvider};
pub use vault::{
    KeyVaultProvider, VaultConfig,
    LocalVaultProvider,
};
pub use error::VaultError;
pub use evaluators::{
    AccuracyEvaluator, PerformanceEvaluator, CostEvaluator,
};
pub use framework::{
    EvaluationResult, PerformanceMetrics,
};
pub use reporting::{
    TestExecutionReport, TestReportGenerator, MarkdownReportGenerator, JsonReportGenerator,
    ReportBuilder, TestCaseReport, TestStatus, SecurityAuditReport,
};

#[cfg(feature = "aws-secrets")]
pub use vault::AwsSecretsProvider;

#[cfg(feature = "azure-keyvault")]
pub use vault::AzureKeyVaultProvider;

#[cfg(feature = "supabase-vault")]
pub use vault::SupabaseVaultProvider;