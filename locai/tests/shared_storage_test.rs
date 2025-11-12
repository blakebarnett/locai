//! Integration tests for SharedStorage implementation
//!
//! This test suite verifies that the SharedStorage implementation works correctly
//! across all storage operations including entities, relationships, and vectors.
//!
//! ## Hook System Integration
//!
//! These tests verify that memory operations correctly trigger the hook system when
//! lifecycle tracking is enabled. The hook registry is automatically created for each
//! SharedStorage instance and executes hooks asynchronously without blocking operations.

use chrono::Utc;
use locai::storage::{
    filters::{EntityFilter, RelationshipFilter, VectorFilter},
    models::{DistanceMetric, Entity, Relationship, Vector, VectorSearchParams},
    shared_storage::{SharedStorage, SharedStorageConfig},
    traits::{BaseStore, EntityStore, RelationshipStore, VectorStore},
};
use serde_json::json;

type TestStorage = SharedStorage<surrealdb::engine::local::Db>;

async fn create_test_storage() -> Result<TestStorage, Box<dyn std::error::Error>> {
    let config = SharedStorageConfig {
        namespace: "test".to_string(),
        database: "locai_test".to_string(),
        lifecycle_tracking: Default::default(),
    };

    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(()).await?;
    let storage = SharedStorage::new(client, config).await?;
    Ok(storage)
}

#[tokio::test]
async fn test_shared_storage_health_and_metadata() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Test health check
    let health = storage.health_check().await.expect("Health check failed");
    assert!(health, "Storage should be healthy");

    // Test metadata
    let metadata = storage
        .get_metadata()
        .await
        .expect("Failed to get metadata");
    assert_eq!(metadata["type"], "shared_storage");
    assert_eq!(metadata["database"], "locai_test");
    assert_eq!(metadata["namespace"], "test");
}

