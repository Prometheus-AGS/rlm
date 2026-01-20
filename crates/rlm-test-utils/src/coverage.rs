//! Code coverage analysis and reporting for RLM integration tests.
//!
//! This module provides comprehensive coverage analysis using cargo-tarpaulin
//! and generates markdown reports with timestamps and detailed statistics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use tokio::fs;

use crate::error::{TestError, TestResult};

/// Coverage analysis configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageConfig {
    /// Output directory for coverage reports.
    pub output_dir: PathBuf,
    /// Coverage engine to use (llvm, ptrace).
    pub engine: CoverageEngine,
    /// Output formats to generate.
    pub output_formats: Vec<CoverageFormat>,
    /// Packages to include in coverage analysis.
    pub include_packages: Vec<String>,
    /// Files to exclude from coverage.
    pub exclude_files: Vec<String>,
    /// Minimum coverage thresholds.
    pub thresholds: CoverageThresholds,
    /// Enable branch coverage analysis.
    pub branch_coverage: bool,
    /// Timeout for coverage analysis in seconds.
    pub timeout_seconds: u64,
}

/// Coverage analysis engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageEngine {
    /// LLVM-based coverage (recommended).
    Llvm,
    /// Ptrace-based coverage.
    Ptrace,
}

/// Coverage report output formats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CoverageFormat {
    /// HTML report.
    Html,
    /// Markdown report with timestamps.
    Markdown,
    /// JSON data for programmatic access.
    Json,
    /// LCOV format for CI integration.
    Lcov,
    /// XML format (Cobertura).
    Xml,
    /// Plain text summary.
    Stdout,
}

/// Coverage thresholds for validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageThresholds {
    /// Minimum line coverage percentage.
    pub line_coverage: f32,
    /// Minimum branch coverage percentage.
    pub branch_coverage: f32,
    /// Minimum function coverage percentage.
    pub function_coverage: f32,
}

/// Complete coverage analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Report generation timestamp.
    pub generated_at: SystemTime,
    /// Overall coverage metrics.
    pub overall: CoverageMetrics,
    /// Coverage by package/crate.
    pub by_package: HashMap<String, CoverageMetrics>,
    /// Coverage by individual file.
    pub by_file: HashMap<String, FileCoverageMetrics>,
    /// Configuration used for analysis.
    pub config: CoverageConfig,
    /// Analysis duration.
    pub analysis_duration_seconds: f64,
}

/// Coverage metrics for a scope (overall, package, or file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageMetrics {
    /// Line coverage percentage.
    pub line_coverage: f32,
    /// Branch coverage percentage.
    pub branch_coverage: f32,
    /// Function coverage percentage.
    pub function_coverage: f32,
    /// Total lines of code.
    pub total_lines: u32,
    /// Lines covered by tests.
    pub covered_lines: u32,
    /// Total branches.
    pub total_branches: u32,
    /// Branches covered by tests.
    pub covered_branches: u32,
    /// Total functions.
    pub total_functions: u32,
    /// Functions covered by tests.
    pub covered_functions: u32,
}

/// File-specific coverage metrics with line-by-line details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverageMetrics {
    /// Basic coverage metrics.
    pub metrics: CoverageMetrics,
    /// Coverage status for each line (1-indexed).
    pub line_coverage_map: HashMap<u32, LineCoverageStatus>,
    /// Uncovered line ranges.
    pub uncovered_ranges: Vec<LineRange>,
}

/// Coverage status for a single line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineCoverageStatus {
    /// Line is covered by tests.
    Covered {
        /// Number of times hit.
        hit_count: u32,
    },
    /// Line is not covered by tests.
    Uncovered,
    /// Line is not executable (comment, blank, etc.).
    NonExecutable,
}

/// Range of source code lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    /// Starting line number (1-indexed).
    pub start: u32,
    /// Ending line number (1-indexed, inclusive).
    pub end: u32,
}

/// Coverage analyzer using cargo-tarpaulin.
#[derive(Debug)]
pub struct CoverageAnalyzer {
    config: CoverageConfig,
}

