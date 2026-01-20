//! Comprehensive test reporting with markdown generation and coverage statistics.
//!
//! This module provides rich reporting capabilities for RLM integration tests,
//! including coverage analysis, performance metrics, cost tracking, and security audit results.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::error::{TestError, TestResult};
use crate::coverage::CoverageReport;
use crate::framework::{EvaluationResult, PerformanceMetrics};

/// Comprehensive test report containing all test execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionReport {
    /// Report metadata
    pub metadata: ReportMetadata,
    /// Test execution summary
    pub summary: TestSummary,
    /// Coverage analysis results
    pub coverage: Option<CoverageReport>,
    /// Individual test case results
    pub test_cases: Vec<TestCaseReport>,
    /// Performance benchmarks
    pub performance: PerformanceReport,
    /// Cost analysis
    pub cost_analysis: CostReport,
    /// Security audit results
    pub security_audit: SecurityAuditReport,
    /// Environment and configuration details
    pub environment: EnvironmentReport,
}

/// Report metadata and generation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Unique report identifier
    pub report_id: Uuid,
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
    /// Git commit hash (if available)
    pub commit_hash: Option<String>,
    /// Git branch name (if available)
    pub branch_name: Option<String>,
    /// Report format version
    pub version: String,
    /// Total execution time for all tests
    pub total_execution_time: std::time::Duration,
}

/// High-level test execution summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    /// Total number of test cases executed
    pub total_tests: usize,
    /// Number of passed tests
    pub passed_tests: usize,
    /// Number of failed tests
    pub failed_tests: usize,
    /// Number of skipped tests
    pub skipped_tests: usize,
    /// Overall pass rate (0.0 to 1.0)
    pub pass_rate: f64,
    /// Average test execution time
    pub average_execution_time: std::time::Duration,
    /// Test categories and their results
    pub categories: HashMap<String, CategorySummary>,
}

/// Summary for a specific test category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    /// Category name (e.g., "accuracy", "performance", "security")
    pub name: String,
    /// Number of tests in this category
    pub test_count: usize,
    /// Number of passed tests in this category
    pub passed: usize,
    /// Average score for this category (0.0 to 1.0)
    pub average_score: f64,
}

/// Individual test case execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseReport {
    /// Test case identifier
    pub test_id: Uuid,
    /// Test case name
    pub name: String,
    /// Test category (accuracy, performance, cost, etc.)
    pub category: String,
    /// Execution status
    pub status: TestStatus,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution duration
    pub duration: std::time::Duration,
    /// Evaluation results
    pub evaluations: Vec<EvaluationResult>,
    /// Performance metrics
    pub performance: Option<PerformanceMetrics>,
    /// Error details (if failed)
    pub error_details: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Test execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    /// Test passed successfully
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test execution was interrupted
    Interrupted,
}

/// Performance benchmarking report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Overall performance metrics
    pub overall: PerformanceMetrics,
    /// Performance by test category
    pub by_category: HashMap<String, PerformanceMetrics>,
    /// Percentile analysis
    pub percentiles: PerformancePercentiles,
    /// Comparison to baseline (if available)
    pub baseline_comparison: Option<BaselineComparison>,
}

/// Performance percentile analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePercentiles {
    /// 50th percentile (median) latency
    pub p50_latency_ms: f64,
    /// 95th percentile latency
    pub p95_latency_ms: f64,
    /// 99th percentile latency
    pub p99_latency_ms: f64,
    /// 50th percentile tokens per second
    pub p50_tokens_per_second: f64,
    /// 95th percentile tokens per second
    pub p95_tokens_per_second: f64,
}

/// Comparison against baseline performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineComparison {
    /// Baseline report timestamp
    pub baseline_date: DateTime<Utc>,
    /// Latency change percentage
    pub latency_change_percent: f64,
    /// Throughput change percentage
    pub throughput_change_percent: f64,
    /// Cost change percentage
    pub cost_change_percent: f64,
    /// Overall performance trend
    pub trend: PerformanceTrend,
}

