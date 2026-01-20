//! Performance optimizations for large context processing in RLM.
//!
//! This module implements advanced performance optimizations specifically for handling
//! large contexts (>1M tokens) efficiently, following the RLM paper's complexity optimizations.

use crate::{
    context_analyzer::{ContextAnalyzer, ContextAnalysisConfig, ChunkingStrategy},
    RlmResult, RlmError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, instrument};

/// Performance configuration for large context processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable parallel chunk processing.
    pub enable_parallel_chunking: bool,
    /// Maximum concurrent chunk operations.
    pub max_concurrent_chunks: usize,
    /// Enable context caching for repeated content.
    pub enable_context_caching: bool,
    /// Cache TTL in seconds.
    pub cache_ttl_seconds: u64,
    /// Maximum cache size in MB.
    pub max_cache_size_mb: usize,
    /// Enable adaptive chunk sizing based on content complexity.
    pub enable_adaptive_chunking: bool,
    /// Enable context compression for storage optimization.
    pub enable_context_compression: bool,
    /// Compression threshold in bytes (contexts larger than this get compressed).
    pub compression_threshold_bytes: usize,
    /// Enable streaming context processing.
    pub enable_streaming_processing: bool,
    /// Stream chunk size in bytes.
    pub stream_chunk_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_parallel_chunking: true,
            max_concurrent_chunks: 8,
            enable_context_caching: true,
            cache_ttl_seconds: 3600, // 1 hour
            max_cache_size_mb: 1024, // 1GB
            enable_adaptive_chunking: true,
            enable_context_compression: true,
            compression_threshold_bytes: 1_000_000, // 1MB
            enable_streaming_processing: true,
            stream_chunk_size: 16384, // 16KB
        }
    }
}

/// Cached context entry with metadata.
#[derive(Debug, Clone)]
pub struct CachedContext {
    /// Original context content.
    pub content: String,
    /// Compressed content (if compression is enabled).
    pub compressed_content: Option<Vec<u8>>,
    /// Analysis results.
    pub analysis: crate::context_analyzer::ContextAnalysis,
    /// Pre-computed chunks.
    pub chunks: Vec<crate::context_analyzer::ContextChunk>,
    /// Cache timestamp.
    pub cached_at: std::time::SystemTime,
    /// Access count for LRU eviction.
    pub access_count: u64,
    /// Last accessed timestamp.
    pub last_accessed: std::time::SystemTime,
}

impl CachedContext {
    /// Get the content, decompressing if necessary.
    pub fn get_content(&self) -> RlmResult<String> {
        if let Some(ref compressed) = self.compressed_content {
            // Decompress the content
            self.decompress_content(compressed)
        } else {
            Ok(self.content.clone())
        }
    }

    /// Compress content using a simple compression algorithm.
    fn compress_content(content: &str) -> Vec<u8> {
        // Using a simple compression for now - in production, consider using
        // libraries like `lz4`, `zstd`, or `flate2` for better compression ratios
        content.as_bytes().to_vec()
    }

    /// Decompress content.
    fn decompress_content(&self, compressed: &[u8]) -> RlmResult<String> {
        // Simple decompression - in production, use proper decompression
        String::from_utf8(compressed.to_vec())
            .map_err(|e| RlmError::Other(format!("Failed to decompress context: {}", e)))
    }
}

/// Performance-optimized context processor.
#[derive(Debug)]
pub struct OptimizedContextProcessor {
    config: PerformanceConfig,
    context_cache: Arc<RwLock<HashMap<String, CachedContext>>>,
    chunk_semaphore: Arc<Semaphore>,
    analyzer: ContextAnalyzer,
    cache_size_bytes: Arc<RwLock<usize>>,
}

