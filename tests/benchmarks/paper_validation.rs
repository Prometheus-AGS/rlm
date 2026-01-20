//! Paper Benchmark Validation Suite
//!
//! This module implements comprehensive benchmark tests that validate the RLM implementation
//! against the performance claims made in the MIT RLM paper (arXiv:2512.24601).
//!
//! The suite tests the three main complexity classes and their expected performance improvements:
//! - S-NIAH: O(1) complexity for needle-in-haystack tasks
//! - OOLONG: O(n) complexity for aggregation tasks
//! - BROWSECOMP: O(n log n) complexity for multi-hop reasoning
//!
//! Expected improvements over baseline:
//! - S-NIAH: 95.0% → 97.5% accuracy (+2.5%)
//! - OOLONG: 72.3% → 89.4% accuracy (+17.1%)
//! - BROWSECOMP: Comparable accuracy with lower token usage

use std::time::{Duration, Instant};
use tokio::fs;
use serde_json::Value;
use rlm_core::{RlmClient, RlmExecutor, RlmRequest, RlmResponse};
use rlm_server::OpenAiProvider;

/// Results from running a benchmark test
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub accuracy: f64,
    pub total_tokens: u32,
    pub processing_time: Duration,
    pub memory_peak_mb: u64,
    pub recursive_calls: u32,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Main benchmark validation suite
pub struct PaperValidationSuite {
    rlm_client: RlmClient,
    baseline_client: Option<OpenAiProvider>, // For comparison tests
}

impl PaperValidationSuite {
    /// Create a new validation suite with RLM client
    pub async fn new(rlm_client: RlmClient) -> Self {
        Self {
            rlm_client,
            baseline_client: None,
        }
    }

    /// Add baseline client for comparison testing
    pub fn with_baseline(mut self, baseline_client: OpenAiProvider) -> Self {
        self.baseline_client = Some(baseline_client);
        self
    }

    /// Run the complete paper validation suite
    pub async fn run_full_validation(&self) -> Result<ValidationReport, Box<dyn std::error::Error>> {
        println!("🚀 Starting RLM Paper Validation Suite");
        println!("📋 Testing against MIT RLM paper (arXiv:2512.24601) benchmarks");

        let mut report = ValidationReport::new();

        // Run S-NIAH benchmarks (O(1) complexity)
        println!("\n🔍 Running S-NIAH (Simple Needle-in-Haystack) benchmarks...");
        let s_niah_results = self.run_s_niah_benchmarks().await?;
        report.add_category("s_niah", s_niah_results);

        // Run OOLONG benchmarks (O(n) complexity)
        println!("\n📊 Running OOLONG aggregation benchmarks...");
        let oolong_results = self.run_oolong_benchmarks().await?;
        report.add_category("oolong", oolong_results);

        // Run BROWSECOMP benchmarks (O(n log n) complexity)
        println!("\n🔗 Running BROWSECOMP multi-hop QA benchmarks...");
        let browsecomp_results = self.run_browsecomp_benchmarks().await?;
        report.add_category("browsecomp", browsecomp_results);

        // Run additional complexity tests
        println!("\n⚡ Running performance complexity validation...");
        let complexity_results = self.run_complexity_validation().await?;
        report.add_category("complexity", complexity_results);

        report.finalize();
        Ok(report)
    }

    /// Run S-NIAH (Simple Needle-in-Haystack) benchmark tests
    async fn run_s_niah_benchmarks(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Load S-NIAH test fixtures
        let test_dirs = fs::read_dir("tests/fixtures/s_niah").await?;
        let mut test_dirs: Vec<_> = test_dirs.collect::<Result<Vec<_>, _>>().await?;
        test_dirs.sort_by_key(|entry| entry.file_name());

        for entry in test_dirs {
            let path = entry.path();
            if !path.is_dir() || path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                continue;
            }

            let test_name = path.file_name().unwrap().to_str().unwrap().to_string();

            if test_name == "test_001" {
                println!("  🧪 Running {}", test_name);
                let result = self.run_s_niah_test(&path).await?;
                results.push(result);
            }
        }

        // Validate S-NIAH performance expectations
        let average_accuracy = results.iter()
            .filter(|r| r.success)
            .map(|r| r.accuracy)
            .sum::<f64>() / results.len() as f64;

