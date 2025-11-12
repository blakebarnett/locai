//! Example demonstrating SharedStorage's unified graph and vector storage capabilities
//!
//! This example shows how to use the new SharedStorage implementation that provides
//! a consistent API across different SurrealDB connection types, demonstrating
//! entity operations, relationship management, and vector similarity search.
//!
//! ## Hook System Integration
//!
//! This example demonstrates memory operations that automatically trigger hooks
//! when lifecycle tracking is enabled. The hooks system allows you to:
//! - React to memory creation, access, updates, and deletion
//! - Implement custom logic via the MemoryHook trait
//! - Send webhooks to remote endpoints for event notifications
//! - Integrate with external systems
//!
//! See `locai/src/hooks/` for implementing custom hooks and
//! `examples/memory_hooks.rs` for hook usage examples.

use chrono::Utc;
use locai::storage::{
    filters::{EntityFilter, RelationshipFilter, VectorFilter},
    models::{DistanceMetric, Entity, Relationship, Vector, VectorSearchParams},
    shared_storage::{SharedStorage, SharedStorageConfig},
    traits::{BaseStore, EntityStore, RelationshipStore, VectorStore},
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SharedStorage Example");
    println!("========================");

    // Create SharedStorage configuration for embedded RocksDB
    let config = SharedStorageConfig {
        namespace: "demo".to_string(),
        database: "locai_shared".to_string(),
        lifecycle_tracking: Default::default(),
    };

    // Create a SurrealDB client with embedded RocksDB engine
    let client =
        surrealdb::Surreal::new::<surrealdb::engine::local::RocksDb>("./shared_storage_demo.db")
            .await?;

    // Create SharedStorage instance
    let storage = SharedStorage::new(client, config).await?;

    println!("✅ Created SharedStorage with embedded RocksDB");

    // Clear any existing data from previous runs
    println!("🧹 Clearing any existing data...");
    storage.clear().await?;

    // Test 1: Entity Operations
    println!("\n👤 Testing Entity Operations");
    println!("----------------------------");

    // Create entities representing different types of content
    let author = Entity {
        id: "author_001".to_string(),
        entity_type: "Author".to_string(),
        properties: json!({
            "name": "Dr. Sarah Chen",
            "expertise": "Machine Learning",
            "affiliation": "Tech University",
            "publications": 47
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let paper = Entity {
        id: "paper_001".to_string(),
        entity_type: "ResearchPaper".to_string(),
        properties: json!({
            "title": "Advanced Neural Network Architectures",
            "abstract": "This paper explores novel approaches to neural network design",
            "year": 2024,
            "citations": 0
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let topic = Entity {
        id: "topic_001".to_string(),
        entity_type: "Topic".to_string(),
        properties: json!({
            "name": "Neural Networks",
            "field": "Artificial Intelligence",
            "description": "Computational models inspired by biological neural networks"
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Add entities
    let author_entity = storage.create_entity(author).await?;
    let paper_entity = storage.create_entity(paper).await?;
    let topic_entity = storage.create_entity(topic).await?;

    println!("📝 Created author: {}", author_entity.properties["name"]);
    println!("📄 Created paper: {}", paper_entity.properties["title"]);
    println!("🏷️  Created topic: {}", topic_entity.properties["name"]);

    // Test entity retrieval
    let retrieved_author = storage.get_entity(&author_entity.id).await?;
    if let Some(author) = retrieved_author {
        println!("🔍 Retrieved author: {}", author.properties["name"]);
    }

    // Test 2: Relationship Operations
    println!("\n🔗 Testing Relationship Operations");
    println!("----------------------------------");

    // Create relationships between entities
    let authorship = Relationship {
        id: "rel_001".to_string(),
        source_id: author_entity.id.clone(),
        target_id: paper_entity.id.clone(),
        relationship_type: "authored".to_string(),
        properties: json!({
            "role": "primary_author",
            "contribution_percentage": 85,
            "corresponding_author": true
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let topic_relation = Relationship {
        id: "rel_002".to_string(),
        source_id: paper_entity.id.clone(),
        target_id: topic_entity.id.clone(),
        relationship_type: "covers".to_string(),
        properties: json!({
            "relevance": "primary",
            "coverage_depth": "comprehensive"
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let expertise_relation = Relationship {
        id: "rel_003".to_string(),
        source_id: author_entity.id.clone(),
        target_id: topic_entity.id.clone(),
        relationship_type: "expert_in".to_string(),
        properties: json!({
            "expertise_level": "expert",
            "years_experience": 12
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Add relationships
    let auth_rel = storage.create_relationship(authorship).await?;
    let topic_rel = storage.create_relationship(topic_relation).await?;
    let expertise_rel = storage.create_relationship(expertise_relation).await?;

    println!(
        "📝 Created authorship: {} -> {} ({})",
        auth_rel.source_id, auth_rel.target_id, auth_rel.relationship_type
    );
    println!(
        "📄 Created topic relation: {} -> {} ({})",
        topic_rel.source_id, topic_rel.target_id, topic_rel.relationship_type
    );
    println!(
        "🧠 Created expertise relation: {} -> {} ({})",
        expertise_rel.source_id, expertise_rel.target_id, expertise_rel.relationship_type
    );

    // Test relationship queries
    let author_papers = storage
        .find_related_entities(
            &author_entity.id,
            Some("authored".to_string()),
            Some("outgoing".to_string()),
        )
        .await?;
    println!("📚 Author has written {} papers", author_papers.len());

    // Test 3: Vector Operations
    println!("\n🔢 Testing Vector Operations");
    println!("----------------------------");

    // Create vectors representing document embeddings
    let timestamp = Utc::now().timestamp_millis();
    let vectors = vec![
        Vector {
            id: format!("vec_001_{}", timestamp),
            vector: vec![0.8; 1024], // Neural networks paper
            dimension: 1024,
            metadata: json!({
                "document_id": "paper_001",
                "document_type": "research_paper",
                "title": "Advanced Neural Network Architectures",
                "author": "Dr. Sarah Chen"
            }),
            source_id: Some("paper_001".to_string()),
            created_at: Utc::now(),
        },
        Vector {
            id: format!("vec_002_{}", timestamp),
            vector: vec![0.1; 1024], // Quantum computing paper (different field)
            dimension: 1024,
            metadata: json!({
                "document_id": "paper_002",
                "document_type": "research_paper",
                "title": "Quantum Computing Fundamentals",
                "author": "Dr. Alex Kim"
            }),
            source_id: Some("paper_002".to_string()),
            created_at: Utc::now(),
        },
        Vector {
            id: format!("vec_003_{}", timestamp),
            vector: vec![0.7; 1024], // Deep learning paper (similar to neural networks)
            dimension: 1024,
            metadata: json!({
                "document_id": "paper_003",
                "document_type": "research_paper",
                "title": "Deep Learning Applications",
                "author": "Dr. Maria Rodriguez"
            }),
            source_id: Some("paper_003".to_string()),
            created_at: Utc::now(),
        },
    ];

    // Add vectors to the store
    for vector in vectors {
        let added = storage.add_vector(vector).await?;
        println!(
            "📊 Added vector: {} - {}",
            added.id, added.metadata["title"]
        );
    }

    // Test vector similarity search
    let query_vector = vec![0.9; 1024]; // Query for neural network-related content
    let search_params = VectorSearchParams {
        limit: Some(3),
        threshold: None,
        filter: None,
        include_vectors: true,
        include_metadata: true,
        distance_metric: Some(DistanceMetric::Cosine),
    };

    println!("\n🔍 Searching for similar vectors to neural network content...");
    let results = storage.search_vectors(&query_vector, search_params).await?;

    for (i, (vector, score)) in results.iter().enumerate() {
        println!(
            "  {}. {} (distance: {:.3}) - {}",
            i + 1,
            vector.id,
            score,
            vector.metadata["title"]
        );
    }

    // Test 4: Advanced Filtering
    println!("\n🔍 Testing Advanced Filtering");
    println!("------------------------------");

    // Filter entities by type
    let entity_filter = EntityFilter {
        entity_type: Some("Author".to_string()),
        ..Default::default()
    };
    let authors = storage
        .list_entities(Some(entity_filter), None, None)
        .await?;
    println!("👥 Found {} authors", authors.len());

    // Filter relationships by type
    let rel_filter = RelationshipFilter {
        relationship_type: Some("authored".to_string()),
        ..Default::default()
    };
    let authorships = storage
        .list_relationships(Some(rel_filter), None, None)
        .await?;
    println!("📝 Found {} authorship relationships", authorships.len());

    // Filter vectors by metadata
    let vec_filter = VectorFilter {
        metadata: Some({
            let mut map = std::collections::HashMap::new();
            map.insert("document_type".to_string(), json!("research_paper"));
            map
        }),
        ..Default::default()
    };
    let research_vectors = storage.list_vectors(Some(vec_filter), None, None).await?;
    println!("📄 Found {} research paper vectors", research_vectors.len());

    // Test 5: Database Statistics
    println!("\n📈 Database Statistics");
    println!("---------------------");

    let entity_count = storage.count_entities(None).await?;
    let relationship_count = storage.count_relationships(None).await?;
    let vector_count = storage.count_vectors(None).await?;
    let health = storage.health_check().await?;
    let metadata = storage.get_metadata().await?;

    println!("Entities: {}", entity_count);
    println!("Relationships: {}", relationship_count);
    println!("Vectors: {}", vector_count);
    println!("Health Check: {}", health);
    println!("Storage Type: {}", metadata["type"]);
    println!("Database: {}", metadata["database"]);

    // Test 6: Update and Delete Operations
    println!("\n✏️  Testing Update and Delete Operations");
    println!("---------------------------------------");

    // Update an entity
    let mut updated_author = author_entity.clone();
    updated_author.properties["publications"] = json!(48);
    updated_author.updated_at = Utc::now();

    let updated = storage.update_entity(updated_author).await?;
    println!(
        "📝 Updated author publications: {}",
        updated.properties["publications"]
    );

    // Update vector metadata - use the first vector's ID
    let first_vector_id = format!("vec_001_{}", timestamp);
    let updated_vector = storage
        .update_vector_metadata(
            &first_vector_id,
            json!({
                "document_id": "paper_001",
                "document_type": "research_paper",
                "title": "Advanced Neural Network Architectures (Revised)",
                "author": "Dr. Sarah Chen",
                "revision": 2
            }),
        )
        .await?;
    println!(
        "📊 Updated vector metadata: {}",
        updated_vector.metadata["title"]
    );

    println!("\n🎉 Example completed successfully!");
    println!("\n💡 Key Features Demonstrated:");
    println!("   • Entity CRUD operations with structured data");
    println!("   • Relationship management with properties");
    println!("   • Vector similarity search with metadata");
    println!("   • Advanced filtering across all data types");
    println!("   • Unified storage in a single SurrealDB instance");
    println!("   • BGE-M3 compatible 1024-dimensional vectors");

    println!("\n🪝 Hook System Features:");
    println!("   • Automatic hook execution on memory operations");
    println!("   • Hook registry accessible via storage.hook_registry()");
    println!("   • Non-blocking async hook execution with timeouts");
    println!("   • Extensible MemoryHook trait for custom handlers");

    // Clean up demo database
    println!("\n🧹 Cleaning up demo database...");
    let _ = std::fs::remove_dir_all("./shared_storage_demo.db");

    Ok(())
}
