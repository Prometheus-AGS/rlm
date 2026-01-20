//! Example: Performance Optimization for Large Context Processing
//!
//! This example demonstrates how to configure and use RLM's performance optimizations
//! for handling large contexts efficiently. It showcases different optimization strategies
//! and their impact on processing time and memory usage.

use rlm_core::{
    RlmConfig, RlmExecutor, RlmRequest, OptimizedContextProcessor, PerformanceConfig,
    ContextAnalyzer, ContextAnalysisConfig,
};
use rlm_repl_rhai::RhaiReplBackend;
use rlm_server::{OpenAiProvider, OpenAiConfig, PerformanceProfiles, PerformanceUtils};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for performance monitoring
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting RLM Performance Optimization Example");

    // Demonstrate different performance optimization scenarios
    demonstrate_basic_optimization().await?;
    demonstrate_parallel_chunking().await?;
    demonstrate_context_caching().await?;
    demonstrate_adaptive_chunking().await?;
    demonstrate_performance_profiles().await?;

    Ok(())
}

/// Demonstrate basic performance optimization vs standard processing
async fn demonstrate_basic_optimization() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Basic Performance Optimization Demo ===");

    // Create a large context (simulating a large document)
    let large_context = create_large_test_context(1_000_000); // 1MB context
    info!("Created test context: {} bytes", large_context.len());

    // Test 1: Standard processing without optimizations
    let start_time = Instant::now();
    let standard_analyzer = ContextAnalyzer::new();
    let analysis = standard_analyzer.analyze(&large_context).await?;
    let standard_chunks = standard_analyzer.chunk_context(&large_context, &analysis).await?;
    let standard_time = start_time.elapsed().as_millis();

    info!(
        "Standard processing: {} chunks in {}ms",
        standard_chunks.len(),
        standard_time
    );

    // Test 2: Optimized processing with performance configuration
    let start_time = Instant::now();
    let perf_config = PerformanceConfig {
        enable_parallel_chunking: true,
        max_concurrent_chunks: 8,
        enable_adaptive_chunking: true,
        ..Default::default()
    };
    let processor = OptimizedContextProcessor::new(perf_config);
    let processed = processor.process_context(&large_context).await?;
    let optimized_time = start_time.elapsed().as_millis();

    info!(
        "Optimized processing: {} chunks in {}ms (speedup: {:.2}x)",
        processed.chunks.len(),
        optimized_time,
        standard_time as f64 / optimized_time as f64
    );

    Ok(())
}

/// Demonstrate parallel chunking benefits
async fn demonstrate_parallel_chunking() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Parallel Chunking Demo ===");

    let large_context = create_large_test_context(2_000_000); // 2MB context

    // Test different levels of parallelism
    let parallelism_levels = [1, 2, 4, 8, 16];

    for &max_concurrent in &parallelism_levels {
        let start_time = Instant::now();

        let perf_config = PerformanceConfig {
            enable_parallel_chunking: true,
            max_concurrent_chunks: max_concurrent,
            enable_context_caching: false, // Disable caching for fair comparison
            ..Default::default()
        };

        let processor = OptimizedContextProcessor::new(perf_config);
        let processed = processor.process_context(&large_context).await?;
        let processing_time = start_time.elapsed().as_millis();

        info!(
            "Parallel level {}: {} chunks in {}ms",
            max_concurrent,
            processed.chunks.len(),
            processing_time
        );
    }

    Ok(())
}

