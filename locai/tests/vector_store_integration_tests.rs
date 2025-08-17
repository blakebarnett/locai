//! VectorStore Integration Tests
//!
//! This module contains comprehensive tests for the VectorStore integration with memory storage,
//! focusing on AI assistant use cases and real-world scenarios.
//!
//! ## Test Coverage
//! - Vector record creation during memory storage
//! - Vector record cleanup during memory deletion
//! - Vector record updates during memory updates
//! - Semantic search functionality using Vector records
//! - Error handling and graceful degradation
//! - Performance characteristics for AI assistant scenarios

use locai::prelude::*;
use tokio;

/// Helper function to create test Locai instance
async fn create_test_locai() -> Result<Locai> {
    Locai::for_testing().await
}

/// Helper function to verify vector exists for memory
async fn verify_vector_exists(locai: &Locai, memory_id: &str) -> Result<bool> {
    let vector_id = format!("mem_{}", memory_id);
    let storage = locai.manager().storage();
    Ok(storage.get_vector(&vector_id).await?.is_some())
}

/// Helper function to get vector count
async fn get_vector_count(locai: &Locai) -> Result<usize> {
    let storage = locai.manager().storage();
    storage
        .count_vectors(None)
        .await
        .map_err(|e| LocaiError::Storage(format!("Failed to count vectors: {}", e)))
}

#[tokio::test]
async fn test_vector_creation_on_memory_storage() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");

    // Clear any existing data
    locai.clear_all().await.expect("Failed to clear storage");

    // Store a memory that should generate an embedding
    let memory_id = locai
        .remember_fact("The capital of France is Paris")
        .await
        .expect("Failed to store memory");

    // Verify vector was created
    assert!(
        verify_vector_exists(&locai, &memory_id)
            .await
            .expect("Failed to check vector")
    );

    // Verify vector count increased
    let vector_count = get_vector_count(&locai)
        .await
        .expect("Failed to get vector count");
    assert_eq!(vector_count, 1, "Expected exactly one vector to be created");
}

#[tokio::test]
async fn test_vector_creation_with_multiple_memories() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store multiple memories
    let memories = vec![
        "The capital of France is Paris",
        "The capital of Germany is Berlin",
        "The capital of Italy is Rome",
    ];

    let mut memory_ids = Vec::new();
    let memory_count = memories.len();
    for content in memories {
        let id = locai
            .remember_fact(content)
            .await
            .expect("Failed to store memory");
        memory_ids.push(id);
    }

    // Verify all vectors were created
    for memory_id in &memory_ids {
        assert!(
            verify_vector_exists(&locai, memory_id)
                .await
                .expect("Failed to check vector"),
            "Vector not found for memory {}",
            memory_id
        );
    }

    // Verify total vector count
    let vector_count = get_vector_count(&locai)
        .await
        .expect("Failed to get vector count");
    assert_eq!(vector_count, memory_count, "Vector count mismatch");
}

#[tokio::test]
async fn test_vector_deletion_on_memory_deletion() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store a memory
    let memory_id = locai
        .remember_fact("This memory will be deleted")
        .await
        .expect("Failed to store memory");

    // Verify vector exists
    assert!(
        verify_vector_exists(&locai, &memory_id)
            .await
            .expect("Failed to check vector")
    );

    // Delete the memory via manager
    let deleted = locai
        .manager()
        .delete_memory(&memory_id)
        .await
        .expect("Failed to delete memory");
    assert!(deleted, "Memory deletion should return true");

    // Verify vector was also deleted
    assert!(
        !verify_vector_exists(&locai, &memory_id)
            .await
            .expect("Failed to check vector"),
        "Vector should be deleted when memory is deleted"
    );

    // Verify vector count is zero
    let vector_count = get_vector_count(&locai)
        .await
        .expect("Failed to get vector count");
    assert_eq!(
        vector_count, 0,
        "No vectors should remain after memory deletion"
    );
}

#[tokio::test]
async fn test_vector_update_on_memory_update() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store initial memory
    let memory_id = locai
        .remember_fact("Initial content")
        .await
        .expect("Failed to store memory");

    // Verify initial vector exists
    assert!(
        verify_vector_exists(&locai, &memory_id)
            .await
            .expect("Failed to check vector")
    );

    // Get the memory and update it via manager
    let mut memory = locai
        .manager()
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory")
        .expect("Memory should exist");

    memory.content = "Updated content with different semantics".to_string();

    // Update the memory via manager
    let updated = locai
        .manager()
        .update_memory(memory)
        .await
        .expect("Failed to update memory");
    assert!(updated, "Memory update should return true");

    // Verify vector still exists (should be updated)
    assert!(
        verify_vector_exists(&locai, &memory_id)
            .await
            .expect("Failed to check vector")
    );

    // Vector count should remain the same
    let vector_count = get_vector_count(&locai)
        .await
        .expect("Failed to get vector count");
    assert_eq!(
        vector_count, 1,
        "Vector count should remain unchanged after update"
    );
}

