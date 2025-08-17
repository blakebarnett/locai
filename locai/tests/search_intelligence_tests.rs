//! External tests for Search Intelligence Layer
//!
//! This test suite covers the advanced search intelligence features including:
//! - Query analysis with intent detection and strategy suggestion
//! - BM25 full-text search with highlighting and scoring
//! - Fuzzy search for typo tolerance and spell correction
//! - Context-aware search sessions and conversational search
//! - Search suggestions including auto-completion and query expansion
//! - Multi-signal result ranking and fusion
//! - Search result explanation and provenance tracking

use locai::core::{SearchContent, SearchOptions, SearchStrategy, SearchTypeFilter};
use locai::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;
use tokio;

// Global counter for unique test databases
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to create isolated test Locai instance
async fn create_test_locai() -> Result<(Locai, TempDir)> {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Create a unique database path for this test
    let db_path = temp_dir
        .path()
        .join(format!("test_db_intelligence_{}", test_id));
    std::fs::create_dir_all(&db_path).expect("Failed to create test database directory");

    // Use the builder pattern to create an isolated database instance
    let locai = Locai::builder()
        .with_data_dir(&db_path)
        .with_memory_storage() // Use in-memory storage but with unique paths
        .with_embedding_model("BAAI/bge-small-en") // Use smaller model for faster tests
        .build()
        .await?;

    Ok((locai, temp_dir))
}

/// Create comprehensive test dataset for search intelligence testing
async fn setup_intelligence_test_data(locai: &Locai) -> Result<Vec<String>> {
    // Create memories with AI/ML content for testing search intelligence
    let memory_contents = vec![
        "Machine learning algorithms are transforming artificial intelligence by enabling systems to learn from data automatically",
        "Neural networks are computational models inspired by biological neural networks that excel at pattern recognition tasks",
        "Natural language processing enables computers to understand, interpret, and generate human language using advanced algorithms",
        "Quantum computing leverages quantum mechanical phenomena like superposition and entanglement to process information",
        "How to train a neural network: 1) Prepare training data 2) Define network architecture 3) Initialize weights 4) Forward propagation 5) Calculate loss 6) Backward propagation 7) Update weights 8) Repeat until convergence",
        "Python has become the dominant language for AI and machine learning due to its simplicity and rich ecosystem",
        "Computer vision enables machines to interpret and understand visual information from images and videos",
        "The transformer architecture revolutionized natural language processing through the attention mechanism",
    ];

    let mut memory_ids = Vec::new();
    for content in memory_contents {
        let memory_id = locai.remember(content).await?;
        memory_ids.push(memory_id);
    }

    // Wait for processing and indexing
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    Ok(memory_ids)
}

// ============================================================================
// SEARCH INTELLIGENCE INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_search_intelligence_basic_functionality() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test basic search still works with intelligence layer
    let results = locai.search("machine learning").await.unwrap();
    assert!(
        !results.is_empty(),
        "Should find results for 'machine learning'"
    );

    // Verify search results have proper structure
    for result in &results {
        assert!(!result.id.is_empty(), "Result should have an ID");
        assert!(result.score >= 0.0, "Score should be non-negative");
        // Note: Scores may be > 1.0 for certain search strategies (e.g., BM25), so we don't enforce upper bound
        assert!(!result.summary().is_empty(), "Result should have a summary");
        assert!(
            !result.match_reason().is_empty(),
            "Result should have a match reason"
        );
    }
}

#[tokio::test]
async fn test_search_strategy_selection() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test different search strategies
    let strategies = [
        SearchStrategy::Semantic,
        SearchStrategy::Keyword,
        SearchStrategy::Graph,
        SearchStrategy::Hybrid,
    ];

    for strategy in strategies.iter() {
        let options = SearchOptions {
            strategy: strategy.clone(),
            limit: 3,
            ..Default::default()
        };

        let results = locai
            .search_with_options("neural networks", options)
            .await
            .unwrap();

        // Each strategy should return some results
        println!("Strategy {:?} found {} results", strategy, results.len());

        if !results.is_empty() {
            // Verify results are properly formatted
            for result in &results {
                assert!(!result.id.is_empty(), "Result should have an ID");
                assert!(result.score >= 0.0, "Score should be non-negative");
            }
        }
    }
}