/// Demonstrate context caching benefits
async fn demonstrate_context_caching() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Context Caching Demo ===");

    let test_context = create_large_test_context(500_000); // 500KB context

    let perf_config = PerformanceConfig {
        enable_context_caching: true,
        cache_ttl_seconds: 3600,
        max_cache_size_mb: 100,
        ..Default::default()
    };

    let processor = OptimizedContextProcessor::new(perf_config);

    // First processing - should not be cached
    let start_time = Instant::now();
    let result1 = processor.process_context(&test_context).await?;
    let first_time = start_time.elapsed().as_millis();

    info!(
        "First processing: {}ms, cache_hit={}, compression_ratio={:.2}",
        first_time,
        result1.cache_hit,
        result1.compression_ratio
    );

    // Second processing - should be cached
    let start_time = Instant::now();
    let result2 = processor.process_context(&test_context).await?;
    let second_time = start_time.elapsed().as_millis();

    info!(
        "Second processing: {}ms, cache_hit={}, compression_ratio={:.2}, speedup: {:.2}x",
        second_time,
        result2.cache_hit,
        result2.compression_ratio,
        first_time as f64 / second_time as f64
    );

    // Show cache statistics
    let stats = processor.get_performance_stats().await;
    info!(
        "Cache stats: {} entries, {} bytes, {} total hits",
        stats.cache_entries,
        stats.cache_size_bytes,
        stats.total_cache_hits
    );

    Ok(())
}

/// Demonstrate adaptive chunking
async fn demonstrate_adaptive_chunking() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Adaptive Chunking Demo ===");

    // Test different content types
    let code_context = create_code_context();
    let doc_context = create_documentation_context();
    let structured_context = create_structured_data_context();

    let perf_config = PerformanceConfig {
        enable_adaptive_chunking: true,
        enable_context_caching: false,
        ..Default::default()
    };

    let processor = OptimizedContextProcessor::new(perf_config);

    // Process different content types
    for (name, context) in [
        ("Code", code_context),
        ("Documentation", doc_context),
        ("Structured Data", structured_context),
    ] {
        let start_time = Instant::now();
        let processed = processor.process_context(&context).await?;
        let processing_time = start_time.elapsed().as_millis();

        info!(
            "{} content: {} bytes → {} chunks in {}ms",
            name,
            context.len(),
            processed.chunks.len(),
            processing_time
        );
    }

    Ok(())
}

/// Demonstrate different performance profiles
async fn demonstrate_performance_profiles() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Performance Profiles Demo ===");

    let test_context = create_large_test_context(1_500_000); // 1.5MB context

    // Test different performance profiles
    let profiles = [
        ("Development", PerformanceProfiles::development()),
        ("Production", PerformanceProfiles::production()),
        ("High Throughput", PerformanceProfiles::high_throughput()),
        ("Memory Constrained", PerformanceProfiles::memory_constrained()),
    ];

    for (name, config) in profiles {
        let start_time = Instant::now();

        let processor = OptimizedContextProcessor::new(config.core_config.clone());
        let processed = processor.process_context(&test_context).await?;
        let processing_time = start_time.elapsed().as_millis();

        info!(
            "{} profile: {}ms, {} chunks, cache_hit={}",
            name,
            processing_time,
            processed.chunks.len(),
            processed.cache_hit
        );
    }

    // Show performance recommendations
    let recommendations = PerformanceUtils::get_recommendations();
    info!("\nPerformance Recommendations:");
    for rec in recommendations {
        info!("  • {}", rec);
    }

    Ok(())
}

/// Demonstrate full RLM executor with performance optimizations
#[allow(dead_code)]
async fn demonstrate_full_executor_optimization() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Full Executor Performance Demo ===");

    // Create REPL backend
    let repl = Arc::new(Mutex::new(RhaiReplBackend::new()?));

    // Create LLM provider (requires API key)
    let llm_config = OpenAiConfig::builder()
        .api_key(std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test-key".to_string()))
        .build()?;
    let llm = Arc::new(OpenAiProvider::new(llm_config));

    // Create optimized context processor
    let perf_config = PerformanceProfiles::production();
    let processor = OptimizedContextProcessor::new(perf_config.core_config);

    // Create RLM executor with performance optimizations
    let rlm_config = RlmConfig::default();
    let executor = RlmExecutor::new(rlm_config, repl, llm)
        .with_context_processor(Arc::new(processor));

    // Create test request with large context
    let large_context = create_large_test_context(800_000); // 800KB
    let request = RlmRequest {
        query: "Analyze the key themes and patterns in this large document".to_string(),
        context: large_context,
        max_iterations: 5,
        recursion_depth: 2,
    };

    info!("Processing large context with optimized executor...");
    let start_time = Instant::now();

    // Note: This would require a valid API key for actual execution
    // let response = executor.execute(request).await?;
    // let total_time = start_time.elapsed().as_millis();

    // info!(
    //     "Executor completed in {}ms, tokens used: {}",
    //     total_time,
    //     response.metadata.total_tokens
    // );

    info!("Demo completed (actual execution requires valid API key)");

    Ok(())
}

