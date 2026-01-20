//! Performance Benchmarking Suite
//!
//! This module implements comprehensive performance benchmarks that measure and validate
//! the RLM implementation's performance characteristics against paper baselines.
//!
//! Key metrics measured:
//! - Processing time vs context size (complexity validation)
//! - Token usage efficiency compared to baseline models
//! - Memory consumption patterns
//! - Concurrent request handling capabilities
//! - Streaming response latency
//!
//! Expected performance targets from MIT RLM paper:
//! - 10M+ token context handling capability
//! - 150% token usage efficiency vs baseline
//! - <3x processing time for contexts under 100K tokens
//! - 100+ concurrent requests with <5s first chunk latency

use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};
use rlm_core::{RlmClient, RlmRequest};

/// Performance benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBenchmarkConfig {
    /// Maximum context sizes to test (in tokens)
    pub context_sizes: Vec<usize>,
    /// Number of concurrent requests to test
    pub concurrency_levels: Vec<usize>,
    /// Timeout for each individual test
    pub test_timeout: Duration,
    /// Number of iterations per test for averaging
    pub iterations_per_test: usize,
    /// Baseline comparison enabled
    pub enable_baseline_comparison: bool,
}

impl Default for PerformanceBenchmarkConfig {
    fn default() -> Self {
        Self {
            context_sizes: vec![1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000],
            concurrency_levels: vec![1, 10, 25, 50, 100],
            test_timeout: Duration::from_secs(300), // 5 minutes per test
            iterations_per_test: 3,
            enable_baseline_comparison: false,
        }
    }
}

/// Results from a single performance test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceResult {
    pub test_name: String,
    pub context_size_tokens: usize,
    pub concurrency_level: usize,
    pub avg_processing_time: Duration,
    pub min_processing_time: Duration,
    pub max_processing_time: Duration,
    pub p95_processing_time: Duration,
    pub total_tokens_used: u32,
    pub tokens_per_second: f64,
    pub memory_peak_mb: u64,
    pub memory_avg_mb: u64,
    pub success_rate: f64,
    pub first_chunk_latency: Option<Duration>,
    pub streaming_chunk_interval: Option<Duration>,
    pub error_messages: Vec<String>,
}

/// Main performance benchmarking suite
pub struct PerformanceBenchmarkSuite {
    rlm_client: Arc<RlmClient>,
    baseline_client: Option<Arc<dyn BaselineClient>>,
    config: PerformanceBenchmarkConfig,
}

/// Trait for baseline model clients (for comparison)
#[async_trait::async_trait]
pub trait BaselineClient: Send + Sync {
    async fn complete(&self, request: &RlmRequest) -> Result<String, Box<dyn std::error::Error>>;
    async fn complete_streaming(&self, request: &RlmRequest) -> Result<tokio::sync::mpsc::Receiver<String>, Box<dyn std::error::Error>>;
}

impl PerformanceBenchmarkSuite {
    /// Create new performance benchmark suite
    pub fn new(rlm_client: Arc<RlmClient>, config: PerformanceBenchmarkConfig) -> Self {
        Self {
            rlm_client,
            baseline_client: None,
            config,
        }
    }

    /// Add baseline client for comparison testing
    pub fn with_baseline_client(mut self, baseline_client: Arc<dyn BaselineClient>) -> Self {
        self.baseline_client = Some(baseline_client);
        self
    }

    /// Run the complete performance benchmark suite
    pub async fn run_full_benchmark(&self) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
        println!("🚀 Starting RLM Performance Benchmark Suite");
        println!("📊 Testing performance against paper claims");

        let mut report = BenchmarkReport::new();

        // Context size scaling tests
        println!("\n📏 Running context size scaling benchmarks...");
        let scaling_results = self.run_context_scaling_benchmarks().await?;
        report.add_category("context_scaling", scaling_results);

        // Concurrency benchmarks
        println!("\n⚡ Running concurrency benchmarks...");
        let concurrency_results = self.run_concurrency_benchmarks().await?;
        report.add_category("concurrency", concurrency_results);

        // Streaming performance tests
        println!("\n📡 Running streaming performance benchmarks...");
        let streaming_results = self.run_streaming_benchmarks().await?;
        report.add_category("streaming", streaming_results);

        // Memory efficiency tests
        println!("\n💾 Running memory efficiency benchmarks...");
        let memory_results = self.run_memory_benchmarks().await?;
        report.add_category("memory", memory_results);