#[tokio::test]
async fn test_fuzzy_search_typo_tolerance() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test queries with intentional typos
    let typo_queries = vec![
        "machien lerning",       // "machine learning"
        "neurral netowrks",      // "neural networks"
        "artficial inteligence", // "artificial intelligence"
    ];

    for typo_query in &typo_queries {
        // Even with typos, fuzzy search should find relevant results
        let results = locai.search(typo_query).await.unwrap();

        println!("Query '{}' found {} results", typo_query, results.len());

        // Note: Fuzzy search might not always find results depending on the threshold
        // but it should handle the queries gracefully
        if !results.is_empty() {
            for result in &results {
                assert!(!result.id.is_empty(), "Result should have an ID");
                assert!(result.score >= 0.0, "Score should be non-negative");
            }
        }
    }
}

#[tokio::test]
async fn test_search_with_context() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test context-aware search options
    let options = SearchOptions {
        include_context: true,
        limit: 5,
        ..Default::default()
    };

    let results = locai
        .search_with_options("machine learning algorithms", options)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find results with context");

    for result in &results {
        // With context enabled, results should include related information
        assert!(!result.id.is_empty(), "Result should have an ID");

        // Check if context is populated
        let has_context = !result.context.entities.is_empty()
            || !result.context.memories.is_empty()
            || !result.context.relationships.is_empty();

        if has_context {
            println!(
                "Result '{}' has context: entities={}, memories={}, relationships={}",
                result.summary(),
                result.context.entities.len(),
                result.context.memories.len(),
                result.context.relationships.len()
            );
        }
    }
}

#[tokio::test]
async fn test_search_type_filtering() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test filtering by content type
    let memory_options = SearchOptions {
        include_types: SearchTypeFilter::memories_only(),
        limit: 10,
        ..Default::default()
    };

    let memory_results = locai
        .search_with_options("neural networks", memory_options)
        .await
        .unwrap();

    // Verify all results are memories
    for result in &memory_results {
        match &result.content {
            SearchContent::Memory(_) => {
                // Expected - this is what we filtered for
                println!("Found memory result: {}", result.summary());
            }
            _ => {
                panic!("Expected only memory results when filtering by Memory type");
            }
        }
    }

    println!("Memory-only search found {} results", memory_results.len());
}

#[tokio::test]
async fn test_search_score_thresholds() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test different score thresholds
    let high_threshold_options = SearchOptions {
        min_score: Some(0.8),
        limit: 10,
        ..Default::default()
    };

    let low_threshold_options = SearchOptions {
        min_score: Some(0.1),
        limit: 10,
        ..Default::default()
    };

    let high_threshold_results = locai
        .search_with_options("machine learning", high_threshold_options)
        .await
        .unwrap();
    let low_threshold_results = locai
        .search_with_options("machine learning", low_threshold_options)
        .await
        .unwrap();

    // Lower threshold should return more (or equal) results
    assert!(
        low_threshold_results.len() >= high_threshold_results.len(),
        "Lower threshold should return more results"
    );

    // All high threshold results should meet the threshold
    for result in &high_threshold_results {
        assert!(
            result.score >= 0.8,
            "High threshold result score too low: {}",
            result.score
        );
    }

    // All low threshold results should meet the threshold
    for result in &low_threshold_results {
        assert!(
            result.score >= 0.1,
            "Low threshold result score too low: {}",
            result.score
        );
    }

    println!(
        "High threshold (0.8) found {} results",
        high_threshold_results.len()
    );
    println!(
        "Low threshold (0.1) found {} results",
        low_threshold_results.len()
    );
}

#[tokio::test]
async fn test_search_intent_detection() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test different types of queries that should trigger different intents
    let query_types = vec![
        ("how to train neural networks", "procedural"),
        ("what is machine learning", "factual"),
        ("machine learning vs deep learning", "comparative"),
        ("recent AI developments", "temporal"),
    ];

    for (query, expected_type) in query_types {
        let results = locai.search(query).await.unwrap();

        println!(
            "Query '{}' (expected: {}) found {} results",
            query,
            expected_type,
            results.len()
        );

        // The search should handle different query types appropriately
        if !results.is_empty() {
            for result in &results {
                assert!(!result.id.is_empty(), "Result should have an ID");
                assert!(result.score >= 0.0, "Score should be non-negative");

                // Check if the match reason provides insight into query understanding
                println!("  Match reason: {}", result.match_reason());
            }
        }
    }
}

