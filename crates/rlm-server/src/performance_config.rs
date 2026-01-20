//! Performance configuration for RLM server optimizations.
//!
//! This module provides configuration structures and utilities for enabling
//! performance optimizations in the RLM server, particularly for large context processing.

use rlm_core::{PerformanceConfig, OptimizedContextProcessor};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Server-side performance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPerformanceConfig {
    /// Enable performance optimizations.
    pub enabled: bool,
    /// Core performance configuration.
    pub core_config: PerformanceConfig,
    /// Enable request-level performance metrics logging.
    pub enable_metrics_logging: bool,
    /// Performance metrics log level.
    pub metrics_log_level: String,
    /// Cache warmup on server start.
    pub enable_cache_warmup: bool,
    /// Warmup cache with sample contexts.
    pub warmup_contexts: Vec<String>,
}

impl Default for ServerPerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            core_config: PerformanceConfig::default(),
            enable_metrics_logging: true,
            metrics_log_level: "info".to_string(),
            enable_cache_warmup: false,
            warmup_contexts: vec![
                // Sample contexts for cache warmup
                "Sample document content for performance testing...".repeat(100),
                "Code repository analysis context...".repeat(200),
                "Large dataset processing context...".repeat(500),
            ],
        }
    }
}

/// Performance configuration builder for the server.
#[derive(Debug)]
pub struct ServerPerformanceBuilder {
    config: ServerPerformanceConfig,
}

impl ServerPerformanceBuilder {
    /// Create a new performance configuration builder.
    pub fn new() -> Self {
        Self {
            config: ServerPerformanceConfig::default(),
        }
    }

    /// Enable or disable performance optimizations.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Configure parallel chunking.
    pub fn with_parallel_chunking(mut self, enabled: bool, max_concurrent: usize) -> Self {
        self.config.core_config.enable_parallel_chunking = enabled;
        self.config.core_config.max_concurrent_chunks = max_concurrent;
        self
    }

    /// Configure context caching.
    pub fn with_context_caching(mut self, enabled: bool, ttl_seconds: u64, max_size_mb: usize) -> Self {
        self.config.core_config.enable_context_caching = enabled;
        self.config.core_config.cache_ttl_seconds = ttl_seconds;
        self.config.core_config.max_cache_size_mb = max_size_mb;
        self
    }

    /// Configure adaptive chunking.
    pub fn with_adaptive_chunking(mut self, enabled: bool) -> Self {
        self.config.core_config.enable_adaptive_chunking = enabled;
        self
    }

    /// Configure context compression.
    pub fn with_compression(mut self, enabled: bool, threshold_bytes: usize) -> Self {
        self.config.core_config.enable_context_compression = enabled;
        self.config.core_config.compression_threshold_bytes = threshold_bytes;
        self
    }

    /// Configure streaming processing.
    pub fn with_streaming(mut self, enabled: bool, chunk_size: usize) -> Self {
        self.config.core_config.enable_streaming_processing = enabled;
        self.config.core_config.stream_chunk_size = chunk_size;
        self
    }

    /// Enable performance metrics logging.
    pub fn with_metrics_logging(mut self, enabled: bool, level: &str) -> Self {
        self.config.enable_metrics_logging = enabled;
        self.config.metrics_log_level = level.to_string();
        self
    }

    /// Configure cache warmup.
    pub fn with_cache_warmup(mut self, enabled: bool, contexts: Vec<String>) -> Self {
        self.config.enable_cache_warmup = enabled;
        self.config.warmup_contexts = contexts;
        self
    }

    /// Build the performance configuration.
    pub fn build(self) -> ServerPerformanceConfig {
        self.config
    }

    /// Create optimized context processor from configuration.
    pub fn build_processor(self) -> Result<Arc<OptimizedContextProcessor>, Box<dyn std::error::Error>> {
        let config = self.config.clone();
        if !config.enabled {
            return Err("Performance optimizations are disabled".into());
        }

        let processor = OptimizedContextProcessor::new(config.core_config);
        Ok(Arc::new(processor))
    }
}