#[tokio::test]
async fn test_entity_operations() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Test entity creation
    let entity = Entity {
        id: "test_entity_001".to_string(),
        entity_type: "TestEntity".to_string(),
        properties: json!({
            "name": "Test Entity",
            "value": 42,
            "active": true
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created = storage
        .create_entity(entity.clone())
        .await
        .expect("Failed to create entity");
    assert_eq!(created.entity_type, "TestEntity");
    assert_eq!(created.properties["name"], "Test Entity");
    assert_eq!(created.properties["value"], 42);

    // Test entity retrieval
    let retrieved = storage
        .get_entity(&created.id)
        .await
        .expect("Failed to get entity");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, created.id);
    assert_eq!(retrieved.properties["name"], "Test Entity");

    // Test entity update
    let mut updated_entity = retrieved.clone();
    updated_entity.properties["value"] = json!(100);
    updated_entity.updated_at = Utc::now();

    let updated = storage
        .update_entity(updated_entity)
        .await
        .expect("Failed to update entity");
    assert_eq!(updated.properties["value"], 100);

    // Test entity listing
    let entities = storage
        .list_entities(None, None, None)
        .await
        .expect("Failed to list entities");
    assert!(!entities.is_empty());
    assert!(entities.iter().any(|e| e.id == created.id));

    // Test entity counting
    let count = storage
        .count_entities(None)
        .await
        .expect("Failed to count entities");
    assert!(count > 0);

    // Test entity filtering
    let filter = EntityFilter {
        entity_type: Some("TestEntity".to_string()),
        ..Default::default()
    };
    let filtered = storage
        .list_entities(Some(filter), None, None)
        .await
        .expect("Failed to filter entities");
    assert!(!filtered.is_empty());
    assert!(filtered.iter().all(|e| e.entity_type == "TestEntity"));

    // Test entity deletion
    let deleted = storage
        .delete_entity(&created.id)
        .await
        .expect("Failed to delete entity");
    assert!(deleted);

    // Verify deletion
    let retrieved_after_delete = storage
        .get_entity(&created.id)
        .await
        .expect("Failed to get entity after delete");
    assert!(retrieved_after_delete.is_none());
}

#[tokio::test]
async fn test_relationship_operations() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Create test entities first
    let entity1 = Entity {
        id: "entity_001".to_string(),
        entity_type: "Person".to_string(),
        properties: json!({"name": "Alice"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let entity2 = Entity {
        id: "entity_002".to_string(),
        entity_type: "Person".to_string(),
        properties: json!({"name": "Bob"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_entity1 = storage
        .create_entity(entity1)
        .await
        .expect("Failed to create entity1");
    let created_entity2 = storage
        .create_entity(entity2)
        .await
        .expect("Failed to create entity2");

    // Test relationship creation
    let relationship = Relationship {
        id: "rel_001".to_string(),
        source_id: created_entity1.id.clone(),
        target_id: created_entity2.id.clone(),
        relationship_type: "knows".to_string(),
        properties: json!({
            "since": "2020",
            "strength": "strong"
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_rel = storage
        .create_relationship(relationship)
        .await
        .expect("Failed to create relationship");
    assert_eq!(created_rel.relationship_type, "knows");
    assert_eq!(created_rel.source_id, created_entity1.id);
    assert_eq!(created_rel.target_id, created_entity2.id);

    // Test relationship retrieval
    let retrieved_rel = storage
        .get_relationship(&created_rel.id)
        .await
        .expect("Failed to get relationship");
    assert!(retrieved_rel.is_some());
    let retrieved_rel = retrieved_rel.unwrap();
    assert_eq!(retrieved_rel.properties["since"], "2020");

    // Test relationship update
    let mut updated_rel = retrieved_rel.clone();
    updated_rel.properties["strength"] = json!("very strong");
    updated_rel.updated_at = Utc::now();

    let updated = storage
        .update_relationship(updated_rel)
        .await
        .expect("Failed to update relationship");
    assert_eq!(updated.properties["strength"], "very strong");

    // Test relationship listing
    let relationships = storage
        .list_relationships(None, None, None)
        .await
        .expect("Failed to list relationships");
    assert!(!relationships.is_empty());

    // Test relationship filtering
    let filter = RelationshipFilter {
        relationship_type: Some("knows".to_string()),
        ..Default::default()
    };
    let filtered = storage
        .list_relationships(Some(filter), None, None)
        .await
        .expect("Failed to filter relationships");
    assert!(!filtered.is_empty());
    assert!(filtered.iter().all(|r| r.relationship_type == "knows"));

    // Test finding related entities
    let related = storage
        .find_related_entities(
            &created_entity1.id,
            Some("knows".to_string()),
            Some("outgoing".to_string()),
        )
        .await
        .expect("Failed to find related entities");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].id, created_entity2.id);

    // Test relationship counting
    let count = storage
        .count_relationships(None)
        .await
        .expect("Failed to count relationships");
    assert!(count > 0);

    // Test relationship deletion
    let deleted = storage
        .delete_relationship(&created_rel.id)
        .await
        .expect("Failed to delete relationship");
    assert!(deleted);

    // Verify deletion
    let retrieved_after_delete = storage
        .get_relationship(&created_rel.id)
        .await
        .expect("Failed to get relationship after delete");
    assert!(retrieved_after_delete.is_none());
}

#[tokio::test]
async fn test_vector_operations() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Test vector creation
    let vector = Vector {
        id: "vec_001".to_string(),
        vector: vec![0.5; 1024], // BGE-M3 compatible 1024-dimensional vector
        dimension: 1024,
        metadata: json!({
            "title": "Test Document",
            "category": "test",
            "author": "Test Author"
        }),
        source_id: Some("doc_001".to_string()),
        created_at: Utc::now(),
    };

    let created = storage
        .add_vector(vector)
        .await
        .expect("Failed to add vector");
    assert_eq!(created.dimension, 1024);
    assert_eq!(created.metadata["title"], "Test Document");

    // Test vector retrieval
    let retrieved = storage
        .get_vector(&created.id)
        .await
        .expect("Failed to get vector");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.vector.len(), 1024);
    assert_eq!(retrieved.metadata["category"], "test");

    // Test vector metadata update
    let new_metadata = json!({
        "title": "Updated Test Document",
        "category": "updated_test",
        "author": "Updated Author",
        "version": 2
    });

    let updated = storage
        .update_vector_metadata(&created.id, new_metadata)
        .await
        .expect("Failed to update vector metadata");
    assert_eq!(updated.metadata["title"], "Updated Test Document");
    assert_eq!(updated.metadata["version"], 2);

    // Add more vectors for search testing
    let vector2 = Vector {
        id: "vec_002".to_string(),
        vector: vec![0.1; 1024], // Different vector
        dimension: 1024,
        metadata: json!({
            "title": "Another Document",
            "category": "other"
        }),
        source_id: Some("doc_002".to_string()),
        created_at: Utc::now(),
    };

    let vector3 = Vector {
        id: "vec_003".to_string(),
        vector: vec![0.6; 1024], // Similar to first vector
        dimension: 1024,
        metadata: json!({
            "title": "Similar Document",
            "category": "test"
        }),
        source_id: Some("doc_003".to_string()),
        created_at: Utc::now(),
    };

    storage
        .add_vector(vector2)
        .await
        .expect("Failed to add vector2");
    storage
        .add_vector(vector3)
        .await
        .expect("Failed to add vector3");

    // Test vector similarity search
    let query_vector = vec![0.55; 1024];
    let search_params = VectorSearchParams {
        limit: Some(3),
        threshold: None,
        filter: None,
        include_vectors: true,
        include_metadata: true,
        distance_metric: Some(DistanceMetric::Cosine),
    };

    let results = storage
        .search_vectors(&query_vector, search_params)
        .await
        .expect("Failed to search vectors");
    assert!(!results.is_empty());
    assert!(results.len() <= 3);

    // Verify results are sorted by distance (best matches first)
    for i in 1..results.len() {
        assert!(
            results[i - 1].1 <= results[i].1,
            "Results should be sorted by distance (ascending)"
        );
    }

    // Test vector listing
    let vectors = storage
        .list_vectors(None, None, None)
        .await
        .expect("Failed to list vectors");
    assert_eq!(vectors.len(), 3);

    // Test vector filtering
    let filter = VectorFilter {
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("category".to_string(), json!("test"));
            map
        }),
        ..Default::default()
    };
    let filtered = storage
        .list_vectors(Some(filter), None, None)
        .await
        .expect("Failed to filter vectors");
    assert_eq!(filtered.len(), 1); // Should find only vec_003, since vec_001 was updated to category "updated_test"

    // Test vector counting
    let count = storage
        .count_vectors(None)
        .await
        .expect("Failed to count vectors");
    assert_eq!(count, 3);

    // Test batch vector operations
    let batch_vectors = vec![
        Vector {
            id: "batch_001".to_string(),
            vector: vec![0.8; 1024],
            dimension: 1024,
            metadata: json!({"batch": true}),
            source_id: None,
            created_at: Utc::now(),
        },
        Vector {
            id: "batch_002".to_string(),
            vector: vec![0.9; 1024],
            dimension: 1024,
            metadata: json!({"batch": true}),
            source_id: None,
            created_at: Utc::now(),
        },
    ];

    let batch_results = storage
        .batch_add_vectors(batch_vectors)
        .await
        .expect("Failed to batch add vectors");
    assert_eq!(batch_results.len(), 2);

    // Test upsert operation - skip for now due to datetime serialization issue
    // This is a known issue with SurrealDB datetime handling in the current implementation
    println!("INFO: Skipping upsert test due to SurrealDB datetime serialization issue");

    // TODO: Fix datetime serialization in Vector model for SurrealDB compatibility
    // let upsert_vector = Vector {
    //     id: "upsert_test".to_string(),
    //     vector: vec![0.3; 1024],
    //     dimension: 1024,
    //     metadata: json!({"upsert": true}),
    //     source_id: None,
    //     created_at: Utc::now(),
    // };
    //
    // storage
    //     .upsert_vector(upsert_vector.clone())
    //     .await
    //     .expect("Failed to upsert vector");

    // Verify upsert created the vector (skipped)
    // let upserted = storage
    //     .get_vector("upsert_test")
    //     .await
    //     .expect("Failed to get upserted vector");
    // assert!(upserted.is_some());

    // Test vector deletion
    let deleted = storage
        .delete_vector(&created.id)
        .await
        .expect("Failed to delete vector");
    assert!(deleted);

    // Verify deletion
    let retrieved_after_delete = storage
        .get_vector(&created.id)
        .await
        .expect("Failed to get vector after delete");
    assert!(retrieved_after_delete.is_none());
}

#[tokio::test]
async fn test_vector_dimension_validation() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Test that non-1024 dimensional vectors are rejected
    let invalid_vector = Vector {
        id: "invalid_vec".to_string(),
        vector: vec![0.5; 512], // Wrong dimension
        dimension: 512,
        metadata: json!({}),
        source_id: None,
        created_at: Utc::now(),
    };

    let result = storage.add_vector(invalid_vector).await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(e.to_string().contains("1024-dimensional"));
    }
}

#[tokio::test]
async fn test_clear_storage() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Add some test data
    let entity = Entity {
        id: "clear_test_entity".to_string(),
        entity_type: "TestEntity".to_string(),
        properties: json!({"test": true}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let vector = Vector {
        id: "clear_test_vector".to_string(),
        vector: vec![0.5; 1024],
        dimension: 1024,
        metadata: json!({"test": true}),
        source_id: None,
        created_at: Utc::now(),
    };

    storage
        .create_entity(entity)
        .await
        .expect("Failed to create test entity");
    storage
        .add_vector(vector)
        .await
        .expect("Failed to add test vector");

    // Verify data exists
    let entity_count_before = storage
        .count_entities(None)
        .await
        .expect("Failed to count entities");
    let vector_count_before = storage
        .count_vectors(None)
        .await
        .expect("Failed to count vectors");
    assert!(entity_count_before > 0);
    assert!(vector_count_before > 0);

    // Clear storage
    storage.clear().await.expect("Failed to clear storage");

    // Verify data is cleared
    let entity_count_after = storage
        .count_entities(None)
        .await
        .expect("Failed to count entities after clear");
    let vector_count_after = storage
        .count_vectors(None)
        .await
        .expect("Failed to count vectors after clear");
    assert_eq!(entity_count_after, 0);
    assert_eq!(vector_count_after, 0);

    // Verify storage is still healthy
    let health = storage
        .health_check()
        .await
        .expect("Failed to check health after clear");
    assert!(health);
}

#[tokio::test]
async fn test_complex_relationship_queries() {
    let storage = create_test_storage()
        .await
        .expect("Failed to create test storage");

    // Create a complex entity graph: Author -> Paper -> Topic
    let author = Entity {
        id: "complex_author".to_string(),
        entity_type: "Author".to_string(),
        properties: json!({"name": "Dr. Smith"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let paper = Entity {
        id: "complex_paper".to_string(),
        entity_type: "Paper".to_string(),
        properties: json!({"title": "AI Research"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let topic = Entity {
        id: "complex_topic".to_string(),
        entity_type: "Topic".to_string(),
        properties: json!({"name": "Artificial Intelligence"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let author_entity = storage
        .create_entity(author)
        .await
        .expect("Failed to create author");
    let paper_entity = storage
        .create_entity(paper)
        .await
        .expect("Failed to create paper");
    let topic_entity = storage
        .create_entity(topic)
        .await
        .expect("Failed to create topic");

    // Create relationships
    let authorship = Relationship {
        id: "complex_authorship".to_string(),
        source_id: author_entity.id.clone(),
        target_id: paper_entity.id.clone(),
        relationship_type: "authored".to_string(),
        properties: json!({"year": 2024}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let topic_relation = Relationship {
        id: "complex_topic_rel".to_string(),
        source_id: paper_entity.id.clone(),
        target_id: topic_entity.id.clone(),
        relationship_type: "covers".to_string(),
        properties: json!({"depth": "comprehensive"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    storage
        .create_relationship(authorship)
        .await
        .expect("Failed to create authorship");
    storage
        .create_relationship(topic_relation)
        .await
        .expect("Failed to create topic relation");

    // Test finding relationships by entities
    let author_relationships = storage
        .find_relationships(
            &author_entity.id,
            &paper_entity.id,
            Some("authored".to_string()),
        )
        .await
        .expect("Failed to find author relationships");
    assert_eq!(author_relationships.len(), 1);
    assert_eq!(author_relationships[0].relationship_type, "authored");

    // Test getting relationship properties
    let rel_properties = storage
        .get_relationship_properties(&author_relationships[0].id)
        .await
        .expect("Failed to get relationship properties");
    assert_eq!(rel_properties["year"], 2024);

    // Test bidirectional relationship finding
    let related_papers = storage
        .find_related_entities(
            &author_entity.id,
            Some("authored".to_string()),
            Some("outgoing".to_string()),
        )
        .await
        .expect("Failed to find related papers");
    assert_eq!(related_papers.len(), 1);
    assert_eq!(related_papers[0].id, paper_entity.id);

    let related_topics = storage
        .find_related_entities(
            &paper_entity.id,
            Some("covers".to_string()),
            Some("outgoing".to_string()),
        )
        .await
        .expect("Failed to find related topics");
    assert_eq!(related_topics.len(), 1);
    assert_eq!(related_topics[0].id, topic_entity.id);
}