/// Performance trend indication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    /// Performance improved
    Improved,
    /// Performance remained stable
    Stable,
    /// Performance degraded
    Degraded,
}

/// Cost analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    /// Total cost for all tests
    pub total_cost: f64,
    /// Average cost per test
    pub average_cost_per_test: f64,
    /// Cost breakdown by provider
    pub by_provider: HashMap<String, f64>,
    /// Cost breakdown by test category
    pub by_category: HashMap<String, f64>,
    /// Budget utilization
    pub budget_utilization: BudgetUtilization,
    /// Cost efficiency metrics
    pub efficiency: CostEfficiencyMetrics,
}

/// Budget utilization tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetUtilization {
    /// Daily budget limit
    pub daily_budget: f64,
    /// Amount used today
    pub used_today: f64,
    /// Remaining budget
    pub remaining_budget: f64,
    /// Utilization percentage
    pub utilization_percent: f64,
}

/// Cost efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEfficiencyMetrics {
    /// Cost per successful test
    pub cost_per_success: f64,
    /// Cost per token processed
    pub cost_per_token: f64,
    /// Cost per accuracy point achieved
    pub cost_per_accuracy_point: f64,
    /// Most expensive test category
    pub most_expensive_category: String,
    /// Most efficient test category
    pub most_efficient_category: String,
}

/// Security audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditReport {
    /// Vault security assessment
    pub vault_security: VaultSecurityReport,
    /// API key security assessment
    pub api_key_security: ApiKeySecurityReport,
    /// Network security assessment
    pub network_security: NetworkSecurityReport,
    /// Overall security score (0.0 to 1.0)
    pub overall_security_score: f64,
    /// Security recommendations
    pub recommendations: Vec<SecurityRecommendation>,
}

/// Vault security assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSecurityReport {
    /// Vault provider type
    pub provider: String,
    /// Encryption in transit
    pub encryption_in_transit: bool,
    /// Encryption at rest
    pub encryption_at_rest: bool,
    /// Access logging enabled
    pub access_logging: bool,
    /// Row-level security (for database vaults)
    pub row_level_security: Option<bool>,
    /// Security score for vault (0.0 to 1.0)
    pub security_score: f64,
}

/// API key security assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeySecurityReport {
    /// Keys stored securely (not in plain text)
    pub secure_storage: bool,
    /// Key rotation supported
    pub rotation_supported: bool,
    /// Keys not exposed in logs
    pub no_log_exposure: bool,
    /// Minimum key length requirements met
    pub minimum_length_met: bool,
    /// Security score for API keys (0.0 to 1.0)
    pub security_score: f64,
}

/// Network security assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityReport {
    /// HTTPS/TLS used for all connections
    pub tls_enforced: bool,
    /// Certificate validation enabled
    pub certificate_validation: bool,
    /// Rate limiting configured
    pub rate_limiting: bool,
    /// Request timeout configured
    pub timeout_configured: bool,
    /// Security score for network (0.0 to 1.0)
    pub security_score: f64,
}

/// Security recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRecommendation {
    /// Recommendation category
    pub category: String,
    /// Severity level
    pub severity: SecuritySeverity,
    /// Recommendation title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Remediation steps
    pub remediation: Vec<String>,
}

/// Security recommendation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Environment and configuration report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentReport {
    /// Rust version used
    pub rust_version: String,
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Vault provider configuration
    pub vault_config: HashMap<String, serde_json::Value>,
    /// LLM provider configurations
    pub llm_configs: HashMap<String, serde_json::Value>,
    /// Test framework version
    pub framework_version: String,
    /// Feature flags enabled
    pub features_enabled: Vec<String>,
}

/// Trait for generating test reports in different formats
#[async_trait]
pub trait TestReportGenerator: Send + Sync {
    /// Generate a report from test execution data
    async fn generate_report(&self, report: &TestExecutionReport) -> TestResult<String>;