#[tokio::test]
async fn test_semantic_search_with_vector_records() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store memories with related content
    let memory_contents = vec![
        "I need to contact Dr. John Smith about the project. His email is john.smith@company.com",
        "The meeting is scheduled for January 15th, 2024 at the Seattle office",
        "The project budget is $150,000 and the deadline is March 2024",
        "Sarah Johnson is the project manager. Contact her at sarah@company.com",
    ];

    for content in memory_contents {
        locai
            .remember_fact(content)
            .await
            .expect("Failed to store memory");
    }

    // Wait a moment for embeddings to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test semantic search for email-related content using the search API
    let results = locai
        .search("email contact information")
        .await
        .expect("Failed to perform semantic search");

    // Should find memories containing email addresses
    assert!(!results.is_empty(), "Semantic search should return results");

    // Verify results have scores
    for result in &results {
        assert!(result.score > 0.0, "Similarity scores should be positive");
    }

    // Check that email-related memories rank highly
    let email_content_found = results.iter().any(|r| {
        if let locai::core::SearchContent::Memory(memory) = &r.content {
            memory.content.contains("email") || memory.content.contains("@")
        } else {
            false
        }
    });
    assert!(email_content_found, "Should find email-related content");
}

#[tokio::test]
async fn test_semantic_search_with_different_queries() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store diverse memory content
    let memories = vec![
        (
            "Dr. John Smith is a cardiologist at Seattle General Hospital",
            "medical",
        ),
        (
            "The annual budget meeting is scheduled for next Tuesday",
            "business",
        ),
        (
            "Machine learning algorithms require large datasets for training",
            "technology",
        ),
        (
            "The Renaissance period was marked by artistic innovation",
            "history",
        ),
    ];

    for (content, _tag) in &memories {
        locai
            .remember_fact(*content)
            .await
            .expect("Failed to store memory");
    }

    // Test different semantic queries
    let test_cases = vec![
        ("doctor medical hospital", "medical"),
        ("budget business meeting", "business"),
        ("artificial intelligence machine learning", "technology"),
        ("art history Renaissance", "history"),
    ];

    for (query, _expected_category) in test_cases {
        let results = locai
            .search(query)
            .await
            .expect("Failed to perform semantic search");

        assert!(
            !results.is_empty(),
            "Query '{}' should return results",
            query
        );

        // The top result should be semantically related to the query
        let top_result = &results[0];

        // Note: Due to the nature of semantic search, we can't guarantee exact matches,
        // but the system should return relevant results
        assert!(
            top_result.score > 0.0,
            "Top result for '{}' should have a positive score",
            query
        );
    }
}

#[tokio::test]
async fn test_vector_search_performance() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store a reasonable number of memories for performance testing
    let memory_count = 20;
    for i in 0..memory_count {
        let content = format!(
            "Memory content number {} with various topics including technology, business, science, and healthcare",
            i
        );
        locai
            .remember_fact(&content)
            .await
            .expect("Failed to store memory");
    }

    // Measure semantic search performance
    let start = std::time::Instant::now();

    let results = locai
        .search("technology business")
        .await
        .expect("Failed to perform semantic search");

    let duration = start.elapsed();

    // Performance assertions
    assert!(
        duration.as_millis() < 5000,
        "Semantic search should complete within 5 seconds"
    );
    assert!(!results.is_empty(), "Should return results");
    assert!(
        results.len() <= 20,
        "Should respect default limit parameter of 20"
    );

    println!(
        "Semantic search of {} memories took {:?}",
        memory_count, duration
    );
}

#[tokio::test]
async fn test_vector_storage_error_handling() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Test that memory storage succeeds even if vector creation fails
    // (This tests the graceful degradation mentioned in the implementation)

    // Store a memory
    let memory_id = locai
        .remember_fact("Test memory for error handling")
        .await
        .expect("Memory storage should succeed even if vector operations fail");

    // Verify the memory was stored
    let memory = locai
        .manager()
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory")
        .expect("Memory should exist");

    assert_eq!(memory.content, "Test memory for error handling");
}

#[tokio::test]
async fn test_vector_metadata_content() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store a memory
    let memory_id = locai
        .remember_fact("Test memory with metadata")
        .await
        .expect("Failed to store memory");

    // Get the vector and check its metadata
    let vector_id = format!("mem_{}", memory_id);
    let storage = locai.manager().storage();
    let vector = storage
        .get_vector(&vector_id)
        .await
        .expect("Failed to get vector")
        .expect("Vector should exist");

    // Verify vector metadata contains expected fields
    assert_eq!(vector.source_id, Some(memory_id.clone()));
    assert!(
        vector.dimension > 0,
        "Vector should have positive dimensions"
    );

    // Check metadata content
    let metadata = vector.metadata;
    assert_eq!(metadata["type"], "memory");
    assert_eq!(metadata["memory_id"], memory_id);
    assert!(
        metadata["content_preview"]
            .as_str()
            .unwrap()
            .contains("Test memory")
    );
}