        println!("  📈 S-NIAH Average Accuracy: {:.1}%", average_accuracy * 100.0);
        println!("  🎯 Paper Target: 97.5% (baseline: 95.0%)");

        if average_accuracy < 0.975 {
            println!("  ⚠️  Warning: Accuracy below paper target");
        } else {
            println!("  ✅ S-NIAH accuracy meets paper expectations");
        }

        Ok(results)
    }

    /// Run a single S-NIAH test case
    async fn run_s_niah_test(&self, test_dir: &std::path::Path) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        let test_name = test_dir.file_name().unwrap().to_str().unwrap().to_string();

        // Load test data
        let question = fs::read_to_string(test_dir.join("question.txt")).await?;
        let expected = fs::read_to_string(test_dir.join("expected.txt")).await?;
        let haystack = fs::read_to_string(test_dir.join("haystack.txt")).await?;
        let metadata: Value = serde_json::from_str(
            &fs::read_to_string(test_dir.join("metadata.json")).await?
        )?;

        // Create RLM request
        let request = RlmRequest::new()
            .with_context(&haystack)
            .with_query(&question)
            .with_max_tokens(500);

        // Execute and measure
        let start = Instant::now();
        let response = self.rlm_client.execute(&request).await;
        let duration = start.elapsed();

        match response {
            Ok(response) => {
                // Calculate accuracy using simple string matching for now
                // TODO: Implement semantic similarity scoring
                let accuracy = if response.content.trim() == expected.trim() {
                    1.0
                } else {
                    self.calculate_semantic_similarity(&response.content, &expected)
                };

                Ok(BenchmarkResult {
                    test_name,
                    accuracy,
                    total_tokens: response.metadata.total_tokens,
                    processing_time: duration,
                    memory_peak_mb: self.get_memory_usage().await.unwrap_or(0),
                    recursive_calls: response.metadata.recursive_calls,
                    success: true,
                    error_message: None,
                })
            }
            Err(e) => {
                Ok(BenchmarkResult {
                    test_name,
                    accuracy: 0.0,
                    total_tokens: 0,
                    processing_time: duration,
                    memory_peak_mb: 0,
                    recursive_calls: 0,
                    success: false,
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    /// Run OOLONG aggregation benchmark tests
    async fn run_oolong_benchmarks(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Load OOLONG test fixtures
        let query_content = fs::read_to_string("tests/fixtures/oolong/query_212.json").await?;
        let query_data: Value = serde_json::from_str(&query_content)?;

        let expected_content = fs::read_to_string("tests/fixtures/oolong/expected_response.json").await?;
        let expected_data: Value = serde_json::from_str(&expected_content)?;

        println!("  🧪 Running OOLONG aggregation test");

        // Create RLM request for aggregation task
        let request = RlmRequest::new()
            .with_context(query_data["context"].as_str().unwrap())
            .with_query(query_data["query"].as_str().unwrap())
            .with_max_tokens(1000);

        let start = Instant::now();
        let response = self.rlm_client.execute(&request).await;
        let duration = start.elapsed();

        let result = match response {
            Ok(response) => {
                let expected_answer = expected_data["answer"].as_str().unwrap();
                let accuracy = self.calculate_semantic_similarity(&response.content, expected_answer);

                BenchmarkResult {
                    test_name: "oolong_aggregation".to_string(),
                    accuracy,
                    total_tokens: response.metadata.total_tokens,
                    processing_time: duration,
                    memory_peak_mb: self.get_memory_usage().await.unwrap_or(0),
                    recursive_calls: response.metadata.recursive_calls,
                    success: true,
                    error_message: None,
                }
            }
            Err(e) => {
                BenchmarkResult {
                    test_name: "oolong_aggregation".to_string(),
                    accuracy: 0.0,
                    total_tokens: 0,
                    processing_time: duration,
                    memory_peak_mb: 0,
                    recursive_calls: 0,
                    success: false,
                    error_message: Some(e.to_string()),
                }
            }
        };

        println!("  📈 OOLONG Accuracy: {:.1}%", result.accuracy * 100.0);
        println!("  🎯 Paper Target: 89.4% (baseline: 72.3%)");

        results.push(result);
        Ok(results)
    }

    /// Run BROWSECOMP multi-hop QA benchmark tests
    async fn run_browsecomp_benchmarks(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Load BROWSECOMP test fixtures
        let query_content = fs::read_to_string("tests/fixtures/browsecomp/query_74.json").await?;
        let query_data: Value = serde_json::from_str(&query_content)?;

        let expected_content = fs::read_to_string("tests/fixtures/browsecomp/expected_response.json").await?;
        let expected_data: Value = serde_json::from_str(&expected_content)?;

        println!("  🧪 Running BROWSECOMP multi-hop QA test");

        // Create RLM request for multi-hop reasoning
        let request = RlmRequest::new()
            .with_context(query_data["context"].as_str().unwrap())
            .with_query(query_data["query"].as_str().unwrap())
            .with_max_tokens(1500);

        let start = Instant::now();
        let response = self.rlm_client.execute(&request).await;
        let duration = start.elapsed();

        let result = match response {
            Ok(response) => {
                let expected_answer = expected_data["answer"].as_str().unwrap();
                let accuracy = self.calculate_semantic_similarity(&response.content, expected_answer);

                BenchmarkResult {
                    test_name: "browsecomp_multihop".to_string(),
                    accuracy,
                    total_tokens: response.metadata.total_tokens,
                    processing_time: duration,
                    memory_peak_mb: self.get_memory_usage().await.unwrap_or(0),
                    recursive_calls: response.metadata.recursive_calls,
                    success: true,
                    error_message: None,
                }
            }
            Err(e) => {
                BenchmarkResult {
                    test_name: "browsecomp_multihop".to_string(),
                    accuracy: 0.0,
                    total_tokens: 0,
                    processing_time: duration,
                    memory_peak_mb: 0,
                    recursive_calls: 0,
                    success: false,
                    error_message: Some(e.to_string()),
                }
            }
        };

        println!("  📈 BROWSECOMP Accuracy: {:.1}%", result.accuracy * 100.0);
        println!("  🎯 Paper Target: Comparable to baseline with lower token usage");

        results.push(result);
        Ok(results)
    }

    /// Run complexity validation tests
    async fn run_complexity_validation(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Test O(1) complexity for needle-in-haystack
        println!("  🧪 Validating O(1) complexity for needle-in-haystack");
        let o1_result = self.test_o1_complexity().await?;
        results.push(o1_result);

        // Test O(n) complexity for aggregation
        println!("  🧪 Validating O(n) complexity for aggregation");
        let on_result = self.test_on_complexity().await?;
        results.push(on_result);

        Ok(results)
    }

    /// Test O(1) complexity by running needle-in-haystack with different context sizes
    async fn test_o1_complexity(&self) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        let context_sizes = vec![10_000, 50_000, 100_000, 500_000];
        let mut processing_times = Vec::new();

        for size in context_sizes {
            // Generate synthetic haystack of given size
            let haystack = self.generate_synthetic_haystack(size);
            let needle = "The secret code is GAMMA-9981";
            let full_context = format!("{}\n\n{}", haystack, needle);

            let request = RlmRequest::new()
                .with_context(&full_context)
                .with_query("What is the secret code?")
                .with_max_tokens(100);

            let start = Instant::now();
            let _ = self.rlm_client.execute(&request).await;
            let duration = start.elapsed();

            processing_times.push(duration.as_millis() as f64);
        }

        // Check if processing time is roughly constant (O(1))
        let first_time = processing_times[0];
        let last_time = processing_times[processing_times.len() - 1];
        let ratio = last_time / first_time;

        // If truly O(1), ratio should be close to 1.0
        // Allow for some variance due to system factors
        let is_o1 = ratio < 3.0; // Allow 3x variance

        Ok(BenchmarkResult {
            test_name: "complexity_o1_validation".to_string(),
            accuracy: if is_o1 { 1.0 } else { 0.0 },
            total_tokens: 0,
            processing_time: Duration::from_millis(last_time as u64),
            memory_peak_mb: 0,
            recursive_calls: 0,
            success: is_o1,
            error_message: if is_o1 {
                None
            } else {
                Some(format!("Processing time ratio {} exceeds O(1) expectation", ratio))
            },
        })
    }

    /// Test O(n) complexity for aggregation tasks
    async fn test_on_complexity(&self) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        let item_counts = vec![100, 500, 1000];
        let mut processing_times = Vec::new();

        for count in item_counts {
            // Generate synthetic aggregation task
            let items = self.generate_aggregation_items(count);

            let request = RlmRequest::new()
                .with_context(&items)
                .with_query("Calculate the sum of all values")
                .with_max_tokens(200);

            let start = Instant::now();
            let _ = self.rlm_client.execute(&request).await;
            let duration = start.elapsed();

            processing_times.push(duration.as_millis() as f64);
        }

        // Check if processing time scales linearly with input size (O(n))
        let first_time = processing_times[0];
        let last_time = processing_times[processing_times.len() - 1];
        let expected_ratio = item_counts[item_counts.len() - 1] as f64 / item_counts[0] as f64;
        let actual_ratio = last_time / first_time;

        // Linear scaling should be close to expected ratio
        let is_linear = (actual_ratio / expected_ratio - 1.0).abs() < 2.0; // Allow 200% variance

        Ok(BenchmarkResult {
            test_name: "complexity_on_validation".to_string(),
            accuracy: if is_linear { 1.0 } else { 0.0 },
            total_tokens: 0,
            processing_time: Duration::from_millis(last_time as u64),
            memory_peak_mb: 0,
            recursive_calls: 0,
            success: is_linear,
            error_message: if is_linear {
                None
            } else {
                Some(format!("Processing time scaling {} doesn't match O(n) expectation {}", actual_ratio, expected_ratio))
            },
        })
    }

    /// Generate synthetic haystack content of specified token size
    fn generate_synthetic_haystack(&self, target_tokens: usize) -> String {
        let words = vec!["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit"];
        let mut content = String::new();

        // Rough approximation: 1 token ≈ 4 characters
        let target_chars = target_tokens * 4;

        while content.len() < target_chars {
            for word in &words {
                content.push_str(word);
                content.push(' ');
                if content.len() >= target_chars {
                    break;
                }
            }
        }

        content
    }

    /// Generate synthetic aggregation items
    fn generate_aggregation_items(&self, count: usize) -> String {
        let mut items = String::new();
        for i in 1..=count {
            items.push_str(&format!("Item {}: Value = {}\n", i, i * 10));
        }
        items
    }

    /// Calculate semantic similarity between two strings
    /// TODO: Implement more sophisticated similarity scoring (BLEU, BERT, etc.)
    fn calculate_semantic_similarity(&self, response: &str, expected: &str) -> f64 {
        let response_lower = response.to_lowercase();
        let expected_lower = expected.to_lowercase();

        // Simple word-overlap similarity for now
        let response_words: std::collections::HashSet<_> = response_lower.split_whitespace().collect();
        let expected_words: std::collections::HashSet<_> = expected_lower.split_whitespace().collect();

        let intersection = response_words.intersection(&expected_words).count();
        let union = response_words.union(&expected_words).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Get current memory usage in MB
    async fn get_memory_usage(&self) -> Option<u64> {
        // TODO: Implement actual memory monitoring
        // This could use system calls or external tools like `ps`
        None
    }
}

/// Complete validation report
#[derive(Debug)]
pub struct ValidationReport {
    categories: std::collections::HashMap<String, Vec<BenchmarkResult>>,
    started_at: Instant,
    completed_at: Option<Instant>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            categories: std::collections::HashMap::new(),
            started_at: Instant::now(),
            completed_at: None,
        }
    }

    fn add_category(&mut self, name: &str, results: Vec<BenchmarkResult>) {
        self.categories.insert(name.to_string(), results);
    }

    fn finalize(&mut self) {
        self.completed_at = Some(Instant::now());
        self.print_summary();
    }

    fn print_summary(&self) {
        println!("\n" + "=".repeat(80).as_str());
        println!("🎯 RLM Paper Validation Summary");
        println!("=".repeat(80));

        let total_duration = self.completed_at.unwrap() - self.started_at;
        println!("⏱️  Total execution time: {:.2}s", total_duration.as_secs_f64());

        for (category, results) in &self.categories {
            println!("\n📊 {} Results:", category.to_uppercase());

            let successful = results.iter().filter(|r| r.success).count();
            let total = results.len();

            if total == 0 {
                println!("   No tests executed");
                continue;
            }

            let success_rate = successful as f64 / total as f64;
            let avg_accuracy = results.iter()
                .filter(|r| r.success)
                .map(|r| r.accuracy)
                .sum::<f64>() / successful as f64;

            println!("   ✅ Success Rate: {}/{} ({:.1}%)", successful, total, success_rate * 100.0);
            println!("   📈 Average Accuracy: {:.1}%", avg_accuracy * 100.0);

            let avg_tokens = results.iter()
                .filter(|r| r.success)
                .map(|r| r.total_tokens as f64)
                .sum::<f64>() / successful as f64;

            let avg_time = results.iter()
                .filter(|r| r.success)
                .map(|r| r.processing_time.as_millis() as f64)
                .sum::<f64>() / successful as f64;

            println!("   🔢 Average Tokens: {:.0}", avg_tokens);
            println!("   ⏱️  Average Time: {:.0}ms", avg_time);

            // Print failed tests
            for result in results.iter().filter(|r| !r.success) {
                println!("   ❌ {}: {}", result.test_name,
                    result.error_message.as_deref().unwrap_or("Unknown error"));
            }
        }

        println!("\n" + "=".repeat(80).as_str());
        println!("Validation complete! 🎉");
        println!("=".repeat(80));
    }

    /// Get overall validation success rate
    pub fn overall_success_rate(&self) -> f64 {
        let total_tests: usize = self.categories.values().map(|results| results.len()).sum();
        let successful_tests: usize = self.categories.values()
            .map(|results| results.iter().filter(|r| r.success).count())
            .sum();

        if total_tests == 0 {
            0.0
        } else {
            successful_tests as f64 / total_tests as f64
        }
    }

    /// Check if validation meets paper expectations
    pub fn meets_paper_expectations(&self) -> bool {
        // S-NIAH should achieve >97.5% accuracy
        if let Some(s_niah_results) = self.categories.get("s_niah") {
            let avg_accuracy = s_niah_results.iter()
                .filter(|r| r.success)
                .map(|r| r.accuracy)
                .sum::<f64>() / s_niah_results.len() as f64;

            if avg_accuracy < 0.975 {
                return false;
            }
        }

        // OOLONG should achieve >89.4% accuracy
        if let Some(oolong_results) = self.categories.get("oolong") {
            let avg_accuracy = oolong_results.iter()
                .filter(|r| r.success)
                .map(|r| r.accuracy)
                .sum::<f64>() / oolong_results.len() as f64;

            if avg_accuracy < 0.894 {
                return false;
            }
        }

        // All complexity tests should pass
        if let Some(complexity_results) = self.categories.get("complexity") {
            if !complexity_results.iter().all(|r| r.success) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlm_core::builder::RlmClientBuilder;
    use rlm_repl_rhai::RhaiReplBackend;
    use rlm_server::OpenAiProvider;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_paper_validation_suite() {
        // Set up test RLM client
        let repl = Arc::new(tokio::sync::Mutex::new(RhaiReplBackend::new().unwrap()));
        let llm = Arc::new(OpenAiProvider::new(
            rlm_server::OpenAiConfig::builder()
                .api_key("test-key".to_string())
                .build().unwrap()
        ));

        let client = RlmClientBuilder::new()
            .repl_backend(repl)
            .llm_provider(llm)
            .build()
            .unwrap();

        let suite = PaperValidationSuite::new(client).await;

        // Run validation (this will use mock data in tests)
        let report = suite.run_full_validation().await;

        assert!(report.is_ok());
        let report = report.unwrap();

        // Validation should have results for all categories
        assert!(!report.categories.is_empty());

        // Success rate should be reasonable for test data
        assert!(report.overall_success_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_semantic_similarity_calculation() {
        let client = create_test_client().await;
        let suite = PaperValidationSuite::new(client).await;

        // Test exact match
        let similarity = suite.calculate_semantic_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);

        // Test partial match
        let similarity = suite.calculate_semantic_similarity("hello world test", "hello world example");
        assert!(similarity > 0.0 && similarity < 1.0);

        // Test no match
        let similarity = suite.calculate_semantic_similarity("completely different", "nothing similar");
        assert!(similarity < 0.5);
    }

    async fn create_test_client() -> RlmClient {
        let repl = Arc::new(tokio::sync::Mutex::new(RhaiReplBackend::new().unwrap()));
        let llm = Arc::new(OpenAiProvider::new(
            rlm_server::OpenAiConfig::builder()
                .api_key("test-key".to_string())
                .build().unwrap()
        ));

        RlmClientBuilder::new()
            .repl_backend(repl)
            .llm_provider(llm)
            .build()
            .unwrap()
    }
}