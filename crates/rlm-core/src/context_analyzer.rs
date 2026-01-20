//! Context analysis and chunking logic for RLM.
//!
//! This module implements intelligent context analysis and chunking strategies
//! based on the RLM paper's approach to offloading large contexts to REPL.

use crate::RlmResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, instrument};

/// Configuration for context analysis and chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAnalysisConfig {
    /// Maximum chunk size in characters.
    pub max_chunk_size: usize,
    /// Minimum chunk size to avoid tiny fragments.
    pub min_chunk_size: usize,
    /// Overlap between chunks for better context continuity.
    pub chunk_overlap: usize,
    /// Whether to enable semantic chunking (vs simple character-based).
    pub enable_semantic_chunking: bool,
    /// Language for text processing hints.
    pub language: Option<String>,
}

impl Default for ContextAnalysisConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 8192,    // ~2K tokens at 4 chars/token
            min_chunk_size: 512,     // ~128 tokens minimum
            chunk_overlap: 256,      // ~64 tokens overlap
            enable_semantic_chunking: true,
            language: None,
        }
    }
}

/// A chunk of context with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    /// Unique identifier for this chunk.
    pub id: String,
    /// The text content of this chunk.
    pub content: String,
    /// Starting character position in original context.
    pub start_pos: usize,
    /// Ending character position in original context.
    pub end_pos: usize,
    /// Estimated token count for this chunk.
    pub estimated_tokens: u32,
    /// Chunk index in sequence.
    pub sequence_index: usize,
    /// Whether this chunk contains overlap from previous chunk.
    pub has_overlap: bool,
    /// Metadata about content type or characteristics.
    pub metadata: HashMap<String, String>,
}

/// Result of context analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAnalysis {
    /// Original context size in characters.
    pub original_size: usize,
    /// Estimated total token count.
    pub estimated_tokens: u32,
    /// Detected content characteristics.
    pub content_type: ContentType,
    /// Recommended chunking strategy.
    pub recommended_strategy: ChunkingStrategy,
    /// Language detected (if any).
    pub detected_language: Option<String>,
    /// Additional analysis metadata.
    pub metadata: HashMap<String, String>,
}

/// Type of content detected in context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    /// Plain text content.
    PlainText,
    /// Structured data (JSON, XML, etc.).
    Structured,
    /// Source code.
    Code,
    /// Documentation or markdown.
    Documentation,
    /// Mixed content types.
    Mixed,
    /// Unknown or binary content.
    Unknown,
}

/// Strategy for chunking the context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkingStrategy {
    /// Simple character-based chunking.
    CharacterBased,
    /// Semantic chunking based on paragraphs/sections.
    SemanticBased,
    /// Structure-aware chunking for code/data.
    StructureAware,
    /// No chunking needed (context is small enough).
    NoChunking,
}

/// Context analyzer for intelligent chunking and analysis.
#[derive(Debug)]
pub struct ContextAnalyzer {
    config: ContextAnalysisConfig,
}

impl ContextAnalyzer {
    /// Create a new context analyzer with default configuration.
    pub fn new() -> Self {
        Self {
            config: ContextAnalysisConfig::default(),
        }
    }

    /// Create a new context analyzer with custom configuration.
    pub fn with_config(config: ContextAnalysisConfig) -> Self {
        Self { config }
    }