impl Default for ServerPerformanceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance configuration profiles for different use cases.
#[derive(Debug)]
pub struct PerformanceProfiles;

impl PerformanceProfiles {
    /// Development profile - minimal optimizations, good for debugging.
    pub fn development() -> ServerPerformanceConfig {
        ServerPerformanceBuilder::new()
            .enabled(true)
            .with_parallel_chunking(false, 2)
            .with_context_caching(true, 300, 128) // 5 min TTL, 128MB cache
            .with_adaptive_chunking(false)
            .with_compression(false, 1_000_000)
            .with_streaming(false, 8192)
            .with_metrics_logging(true, "debug")
            .with_cache_warmup(false, vec![])
            .build()
    }

    /// Production profile - all optimizations enabled for maximum performance.
    pub fn production() -> ServerPerformanceConfig {
        ServerPerformanceBuilder::new()
            .enabled(true)
            .with_parallel_chunking(true, 8)
            .with_context_caching(true, 3600, 1024) // 1 hour TTL, 1GB cache
            .with_adaptive_chunking(true)
            .with_compression(true, 500_000) // 500KB compression threshold
            .with_streaming(true, 16384)
            .with_metrics_logging(true, "info")
            .with_cache_warmup(true, vec![
                "Large document analysis context...".repeat(1000),
                "Code repository processing context...".repeat(2000),
            ])
            .build()
    }

    /// High-throughput profile - optimized for handling many concurrent requests.
    pub fn high_throughput() -> ServerPerformanceConfig {
        ServerPerformanceBuilder::new()
            .enabled(true)
            .with_parallel_chunking(true, 16) // More concurrent chunks
            .with_context_caching(true, 7200, 2048) // 2 hour TTL, 2GB cache
            .with_adaptive_chunking(true)
            .with_compression(true, 250_000) // Lower compression threshold
            .with_streaming(true, 32768) // Larger stream chunks
            .with_metrics_logging(true, "warn") // Less verbose logging
            .with_cache_warmup(true, vec![
                "High volume processing context...".repeat(5000),
            ])
            .build()
    }

    /// Memory-constrained profile - optimized for systems with limited memory.
    pub fn memory_constrained() -> ServerPerformanceConfig {
        ServerPerformanceBuilder::new()
            .enabled(true)
            .with_parallel_chunking(true, 4) // Fewer concurrent chunks
            .with_context_caching(true, 1800, 256) // 30 min TTL, 256MB cache
            .with_adaptive_chunking(true)
            .with_compression(true, 100_000) // Aggressive compression
            .with_streaming(true, 8192) // Smaller stream chunks
            .with_metrics_logging(false, "error")
            .with_cache_warmup(false, vec![])
            .build()
    }
}

/// Utility functions for performance management.
#[derive(Debug)]
pub struct PerformanceUtils;

impl PerformanceUtils {
    /// Warmup the context processor cache with sample contexts.
    pub async fn warmup_cache(
        processor: &OptimizedContextProcessor,
        contexts: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting cache warmup with {} contexts", contexts.len());

        for (i, context) in contexts.iter().enumerate() {
            match processor.process_context(context).await {
                Ok(result) => {
                    info!(
                        "Warmed up context {} ({} bytes) in {}ms, cache_hit={}",
                        i + 1,
                        context.len(),
                        result.processing_time_ms,
                        result.cache_hit
                    );
                }
                Err(e) => {
                    warn!("Failed to warm up context {}: {}", i + 1, e);
                }
            }
        }

        info!("Cache warmup completed");
        Ok(())
    }