/// Markdown report generator.
#[derive(Debug)]
pub struct MarkdownReporter {
    config: CoverageConfig,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("coverage"),
            engine: CoverageEngine::Llvm,
            output_formats: vec![
                CoverageFormat::Html,
                CoverageFormat::Markdown,
                CoverageFormat::Json,
                CoverageFormat::Lcov,
            ],
            include_packages: vec![
                "rlm-core".to_string(),
                "rlm-server".to_string(),
                "rlm-repl-rhai".to_string(),
            ],
            exclude_files: vec![
                "*/tests/*".to_string(),
                "*/examples/*".to_string(),
                "**/target/**".to_string(),
            ],
            thresholds: CoverageThresholds {
                line_coverage: 100.0,
                branch_coverage: 95.0,
                function_coverage: 100.0,
            },
            branch_coverage: true,
            timeout_seconds: 600,
        }
    }
}

impl CoverageAnalyzer {
    /// Create a new coverage analyzer.
    pub fn new(config: CoverageConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn default() -> Self {
        Self::new(CoverageConfig::default())
    }

    /// Run comprehensive coverage analysis.
    #[instrument(skip(self))]
    pub async fn analyze(&self) -> TestResult<CoverageReport> {
        info!("Starting comprehensive coverage analysis");

        let start_time = std::time::Instant::now();
        let generated_at = SystemTime::now();

        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir).await?;

        // Build tarpaulin command
        let mut cmd = Command::new("cargo");
        cmd.arg("tarpaulin");

        // Configure engine
        match self.config.engine {
            CoverageEngine::Llvm => cmd.args(&["--engine", "llvm"]),
            CoverageEngine::Ptrace => cmd.args(&["--engine", "ptrace"]),
        };

        // Configure output formats
        for format in &self.config.output_formats {
            cmd.arg("--out");
            match format {
                CoverageFormat::Html => cmd.arg("Html"),
                CoverageFormat::Markdown => cmd.arg("Markdown"),
                CoverageFormat::Json => cmd.arg("Json"),
                CoverageFormat::Lcov => cmd.arg("Lcov"),
                CoverageFormat::Xml => cmd.arg("Xml"),
                CoverageFormat::Stdout => cmd.arg("Stdout"),
            };
        }

        // Set output directory
        cmd.args(&["--output-dir", self.config.output_dir.to_str().unwrap()]);

        // Configure packages
        if !self.config.include_packages.is_empty() {
            for package in &self.config.include_packages {
                cmd.args(&["--packages", package]);
            }
        } else {
            cmd.arg("--workspace");
        }

        // Configure exclusions
        for exclude in &self.config.exclude_files {
            cmd.args(&["--exclude-files", exclude]);
        }

        // Enable branch coverage if requested
        if self.config.branch_coverage {
            cmd.arg("--branch");
        }

        // Set timeout
        cmd.args(&["--timeout", &self.config.timeout_seconds.to_string()]);

        // Additional useful flags
        cmd.args(&[
            "--all-features",
            "--include-tests",
            "--no-dead-code",
            "--follow-exec",
        ]);

        info!("Running: {:?}", cmd);

        // Execute tarpaulin
        let output = cmd.output()
            .map_err(|e| TestError::coverage(format!("Failed to execute cargo-tarpaulin: {}. Make sure it's installed with 'cargo install cargo-tarpaulin'", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TestError::coverage(format!("Tarpaulin failed: {}", stderr)));
        }

        let analysis_duration = start_time.elapsed().as_secs_f64();
        info!("Coverage analysis completed in {:.1}s", analysis_duration);

        // Parse tarpaulin JSON output
        let json_path = self.config.output_dir.join("tarpaulin-report.json");
        let report = self.parse_tarpaulin_output(&json_path, generated_at, analysis_duration).await?;

        // Generate markdown report if requested
        if self.config.output_formats.contains(&CoverageFormat::Markdown) {
            let markdown_reporter = MarkdownReporter::new(self.config.clone());
            markdown_reporter.generate_report(&report).await?;
        }

        Ok(report)
    }

    /// Parse tarpaulin JSON output into our report format.
    async fn parse_tarpaulin_output(
        &self,
        json_path: &Path,
        generated_at: SystemTime,
        analysis_duration: f64,
    ) -> TestResult<CoverageReport> {
        if !json_path.exists() {
            return Err(TestError::coverage(
                "Tarpaulin JSON report not found. Make sure JSON output format is enabled."
            ));
        }

        let json_content = fs::read_to_string(json_path).await?;
        let tarpaulin_data: serde_json::Value = serde_json::from_str(&json_content)?;

        // Parse overall metrics
        let overall = self.parse_metrics(&tarpaulin_data)?;

        // Parse by-package metrics
        let mut by_package = HashMap::new();
        if let Some(files) = tarpaulin_data["files"].as_object() {
            for (file_path, file_data) in files {
                if let Some(package) = self.extract_package_name(file_path) {
                    let metrics = self.parse_file_metrics(file_data)?;
                    by_package.entry(package.to_string())
                        .and_modify(|existing: &mut CoverageMetrics| {
                            // Aggregate metrics
                            existing.total_lines += metrics.total_lines;
                            existing.covered_lines += metrics.covered_lines;
                            existing.total_branches += metrics.total_branches;
                            existing.covered_branches += metrics.covered_branches;
                            existing.total_functions += metrics.total_functions;
                            existing.covered_functions += metrics.covered_functions;

                            // Recalculate percentages
                            existing.line_coverage = if existing.total_lines > 0 {
                                (existing.covered_lines as f32 / existing.total_lines as f32) * 100.0
                            } else { 0.0 };
                            existing.branch_coverage = if existing.total_branches > 0 {
                                (existing.covered_branches as f32 / existing.total_branches as f32) * 100.0
                            } else { 0.0 };
                            existing.function_coverage = if existing.total_functions > 0 {
                                (existing.covered_functions as f32 / existing.total_functions as f32) * 100.0
                            } else { 0.0 };
                        })
                        .or_insert(metrics);
                }
            }
        }

        // Parse by-file metrics
        let mut by_file = HashMap::new();
        if let Some(files) = tarpaulin_data["files"].as_object() {
            for (file_path, file_data) in files {
                let metrics = self.parse_file_metrics(file_data)?;
                let line_coverage_map = self.parse_line_coverage(file_data)?;
                let uncovered_ranges = self.calculate_uncovered_ranges(&line_coverage_map);

                by_file.insert(file_path.clone(), FileCoverageMetrics {
                    metrics,
                    line_coverage_map,
                    uncovered_ranges,
                });
            }
        }

        Ok(CoverageReport {
            generated_at,
            overall,
            by_package,
            by_file,
            config: self.config.clone(),
            analysis_duration_seconds: analysis_duration,
        })
    }

    /// Parse overall coverage metrics.
    fn parse_metrics(&self, data: &serde_json::Value) -> TestResult<CoverageMetrics> {
        let line_coverage = data["line_coverage"]
            .as_f64()
            .unwrap_or(0.0) as f32;

        let branch_coverage = data["branch_coverage"]
            .as_f64()
            .unwrap_or(0.0) as f32;

        let function_coverage = data["function_coverage"]
            .as_f64()
            .unwrap_or(0.0) as f32;

        Ok(CoverageMetrics {
            line_coverage,
            branch_coverage,
            function_coverage,
            total_lines: data["total_lines"].as_u64().unwrap_or(0) as u32,
            covered_lines: data["covered_lines"].as_u64().unwrap_or(0) as u32,
            total_branches: data["total_branches"].as_u64().unwrap_or(0) as u32,
            covered_branches: data["covered_branches"].as_u64().unwrap_or(0) as u32,
            total_functions: data["total_functions"].as_u64().unwrap_or(0) as u32,
            covered_functions: data["covered_functions"].as_u64().unwrap_or(0) as u32,
        })
    }

    /// Parse coverage metrics for a single file.
    fn parse_file_metrics(&self, file_data: &serde_json::Value) -> TestResult<CoverageMetrics> {
        // Parse similar to overall metrics but for individual file
        self.parse_metrics(file_data)
    }

    /// Parse line-by-line coverage data.
    fn parse_line_coverage(
        &self,
        file_data: &serde_json::Value,
    ) -> TestResult<HashMap<u32, LineCoverageStatus>> {
        let mut line_coverage = HashMap::new();

        if let Some(lines) = file_data["lines"].as_object() {
            for (line_num_str, line_data) in lines {
                if let Ok(line_num) = line_num_str.parse::<u32>() {
                    let status = if let Some(hit_count) = line_data["count"].as_u64() {
                        if hit_count > 0 {
                            LineCoverageStatus::Covered {
                                hit_count: hit_count as u32,
                            }
                        } else {
                            LineCoverageStatus::Uncovered
                        }
                    } else {
                        LineCoverageStatus::NonExecutable
                    };

                    line_coverage.insert(line_num, status);
                }
            }
        }

        Ok(line_coverage)
    }

    /// Calculate ranges of uncovered lines.
    fn calculate_uncovered_ranges(&self, line_coverage: &HashMap<u32, LineCoverageStatus>) -> Vec<LineRange> {
        let mut uncovered_lines: Vec<u32> = line_coverage
            .iter()
            .filter_map(|(line_num, status)| {
                match status {
                    LineCoverageStatus::Uncovered => Some(*line_num),
                    _ => None,
                }
            })
            .collect();

        uncovered_lines.sort();

        let mut ranges = Vec::new();
        if uncovered_lines.is_empty() {
            return ranges;
        }

        let mut start = uncovered_lines[0];
        let mut end = start;

        for &line_num in uncovered_lines.iter().skip(1) {
            if line_num == end + 1 {
                end = line_num;
            } else {
                ranges.push(LineRange { start, end });
                start = line_num;
                end = line_num;
            }
        }

        ranges.push(LineRange { start, end });
        ranges
    }

    /// Extract package name from file path.
    fn extract_package_name<'a>(&self, file_path: &'a str) -> Option<&'a str> {
        // Extract package name from path like "crates/rlm-core/src/lib.rs"
        if let Some(crates_pos) = file_path.find("crates/") {
            let after_crates = &file_path[crates_pos + 7..];
            if let Some(slash_pos) = after_crates.find('/') {
                return Some(&after_crates[..slash_pos]);
            }
        }
        None
    }
}