// Helper functions for creating test contexts

fn create_large_test_context(target_size: usize) -> String {
    let base_text = "This is a sample document for performance testing. It contains various types of content including technical descriptions, code snippets, and analytical data. ";
    let mut content = String::with_capacity(target_size + base_text.len());

    while content.len() < target_size {
        content.push_str(base_text);
        content.push_str(&format!("Section {} continues with more detailed information. ", content.len() / 1000));
    }

    content.truncate(target_size);
    content
}

fn create_code_context() -> String {
    r#"
    // Example Rust code for performance testing
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    pub struct DataProcessor {
        cache: RwLock<HashMap<String, Vec<u8>>>,
        config: ProcessorConfig,
    }

    impl DataProcessor {
        pub fn new(config: ProcessorConfig) -> Self {
            Self {
                cache: RwLock::new(HashMap::new()),
                config,
            }
        }

        pub async fn process(&self, input: &str) -> Result<Vec<u8>, ProcessorError> {
            let cache_key = self.calculate_key(input);

            {
                let cache = self.cache.read().await;
                if let Some(cached_result) = cache.get(&cache_key) {
                    return Ok(cached_result.clone());
                }
            }

            let result = self.perform_processing(input).await?;

            {
                let mut cache = self.cache.write().await;
                cache.insert(cache_key, result.clone());
            }

            Ok(result)
        }

        async fn perform_processing(&self, input: &str) -> Result<Vec<u8>, ProcessorError> {
            // Complex processing logic here
            tokio::time::sleep(self.config.processing_delay).await;
            Ok(input.as_bytes().to_vec())
        }

        fn calculate_key(&self, input: &str) -> String {
            // Simple hash calculation
            format!("key_{}", input.len())
        }
    }

    #[derive(Debug, Clone)]
    pub struct ProcessorConfig {
        pub processing_delay: std::time::Duration,
        pub max_cache_size: usize,
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ProcessorError {
        #[error("Processing failed: {0}")]
        ProcessingFailed(String),
        #[error("Cache error: {0}")]
        CacheError(String),
    }
    "#.repeat(20) // Repeat to make it larger
}

fn create_documentation_context() -> String {
    r#"
    # Performance Optimization Guide

    ## Overview

    This guide covers performance optimization strategies for large-scale applications.
    Performance optimization is crucial for maintaining responsiveness and efficiency
    as your application scales.

    ## Key Strategies

    ### 1. Caching

    Implement multi-level caching to reduce redundant computations:

    - **Memory caching**: For frequently accessed data
    - **Distributed caching**: For shared data across instances
    - **CDN caching**: For static assets and content

    ### 2. Parallel Processing

    Utilize parallel processing for CPU-intensive tasks:

    - **Thread pools**: For concurrent task execution
    - **Async operations**: For I/O-bound operations
    - **Worker queues**: For background processing

    ### 3. Data Optimization

    Optimize data structures and algorithms:

    - **Efficient data structures**: Choose appropriate collections
    - **Algorithm complexity**: Minimize time and space complexity
    - **Data compression**: Reduce memory and network overhead

    ### 4. Monitoring and Profiling

    Continuous monitoring is essential for performance optimization:

    - **Performance metrics**: Track response times, throughput
    - **Resource utilization**: Monitor CPU, memory, disk usage
    - **Error rates**: Track and analyze failure patterns

    ## Implementation Examples

    ### Caching Implementation

    ```rust
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    pub struct Cache<K, V> {
        data: RwLock<HashMap<K, V>>,
        ttl: std::time::Duration,
    }

    impl<K, V> Cache<K, V>
    where
        K: Eq + std::hash::Hash + Clone,
        V: Clone,
    {
        pub async fn get(&self, key: &K) -> Option<V> {
            let cache = self.data.read().await;
            cache.get(key).cloned()
        }

        pub async fn set(&self, key: K, value: V) {
            let mut cache = self.data.write().await;
            cache.insert(key, value);
        }
    }
    ```

    ### Parallel Processing Example

    ```rust
    use tokio::task::JoinSet;

    pub async fn process_in_parallel<T>(
        items: Vec<T>,
        processor: impl Fn(T) -> BoxFuture<Result<(), Error>> + Send + Sync + 'static,
    ) -> Result<(), Error> {
        let mut join_set = JoinSet::new();

        for item in items {
            join_set.spawn(processor(item));
        }

        while let Some(result) = join_set.join_next().await {
            result??;
        }

        Ok(())
    }
    ```

    ## Best Practices

    1. **Profile before optimizing**: Always measure first
    2. **Focus on bottlenecks**: Optimize the slowest parts first
    3. **Test thoroughly**: Ensure optimizations don't break functionality
    4. **Monitor continuously**: Track performance over time
    5. **Document changes**: Keep records of optimization efforts

    ## Common Pitfalls

    - **Premature optimization**: Optimizing before identifying bottlenecks
    - **Over-engineering**: Adding unnecessary complexity
    - **Ignoring trade-offs**: Every optimization has costs
    - **Not measuring impact**: Failing to verify optimization benefits

    ## Conclusion

    Performance optimization is an ongoing process that requires careful analysis,
    methodical implementation, and continuous monitoring. By following these
    guidelines and best practices, you can build applications that scale
    effectively and provide excellent user experiences.
    "#.repeat(10) // Repeat to make it larger
}

fn create_structured_data_context() -> String {
    let json_data = r#"{
        "performance_metrics": {
            "response_times": {
                "p50": 45.2,
                "p95": 158.7,
                "p99": 342.1
            },
            "throughput": {
                "requests_per_second": 1250.5,
                "bytes_per_second": 15680000
            },
            "error_rates": {
                "client_errors": 0.015,
                "server_errors": 0.002
            },
            "resource_utilization": {
                "cpu_percent": 72.5,
                "memory_percent": 68.3,
                "disk_io_percent": 23.1,
                "network_io_percent": 45.7
            }
        },
        "optimization_history": [
            {
                "timestamp": "2024-01-15T10:30:00Z",
                "change": "Implemented connection pooling",
                "impact": {
                    "response_time_improvement": "15%",
                    "throughput_increase": "23%"
                }
            },
            {
                "timestamp": "2024-01-20T14:15:00Z",
                "change": "Added Redis caching layer",
                "impact": {
                    "cache_hit_rate": "85%",
                    "response_time_improvement": "32%"
                }
            },
            {
                "timestamp": "2024-01-25T09:45:00Z",
                "change": "Optimized database queries",
                "impact": {
                    "query_time_reduction": "40%",
                    "database_load_reduction": "28%"
                }
            }
        ],
        "current_configuration": {
            "cache_settings": {
                "ttl_seconds": 3600,
                "max_size_mb": 1024,
                "eviction_policy": "LRU"
            },
            "connection_pool": {
                "max_connections": 100,
                "min_idle": 10,
                "connection_timeout_ms": 5000
            },
            "rate_limiting": {
                "requests_per_minute": 1000,
                "burst_capacity": 150
            }
        }
    }"#;

    json_data.repeat(100) // Repeat to make it larger
}