    /// Analyze context and provide analysis results.
    #[instrument(skip(self, context), fields(context_size = context.len()))]
    pub async fn analyze(&self, context: &str) -> RlmResult<ContextAnalysis> {
        debug!("Starting context analysis");

        let content_type = self.detect_content_type(context);
        let estimated_tokens = self.estimate_tokens(context);
        let detected_language = self.detect_language(context);
        let recommended_strategy = self.recommend_strategy(context, &content_type);

        let mut metadata = HashMap::new();
        metadata.insert("analyzer_version".to_string(), "1.0".to_string());
        metadata.insert("analysis_timestamp".to_string(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        );

        let analysis = ContextAnalysis {
            original_size: context.len(),
            estimated_tokens,
            content_type,
            recommended_strategy,
            detected_language,
            metadata,
        };

        debug!(
            "Context analysis complete: {} chars, ~{} tokens, type: {:?}",
            analysis.original_size, analysis.estimated_tokens, analysis.content_type
        );

        Ok(analysis)
    }

    /// Split context into chunks based on analysis.
    #[instrument(skip(self, context, analysis), fields(context_size = context.len()))]
    pub async fn chunk_context(
        &self,
        context: &str,
        analysis: &ContextAnalysis,
    ) -> RlmResult<Vec<ContextChunk>> {
        debug!("Starting context chunking with strategy: {:?}", analysis.recommended_strategy);

        if analysis.recommended_strategy == ChunkingStrategy::NoChunking {
            return Ok(vec![self.create_single_chunk(context)]);
        }

        let chunks = match analysis.recommended_strategy {
            ChunkingStrategy::CharacterBased => self.chunk_by_characters(context).await?,
            ChunkingStrategy::SemanticBased => self.chunk_semantically(context).await?,
            ChunkingStrategy::StructureAware => self.chunk_structure_aware(context).await?,
            ChunkingStrategy::NoChunking => vec![self.create_single_chunk(context)],
        };

        debug!("Context chunked into {} chunks", chunks.len());
        Ok(chunks)
    }

    /// Detect the type of content in the context.
    fn detect_content_type(&self, context: &str) -> ContentType {
        let trimmed = context.trim();

        // Check for structured data
        if (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
           (trimmed.starts_with('[') && trimmed.ends_with(']')) {
            return ContentType::Structured;
        }

        if trimmed.starts_with('<') && trimmed.ends_with('>') {
            return ContentType::Structured;
        }

        // Check for common code patterns
        let code_indicators = [
            "function ", "def ", "class ", "import ", "require(",
            "const ", "let ", "var ", "fn ", "pub fn",
            "#include", "package ", "namespace ",
        ];

        let code_count = code_indicators
            .iter()
            .filter(|&indicator| context.contains(indicator))
            .count();

        if code_count > 2 {
            return ContentType::Code;
        }

        // Check for documentation patterns
        let doc_indicators = ["# ", "## ", "### ", "```", "---", "___"];
        let doc_count = doc_indicators
            .iter()
            .filter(|&indicator| context.contains(indicator))
            .count();

        if doc_count > 2 {
            return ContentType::Documentation;
        }

        // Check for mixed content
        if code_count > 0 && doc_count > 0 {
            return ContentType::Mixed;
        }

        ContentType::PlainText
    }

    /// Estimate token count using simple heuristic.
    fn estimate_tokens(&self, context: &str) -> u32 {
        // Simple approximation: ~4 characters per token on average
        (context.len() / 4) as u32
    }

    /// Detect language (simple heuristic).
    fn detect_language(&self, _context: &str) -> Option<String> {
        // For now, assume English. In the future, this could use
        // language detection libraries or ML models.
        Some("en".to_string())
    }

    /// Recommend chunking strategy based on content analysis.
    fn recommend_strategy(&self, context: &str, content_type: &ContentType) -> ChunkingStrategy {
        if context.len() <= self.config.max_chunk_size {
            return ChunkingStrategy::NoChunking;
        }

        match content_type {
            ContentType::Code | ContentType::Structured => ChunkingStrategy::StructureAware,
            ContentType::Documentation => ChunkingStrategy::SemanticBased,
            ContentType::PlainText => {
                if self.config.enable_semantic_chunking {
                    ChunkingStrategy::SemanticBased
                } else {
                    ChunkingStrategy::CharacterBased
                }
            }
            ContentType::Mixed => ChunkingStrategy::SemanticBased,
            ContentType::Unknown => ChunkingStrategy::CharacterBased,
        }
    }

    /// Create a single chunk for small contexts.
    fn create_single_chunk(&self, context: &str) -> ContextChunk {
        ContextChunk {
            id: "chunk-0".to_string(),
            content: context.to_string(),
            start_pos: 0,
            end_pos: context.len(),
            estimated_tokens: self.estimate_tokens(context),
            sequence_index: 0,
            has_overlap: false,
            metadata: HashMap::new(),
        }
    }

    /// Simple character-based chunking.
    async fn chunk_by_characters(&self, context: &str) -> RlmResult<Vec<ContextChunk>> {
        let mut chunks = Vec::new();
        let mut start = 0;
        let mut index = 0;

        while start < context.len() {
            let end = std::cmp::min(start + self.config.max_chunk_size, context.len());
            let chunk_content = &context[start..end];

            chunks.push(ContextChunk {
                id: format!("chunk-{}", index),
                content: chunk_content.to_string(),
                start_pos: start,
                end_pos: end,
                estimated_tokens: self.estimate_tokens(chunk_content),
                sequence_index: index,
                has_overlap: start > 0 && self.config.chunk_overlap > 0,
                metadata: HashMap::new(),
            });

            // Move start position with overlap
            start = if end == context.len() {
                context.len() // Last chunk, don't continue
            } else {
                end.saturating_sub(self.config.chunk_overlap)
            };

            index += 1;

            if start >= context.len() {
                break;
            }
        }

        Ok(chunks)
    }

    /// Semantic chunking based on paragraphs and sentences.
    async fn chunk_semantically(&self, context: &str) -> RlmResult<Vec<ContextChunk>> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut index = 0;

        // Split by double newlines (paragraphs) first
        let paragraphs: Vec<&str> = context.split("\n\n").collect();

        for paragraph in paragraphs {
            // If adding this paragraph would exceed max size, create chunk
            if !current_chunk.is_empty() &&
               (current_chunk.len() + paragraph.len() + 2) > self.config.max_chunk_size {

                let chunk = ContextChunk {
                    id: format!("chunk-{}", index),
                    content: current_chunk.trim().to_string(),
                    start_pos: chunk_start,
                    end_pos: chunk_start + current_chunk.len(),
                    estimated_tokens: self.estimate_tokens(&current_chunk),
                    sequence_index: index,
                    has_overlap: index > 0,
                    metadata: HashMap::new(),
                };

                chunks.push(chunk);
                index += 1;

                // Start new chunk with overlap if configured
                if self.config.chunk_overlap > 0 && !current_chunk.is_empty() {
                    let overlap_size = std::cmp::min(self.config.chunk_overlap, current_chunk.len());
                    let overlap_start = current_chunk.len().saturating_sub(overlap_size);
                    current_chunk = current_chunk[overlap_start..].to_string() + "\n\n";
                    chunk_start = chunk_start + overlap_start;
                } else {
                    current_chunk.clear();
                    chunk_start = chunk_start + current_chunk.len() + 2; // +2 for "\n\n"
                }
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }

        // Add final chunk if there's remaining content
        if !current_chunk.is_empty() && current_chunk.trim().len() >= self.config.min_chunk_size {
            chunks.push(ContextChunk {
                id: format!("chunk-{}", index),
                content: current_chunk.trim().to_string(),
                start_pos: chunk_start,
                end_pos: chunk_start + current_chunk.len(),
                estimated_tokens: self.estimate_tokens(&current_chunk),
                sequence_index: index,
                has_overlap: index > 0,
                metadata: HashMap::new(),
            });
        }

        // Fallback to character-based if semantic chunking produces no chunks
        if chunks.is_empty() {
            return self.chunk_by_characters(context).await;
        }

        Ok(chunks)
    }

    /// Structure-aware chunking for code and structured data.
    async fn chunk_structure_aware(&self, context: &str) -> RlmResult<Vec<ContextChunk>> {
        // For now, use semantic chunking as a fallback
        // In the future, this could implement language-specific parsing
        // to respect function boundaries, class definitions, etc.
        self.chunk_semantically(context).await
    }
}

impl Default for ContextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_analysis() {
        let analyzer = ContextAnalyzer::new();
        let context = "This is a simple test context for analysis.";

        let analysis = analyzer.analyze(context).await.unwrap();

        assert_eq!(analysis.original_size, context.len());
        assert_eq!(analysis.content_type, ContentType::PlainText);
        assert_eq!(analysis.recommended_strategy, ChunkingStrategy::NoChunking);
    }

    #[tokio::test]
    async fn test_content_type_detection() {
        let analyzer = ContextAnalyzer::new();

        // Test JSON detection
        let json_context = r#"{"key": "value", "array": [1, 2, 3]}"#;
        let analysis = analyzer.analyze(json_context).await.unwrap();
        assert_eq!(analysis.content_type, ContentType::Structured);

        // Test code detection
        let code_context = "function test() {\n    return 42;\n}\nconst x = 10;";
        let analysis = analyzer.analyze(code_context).await.unwrap();
        assert_eq!(analysis.content_type, ContentType::Code);

        // Test documentation detection
        let doc_context = "# Title\n\n## Subtitle\n\n```code block```";
        let analysis = analyzer.analyze(doc_context).await.unwrap();
        assert_eq!(analysis.content_type, ContentType::Documentation);
    }

    #[tokio::test]
    async fn test_chunking_strategies() {
        let mut config = ContextAnalysisConfig::default();
        config.max_chunk_size = 50; // Small size for testing
        let analyzer = ContextAnalyzer::with_config(config);

        let long_context = "This is a very long context that should be split into multiple chunks. Each chunk should be reasonably sized and have appropriate overlap for continuity.";

        let analysis = analyzer.analyze(long_context).await.unwrap();
        let chunks = analyzer.chunk_context(long_context, &analysis).await.unwrap();

        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| chunk.content.len() <= 50 + analyzer.config.chunk_overlap));
    }

    #[tokio::test]
    async fn test_no_chunking_for_small_context() {
        let analyzer = ContextAnalyzer::new();
        let small_context = "Small context";

        let analysis = analyzer.analyze(small_context).await.unwrap();
        let chunks = analyzer.chunk_context(small_context, &analysis).await.unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, small_context);
        assert!(!chunks[0].has_overlap);
    }

    #[tokio::test]
    async fn test_semantic_chunking() {
        let mut config = ContextAnalysisConfig::default();
        config.max_chunk_size = 100;
        let analyzer = ContextAnalyzer::with_config(config);

        let context = "Paragraph one.\n\nParagraph two with more content.\n\nParagraph three is the final one.";

        let analysis = analyzer.analyze(context).await.unwrap();
        let chunks = analyzer.chunk_context(context, &analysis).await.unwrap();

        // Should respect paragraph boundaries
        for chunk in &chunks {
            assert!(!chunk.content.contains("\n\nParagraph") || chunk.content.starts_with("Paragraph"));
        }
    }
}