impl MarkdownReporter {
    /// Create a new markdown reporter.
    pub fn new(config: CoverageConfig) -> Self {
        Self { config }
    }

    /// Generate comprehensive markdown report.
    #[instrument(skip(self, report))]
    pub async fn generate_report(&self, report: &CoverageReport) -> TestResult<()> {
        info!("Generating markdown coverage report");

        let report_path = self.config.output_dir.join("INTEGRATION_COVERAGE_REPORT.md");
        let content = self.format_markdown_report(report).await?;

        fs::write(&report_path, content).await?;

        info!("Markdown report generated: {}", report_path.display());
        Ok(())
    }

    /// Format the complete markdown report.
    async fn format_markdown_report(&self, report: &CoverageReport) -> TestResult<String> {
        let timestamp = report.generated_at
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let mut content = String::new();

        // Header
        content.push_str("# RLM Integration Test Coverage Report\n\n");
        content.push_str(&format!("**Generated**: {}\n", datetime));
        content.push_str(&format!("**Analysis Duration**: {:.1}s\n\n", report.analysis_duration_seconds));

        // Overall summary
        content.push_str("## Coverage Summary\n\n");
        content.push_str("| Metric | Value | Target | Status |\n");
        content.push_str("|--------|-------|--------|--------|\n");

        let line_status = if report.overall.line_coverage >= report.config.thresholds.line_coverage {
            "✅"
        } else {
            "❌"
        };

        let branch_status = if report.overall.branch_coverage >= report.config.thresholds.branch_coverage {
            "✅"
        } else {
            "❌"
        };

        let function_status = if report.overall.function_coverage >= report.config.thresholds.function_coverage {
            "✅"
        } else {
            "❌"
        };

        content.push_str(&format!(
            "| Line Coverage | {:.1}% | {:.1}% | {} |\n",
            report.overall.line_coverage,
            report.config.thresholds.line_coverage,
            line_status
        ));

        content.push_str(&format!(
            "| Branch Coverage | {:.1}% | {:.1}% | {} |\n",
            report.overall.branch_coverage,
            report.config.thresholds.branch_coverage,
            branch_status
        ));

        content.push_str(&format!(
            "| Function Coverage | {:.1}% | {:.1}% | {} |\n",
            report.overall.function_coverage,
            report.config.thresholds.function_coverage,
            function_status
        ));

        content.push_str("\n");

        // Package breakdown
        if !report.by_package.is_empty() {
            content.push_str("## Coverage by Package\n\n");
            content.push_str("| Package | Line Coverage | Branch Coverage | Function Coverage |\n");
            content.push_str("|---------|---------------|-----------------|-------------------|\n");

            let mut packages: Vec<_> = report.by_package.iter().collect();
            packages.sort_by_key(|(name, _)| *name);

            for (package_name, metrics) in packages {
                content.push_str(&format!(
                    "| {} | {:.1}% | {:.1}% | {:.1}% |\n",
                    package_name,
                    metrics.line_coverage,
                    metrics.branch_coverage,
                    metrics.function_coverage
                ));
            }

            content.push_str("\n");
        }

        // Files with less than perfect coverage
        let imperfect_files: Vec<_> = report.by_file
            .iter()
            .filter(|(_, file_metrics)| file_metrics.metrics.line_coverage < 100.0)
            .collect();

        if !imperfect_files.is_empty() {
            content.push_str("## Files Needing Attention\n\n");
            content.push_str("| File | Line Coverage | Uncovered Lines |\n");
            content.push_str("|------|---------------|----------------|\n");

            for (file_path, file_metrics) in imperfect_files {
                let uncovered_summary = if file_metrics.uncovered_ranges.is_empty() {
                    "None".to_string()
                } else {
                    file_metrics.uncovered_ranges
                        .iter()
                        .map(|range| {
                            if range.start == range.end {
                                range.start.to_string()
                            } else {
                                format!("{}-{}", range.start, range.end)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                content.push_str(&format!(
                    "| {} | {:.1}% | {} |\n",
                    file_path,
                    file_metrics.metrics.line_coverage,
                    uncovered_summary
                ));
            }

            content.push_str("\n");
        }

        // Configuration details
        content.push_str("## Analysis Configuration\n\n");
        content.push_str(&format!("- **Engine**: {:?}\n", report.config.engine));
        content.push_str(&format!("- **Branch Coverage**: {}\n", report.config.branch_coverage));
        content.push_str(&format!("- **Timeout**: {}s\n", report.config.timeout_seconds));
        content.push_str(&format!("- **Packages**: {}\n", report.config.include_packages.join(", ")));

        if !report.config.exclude_files.is_empty() {
            content.push_str(&format!("- **Excluded**: {}\n", report.config.exclude_files.join(", ")));
        }

        content.push_str("\n");

        // Footer
        content.push_str("---\n");
        content.push_str("*Generated by RLM Integration Testing Framework*\n");

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_coverage_config_default() {
        let config = CoverageConfig::default();
        assert_eq!(config.thresholds.line_coverage, 100.0);
        assert_eq!(config.thresholds.function_coverage, 100.0);
        assert!(config.branch_coverage);
        assert!(matches!(config.engine, CoverageEngine::Llvm));
    }

    #[test]
    fn test_line_range_calculation() {
        let analyzer = CoverageAnalyzer::default();

        let mut line_coverage = HashMap::new();
        line_coverage.insert(1, LineCoverageStatus::Covered { hit_count: 1 });
        line_coverage.insert(2, LineCoverageStatus::Uncovered);
        line_coverage.insert(3, LineCoverageStatus::Uncovered);
        line_coverage.insert(4, LineCoverageStatus::Covered { hit_count: 2 });
        line_coverage.insert(7, LineCoverageStatus::Uncovered);

        let ranges = analyzer.calculate_uncovered_ranges(&line_coverage);

        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].start, 2);
        assert_eq!(ranges[0].end, 3);
        assert_eq!(ranges[1].start, 7);
        assert_eq!(ranges[1].end, 7);
    }

    #[test]
    fn test_package_name_extraction() {
        let analyzer = CoverageAnalyzer::default();

        assert_eq!(
            analyzer.extract_package_name("crates/rlm-core/src/lib.rs"),
            Some("rlm-core")
        );

        assert_eq!(
            analyzer.extract_package_name("crates/rlm-server/src/main.rs"),
            Some("rlm-server")
        );

        assert_eq!(
            analyzer.extract_package_name("src/lib.rs"),
            None
        );
    }

    #[tokio::test]
    async fn test_markdown_reporter() {
        let temp_dir = TempDir::new().unwrap();
        let config = CoverageConfig {
            output_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let reporter = MarkdownReporter::new(config);

        let report = CoverageReport {
            generated_at: SystemTime::now(),
            overall: CoverageMetrics {
                line_coverage: 95.5,
                branch_coverage: 90.0,
                function_coverage: 100.0,
                total_lines: 1000,
                covered_lines: 955,
                total_branches: 100,
                covered_branches: 90,
                total_functions: 50,
                covered_functions: 50,
            },
            by_package: HashMap::new(),
            by_file: HashMap::new(),
            config: CoverageConfig::default(),
            analysis_duration_seconds: 30.5,
        };

        let markdown = reporter.format_markdown_report(&report).await.unwrap();

        assert!(markdown.contains("# RLM Integration Test Coverage Report"));
        assert!(markdown.contains("95.5%"));
        assert!(markdown.contains("❌")); // Line coverage below threshold
        assert!(markdown.contains("✅")); // Function coverage meets threshold
    }
}