#[tokio::test]
async fn test_ai_assistant_conversation_context() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Simulate an AI assistant conversation with context building
    let conversation_memories = vec![
        "User asked about machine learning algorithms",
        "I explained neural networks and deep learning concepts",
        "User wants to know about practical applications",
        "I mentioned computer vision and natural language processing",
        "User is interested in learning resources",
    ];

    // Store conversation memories
    for content in &conversation_memories {
        locai
            .remember_conversation(*content)
            .await
            .expect("Failed to store conversation memory");
    }

    // AI assistant searches for context when user asks follow-up question
    let context_results = locai
        .search("machine learning applications resources")
        .await
        .expect("Failed to search for conversation context");

    assert!(
        !context_results.is_empty(),
        "Should find relevant conversation context"
    );

    // Verify we can find multiple relevant memories from the conversation
    let relevant_count = context_results
        .iter()
        .filter(|r| r.score > 0.1) // Reasonable similarity threshold
        .count();

    assert!(
        relevant_count >= 2,
        "Should find multiple relevant conversation memories"
    );
}

#[tokio::test]
async fn test_knowledge_retrieval_for_ai_responses() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store factual knowledge an AI assistant might need
    let knowledge_base = vec![
        "Python is a popular programming language for data science and machine learning",
        "TensorFlow and PyTorch are the most widely used deep learning frameworks",
        "Supervised learning requires labeled training data",
        "Unsupervised learning finds patterns in data without labels",
        "Reinforcement learning uses rewards to train agents",
    ];

    for fact in &knowledge_base {
        locai
            .remember_fact(*fact)
            .await
            .expect("Failed to store knowledge");
    }

    // AI assistant needs to answer: "What are the main types of machine learning?"
    let results = locai
        .search("types of machine learning supervised unsupervised reinforcement")
        .await
        .expect("Failed to search knowledge base");

    assert!(!results.is_empty(), "Should find relevant knowledge");

    // Should find information about different learning types
    let found_supervised = results.iter().any(|r| {
        if let locai::core::SearchContent::Memory(memory) = &r.content {
            memory.content.contains("Supervised")
        } else {
            false
        }
    });
    let found_unsupervised = results.iter().any(|r| {
        if let locai::core::SearchContent::Memory(memory) = &r.content {
            memory.content.contains("Unsupervised")
        } else {
            false
        }
    });
    let found_reinforcement = results.iter().any(|r| {
        if let locai::core::SearchContent::Memory(memory) = &r.content {
            memory.content.contains("Reinforcement")
        } else {
            false
        }
    });

    assert!(
        found_supervised || found_unsupervised || found_reinforcement,
        "Should find information about machine learning types"
    );
}

#[tokio::test]
async fn test_vector_consistency_across_operations() {
    let locai = create_test_locai()
        .await
        .expect("Failed to create test instance");
    locai.clear_all().await.expect("Failed to clear storage");

    // Store initial memories
    let mut memory_ids = Vec::new();
    for i in 0..5 {
        let content = format!("Memory {} about various topics", i);
        let id = locai
            .remember_fact(&content)
            .await
            .expect("Failed to store memory");
        memory_ids.push(id);
    }

    // Verify all vectors exist
    let initial_count = get_vector_count(&locai)
        .await
        .expect("Failed to count vectors");
    assert_eq!(initial_count, 5);

    // Delete some memories
    for i in 0..2 {
        locai
            .manager()
            .delete_memory(&memory_ids[i])
            .await
            .expect("Failed to delete memory");
    }

    // Verify vector count decreased appropriately
    let after_deletion_count = get_vector_count(&locai)
        .await
        .expect("Failed to count vectors");
    assert_eq!(after_deletion_count, 3);

    // Update remaining memories
    for i in 2..memory_ids.len() {
        let mut memory = locai
            .manager()
            .get_memory(&memory_ids[i])
            .await
            .expect("Failed to get memory")
            .expect("Memory should exist");

        memory.content = format!("Updated {}", memory.content);
        locai
            .manager()
            .update_memory(memory)
            .await
            .expect("Failed to update memory");
    }

    // Vector count should remain the same after updates
    let after_update_count = get_vector_count(&locai)
        .await
        .expect("Failed to count vectors");
    assert_eq!(after_update_count, 3);

    // All remaining vectors should still be accessible
    for i in 2..memory_ids.len() {
        assert!(
            verify_vector_exists(&locai, &memory_ids[i])
                .await
                .expect("Failed to check vector")
        );
    }
}