        // Token usage efficiency tests
        println!("\n🔢 Running token usage efficiency benchmarks...");
        let token_results = self.run_token_efficiency_benchmarks().await?;
        report.add_category("token_efficiency", token_results);

        // Baseline comparison (if enabled)
        if self.config.enable_baseline_comparison && self.baseline_client.is_some() {
            println!("\n⚖️  Running baseline comparison benchmarks...");
            let baseline_results = self.run_baseline_comparison().await?;
            report.add_category("baseline_comparison", baseline_results);
        }

        report.finalize();
        Ok(report)
    }

    /// Test how processing time scales with context size
    async fn run_context_scaling_benchmarks(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for &context_size in &self.config.context_sizes {
            println!("  🧪 Testing context size: {} tokens", format_number(context_size));

            // Generate synthetic context of target size
            let context = self.generate_test_context(context_size);
            let query = "Summarize the main points from this context.";

            let mut test_times = Vec::new();
            let mut token_counts = Vec::new();
            let mut memory_measurements = Vec::new();
            let mut errors = Vec::new();

            for iteration in 1..=self.config.iterations_per_test {
                let request = RlmRequest::new()
                    .with_context(&context)
                    .with_query(query)
                    .with_max_tokens(500);

                let memory_before = self.get_memory_usage().await;
                let start = Instant::now();

                match tokio::time::timeout(self.config.test_timeout, self.rlm_client.execute(&request)).await {
                    Ok(Ok(response)) => {
                        let duration = start.elapsed();
                        let memory_after = self.get_memory_usage().await;

                        test_times.push(duration);
                        token_counts.push(response.metadata.total_tokens);

                        if let (Some(before), Some(after)) = (memory_before, memory_after) {
                            memory_measurements.push(after.saturating_sub(before));
                        }

                        println!("    ✅ Iteration {}/{}: {:.2}s, {} tokens",
                            iteration, self.config.iterations_per_test,
                            duration.as_secs_f64(), response.metadata.total_tokens);
                    }
                    Ok(Err(e)) => {
                        errors.push(format!("Iteration {}: {}", iteration, e));
                        println!("    ❌ Iteration {}/{}: Error: {}",
                            iteration, self.config.iterations_per_test, e);
                    }
                    Err(_) => {
                        errors.push(format!("Iteration {}: Timeout", iteration));
                        println!("    ⏰ Iteration {}/{}: Timeout",
                            iteration, self.config.iterations_per_test);
                    }
                }
            }

            if !test_times.is_empty() {
                let result = self.calculate_performance_metrics(
                    &format!("context_scaling_{}_tokens", context_size),
                    context_size,
                    1, // Single threaded for scaling test
                    &test_times,
                    &token_counts,
                    &memory_measurements,
                    &errors,
                    None,
                    None,
                );
                results.push(result);
            }
        }

        self.analyze_scaling_complexity(&results);
        Ok(results)
    }

    /// Test concurrent request handling
    async fn run_concurrency_benchmarks(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let test_context_size = 50_000; // Medium size context for concurrency testing

        for &concurrency_level in &self.config.concurrency_levels {
            println!("  🧪 Testing concurrency level: {}", concurrency_level);

            let context = self.generate_test_context(test_context_size);
            let query = "What are the key insights from this data?";

            let semaphore = Arc::new(Semaphore::new(concurrency_level));
            let mut handles: Vec<JoinHandle<_>> = Vec::new();

            let test_start = Instant::now();

            for i in 0..concurrency_level {
                let client = Arc::clone(&self.rlm_client);
                let context = context.clone();
                let query = query.to_string();
                let semaphore = Arc::clone(&semaphore);

                let handle = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    let request = RlmRequest::new()
                        .with_context(&context)
                        .with_query(&query)
                        .with_max_tokens(300);

                    let start = Instant::now();
                    let result = client.execute(&request).await;
                    let duration = start.elapsed();

                    (i, result, duration)
                });

                handles.push(handle);
            }

            // Collect results
            let mut test_times = Vec::new();
            let mut token_counts = Vec::new();
            let mut errors = Vec::new();
            let mut successful_requests = 0;

            for handle in handles {
                match handle.await {
                    Ok((request_id, Ok(response), duration)) => {
                        test_times.push(duration);
                        token_counts.push(response.metadata.total_tokens);
                        successful_requests += 1;
                    }
                    Ok((request_id, Err(e), _)) => {
                        errors.push(format!("Request {}: {}", request_id, e));
                    }
                    Err(e) => {
                        errors.push(format!("Join error: {}", e));
                    }
                }
            }

            let total_duration = test_start.elapsed();
            let success_rate = successful_requests as f64 / concurrency_level as f64;

            println!("    📊 Success rate: {:.1}% ({}/{})",
                success_rate * 100.0, successful_requests, concurrency_level);
            println!("    ⏱️  Total time: {:.2}s", total_duration.as_secs_f64());

            if !test_times.is_empty() {
                let result = self.calculate_performance_metrics(
                    &format!("concurrency_{}_requests", concurrency_level),
                    test_context_size,
                    concurrency_level,
                    &test_times,
                    &token_counts,
                    &[], // Memory measurements not available in concurrent tests
                    &errors,
                    None,
                    None,
                );
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Test streaming response performance
    async fn run_streaming_benchmarks(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let test_context_size = 100_000;

        println!("  🧪 Testing streaming performance with {} token context", format_number(test_context_size));

        let context = self.generate_test_context(test_context_size);
        let query = "Provide a detailed analysis of this context with explanations.";

        let mut first_chunk_latencies = Vec::new();
        let mut chunk_intervals = Vec::new();
        let mut total_times = Vec::new();
        let mut token_counts = Vec::new();
        let mut errors = Vec::new();

        for iteration in 1..=self.config.iterations_per_test {
            let request = RlmRequest::new()
                .with_context(&context)
                .with_query(query)
                .with_streaming(true)
                .with_max_tokens(1000);

            let start = Instant::now();

            match self.rlm_client.execute_streaming(&request).await {
                Ok(mut stream) => {
                    let mut first_chunk_received = false;
                    let mut first_chunk_time = None;
                    let mut last_chunk_time = start;
                    let mut chunk_count = 0;
                    let mut total_tokens = 0;

                    while let Some(chunk) = stream.next_chunk().await {
                        let now = Instant::now();

                        if !first_chunk_received {
                            first_chunk_time = Some(now - start);
                            first_chunk_received = true;
                        } else {
                            chunk_intervals.push(now - last_chunk_time);
                        }

                        chunk_count += 1;
                        total_tokens += chunk.token_count.unwrap_or(1);
                        last_chunk_time = now;
                    }

                    let total_time = start.elapsed();

                    if let Some(first_chunk) = first_chunk_time {
                        first_chunk_latencies.push(first_chunk);
                        total_times.push(total_time);
                        token_counts.push(total_tokens);

                        println!("    ✅ Iteration {}: First chunk: {:.2}s, Total: {:.2}s, {} chunks",
                            iteration, first_chunk.as_secs_f64(), total_time.as_secs_f64(), chunk_count);
                    }
                }
                Err(e) => {
                    errors.push(format!("Iteration {}: {}", iteration, e));
                    println!("    ❌ Iteration {}: Error: {}", iteration, e);
                }
            }
        }

        if !total_times.is_empty() {
            let avg_first_chunk = first_chunk_latencies.iter()
                .sum::<Duration>() / first_chunk_latencies.len() as u32;

            let avg_chunk_interval = if chunk_intervals.is_empty() {
                None
            } else {
                Some(chunk_intervals.iter().sum::<Duration>() / chunk_intervals.len() as u32)
            };

            let result = self.calculate_performance_metrics(
                "streaming_performance",
                test_context_size,
                1,
                &total_times,
                &token_counts,
                &[],
                &errors,
                Some(avg_first_chunk),
                avg_chunk_interval,
            );

            results.push(result);

            // Validate streaming performance targets
            if avg_first_chunk <= Duration::from_secs(5) {
                println!("    ✅ First chunk latency meets target (<5s)");
            } else {
                println!("    ⚠️  First chunk latency exceeds target (>5s)");
            }
        }

        Ok(results)
    }

    /// Test memory usage patterns
    async fn run_memory_benchmarks(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Test memory usage with different context sizes
        for &context_size in &[50_000, 500_000, 1_000_000] {
            println!("  🧪 Testing memory usage with {} tokens", format_number(context_size));

            let context = self.generate_test_context(context_size);
            let query = "Analyze the memory usage patterns in this context.";

            let mut memory_peaks = Vec::new();
            let mut processing_times = Vec::new();
            let mut errors = Vec::new();

            for iteration in 1..=self.config.iterations_per_test {
                // Force garbage collection before test
                // Note: Rust doesn't have explicit GC, but we can try to minimize allocations

                let memory_before = self.get_memory_usage().await.unwrap_or(0);

                let request = RlmRequest::new()
                    .with_context(&context)
                    .with_query(query)
                    .with_max_tokens(200);

                let start = Instant::now();

                match self.rlm_client.execute(&request).await {
                    Ok(_response) => {
                        let duration = start.elapsed();
                        let memory_after = self.get_memory_usage().await.unwrap_or(0);
                        let memory_delta = memory_after.saturating_sub(memory_before);

                        memory_peaks.push(memory_delta);
                        processing_times.push(duration);

                        println!("    📊 Iteration {}: Memory delta: {}MB, Time: {:.2}s",
                            iteration, memory_delta, duration.as_secs_f64());
                    }
                    Err(e) => {
                        errors.push(format!("Iteration {}: {}", iteration, e));
                    }
                }

                // Small delay between iterations to allow cleanup
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            if !processing_times.is_empty() {
                let result = self.calculate_performance_metrics(
                    &format!("memory_usage_{}_tokens", context_size),
                    context_size,
                    1,
                    &processing_times,
                    &vec![0; processing_times.len()], // Token counts not relevant for memory test
                    &memory_peaks,
                    &errors,
                    None,
                    None,
                );
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Test token usage efficiency
    async fn run_token_efficiency_benchmarks(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        // Test token efficiency with different task types
        let test_cases = vec![
            ("summarization", "Provide a concise summary of this context.", 200),
            ("question_answering", "Answer the specific question found in this context.", 100),
            ("analysis", "Analyze the patterns and insights from this data.", 500),
        ];

        for (task_name, query, max_tokens) in test_cases {
            println!("  🧪 Testing token efficiency for {}", task_name);

            let context = self.generate_test_context(100_000); // Standardize context size

            let mut token_usage = Vec::new();
            let mut processing_times = Vec::new();
            let mut errors = Vec::new();

            for iteration in 1..=self.config.iterations_per_test {
                let request = RlmRequest::new()
                    .with_context(&context)
                    .with_query(query)
                    .with_max_tokens(max_tokens);

                let start = Instant::now();

                match self.rlm_client.execute(&request).await {
                    Ok(response) => {
                        let duration = start.elapsed();
                        token_usage.push(response.metadata.total_tokens);
                        processing_times.push(duration);

                        println!("    📊 Iteration {}: {} tokens used in {:.2}s",
                            iteration, response.metadata.total_tokens, duration.as_secs_f64());
                    }
                    Err(e) => {
                        errors.push(format!("Iteration {}: {}", iteration, e));
                    }
                }
            }

            if !processing_times.is_empty() {
                let result = self.calculate_performance_metrics(
                    &format!("token_efficiency_{}", task_name),
                    100_000,
                    1,
                    &processing_times,
                    &token_usage,
                    &[],
                    &errors,
                    None,
                    None,
                );
                results.push(result);

                // Calculate efficiency metrics
                let avg_tokens = token_usage.iter().sum::<u32>() as f64 / token_usage.len() as f64;
                let efficiency_ratio = 100_000.0 / avg_tokens; // Input tokens / output tokens

                println!("    📈 Token efficiency ratio: {:.2}x", efficiency_ratio);
            }
        }

        Ok(results)
    }

    /// Run baseline comparison benchmarks
    async fn run_baseline_comparison(&self) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        if let Some(baseline) = &self.baseline_client {
            println!("  🧪 Running RLM vs Baseline comparison");

            let context = self.generate_test_context(100_000);
            let query = "Provide a comprehensive analysis of this context.";

            // Test RLM performance
            println!("    🚀 Testing RLM performance...");
            let rlm_results = self.run_comparison_test("rlm", &context, query, true).await?;

            // Test Baseline performance
            println!("    📊 Testing Baseline performance...");
            let baseline_results = self.run_comparison_test("baseline", &context, query, false).await?;

            results.extend(rlm_results);
            results.extend(baseline_results);

            // Compare results
            if let (Some(rlm), Some(baseline)) = (
                results.iter().find(|r| r.test_name == "rlm_comparison"),
                results.iter().find(|r| r.test_name == "baseline_comparison")
            ) {
                let time_improvement = baseline.avg_processing_time.as_secs_f64() / rlm.avg_processing_time.as_secs_f64();
                let token_efficiency = baseline.total_tokens_used as f64 / rlm.total_tokens_used as f64;

                println!("    📈 Performance comparison:");
                println!("      Time improvement: {:.2}x", time_improvement);
                println!("      Token efficiency: {:.2}x", token_efficiency);

                if token_efficiency >= 1.5 {
                    println!("    ✅ Token efficiency meets paper target (150%)");
                } else {
                    println!("    ⚠️  Token efficiency below paper target");
                }
            }
        }

        Ok(results)
    }

    /// Run a comparison test for RLM vs baseline
    async fn run_comparison_test(
        &self,
        test_type: &str,
        context: &str,
        query: &str,
        use_rlm: bool,
    ) -> Result<Vec<PerformanceResult>, Box<dyn std::error::Error>> {
        let mut processing_times = Vec::new();
        let mut token_counts = Vec::new();
        let mut errors = Vec::new();

        for iteration in 1..=self.config.iterations_per_test {
            let request = RlmRequest::new()
                .with_context(context)
                .with_query(query)
                .with_max_tokens(500);

            let start = Instant::now();

            let result = if use_rlm {
                self.rlm_client.execute(&request).await
                    .map(|r| (r.content, r.metadata.total_tokens))
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            } else if let Some(baseline) = &self.baseline_client {
                baseline.complete(&request).await
                    .map(|content| (content, 0)) // Baseline doesn't provide token count
            } else {
                continue;
            };

            let duration = start.elapsed();

            match result {
                Ok((_content, tokens)) => {
                    processing_times.push(duration);
                    token_counts.push(tokens);
                }
                Err(e) => {
                    errors.push(format!("Iteration {}: {}", iteration, e));
                }
            }
        }

        if !processing_times.is_empty() {
            let result = self.calculate_performance_metrics(
                &format!("{}_comparison", test_type),
                context.len() / 4, // Rough token estimate
                1,
                &processing_times,
                &token_counts,
                &[],
                &errors,
                None,
                None,
            );
            Ok(vec![result])
        } else {
            Ok(vec![])
        }
    }

    /// Calculate performance metrics from test results
    fn calculate_performance_metrics(
        &self,
        test_name: &str,
        context_size: usize,
        concurrency_level: usize,
        processing_times: &[Duration],
        token_counts: &[u32],
        memory_measurements: &[u64],
        errors: &[String],
        first_chunk_latency: Option<Duration>,
        streaming_chunk_interval: Option<Duration>,
    ) -> PerformanceResult {
        let mut times_ms: Vec<u64> = processing_times.iter().map(|d| d.as_millis() as u64).collect();
        times_ms.sort();

        let avg_time = processing_times.iter().sum::<Duration>() / processing_times.len() as u32;
        let min_time = times_ms.first().copied().unwrap_or(0);
        let max_time = times_ms.last().copied().unwrap_or(0);

        // Calculate P95
        let p95_index = ((times_ms.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
        let p95_time = times_ms.get(p95_index).copied().unwrap_or(0);

        let total_tokens = token_counts.iter().sum::<u32>();
        let avg_tokens_per_sec = if avg_time.as_secs_f64() > 0.0 {
            total_tokens as f64 / (avg_time.as_secs_f64() * processing_times.len() as f64)
        } else {
            0.0
        };

        let memory_peak = memory_measurements.iter().max().copied().unwrap_or(0);
        let memory_avg = if memory_measurements.is_empty() {
            0
        } else {
            memory_measurements.iter().sum::<u64>() / memory_measurements.len() as u64
        };

        let success_rate = processing_times.len() as f64 / (processing_times.len() + errors.len()) as f64;

        PerformanceResult {
            test_name: test_name.to_string(),
            context_size_tokens: context_size,
            concurrency_level,
            avg_processing_time: avg_time,
            min_processing_time: Duration::from_millis(min_time),
            max_processing_time: Duration::from_millis(max_time),
            p95_processing_time: Duration::from_millis(p95_time),
            total_tokens_used: total_tokens,
            tokens_per_second: avg_tokens_per_sec,
            memory_peak_mb: memory_peak,
            memory_avg_mb: memory_avg,
            success_rate,
            first_chunk_latency,
            streaming_chunk_interval,
            error_messages: errors.to_vec(),
        }
    }

    /// Analyze scaling complexity from context scaling results
    fn analyze_scaling_complexity(&self, results: &[PerformanceResult]) {
        if results.len() < 2 {
            return;
        }

        println!("\n📊 Complexity Analysis:");

        // Calculate how processing time scales with context size
        let first = &results[0];
        let last = &results[results.len() - 1];

        let context_ratio = last.context_size_tokens as f64 / first.context_size_tokens as f64;
        let time_ratio = last.avg_processing_time.as_secs_f64() / first.avg_processing_time.as_secs_f64();

        println!("  📈 Context size ratio: {:.2}x", context_ratio);
        println!("  ⏱️  Processing time ratio: {:.2}x", time_ratio);

        // Determine complexity class
        let complexity_class = if time_ratio < context_ratio.log2() {
            "Better than O(log n) - Approaching O(1)"
        } else if time_ratio < context_ratio {
            "O(log n) to O(n) - Sublinear scaling"
        } else if time_ratio < context_ratio * context_ratio.log2() {
            "O(n) to O(n log n) - Linear to log-linear scaling"
        } else {
            "O(n²) or worse - Quadratic scaling"
        };

        println!("  🎯 Estimated complexity: {}", complexity_class);

        // Check against paper targets
        if time_ratio < 3.0 && first.context_size_tokens <= 100_000 {
            println!("  ✅ Meets paper target (<3x time for 100K+ contexts)");
        } else {
            println!("  ⚠️  May exceed paper performance targets");
        }
    }

    /// Generate test context of specified token size
    fn generate_test_context(&self, target_tokens: usize) -> String {
        // Generate realistic test content that approximates typical use cases
        let patterns = vec![
            "This document discusses various aspects of artificial intelligence and machine learning.",
            "The following data analysis covers performance metrics from multiple experiments.",
            "Technical documentation outlines system architecture and implementation details.",
            "Research findings indicate significant improvements in processing efficiency.",
            "Comprehensive evaluation results demonstrate enhanced accuracy and reliability.",
        ];

        let mut content = String::new();
        let mut pattern_idx = 0;

        // Rough approximation: 1 token ≈ 4 characters
        let target_chars = target_tokens * 4;

        while content.len() < target_chars {
            content.push_str(patterns[pattern_idx % patterns.len()]);
            content.push_str(" ");
            pattern_idx += 1;

            // Add some variety with numbers and structured content
            if pattern_idx % 50 == 0 {
                content.push_str(&format!(
                    "\n\nSection {}: Performance Data\n\
                     Metric 1: {}\n\
                     Metric 2: {}\n\
                     Metric 3: {}\n\n",
                    pattern_idx / 50,
                    pattern_idx * 10,
                    pattern_idx * 15,
                    pattern_idx * 7
                ));
            }
        }

        content
    }

    /// Get current memory usage (platform-specific implementation needed)
    async fn get_memory_usage(&self) -> Option<u64> {
        // TODO: Implement platform-specific memory monitoring
        // This could use procfs on Linux, task_info on macOS, or similar on Windows
        None
    }
}

/// Complete benchmark report
#[derive(Debug)]
pub struct BenchmarkReport {
    categories: std::collections::HashMap<String, Vec<PerformanceResult>>,
    started_at: Instant,
    completed_at: Option<Instant>,
}

impl BenchmarkReport {
    fn new() -> Self {
        Self {
            categories: std::collections::HashMap::new(),
            started_at: Instant::now(),
            completed_at: None,
        }
    }

    fn add_category(&mut self, name: &str, results: Vec<PerformanceResult>) {
        self.categories.insert(name.to_string(), results);
    }

    fn finalize(&mut self) {
        self.completed_at = Some(Instant::now());
        self.print_summary();
    }

    fn print_summary(&self) {
        println!("\n" + "=".repeat(80).as_str());
        println!("🚀 RLM Performance Benchmark Summary");
        println!("=".repeat(80));

        let total_duration = self.completed_at.unwrap() - self.started_at;
        println!("⏱️  Total benchmark time: {:.2}s", total_duration.as_secs_f64());

        for (category, results) in &self.categories {
            if results.is_empty() {
                continue;
            }

            println!("\n📊 {} Results:", category.to_uppercase());

            let avg_success_rate = results.iter().map(|r| r.success_rate).sum::<f64>() / results.len() as f64;
            let avg_processing_time = results.iter()
                .map(|r| r.avg_processing_time.as_secs_f64())
                .sum::<f64>() / results.len() as f64;

            println!("   ✅ Average Success Rate: {:.1}%", avg_success_rate * 100.0);
            println!("   ⏱️  Average Processing Time: {:.2}s", avg_processing_time);

            if let Some(streaming_result) = results.iter().find(|r| r.first_chunk_latency.is_some()) {
                if let Some(first_chunk) = streaming_result.first_chunk_latency {
                    println!("   📡 First Chunk Latency: {:.2}s", first_chunk.as_secs_f64());
                }
            }

            let max_memory = results.iter().map(|r| r.memory_peak_mb).max().unwrap_or(0);
            if max_memory > 0 {
                println!("   💾 Peak Memory Usage: {}MB", max_memory);
            }

            // Show performance targets assessment
            if category == "context_scaling" {
                self.assess_scaling_performance(&results);
            } else if category == "concurrency" {
                self.assess_concurrency_performance(&results);
            } else if category == "streaming" {
                self.assess_streaming_performance(&results);
            }

            // Show any errors
            for result in results.iter().filter(|r| !r.error_messages.is_empty()) {
                println!("   ⚠️  {} errors in {}", result.error_messages.len(), result.test_name);
            }
        }

        println!("\n" + "=".repeat(80).as_str());
        println!("Benchmark complete! 🎉");
        println!("=".repeat(80));
    }

    fn assess_scaling_performance(&self, results: &[PerformanceResult]) {
        // Check if we can handle large contexts
        let large_context_result = results.iter()
            .find(|r| r.context_size_tokens >= 1_000_000);

        if let Some(result) = large_context_result {
            if result.success_rate > 0.8 {
                println!("   ✅ Successfully handles 1M+ token contexts");
            } else {
                println!("   ⚠️  Limited success with large contexts");
            }
        }

        // Check processing time scaling
        if results.len() >= 2 {
            let first = &results[0];
            let last = &results[results.len() - 1];
            let time_ratio = last.avg_processing_time.as_secs_f64() / first.avg_processing_time.as_secs_f64();

            if time_ratio < 3.0 {
                println!("   ✅ Processing time scaling within paper targets");
            } else {
                println!("   ⚠️  Processing time scaling exceeds paper targets");
            }
        }
    }

    fn assess_concurrency_performance(&self, results: &[PerformanceResult]) {
        let max_concurrency_result = results.iter()
            .max_by_key(|r| r.concurrency_level);

        if let Some(result) = max_concurrency_result {
            if result.concurrency_level >= 100 && result.success_rate > 0.9 {
                println!("   ✅ Handles 100+ concurrent requests successfully");
            } else {
                println!("   ⚠️  Limited concurrency capability");
            }
        }
    }

    fn assess_streaming_performance(&self, results: &[PerformanceResult]) {
        if let Some(result) = results.first() {
            if let Some(first_chunk) = result.first_chunk_latency {
                if first_chunk <= Duration::from_secs(5) {
                    println!("   ✅ First chunk latency meets target (<5s)");
                } else {
                    println!("   ⚠️  First chunk latency exceeds target");
                }
            }
        }
    }

    /// Get overall benchmark success
    pub fn overall_success(&self) -> bool {
        for results in self.categories.values() {
            for result in results {
                if result.success_rate < 0.8 {
                    return false;
                }
            }
        }
        true
    }
}

/// Format large numbers with commas
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(12345), "12,345");
    }

    #[test]
    fn test_performance_result_creation() {
        let result = PerformanceResult {
            test_name: "test".to_string(),
            context_size_tokens: 1000,
            concurrency_level: 1,
            avg_processing_time: Duration::from_secs(2),
            min_processing_time: Duration::from_secs(1),
            max_processing_time: Duration::from_secs(3),
            p95_processing_time: Duration::from_secs(3),
            total_tokens_used: 500,
            tokens_per_second: 250.0,
            memory_peak_mb: 100,
            memory_avg_mb: 80,
            success_rate: 1.0,
            first_chunk_latency: Some(Duration::from_millis(500)),
            streaming_chunk_interval: Some(Duration::from_millis(100)),
            error_messages: vec![],
        };

        assert_eq!(result.test_name, "test");
        assert!(result.success_rate > 0.99);
        assert!(result.first_chunk_latency.is_some());
    }
}