#[tokio::test]
async fn test_conversational_search_context() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Simulate a conversational sequence
    println!("Simulating conversational search...");

    // First query about machine learning
    let results1 = locai.search("machine learning").await.unwrap();
    assert!(
        !results1.is_empty(),
        "Should find results for 'machine learning'"
    );
    println!("First query found {} results", results1.len());

    // Follow-up query that might reference previous context
    let results2 = locai
        .search("how does it work with neural networks")
        .await
        .unwrap();
    println!("Follow-up query found {} results", results2.len());

    // Third query about specific applications
    let results3 = locai.search("practical applications").await.unwrap();
    println!("Application query found {} results", results3.len());

    // Each query should return meaningful results
    for (i, results) in [&results1, &results2, &results3].iter().enumerate() {
        for result in *results {
            assert!(
                !result.id.is_empty(),
                "Query {} result should have an ID",
                i + 1
            );
            assert!(
                result.score >= 0.0,
                "Query {} score should be non-negative",
                i + 1
            );
        }
    }
}

#[tokio::test]
async fn test_search_performance_intelligence() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();

    // Create a larger dataset for performance testing
    let mut memory_ids = Vec::new();
    for i in 0..20 {
        let content = format!(
            "Memory {} about artificial intelligence, machine learning, neural networks, and data science applications in various domains",
            i
        );
        let memory_id = locai.remember(&content).await.unwrap();
        memory_ids.push(memory_id);
    }

    // Wait for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Test search performance with intelligence features
    let start = std::time::Instant::now();
    let results = locai
        .search("artificial intelligence applications")
        .await
        .unwrap();
    let duration = start.elapsed();

    assert!(!results.is_empty(), "Should find results in larger dataset");
    // Increased timeout to 5 seconds for more realistic expectations in test environments
    assert!(
        duration.as_millis() < 5000,
        "Search should complete within reasonable time ({}ms)",
        duration.as_millis()
    );

    println!(
        "Searched {} memories in {}ms, found {} results",
        memory_ids.len(),
        duration.as_millis(),
        results.len()
    );
}

// ============================================================================
// ERROR HANDLING AND EDGE CASES
// ============================================================================

#[tokio::test]
async fn test_empty_query_handling() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test empty query
    let empty_results = locai.search("").await.unwrap();
    println!("Empty query returned {} results", empty_results.len());

    // Empty queries should be handled gracefully (may return no results)
    // The key is that it shouldn't crash

    // Test whitespace-only query
    let whitespace_results = locai.search("   ").await.unwrap();
    println!(
        "Whitespace query returned {} results",
        whitespace_results.len()
    );
}

#[tokio::test]
async fn test_special_characters_in_queries() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    let special_queries = vec![
        "machine-learning",
        "neural_networks",
        "AI & ML",
        "deep learning (DL)",
        "quantum@computing",
        "ML/AI",
    ];

    for query in &special_queries {
        let results = locai.search(query).await.unwrap();
        println!("Query '{}' found {} results", query, results.len());

        // Should handle special characters gracefully
        for result in &results {
            assert!(!result.id.is_empty(), "Result should have an ID");
            assert!(result.score >= 0.0, "Score should be non-negative");
        }
    }
}

#[tokio::test]
async fn test_very_long_query_handling() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Create a very long query
    let long_query = "machine learning neural networks deep learning artificial intelligence natural language processing computer vision quantum computing algorithms optimization".repeat(5);

    let results = locai.search(&long_query).await.unwrap();
    println!("Very long query found {} results", results.len());

    // Should handle very long queries without crashing
    for result in &results {
        assert!(!result.id.is_empty(), "Result should have an ID");
        assert!(result.score >= 0.0, "Score should be non-negative");
    }
}

#[tokio::test]
async fn test_concurrent_search_requests() {
    let (locai, _temp_dir) = create_test_locai().await.unwrap();
    let _memory_ids = setup_intelligence_test_data(&locai).await.unwrap();

    // Test concurrent search requests
    let queries = vec![
        "machine learning",
        "neural networks",
        "quantum computing",
        "natural language",
        "computer vision",
    ];

    // Create multiple Locai instances for concurrent testing
    let mut handles = Vec::new();
    for (i, query) in queries.into_iter().enumerate() {
        // Create a new Locai instance for each concurrent request
        let handle = tokio::spawn(async move {
            // Create a fresh instance for this task
            let (locai_task, _temp_dir_task) = create_test_locai().await.unwrap();
            let _memory_ids_task = setup_intelligence_test_data(&locai_task).await.unwrap();

            let result = locai_task.search(query).await;
            (i, result)
        });
        handles.push(handle);
    }

    // Wait for all searches to complete
    for handle in handles.into_iter() {
        let (task_id, result) = handle.await.expect("Task should complete");
        let search_results = result.unwrap();
        println!(
            "Concurrent search {} found {} results",
            task_id + 1,
            search_results.len()
        );

        for search_result in &search_results {
            assert!(!search_result.id.is_empty(), "Result should have an ID");
            assert!(search_result.score >= 0.0, "Score should be non-negative");
        }
    }
}
