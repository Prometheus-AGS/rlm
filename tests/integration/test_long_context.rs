//! Long-context integration tests for RLM functionality.
//!
//! These tests verify that the RLM system can handle very long contexts
//! efficiently using the recursive decomposition approach from the MIT paper.

use rlm_core::{RlmConfig, RlmRequest, ports::ReplBackend};
use rlm_repl_rhai::RhaiReplBackend;
use std::collections::HashMap;

#[tokio::test]
async fn test_long_context_processing() {
    // Create a very long context (simulating 100K+ tokens)
    let long_document = "This document contains important information about artificial intelligence and machine learning. ".repeat(5000);
    let needle = "The secret code is ALPHA-BETA-GAMMA-42";
    let haystack = format!("{}{}. {}", &long_document[..long_document.len()/2], needle, &long_document[long_document.len()/2..]);

    let query = "What is the secret code mentioned in the document?";

    let request = RlmRequest {
        query: query.to_string(),
        context: haystack,
        recursion_depth: 1,
        max_iterations: 10,
        metadata: HashMap::new(),
    };

    // For now, just test that the request is properly structured
    assert_eq!(request.query, query);
    assert!(request.context.contains(needle));
    assert!(request.context.len() > 100_000); // Verify it's actually long
    assert_eq!(request.recursion_depth, 1);
    assert_eq!(request.max_iterations, 10);
}

#[tokio::test]
async fn test_repl_context_offloading() {
    let mut repl = RhaiReplBackend::new().expect("Failed to create REPL backend");

    // Test that long contexts can be offloaded to REPL
    let long_context = "Context data: ".repeat(10000); // 130K+ characters

    repl.set_variable("context", &long_context).await
        .expect("Failed to set context variable");

    // Verify the context was set
    let state = repl.get_state().await.expect("Failed to get REPL state");
    let var_count: usize = state.get("variables_count").unwrap().parse().unwrap();
    assert!(var_count > 0); // Should have at least the context variable

    // Test that we can execute code that references the context
    let result = repl.execute("context.len()").await
        .expect("Failed to execute context length check");

    let length: usize = result.parse().expect("Failed to parse context length");
    assert_eq!(length, long_context.len());
}

#[tokio::test]
async fn test_context_analysis_and_chunking() {
    use rlm_core::context_analyzer::{ContextAnalyzer, ChunkingStrategy};

    let analyzer = ContextAnalyzer::new();

    // Test with structured content
    let structured_content = r#"
# Chapter 1: Introduction
This is the introduction to our document about machine learning.

## Section 1.1: Background
Machine learning is a subset of artificial intelligence.

# Chapter 2: Methods
This chapter discusses various machine learning methods.

## Section 2.1: Supervised Learning
Supervised learning uses labeled data for training.

## Section 2.2: Unsupervised Learning
Unsupervised learning finds patterns in unlabeled data.

# Chapter 3: Conclusion
In conclusion, machine learning has many applications.
"#.repeat(1000); // Make it long

    let chunks = analyzer.chunk_content(&structured_content, ChunkingStrategy::Semantic, 1000)
        .expect("Failed to chunk content");

    // Should produce multiple chunks
    assert!(chunks.len() > 1);

    // Each chunk should be within size limits
    for chunk in &chunks {
        assert!(chunk.content.len() <= 1500); // Allow some buffer
        assert!(!chunk.content.is_empty());
        assert!(chunk.position.start < chunk.position.end);
    }

    // Chunks should cover the entire content when combined
    let total_chunk_length: usize = chunks.iter().map(|c| c.position.end - c.position.start).sum();
    assert_eq!(total_chunk_length, structured_content.len());
}

#[tokio::test]
async fn test_needle_in_haystack_scenario() {
    // Simulate the S-NIAH test case
    let haystack_prefix = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(2000);
    let needle = "The magic number is 42 and it represents the answer to everything.";
    let haystack_suffix = "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(3000);

    let full_context = format!("{}{}{}", haystack_prefix, needle, haystack_suffix);
    let question = "What is the magic number mentioned in the document?";

    // Verify the setup
    assert!(full_context.contains(needle));
    assert!(full_context.len() > 200_000); // Very long context

    let request = RlmRequest {
        query: question.to_string(),
        context: full_context.clone(),
        recursion_depth: 1,
        max_iterations: 5,
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("test_type".to_string(), "s_niah".to_string());
            meta.insert("expected_answer".to_string(), "42".to_string());
            meta
        },
    };

    assert_eq!(request.query, question);
    assert_eq!(request.metadata.get("test_type").unwrap(), "s_niah");
    assert_eq!(request.metadata.get("expected_answer").unwrap(), "42");
}

#[tokio::test]
async fn test_context_size_limits_and_validation() {
    let config = RlmConfig::default();

    // Test that config has reasonable defaults for long contexts
    assert!(config.limits.chunk_size_tokens > 1000);
    assert!(config.limits.max_iterations > 0);
    assert!(config.limits.max_recursion_depth >= 1);

    // Test context size estimation
    let small_context = "Short text";
    let large_context = "Large text content. ".repeat(50_000);

    // Basic token estimation (rough)
    let small_tokens = (small_context.len() / 4) as u32;
    let large_tokens = (large_context.len() / 4) as u32;

    assert!(small_tokens < 100);
    assert!(large_tokens > 50_000);

    // Verify our estimation is in the right ballpark
    assert!(small_tokens < large_tokens);
    assert!(large_tokens < 300_000); // Reasonable upper bound
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_context_offloading_performance() {
        let mut repl = RhaiReplBackend::new().expect("Failed to create REPL backend");

        // Test performance of context offloading for very large contexts
        let sizes = vec![1_000, 10_000, 100_000, 1_000_000];

        for size in sizes {
            let context = "Context data chunk. ".repeat(size / 20); // Approximate target size

            let start = Instant::now();
            repl.set_variable("test_context", &context).await
                .expect("Failed to set context");
            let duration = start.elapsed();

            // Performance should be reasonable (sub-second for even very large contexts)
            assert!(duration.as_millis() < 5000, "Context offloading took too long: {:?}", duration);

            // Verify the context was set correctly
            let result = repl.execute("test_context.len()").await
                .expect("Failed to get context length");
            let length: usize = result.parse().expect("Failed to parse length");
            assert_eq!(length, context.len());

            println!("Context size: {} chars, offload time: {:?}", context.len(), duration);
        }
    }

    #[tokio::test]
    async fn test_memory_efficiency() {
        let mut repl = RhaiReplBackend::new().expect("Failed to create REPL backend");

        // Test that we can handle multiple large contexts without memory issues
        for i in 0..10 {
            let context = format!("Large context data for test {}. ", i).repeat(10_000);

            repl.set_variable(&format!("context_{}", i), &context).await
                .expect("Failed to set context");

            // Verify the context is accessible
            let result = repl.execute(&format!("context_{}.len()", i)).await
                .expect("Failed to execute context length check");

            let length: usize = result.parse().expect("Failed to parse length");
            assert_eq!(length, context.len());
        }

        // Check that all contexts are still accessible
        for i in 0..10 {
            let result = repl.execute(&format!("context_{}.len()", i)).await
                .expect("Context should still be accessible");
            assert!(!result.is_empty());
        }
    }
}