    /// Save the report to a file
    async fn save_report(&self, report: &TestExecutionReport, path: &Path) -> TestResult<()> {
        let content = self.generate_report(report).await?;
        fs::write(path, content).await
            .map_err(|e| TestError::execution(format!("Failed to save report to {}: {}", path.display(), e)))?;
        Ok(())
    }
}

/// Markdown test report generator
pub struct MarkdownReportGenerator {
    /// Include detailed test case information
    pub include_details: bool,
    /// Include performance charts (ASCII art)
    pub include_charts: bool,
    /// Include security audit section
    pub include_security: bool,
    /// Custom CSS for HTML rendering
    pub custom_css: Option<String>,
}

impl MarkdownReportGenerator {
    /// Create a new markdown report generator with default settings
    pub fn new() -> Self {
        Self {
            include_details: true,
            include_charts: true,
            include_security: true,
            custom_css: None,
        }
    }

    /// Create a minimal markdown report generator
    pub fn minimal() -> Self {
        Self {
            include_details: false,
            include_charts: false,
            include_security: false,
            custom_css: None,
        }
    }

    /// Generate ASCII art chart for performance data
    fn generate_performance_chart(&self, data: &[f64], title: &str) -> String {
        if data.is_empty() {
            return format!("No data available for {}", title);
        }

        let max_value = data.iter().fold(0.0f64, |acc, &x| acc.max(x));
        let min_value = data.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));
        let range = max_value - min_value;

        if range == 0.0 {
            return format!("{}: All values equal ({:.2})", title, max_value);
        }

        let mut chart = format!("\n```\n{}\n", title);
        let _chart_width = 60;
        let chart_height = 10;

        for i in (0..chart_height).rev() {
            let threshold = min_value + (i as f64 / chart_height as f64) * range;
            chart.push_str(&format!("{:6.2} |", threshold));

            for &value in data {
                if value >= threshold {
                    chart.push('█');
                } else {
                    chart.push(' ');
                }
            }
            chart.push('\n');
        }

        chart.push_str(&format!("{:7}+", " "));
        for _ in 0..data.len() {
            chart.push('-');
        }
        chart.push_str("\n```\n");

        chart
    }
}