    /// Get performance recommendations based on system resources.
    pub fn get_recommendations() -> Vec<String> {
        let mut recommendations = Vec::new();

        // Basic system checks - in a real implementation, these would check actual system resources
        let available_memory_gb = 8.0; // Placeholder - would use system info
        let cpu_cores = 8; // Placeholder - would use system info

        if available_memory_gb >= 16.0 {
            recommendations.push("Consider using the high_throughput performance profile".to_string());
            recommendations.push("Enable large context caching (2GB+)".to_string());
        } else if available_memory_gb >= 8.0 {
            recommendations.push("Use the production performance profile".to_string());
            recommendations.push("Enable moderate context caching (1GB)".to_string());
        } else {
            recommendations.push("Use the memory_constrained performance profile".to_string());
            recommendations.push("Enable aggressive compression and smaller cache".to_string());
        }

        if cpu_cores >= 8 {
            recommendations.push("Enable parallel chunking with 8+ concurrent chunks".to_string());
        } else if cpu_cores >= 4 {
            recommendations.push("Enable parallel chunking with 4-6 concurrent chunks".to_string());
        } else {
            recommendations.push("Consider disabling parallel chunking on systems with <4 cores".to_string());
        }

        recommendations.push("Monitor cache hit rates and adjust TTL accordingly".to_string());
        recommendations.push("Use streaming processing for contexts >10MB".to_string());
        recommendations.push("Enable compression for contexts >500KB".to_string());

        recommendations
    }

    /// Validate performance configuration.
    pub fn validate_config(config: &ServerPerformanceConfig) -> Result<(), String> {
        if !config.enabled {
            return Ok(()); // No validation needed if disabled
        }

        let core = &config.core_config;

        if core.max_concurrent_chunks == 0 {
            return Err("max_concurrent_chunks must be greater than 0".to_string());
        }

        if core.max_concurrent_chunks > 32 {
            return Err("max_concurrent_chunks should not exceed 32 for stability".to_string());
        }

        if core.cache_ttl_seconds == 0 {
            return Err("cache_ttl_seconds must be greater than 0".to_string());
        }

        if core.max_cache_size_mb == 0 {
            return Err("max_cache_size_mb must be greater than 0".to_string());
        }

        if core.compression_threshold_bytes == 0 {
            return Err("compression_threshold_bytes must be greater than 0".to_string());
        }

        if core.stream_chunk_size < 1024 {
            return Err("stream_chunk_size should be at least 1024 bytes".to_string());
        }

        if core.stream_chunk_size > 1_000_000 {
            return Err("stream_chunk_size should not exceed 1MB".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_builder() {
        let config = ServerPerformanceBuilder::new()
            .enabled(true)
            .with_parallel_chunking(true, 8)
            .with_context_caching(true, 3600, 1024)
            .build();

        assert!(config.enabled);
        assert!(config.core_config.enable_parallel_chunking);
        assert_eq!(config.core_config.max_concurrent_chunks, 8);
        assert_eq!(config.core_config.cache_ttl_seconds, 3600);
        assert_eq!(config.core_config.max_cache_size_mb, 1024);
    }

    #[test]
    fn test_performance_profiles() {
        let dev_config = PerformanceProfiles::development();
        assert!(!dev_config.core_config.enable_parallel_chunking);
        assert_eq!(dev_config.core_config.max_cache_size_mb, 128);

        let prod_config = PerformanceProfiles::production();
        assert!(prod_config.core_config.enable_parallel_chunking);
        assert_eq!(prod_config.core_config.max_cache_size_mb, 1024);

        let ht_config = PerformanceProfiles::high_throughput();
        assert_eq!(ht_config.core_config.max_concurrent_chunks, 16);
        assert_eq!(ht_config.core_config.max_cache_size_mb, 2048);

        let mc_config = PerformanceProfiles::memory_constrained();
        assert_eq!(mc_config.core_config.max_concurrent_chunks, 4);
        assert_eq!(mc_config.core_config.max_cache_size_mb, 256);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = PerformanceProfiles::production();
        assert!(PerformanceUtils::validate_config(&valid_config).is_ok());

        let mut invalid_config = valid_config.clone();
        invalid_config.core_config.max_concurrent_chunks = 0;
        assert!(PerformanceUtils::validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_recommendations() {
        let recommendations = PerformanceUtils::get_recommendations();
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("performance profile")));
    }
}