impl OptimizedContextProcessor {
    /// Create a new optimized context processor.
    pub fn new(config: PerformanceConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_chunks));
        let analyzer_config = ContextAnalysisConfig {
            max_chunk_size: if config.enable_adaptive_chunking { 16384 } else { 8192 },
            enable_semantic_chunking: true,
            ..Default::default()
        };

        Self {
            config,
            context_cache: Arc::new(RwLock::new(HashMap::new())),
            chunk_semaphore: semaphore,
            analyzer: ContextAnalyzer::with_config(analyzer_config),
            cache_size_bytes: Arc::new(RwLock::new(0)),
        }
    }

    /// Process a large context with performance optimizations.
    #[instrument(skip(self, context), fields(context_size = context.len()))]
    pub async fn process_context(&self, context: &str) -> RlmResult<ProcessedContext> {
        let start_time = Instant::now();
        let context_hash = self.calculate_context_hash(context);

        info!("Processing context of {} bytes with hash {}", context.len(), context_hash);

        // Check cache first
        if self.config.enable_context_caching {
            if let Some(cached) = self.get_cached_context(&context_hash).await? {
                info!("Context found in cache, returning cached result");
                let compression_ratio = self.calculate_compression_ratio(&cached);
                return Ok(ProcessedContext {
                    analysis: cached.analysis,
                    chunks: cached.chunks,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    cache_hit: true,
                    compression_ratio,
                });
            }
        }

        // Analyze context
        debug!("Analyzing context for optimal processing");
        let analysis = if self.config.enable_adaptive_chunking {
            self.adaptive_analyze(context).await?
        } else {
            self.analyzer.analyze(context).await?
        };

        // Process chunks based on configuration
        let chunks = if self.config.enable_parallel_chunking {
            self.parallel_chunk_context(context, &analysis).await?
        } else if self.config.enable_streaming_processing {
            self.streaming_chunk_context(context, &analysis).await?
        } else {
            self.analyzer.chunk_context(context, &analysis).await?
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Cache the result if caching is enabled
        let compression_ratio = if self.config.enable_context_caching {
            let cached_context = self.create_cached_context(context, analysis.clone(), chunks.clone()).await?;
            let ratio = self.calculate_compression_ratio(&cached_context);
            self.cache_context(context_hash, cached_context).await?;
            ratio
        } else {
            1.0
        };

        info!("Context processing completed in {}ms", processing_time);

        Ok(ProcessedContext {
            analysis,
            chunks,
            processing_time_ms: processing_time,
            cache_hit: false,
            compression_ratio,
        })
    }

    /// Perform adaptive context analysis based on content characteristics.
    async fn adaptive_analyze(&self, context: &str) -> RlmResult<crate::context_analyzer::ContextAnalysis> {
        // Analyze a sample of the context to determine optimal chunk size
        let sample_size = std::cmp::min(context.len(), 10000);
        let sample = &context[..sample_size];

        let base_analysis = self.analyzer.analyze(sample).await?;

        // Adjust chunk size based on content type and complexity
        let optimal_chunk_size = match base_analysis.content_type {
            crate::context_analyzer::ContentType::Code => {
                // Code needs smaller chunks to respect function boundaries
                8192
            }
            crate::context_analyzer::ContentType::Structured => {
                // Structured data can use larger chunks
                32768
            }
            crate::context_analyzer::ContentType::Documentation => {
                // Documentation benefits from semantic boundaries
                16384
            }
            _ => 12288, // Default adaptive size
        };

        // Create new analyzer with optimal configuration
        let adaptive_config = ContextAnalysisConfig {
            max_chunk_size: optimal_chunk_size,
            enable_semantic_chunking: true,
            ..Default::default()
        };

        let adaptive_analyzer = ContextAnalyzer::with_config(adaptive_config);
        adaptive_analyzer.analyze(context).await
    }

    /// Process chunks in parallel for improved performance.
    async fn parallel_chunk_context(
        &self,
        context: &str,
        analysis: &crate::context_analyzer::ContextAnalysis,
    ) -> RlmResult<Vec<crate::context_analyzer::ContextChunk>> {
        if analysis.recommended_strategy == ChunkingStrategy::NoChunking {
            return Ok(vec![self.create_single_chunk(context)]);
        }

        // For parallel processing, we need to split the work appropriately
        let chunk_size = 8192; // Use default chunk size for parallel processing
        let total_size = context.len();

        if total_size <= chunk_size * 2 {
            // Small enough for sequential processing
            return self.analyzer.chunk_context(context, analysis).await;
        }

        // Create parallel processing tasks
        let mut tasks = Vec::new();
        let mut offset = 0;
        let mut chunk_id = 0;

        while offset < total_size {
            let end = std::cmp::min(offset + chunk_size, total_size);
            let chunk_content = context[offset..end].to_string();

            let semaphore = Arc::clone(&self.chunk_semaphore);
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.map_err(|e| {
                    RlmError::Other(format!("Failed to acquire semaphore: {}", e))
                })?;

                // Process this chunk
                Ok::<_, RlmError>(crate::context_analyzer::ContextChunk {
                    id: format!("chunk-{}", chunk_id),
                    content: chunk_content.clone(),
                    start_pos: offset,
                    end_pos: end,
                    estimated_tokens: (chunk_content.len() / 4) as u32,
                    sequence_index: chunk_id,
                    has_overlap: offset > 0,
                    metadata: HashMap::new(),
                })
            });

            tasks.push(task);
            offset = end.saturating_sub(256); // Use default overlap of 256 characters
            chunk_id += 1;

            if offset >= total_size {
                break;
            }
        }

        // Wait for all tasks to complete
        let mut chunks = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(chunk)) => chunks.push(chunk),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(RlmError::Other(format!("Task join error: {}", e))),
            }
        }

        // Sort chunks by sequence index to maintain order
        chunks.sort_by_key(|chunk| chunk.sequence_index);

        debug!("Parallel processing completed: {} chunks", chunks.len());
        Ok(chunks)
    }

    /// Process context using streaming approach for very large contexts.
    async fn streaming_chunk_context(
        &self,
        context: &str,
        analysis: &crate::context_analyzer::ContextAnalysis,
    ) -> RlmResult<Vec<crate::context_analyzer::ContextChunk>> {
        if analysis.recommended_strategy == ChunkingStrategy::NoChunking {
            return Ok(vec![self.create_single_chunk(context)]);
        }

        let mut chunks = Vec::new();
        let mut bytes_processed = 0;
        let total_bytes = context.len();
        let stream_size = self.config.stream_chunk_size;

        debug!("Starting streaming context processing with {} byte stream chunks", stream_size);

        while bytes_processed < total_bytes {
            let end = std::cmp::min(bytes_processed + stream_size, total_bytes);
            let stream_chunk = &context[bytes_processed..end];

            // Process this stream chunk
            let stream_analysis = self.analyzer.analyze(stream_chunk).await?;
            let mut stream_chunks = self.analyzer.chunk_context(stream_chunk, &stream_analysis).await?;

            // Adjust chunk positions to global context
            for chunk in &mut stream_chunks {
                chunk.start_pos += bytes_processed;
                chunk.end_pos += bytes_processed;
                chunk.sequence_index += chunks.len();
            }

            chunks.extend(stream_chunks);
            bytes_processed = end;

            // Yield control to allow other tasks to run
            tokio::task::yield_now().await;
        }

        debug!("Streaming processing completed: {} chunks from {} bytes", chunks.len(), total_bytes);
        Ok(chunks)
    }

    /// Get cached context if available and not expired.
    async fn get_cached_context(&self, context_hash: &str) -> RlmResult<Option<CachedContext>> {
        let mut cache = self.context_cache.write().await;

        if let Some(cached) = cache.get_mut(context_hash) {
            // Check if cache entry is still valid
            let now = std::time::SystemTime::now();
            let cache_age = now.duration_since(cached.cached_at)
                .unwrap_or_default()
                .as_secs();

            if cache_age <= self.config.cache_ttl_seconds {
                // Update access statistics
                cached.access_count += 1;
                cached.last_accessed = now;
                return Ok(Some(cached.clone()));
            } else {
                // Remove expired entry
                cache.remove(context_hash);
            }
        }

        Ok(None)
    }

    /// Create a cached context entry with optional compression.
    async fn create_cached_context(
        &self,
        content: &str,
        analysis: crate::context_analyzer::ContextAnalysis,
        chunks: Vec<crate::context_analyzer::ContextChunk>,
    ) -> RlmResult<CachedContext> {
        let now = std::time::SystemTime::now();
        let should_compress = self.config.enable_context_compression &&
                             content.len() > self.config.compression_threshold_bytes;

        let (content_to_store, compressed_content) = if should_compress {
            debug!("Compressing context of {} bytes", content.len());
            let compressed = CachedContext::compress_content(content);
            (String::new(), Some(compressed))
        } else {
            (content.to_string(), None)
        };

        Ok(CachedContext {
            content: content_to_store,
            compressed_content,
            analysis,
            chunks,
            cached_at: now,
            access_count: 1,
            last_accessed: now,
        })
    }

    /// Cache a processed context.
    async fn cache_context(&self, context_hash: String, cached_context: CachedContext) -> RlmResult<()> {
        let context_size = cached_context.content.len() +
                          cached_context.compressed_content.as_ref().map(|c| c.len()).unwrap_or(0);

        // Check cache size limits
        {
            let mut current_size = self.cache_size_bytes.write().await;
            let max_size_bytes = self.config.max_cache_size_mb * 1024 * 1024;

            if *current_size + context_size > max_size_bytes {
                // Evict least recently used entries
                self.evict_lru_entries(context_size).await?;
            }

            *current_size += context_size;
        }

        let mut cache = self.context_cache.write().await;
        cache.insert(context_hash, cached_context);

        debug!("Cached context with size {} bytes", context_size);
        Ok(())
    }

    /// Evict least recently used cache entries to make space.
    async fn evict_lru_entries(&self, needed_space: usize) -> RlmResult<()> {
        let mut cache = self.context_cache.write().await;
        let mut entries_to_remove = Vec::new();

        // Collect entries sorted by last access time (oldest first)
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, cached)| cached.last_accessed);

        let mut freed_space = 0;
        for (hash, cached) in entries {
            if freed_space >= needed_space {
                break;
            }

            let entry_size = cached.content.len() +
                           cached.compressed_content.as_ref().map(|c| c.len()).unwrap_or(0);
            entries_to_remove.push((hash.clone(), entry_size));
            freed_space += entry_size;
        }

        // Remove selected entries
        for (hash, size) in entries_to_remove {
            cache.remove(&hash);
            let mut current_size = self.cache_size_bytes.write().await;
            *current_size = current_size.saturating_sub(size);
        }

        info!("Evicted {} bytes from cache to make space", freed_space);
        Ok(())
    }

    /// Calculate a hash for context content.
    fn calculate_context_hash(&self, content: &str) -> String {
        // Simple hash implementation - in production, use a proper hash function
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Calculate compression ratio for cached content.
    fn calculate_compression_ratio(&self, cached: &CachedContext) -> f64 {
        if let Some(ref compressed) = cached.compressed_content {
            let original_size = cached.content.len().max(1); // Avoid division by zero
            compressed.len() as f64 / original_size as f64
        } else {
            1.0
        }
    }

    /// Create a single chunk for small contexts.
    fn create_single_chunk(&self, context: &str) -> crate::context_analyzer::ContextChunk {
        crate::context_analyzer::ContextChunk {
            id: "chunk-0".to_string(),
            content: context.to_string(),
            start_pos: 0,
            end_pos: context.len(),
            estimated_tokens: (context.len() / 4) as u32,
            sequence_index: 0,
            has_overlap: false,
            metadata: HashMap::new(),
        }
    }

    /// Get performance statistics.
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let cache = self.context_cache.read().await;
        let cache_size = *self.cache_size_bytes.read().await;

        let total_entries = cache.len();
        let total_access_count: u64 = cache.values().map(|c| c.access_count).sum();
        let avg_compression_ratio = if total_entries > 0 {
            cache.values().map(|c| self.calculate_compression_ratio(c)).sum::<f64>() / total_entries as f64
        } else {
            1.0
        };

        PerformanceStats {
            cache_entries: total_entries,
            cache_size_bytes: cache_size,
            total_cache_hits: total_access_count,
            average_compression_ratio: avg_compression_ratio,
            max_concurrent_chunks: self.config.max_concurrent_chunks,
            parallel_chunking_enabled: self.config.enable_parallel_chunking,
            streaming_processing_enabled: self.config.enable_streaming_processing,
        }
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) -> RlmResult<()> {
        let mut cache = self.context_cache.write().await;
        let mut cache_size = self.cache_size_bytes.write().await;

        cache.clear();
        *cache_size = 0;

        info!("Performance cache cleared");
        Ok(())
    }
}