#[async_trait]
impl TestReportGenerator for MarkdownReportGenerator {
    #[instrument(skip(self, report))]
    async fn generate_report(&self, report: &TestExecutionReport) -> TestResult<String> {
        let mut markdown = String::new();

        // Header and metadata
        markdown.push_str(&format!(
            "# RLM Integration Test Report\n\n\
             **Generated:** {}\n\
             **Report ID:** `{}`\n\
             **Total Execution Time:** {:.2}s\n",
            report.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC"),
            report.metadata.report_id,
            report.metadata.total_execution_time.as_secs_f64()
        ));

        if let Some(commit) = &report.metadata.commit_hash {
            markdown.push_str(&format!("**Commit:** `{}`\n", commit));
        }

        if let Some(branch) = &report.metadata.branch_name {
            markdown.push_str(&format!("**Branch:** `{}`\n", branch));
        }

        markdown.push_str("\n");

        // Executive Summary
        markdown.push_str("## Executive Summary\n\n");

        let status_emoji = if report.summary.failed_tests == 0 { "✅" } else { "❌" };
        markdown.push_str(&format!(
            "{} **Overall Status:** {}/{} tests passed ({:.1}% success rate)\n\n",
            status_emoji,
            report.summary.passed_tests,
            report.summary.total_tests,
            report.summary.pass_rate * 100.0
        ));

        // Test Summary Table
        markdown.push_str("### Test Results Summary\n\n");
        markdown.push_str("| Metric | Value |\n|--------|-------|\n");
        markdown.push_str(&format!("| Total Tests | {} |\n", report.summary.total_tests));
        markdown.push_str(&format!("| Passed | {} |\n", report.summary.passed_tests));
        markdown.push_str(&format!("| Failed | {} |\n", report.summary.failed_tests));
        markdown.push_str(&format!("| Skipped | {} |\n", report.summary.skipped_tests));
        markdown.push_str(&format!("| Success Rate | {:.1}% |\n", report.summary.pass_rate * 100.0));
        markdown.push_str(&format!("| Avg Execution Time | {:.2}s |\n\n", report.summary.average_execution_time.as_secs_f64()));

        // Coverage Report
        if let Some(coverage) = &report.coverage {
            markdown.push_str("## Code Coverage\n\n");

            let coverage_emoji = if coverage.overall.line_coverage >= 90.0 { "🟢" }
                               else if coverage.overall.line_coverage >= 80.0 { "🟡" }
                               else { "🔴" };

            markdown.push_str(&format!(
                "{} **Overall Coverage:** {:.1}% ({}/{} lines)\n\n",
                coverage_emoji,
                coverage.overall.line_coverage,
                coverage.overall.covered_lines,
                coverage.overall.total_lines
            ));

            markdown.push_str("### Coverage by Crate\n\n");
            markdown.push_str("| Crate | Coverage | Lines Covered | Total Lines |\n");
            markdown.push_str("|-------|----------|---------------|-------------|\n");

            for (crate_name, metrics) in &coverage.by_package {
                markdown.push_str(&format!(
                    "| {} | {:.1}% | {} | {} |\n",
                    crate_name,
                    metrics.line_coverage,
                    metrics.covered_lines,
                    metrics.total_lines
                ));
            }
            markdown.push_str("\n");
        }

        // Performance Report
        markdown.push_str("## Performance Metrics\n\n");
        markdown.push_str(&format!(
            "- **Average Latency:** {:.2}s\n\
             - **Average Throughput:** {:.1} tokens/sec\n\
             - **P95 Latency:** {:.2}s\n\
             - **P99 Latency:** {:.2}s\n\n",
            report.performance.overall.avg_response_time_ms / 1000.0,
            report.performance.overall.requests_per_second,
            report.performance.percentiles.p95_latency_ms / 1000.0,
            report.performance.percentiles.p99_latency_ms / 1000.0
        ));

        // Cost Analysis
        markdown.push_str("## Cost Analysis\n\n");
        markdown.push_str(&format!(
            "- **Total Cost:** ${:.4}\n\
             - **Average Cost per Test:** ${:.4}\n\
             - **Budget Utilization:** {:.1}% (${:.2}/${:.2})\n\
             - **Cost per Token:** ${:.6}\n\n",
            report.cost_analysis.total_cost,
            report.cost_analysis.average_cost_per_test,
            report.cost_analysis.budget_utilization.utilization_percent,
            report.cost_analysis.budget_utilization.used_today,
            report.cost_analysis.budget_utilization.daily_budget,
            report.cost_analysis.efficiency.cost_per_token
        ));

        // Cost by Provider
        if !report.cost_analysis.by_provider.is_empty() {
            markdown.push_str("### Cost by Provider\n\n");
            markdown.push_str("| Provider | Cost | Percentage |\n");
            markdown.push_str("|----------|------|-----------|\n");

            for (provider, cost) in &report.cost_analysis.by_provider {
                let percentage = (cost / report.cost_analysis.total_cost) * 100.0;
                markdown.push_str(&format!(
                    "| {} | ${:.4} | {:.1}% |\n",
                    provider, cost, percentage
                ));
            }
            markdown.push_str("\n");
        }

        // Security Audit
        if self.include_security {
            markdown.push_str("## Security Audit\n\n");

            let security_emoji = if report.security_audit.overall_security_score >= 0.9 { "🔐" }
                                else if report.security_audit.overall_security_score >= 0.7 { "🔒" }
                                else { "⚠️" };

            markdown.push_str(&format!(
                "{} **Overall Security Score:** {:.1}/10\n\n",
                security_emoji,
                report.security_audit.overall_security_score * 10.0
            ));

            markdown.push_str("### Security Features\n\n");
            markdown.push_str("| Component | Status | Score |\n");
            markdown.push_str("|-----------|--------|---------|\n");
            markdown.push_str(&format!(
                "| Vault Security | {} | {:.1}/10 |\n",
                if report.security_audit.vault_security.security_score >= 0.8 { "✅" } else { "❌" },
                report.security_audit.vault_security.security_score * 10.0
            ));
            markdown.push_str(&format!(
                "| API Key Security | {} | {:.1}/10 |\n",
                if report.security_audit.api_key_security.security_score >= 0.8 { "✅" } else { "❌" },
                report.security_audit.api_key_security.security_score * 10.0
            ));
            markdown.push_str(&format!(
                "| Network Security | {} | {:.1}/10 |\n\n",
                if report.security_audit.network_security.security_score >= 0.8 { "✅" } else { "❌" },
                report.security_audit.network_security.security_score * 10.0
            ));

            // Security Recommendations
            if !report.security_audit.recommendations.is_empty() {
                markdown.push_str("### Security Recommendations\n\n");

                for (i, rec) in report.security_audit.recommendations.iter().enumerate() {
                    let severity_emoji = match rec.severity {
                        SecuritySeverity::Critical => "🚨",
                        SecuritySeverity::High => "⚠️",
                        SecuritySeverity::Medium => "🔶",
                        SecuritySeverity::Low => "ℹ️",
                    };

                    markdown.push_str(&format!(
                        "#### {} {} - {} ({:?})\n\n{}\n\n**Remediation:**\n",
                        i + 1, severity_emoji, rec.title, rec.severity, rec.description
                    ));

                    for step in &rec.remediation {
                        markdown.push_str(&format!("- {}\n", step));
                    }
                    markdown.push_str("\n");
                }
            }
        }

        // Environment Information
        markdown.push_str("## Environment\n\n");
        markdown.push_str(&format!(
            "- **Framework Version:** {}\n\
             - **Rust Version:** {}\n\
             - **Platform:** {} ({})\n\
             - **Features Enabled:** {}\n\n",
            report.environment.framework_version,
            report.environment.rust_version,
            report.environment.os,
            report.environment.arch,
            report.environment.features_enabled.join(", ")
        ));

        // Detailed Test Results (if enabled)
        if self.include_details && !report.test_cases.is_empty() {
            markdown.push_str("## Detailed Test Results\n\n");

            for test_case in &report.test_cases {
                let status_emoji = match test_case.status {
                    TestStatus::Passed => "✅",
                    TestStatus::Failed => "❌",
                    TestStatus::Skipped => "⏭️",
                    TestStatus::Interrupted => "🔄",
                };

                markdown.push_str(&format!(
                    "### {} {} ({})\n\n",
                    status_emoji, test_case.name, test_case.category
                ));

                markdown.push_str(&format!(
                    "- **Duration:** {:.2}s\n\
                     - **Started:** {}\n",
                    test_case.duration.as_secs_f64(),
                    test_case.started_at.format("%H:%M:%S UTC")
                ));

                if let Some(performance) = &test_case.performance {
                    markdown.push_str(&format!(
                        "- **Tokens:** {} (input: {}, output: {})\n\
                         - **Cost:** ${:.4}\n\
                         - **Throughput:** {:.1} tokens/sec\n",
                        performance.total_tokens,
                        performance.total_tokens / 2, // Estimate input tokens
                        performance.total_tokens / 2, // Estimate output tokens
                        performance.total_cost_usd,
                        performance.requests_per_second
                    ));
                }

                if !test_case.evaluations.is_empty() {
                    markdown.push_str("\n**Evaluations:**\n");
                    for eval in &test_case.evaluations {
                        markdown.push_str(&format!(
                            "- **Score:** {:.1}% - {}\n",
                            eval.quality_score * 100.0,
                            eval.details
                        ));
                    }
                }

                if let Some(error) = &test_case.error_details {
                    markdown.push_str(&format!("\n**Error:** `{}`\n", error));
                }

                markdown.push_str("\n");
            }
        }

        // Footer
        markdown.push_str("---\n");
        markdown.push_str(&format!(
            "*Generated by RLM Integration Testing Framework v{} on {}*\n",
            report.environment.framework_version,
            report.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        debug!("Generated markdown report with {} characters", markdown.len());
        Ok(markdown)
    }
}

/// JSON report generator for programmatic consumption
pub struct JsonReportGenerator {
    /// Pretty print the JSON output
    pub pretty_print: bool,
}

impl JsonReportGenerator {
    /// Create a new JSON report generator
    pub fn new(pretty_print: bool) -> Self {
        Self { pretty_print }
    }
}

#[async_trait]
impl TestReportGenerator for JsonReportGenerator {
    async fn generate_report(&self, report: &TestExecutionReport) -> TestResult<String> {
        if self.pretty_print {
            serde_json::to_string_pretty(report)
        } else {
            serde_json::to_string(report)
        }
        .map_err(TestError::from)
    }
}

/// Report builder for constructing test execution reports
pub struct ReportBuilder {
    metadata: ReportMetadata,
    summary: TestSummary,
    test_cases: Vec<TestCaseReport>,
    performance: Option<PerformanceReport>,
    cost_analysis: Option<CostReport>,
    security_audit: Option<SecurityAuditReport>,
    environment: Option<EnvironmentReport>,
    coverage: Option<CoverageReport>,
}

impl ReportBuilder {
    /// Create a new report builder
    pub fn new() -> Self {
        Self {
            metadata: ReportMetadata {
                report_id: Uuid::new_v4(),
                generated_at: Utc::now(),
                commit_hash: None,
                branch_name: None,
                version: "0.1.0".to_string(),
                total_execution_time: std::time::Duration::from_secs(0),
            },
            summary: TestSummary {
                total_tests: 0,
                passed_tests: 0,
                failed_tests: 0,
                skipped_tests: 0,
                pass_rate: 0.0,
                average_execution_time: std::time::Duration::from_secs(0),
                categories: HashMap::new(),
            },
            test_cases: Vec::new(),
            performance: None,
            cost_analysis: None,
            security_audit: None,
            environment: None,
            coverage: None,
        }
    }

    /// Set git commit information
    pub fn with_git_info(mut self, commit_hash: Option<String>, branch_name: Option<String>) -> Self {
        self.metadata.commit_hash = commit_hash;
        self.metadata.branch_name = branch_name;
        self
    }

    /// Add test case results
    pub fn add_test_case(mut self, test_case: TestCaseReport) -> Self {
        self.test_cases.push(test_case);
        self
    }

    /// Set coverage report
    pub fn with_coverage(mut self, coverage: CoverageReport) -> Self {
        self.coverage = Some(coverage);
        self
    }

    /// Set performance report
    pub fn with_performance(mut self, performance: PerformanceReport) -> Self {
        self.performance = Some(performance);
        self
    }

    /// Set cost analysis
    pub fn with_cost_analysis(mut self, cost_analysis: CostReport) -> Self {
        self.cost_analysis = Some(cost_analysis);
        self
    }

    /// Set security audit
    pub fn with_security_audit(mut self, security_audit: SecurityAuditReport) -> Self {
        self.security_audit = Some(security_audit);
        self
    }

    /// Set environment information
    pub fn with_environment(mut self, environment: EnvironmentReport) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Build the final test execution report
    pub fn build(mut self) -> TestResult<TestExecutionReport> {
        // Calculate summary statistics
        self.summary.total_tests = self.test_cases.len();
        self.summary.passed_tests = self.test_cases.iter().filter(|tc| matches!(tc.status, TestStatus::Passed)).count();
        self.summary.failed_tests = self.test_cases.iter().filter(|tc| matches!(tc.status, TestStatus::Failed)).count();
        self.summary.skipped_tests = self.test_cases.iter().filter(|tc| matches!(tc.status, TestStatus::Skipped)).count();

        if self.summary.total_tests > 0 {
            self.summary.pass_rate = self.summary.passed_tests as f64 / self.summary.total_tests as f64;
        }

        let total_duration: std::time::Duration = self.test_cases.iter()
            .map(|tc| tc.duration)
            .sum();

        if !self.test_cases.is_empty() {
            self.summary.average_execution_time = total_duration / self.test_cases.len() as u32;
        }

        self.metadata.total_execution_time = total_duration;

        // Create default reports if not provided
        let performance = self.performance.unwrap_or_else(|| PerformanceReport {
            overall: PerformanceMetrics {
                avg_response_time_ms: 0.0,
                p95_response_time_ms: 0.0,
                total_tokens: 0,
                total_cost_usd: 0.0,
                requests_per_second: 0.0,
                error_rate: 0.0,
            },
            by_category: HashMap::new(),
            percentiles: PerformancePercentiles {
                p50_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                p50_tokens_per_second: 0.0,
                p95_tokens_per_second: 0.0,
            },
            baseline_comparison: None,
        });

        let cost_analysis = self.cost_analysis.unwrap_or_else(|| CostReport {
            total_cost: 0.0,
            average_cost_per_test: 0.0,
            by_provider: HashMap::new(),
            by_category: HashMap::new(),
            budget_utilization: BudgetUtilization {
                daily_budget: 0.0,
                used_today: 0.0,
                remaining_budget: 0.0,
                utilization_percent: 0.0,
            },
            efficiency: CostEfficiencyMetrics {
                cost_per_success: 0.0,
                cost_per_token: 0.0,
                cost_per_accuracy_point: 0.0,
                most_expensive_category: "none".to_string(),
                most_efficient_category: "none".to_string(),
            },
        });

        let security_audit = self.security_audit.unwrap_or_else(|| SecurityAuditReport {
            vault_security: VaultSecurityReport {
                provider: "unknown".to_string(),
                encryption_in_transit: false,
                encryption_at_rest: false,
                access_logging: false,
                row_level_security: None,
                security_score: 0.0,
            },
            api_key_security: ApiKeySecurityReport {
                secure_storage: false,
                rotation_supported: false,
                no_log_exposure: false,
                minimum_length_met: false,
                security_score: 0.0,
            },
            network_security: NetworkSecurityReport {
                tls_enforced: false,
                certificate_validation: false,
                rate_limiting: false,
                timeout_configured: false,
                security_score: 0.0,
            },
            overall_security_score: 0.0,
            recommendations: Vec::new(),
        });

        let environment = self.environment.unwrap_or_else(|| EnvironmentReport {
            rust_version: "unknown".to_string(),
            os: "unknown".to_string(),
            arch: "unknown".to_string(),
            vault_config: HashMap::new(),
            llm_configs: HashMap::new(),
            framework_version: "0.1.0".to_string(),
            features_enabled: Vec::new(),
        });

        Ok(TestExecutionReport {
            metadata: self.metadata,
            summary: self.summary,
            coverage: self.coverage,
            test_cases: self.test_cases,
            performance,
            cost_analysis,
            security_audit,
            environment,
        })
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_markdown_report_generation() {
        let report = ReportBuilder::new()
            .with_git_info(Some("abc123".to_string()), Some("main".to_string()))
            .build()
            .unwrap();

        let generator = MarkdownReportGenerator::new();
        let markdown = generator.generate_report(&report).await.unwrap();

        assert!(markdown.contains("# RLM Integration Test Report"));
        assert!(markdown.contains("abc123"));
        assert!(markdown.contains("main"));
        assert!(markdown.contains("Executive Summary"));
    }

    #[tokio::test]
    async fn test_json_report_generation() {
        let report = ReportBuilder::new().build().unwrap();

        let generator = JsonReportGenerator::new(true);
        let json = generator.generate_report(&report).await.unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("metadata").is_some());
        assert!(parsed.get("summary").is_some());
    }

    #[tokio::test]
    async fn test_report_save_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let report_path = temp_dir.path().join("test-report.md");

        let report = ReportBuilder::new().build().unwrap();
        let generator = MarkdownReportGenerator::new();

        generator.save_report(&report, &report_path).await.unwrap();

        let saved_content = fs::read_to_string(&report_path).await.unwrap();
        assert!(saved_content.contains("RLM Integration Test Report"));
    }
}