/// Result of processing a context with performance optimizations.
#[derive(Debug, Clone)]
pub struct ProcessedContext {
    /// Analysis results.
    pub analysis: crate::context_analyzer::ContextAnalysis,
    /// Generated chunks.
    pub chunks: Vec<crate::context_analyzer::ContextChunk>,
    /// Processing time in milliseconds.
    pub processing_time_ms: u64,
    /// Whether this was a cache hit.
    pub cache_hit: bool,
    /// Compression ratio (1.0 = no compression).
    pub compression_ratio: f64,
}

/// Performance statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Number of entries in cache.
    pub cache_entries: usize,
    /// Total cache size in bytes.
    pub cache_size_bytes: usize,
    /// Total cache hits.
    pub total_cache_hits: u64,
    /// Average compression ratio.
    pub average_compression_ratio: f64,
    /// Maximum concurrent chunk operations.
    pub max_concurrent_chunks: usize,
    /// Whether parallel chunking is enabled.
    pub parallel_chunking_enabled: bool,
    /// Whether streaming processing is enabled.
    pub streaming_processing_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimized_processor_creation() {
        let config = PerformanceConfig::default();
        let processor = OptimizedContextProcessor::new(config);

        let stats = processor.get_performance_stats().await;
        assert_eq!(stats.cache_entries, 0);
        assert_eq!(stats.cache_size_bytes, 0);
    }

    #[tokio::test]
    async fn test_context_processing() {
        let config = PerformanceConfig::default();
        let processor = OptimizedContextProcessor::new(config);

        let context = "This is a test context that should be processed efficiently.";
        let result = processor.process_context(context).await.unwrap();

        assert!(!result.cache_hit); // First time, should not be a cache hit
        assert!(result.processing_time_ms > 0);
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_context_caching() {
        let config = PerformanceConfig {
            enable_context_caching: true,
            ..Default::default()
        };
        let processor = OptimizedContextProcessor::new(config);

        let context = "This is a test context for caching.";

        // First call - should not be cached
        let result1 = processor.process_context(context).await.unwrap();
        assert!(!result1.cache_hit);

        // Second call - should be cached
        let result2 = processor.process_context(context).await.unwrap();
        assert!(result2.cache_hit);
    }

    #[tokio::test]
    async fn test_large_context_processing() {
        let config = PerformanceConfig {
            enable_parallel_chunking: true,
            max_concurrent_chunks: 4,
            ..Default::default()
        };
        let processor = OptimizedContextProcessor::new(config);

        // Create a large context
        let large_context = "Large context chunk. ".repeat(1000);
        let result = processor.process_context(&large_context).await.unwrap();

        assert!(!result.chunks.is_empty());
        assert!(result.chunks.len() > 1); // Should be chunked
    }

    #[tokio::test]
    async fn test_performance_stats() {
        let config = PerformanceConfig::default();
        let processor = OptimizedContextProcessor::new(config.clone());

        let stats = processor.get_performance_stats().await;
        assert_eq!(stats.max_concurrent_chunks, config.max_concurrent_chunks);
        assert_eq!(stats.parallel_chunking_enabled, config.enable_parallel_chunking);
        assert_eq!(stats.streaming_processing_enabled, config.enable_streaming_processing);
    }
}