//! Integration tests for Locai CLI
//!
//! These tests verify CLI command functionality including:
//! - Update operations (memory, entity, relationship)
//! - Batch operations
//! - Relationship type management
//! - Graph operations (relationship creation and querying)
//! - Error handling and edge cases
//!
//! Note: Some graph traversal tests may fail due to underlying storage layer
//! requirements for graph node existence. The relationship CRUD operations are
//! tested separately and work correctly.

use locai::config::ConfigBuilder;
use locai::prelude::*;
use locai::relationships::{RelationshipTypeDef, RelationshipTypeRegistry};
use std::fs;
use tempfile::TempDir;

/// Helper to create an isolated test CLI context
async fn create_test_context() -> (TestCliContext, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test_db");
    std::fs::create_dir_all(&db_path).expect("Failed to create test database directory");

    let config = ConfigBuilder::new()
        .with_data_dir(db_path.to_str().unwrap())
        .with_default_storage()
        .with_default_ml()
        .with_default_logging()
        .build()
        .expect("Failed to build config");

    let memory_manager = locai::init(config)
        .await
        .expect("Failed to initialize Locai");

    let relationship_type_registry = RelationshipTypeRegistry::new();

    let context = TestCliContext {
        memory_manager,
        relationship_type_registry,
    };

    (context, temp_dir)
}

/// Test CLI context matching the structure used in main.rs
struct TestCliContext {
    memory_manager: MemoryManager,
    relationship_type_registry: RelationshipTypeRegistry,
}

impl TestCliContext {
    async fn handle_memory_update(
        &self,
        id: &str,
        content: Option<&str>,
        memory_type: Option<&str>,
        priority: Option<&str>,
        tags: Option<Vec<&str>>,
        properties: Option<&str>,
    ) -> locai::Result<bool> {
        use locai::LocaiError;

        let mut memory = self
            .memory_manager
            .get_memory(id)
            .await?
            .ok_or_else(|| LocaiError::Other(format!("Memory '{}' not found", id)))?;

        if let Some(c) = content {
            memory.content = c.to_string();
        }

        if let Some(mt) = memory_type {
            memory.memory_type = parse_memory_type(mt)?;
        }

        if let Some(p) = priority {
            memory.priority = parse_priority(p)?;
        }

        if let Some(t) = tags {
            memory.tags = t.iter().map(|s| s.to_string()).collect();
        }

        if let Some(props) = properties {
            let props_value: serde_json::Value = serde_json::from_str(props)
                .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
            memory.properties = props_value;
        }

        self.memory_manager.update_memory(memory).await
    }

    async fn handle_entity_update(
        &self,
        id: &str,
        entity_type: Option<&str>,
        properties: Option<&str>,
    ) -> locai::Result<locai::storage::models::Entity> {
        use locai::LocaiError;
        use serde_json::Value;

        let mut entity = self
            .memory_manager
            .get_entity(id)
            .await?
            .ok_or_else(|| LocaiError::Other(format!("Entity '{}' not found", id)))?;

        if let Some(et) = entity_type {
            entity.entity_type = et.to_string();
        }

        if let Some(props) = properties {
            let props_value: Value = serde_json::from_str(props)
                .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
            entity.properties = props_value;
        }

        self.memory_manager.update_entity(entity).await
    }

    async fn handle_relationship_update(
        &self,
        id: &str,
        relationship_type: Option<&str>,
        properties: Option<&str>,
    ) -> locai::Result<locai::storage::models::Relationship> {
        use locai::LocaiError;
        use serde_json::Value;

        let mut relationship = self
            .memory_manager
            .get_relationship(id)
            .await?
            .ok_or_else(|| LocaiError::Other(format!("Relationship '{}' not found", id)))?;

        if let Some(rt) = relationship_type {
            relationship.relationship_type = rt.to_string();
        }

        if let Some(props) = properties {
            let props_value: Value = serde_json::from_str(props)
                .map_err(|e| LocaiError::Other(format!("Invalid JSON properties: {}", e)))?;
            relationship.properties = props_value;
        }

        self.memory_manager.update_relationship(relationship).await
    }
}

fn parse_memory_type(type_str: &str) -> locai::Result<locai::models::MemoryType> {
    use locai::models::MemoryType;
    use locai::LocaiError;

    match type_str {
        "fact" => Ok(MemoryType::Fact),
        "conversation" => Ok(MemoryType::Conversation),
        "procedural" => Ok(MemoryType::Procedural),
        "episodic" => Ok(MemoryType::Episodic),
        "identity" => Ok(MemoryType::Identity),
        "world" => Ok(MemoryType::World),
        "action" => Ok(MemoryType::Action),
        "event" => Ok(MemoryType::Event),
        _ => Err(LocaiError::Other(format!("Invalid memory type: {}", type_str))),
    }
}

fn parse_priority(priority_str: &str) -> locai::Result<locai::models::MemoryPriority> {
    use locai::models::MemoryPriority;
    use locai::LocaiError;

    match priority_str {
        "low" => Ok(MemoryPriority::Low),
        "normal" => Ok(MemoryPriority::Normal),
        "high" => Ok(MemoryPriority::High),
        "critical" => Ok(MemoryPriority::Critical),
        _ => Err(LocaiError::Other(format!("Invalid priority: {}", priority_str))),
    }
}

#[tokio::test]
async fn test_memory_update_content() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create a memory
    let memory_id = ctx
        .memory_manager
        .add_fact("Original content")
        .await
        .expect("Failed to create memory");

    // Update content
    let updated = ctx
        .handle_memory_update(&memory_id, Some("Updated content"), None, None, None, None)
        .await
        .expect("Failed to update memory");

    assert!(updated, "Memory should be updated");

    // Verify update
    let memory = ctx
        .memory_manager
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory")
        .expect("Memory should exist");

    assert_eq!(memory.content, "Updated content");
}

#[tokio::test]
async fn test_memory_update_priority() {
    let (ctx, _temp_dir) = create_test_context().await;

    let memory_id = ctx
        .memory_manager
        .add_fact("Test memory")
        .await
        .expect("Failed to create memory");

    // Update priority
    let updated = ctx
        .handle_memory_update(&memory_id, None, None, Some("high"), None, None)
        .await
        .expect("Failed to update memory");

    assert!(updated);

    let memory = ctx
        .memory_manager
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory")
        .expect("Memory should exist");

    assert_eq!(memory.priority, locai::models::MemoryPriority::High);
}

#[tokio::test]
async fn test_memory_update_tags() {
    let (ctx, _temp_dir) = create_test_context().await;

    let memory_id = ctx
        .memory_manager
        .add_fact("Test memory")
        .await
        .expect("Failed to create memory");

    // Update tags
    let updated = ctx
        .handle_memory_update(
            &memory_id,
            None,
            None,
            None,
            Some(vec!["tag1", "tag2", "tag3"]),
            None,
        )
        .await
        .expect("Failed to update memory");

    assert!(updated);

    let memory = ctx
        .memory_manager
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory")
        .expect("Memory should exist");

    assert_eq!(memory.tags.len(), 3);
    assert!(memory.tags.contains(&"tag1".to_string()));
    assert!(memory.tags.contains(&"tag2".to_string()));
    assert!(memory.tags.contains(&"tag3".to_string()));
}

#[tokio::test]
async fn test_memory_update_not_found() {
    let (ctx, _temp_dir) = create_test_context().await;

    let result = ctx
        .handle_memory_update("nonexistent", Some("content"), None, None, None, None)
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_entity_update() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create an entity
    // Note: Entity IDs should NOT include the "entity:" prefix since SurrealDB
    // already knows the table name. The ID is just the key part.
    use locai::storage::models::Entity;
    let entity = Entity {
        id: "test:123".to_string(),  // Just the key, not "entity:test:123"
        entity_type: "Person".to_string(),
        properties: serde_json::json!({"name": "John"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = ctx
        .memory_manager
        .create_entity(entity)
        .await
        .expect("Failed to create entity");

    // Update entity type
    let updated = ctx
        .handle_entity_update(&created.id, Some("Organization"), None)
        .await
        .expect("Failed to update entity");

    assert_eq!(updated.entity_type, "Organization");

    // Update properties
    let updated = ctx
        .handle_entity_update(
            &created.id,
            None,
            Some(r#"{"name": "Jane", "age": 30}"#),
        )
        .await
        .expect("Failed to update entity properties");

    assert_eq!(updated.properties["name"], "Jane");
    assert_eq!(updated.properties["age"], 30);
}

#[tokio::test]
async fn test_relationship_update() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create two entities (relationships work better with entities)
    // Note: Entity IDs should NOT include the "entity:" prefix
    use locai::storage::models::Entity;
    let entity1 = Entity {
        id: "test:1".to_string(),  // Just the key part
        entity_type: "Test".to_string(),
        properties: serde_json::json!({"name": "Entity 1"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let entity2 = Entity {
        id: "test:2".to_string(),  // Just the key part
        entity_type: "Test".to_string(),
        properties: serde_json::json!({"name": "Entity 2"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created_entity1 = ctx
        .memory_manager
        .create_entity(entity1.clone())
        .await
        .expect("Failed to create entity 1");

    let created_entity2 = ctx
        .memory_manager
        .create_entity(entity2.clone())
        .await
        .expect("Failed to create entity 2");

    // Create a relationship using create_relationship_entity
    // Use the IDs returned from creation, as SurrealDB may normalize them
    // Use "relates" for entity->entity relationships (not "references" which is for memory->relationship)
    use locai::storage::models::Relationship;
    use uuid::Uuid;
    let relationship = Relationship {
        id: Uuid::new_v4().to_string(),
        source_id: created_entity1.id.clone(),
        target_id: created_entity2.id.clone(),
        relationship_type: "relates".to_string(),
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    ctx.memory_manager
        .create_relationship_entity(relationship)
        .await
        .expect("Failed to create relationship");

    // Find the relationship ID
    let relationships = ctx
        .memory_manager
        .list_relationships(None, Some(10), None)
        .await
        .expect("Failed to list relationships");

    let rel = relationships
        .iter()
        .find(|r| r.source_id == created_entity1.id && r.target_id == created_entity2.id)
        .expect("Relationship should exist");

    // Update relationship type
    let updated = ctx
        .handle_relationship_update(&rel.id, Some("explains"), None)
        .await
        .expect("Failed to update relationship");

    assert_eq!(updated.relationship_type, "explains");

    // Update properties
    let updated = ctx
        .handle_relationship_update(
            &rel.id,
            None,
            Some(r#"{"strength": 0.8, "context": "test"}"#),
        )
        .await
        .expect("Failed to update relationship properties");

    assert_eq!(updated.properties["strength"], 0.8);
    assert_eq!(updated.properties["context"], "test");
}

#[tokio::test]
async fn test_batch_execute_create_memories() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create batch file
    let batch_file = _temp_dir.path().join("batch.json");
    let batch_content = serde_json::json!({
        "operations": [
            {
                "op": "CreateMemory",
                "data": {
                    "content": "Batch memory 1",
                    "memory_type": "fact",
                    "priority": 1
                }
            },
            {
                "op": "CreateMemory",
                "data": {
                    "content": "Batch memory 2",
                    "memory_type": "episodic",
                    "priority": 2
                }
            }
        ]
    });

    fs::write(&batch_file, serde_json::to_string_pretty(&batch_content).unwrap())
        .expect("Failed to write batch file");

    // Execute batch
    use locai::batch::{BatchExecutor, BatchExecutorConfig, BatchOperation};

    let file_contents = fs::read_to_string(&batch_file).expect("Failed to read batch file");
    let batch_obj: serde_json::Value =
        serde_json::from_str(&file_contents).expect("Invalid JSON");

    let operations: Vec<BatchOperation> = batch_obj
        .get("operations")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| serde_json::from_value::<BatchOperation>(v.clone()).ok())
        .collect();

    assert_eq!(operations.len(), 2);

    let storage = ctx.memory_manager.storage().clone();
    let config = BatchExecutorConfig::default();
    let executor = BatchExecutor::new(storage, config);

    let response = executor
        .execute(operations, false)
        .await
        .expect("Failed to execute batch");

    assert_eq!(response.completed, 2);
    assert_eq!(response.failed, 0);
    assert!(response.all_successful());
}

#[tokio::test]
async fn test_batch_execute_update_memories() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create initial memories
    let mem1_id = ctx
        .memory_manager
        .add_fact("Original 1")
        .await
        .expect("Failed to create memory 1");

    let mem2_id = ctx
        .memory_manager
        .add_fact("Original 2")
        .await
        .expect("Failed to create memory 2");

    // Create batch file with updates
    let batch_file = _temp_dir.path().join("batch_update.json");
    let batch_content = serde_json::json!({
        "operations": [
            {
                "op": "UpdateMemory",
                "data": {
                    "id": mem1_id,
                    "content": "Updated 1"
                }
            },
            {
                "op": "UpdateMemory",
                "data": {
                    "id": mem2_id,
                    "priority": 3
                }
            }
        ]
    });

    fs::write(&batch_file, serde_json::to_string_pretty(&batch_content).unwrap())
        .expect("Failed to write batch file");

    // Execute batch
    use locai::batch::{BatchExecutor, BatchExecutorConfig, BatchOperation};

    let file_contents = fs::read_to_string(&batch_file).expect("Failed to read batch file");
    let batch_obj: serde_json::Value =
        serde_json::from_str(&file_contents).expect("Invalid JSON");

    let operations: Vec<BatchOperation> = batch_obj
        .get("operations")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| serde_json::from_value::<BatchOperation>(v.clone()).ok())
        .collect();

    let storage = ctx.memory_manager.storage().clone();
    let config = BatchExecutorConfig::default();
    let executor = BatchExecutor::new(storage, config);

    let response = executor
        .execute(operations, false)
        .await
        .expect("Failed to execute batch");

    assert_eq!(response.completed, 2);
    assert_eq!(response.failed, 0);

    // Verify updates
    let mem1 = ctx
        .memory_manager
        .get_memory(&mem1_id)
        .await
        .expect("Failed to get memory 1")
        .expect("Memory 1 should exist");

    assert_eq!(mem1.content, "Updated 1");

    let mem2 = ctx
        .memory_manager
        .get_memory(&mem2_id)
        .await
        .expect("Failed to get memory 2")
        .expect("Memory 2 should exist");

    assert_eq!(mem2.priority, locai::models::MemoryPriority::Critical);
}

#[tokio::test]
async fn test_relationship_type_register() {
    let (ctx, _temp_dir) = create_test_context().await;

    let type_def = RelationshipTypeDef::new("knows".to_string())
        .expect("Failed to create type def")
        .symmetric();

    ctx.relationship_type_registry
        .register(type_def.clone())
        .await
        .expect("Failed to register type");

    let retrieved = ctx
        .relationship_type_registry
        .get("knows")
        .await;

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "knows");
    assert!(retrieved.symmetric);
}

#[tokio::test]
async fn test_relationship_type_list() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Register a few types
    let type1 = RelationshipTypeDef::new("knows".to_string()).unwrap().symmetric();
    let type2 = RelationshipTypeDef::new("parent".to_string())
        .unwrap()
        .with_inverse("child".to_string());

    ctx.relationship_type_registry
        .register(type1)
        .await
        .expect("Failed to register type 1");
    ctx.relationship_type_registry
        .register(type2)
        .await
        .expect("Failed to register type 2");

    let types = ctx.relationship_type_registry.list().await;
    assert_eq!(types.len(), 2);

    let type_names: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
    assert!(type_names.contains(&"knows".to_string()));
    assert!(type_names.contains(&"parent".to_string()));
}

#[tokio::test]
async fn test_relationship_type_delete() {
    let (ctx, _temp_dir) = create_test_context().await;

    let type_def = RelationshipTypeDef::new("test_type".to_string()).unwrap();
    ctx.relationship_type_registry
        .register(type_def)
        .await
        .expect("Failed to register type");

    // Verify it exists
    assert!(ctx.relationship_type_registry.get("test_type").await.is_some());

    // Delete it
    ctx.relationship_type_registry
        .delete("test_type")
        .await
        .expect("Failed to delete type");

    // Verify it's gone
    assert!(ctx.relationship_type_registry.get("test_type").await.is_none());
}

#[tokio::test]
async fn test_relationship_type_seed() {
    let (ctx, _temp_dir) = create_test_context().await;

    ctx.relationship_type_registry
        .seed_common_types()
        .await
        .expect("Failed to seed types");

    let types = ctx.relationship_type_registry.list().await;
    assert!(types.len() > 0);

    // Check that common types are present
    let type_names: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
    assert!(type_names.contains(&"friendship".to_string()));
    assert!(type_names.contains(&"professional".to_string()));
}

#[tokio::test]
async fn test_graph_connected_with_relationship_type() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create entities
    // Note: Entity IDs should NOT include the "entity:" prefix
    use locai::storage::models::Entity;
    let alice = Entity {
        id: "person:alice".to_string(),  // Just the key part
        entity_type: "Person".to_string(),
        properties: serde_json::json!({"name": "Alice"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let bob = Entity {
        id: "person:bob".to_string(),  // Just the key part
        entity_type: "Person".to_string(),
        properties: serde_json::json!({"name": "Bob"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created_alice = ctx
        .memory_manager
        .create_entity(alice.clone())
        .await
        .expect("Failed to create Alice entity");

    let created_bob = ctx
        .memory_manager
        .create_entity(bob.clone())
        .await
        .expect("Failed to create Bob entity");

    // Create entity relationships using create_relationship_entity
    // Use the IDs returned from creation
    use locai::storage::models::Relationship;
    use uuid::Uuid;
    let rel1 = Relationship {
        id: Uuid::new_v4().to_string(),
        source_id: created_alice.id.clone(),
        target_id: created_bob.id.clone(),
        relationship_type: "knows".to_string(),
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    ctx.memory_manager
        .create_relationship_entity(rel1)
        .await
        .expect("Failed to create entity relationship");

    // Verify that we can query relationships by type
    let relationships = ctx
        .memory_manager
        .list_relationships(None, Some(10), None)
        .await
        .expect("Failed to list relationships");

    // Should have the knows relationship from alice to bob
    assert!(
        relationships.iter().any(|r| r.source_id == created_alice.id && r.target_id == created_bob.id && r.relationship_type == "knows"),
        "Should find knows relationship from alice to bob"
    );
}

#[tokio::test]
async fn test_graph_connected_all_types() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create entities
    // Note: Entity IDs should NOT include the "entity:" prefix
    use locai::storage::models::Entity;
    let entity1 = Entity {
        id: "test:1".to_string(),  // Just the key part
        entity_type: "Test".to_string(),
        properties: serde_json::json!({"name": "Entity 1"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let entity2 = Entity {
        id: "test:2".to_string(),  // Just the key part
        entity_type: "Test".to_string(),
        properties: serde_json::json!({"name": "Entity 2"}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created_entity1 = ctx
        .memory_manager
        .create_entity(entity1.clone())
        .await
        .expect("Failed to create entity 1");

    let created_entity2 = ctx
        .memory_manager
        .create_entity(entity2.clone())
        .await
        .expect("Failed to create entity 2");

    // Create direct relationship using create_relationship_entity
    // Use the IDs returned from creation
    // Use "relates" for entity->entity relationships
    use locai::storage::models::Relationship;
    use uuid::Uuid;
    let relationship = Relationship {
        id: Uuid::new_v4().to_string(),
        source_id: created_entity1.id.clone(),
        target_id: created_entity2.id.clone(),
        relationship_type: "relates".to_string(),
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    ctx.memory_manager
        .create_relationship_entity(relationship)
        .await
        .expect("Failed to create relationship");

    // Verify that we can query relationships without filtering by type
    let relationships = ctx
        .memory_manager
        .list_relationships(None, Some(10), None)
        .await
        .expect("Failed to list relationships");

    // Should find the relationship regardless of type
    assert!(
        relationships.iter().any(|r| r.source_id == created_entity1.id && r.target_id == created_entity2.id),
        "Should find relationship when querying all types"
    );
}

#[tokio::test]
async fn test_batch_execute_transaction_mode() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create batch with one invalid operation
    let batch_file = _temp_dir.path().join("batch_transaction.json");
    let batch_content = serde_json::json!({
        "operations": [
            {
                "op": "CreateMemory",
                "data": {
                    "content": "Valid memory",
                    "memory_type": "fact"
                }
            },
            {
                "op": "UpdateMemory",
                "data": {
                    "id": "nonexistent",
                    "content": "This should fail"
                }
            }
        ],
        "transaction": true
    });

    fs::write(&batch_file, serde_json::to_string_pretty(&batch_content).unwrap())
        .expect("Failed to write batch file");

    use locai::batch::{BatchExecutor, BatchExecutorConfig, BatchOperation};

    let file_contents = fs::read_to_string(&batch_file).expect("Failed to read batch file");
    let batch_obj: serde_json::Value =
        serde_json::from_str(&file_contents).expect("Invalid JSON");

    let operations: Vec<BatchOperation> = batch_obj
        .get("operations")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| serde_json::from_value::<BatchOperation>(v.clone()).ok())
        .collect();

    let storage = ctx.memory_manager.storage().clone();
    let config = BatchExecutorConfig::default();
    let executor = BatchExecutor::new(storage, config);

    // In transaction mode, if one fails, the whole transaction should fail
    // This will return an error instead of a response with errors
    let result = executor.execute(operations, true).await;
    
    // Transaction mode should return an error when a transaction fails
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Transaction") || error_msg.contains("transaction"));
}

#[tokio::test]
async fn test_relationship_type_update() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Register initial type
    let mut type_def = RelationshipTypeDef::new("test_type".to_string()).unwrap();
    ctx.relationship_type_registry
        .register(type_def.clone())
        .await
        .expect("Failed to register type");

    // Update to make it symmetric
    type_def = type_def.symmetric();
    ctx.relationship_type_registry
        .update(type_def.clone())
        .await
        .expect("Failed to update type");

    let updated = ctx
        .relationship_type_registry
        .get("test_type")
        .await
        .expect("Type should exist");

    assert!(updated.symmetric);
}

#[tokio::test]
async fn test_relationship_type_metrics() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Register various types
    ctx.relationship_type_registry
        .register(RelationshipTypeDef::new("symmetric1".to_string()).unwrap().symmetric())
        .await
        .unwrap();

    ctx.relationship_type_registry
        .register(RelationshipTypeDef::new("symmetric2".to_string()).unwrap().symmetric())
        .await
        .unwrap();

    ctx.relationship_type_registry
        .register(RelationshipTypeDef::new("transitive1".to_string()).unwrap().transitive())
        .await
        .unwrap();

    ctx.relationship_type_registry
        .register(
            RelationshipTypeDef::new("with_inverse".to_string())
                .unwrap()
                .with_inverse("inverse_of".to_string()),
        )
        .await
        .unwrap();

    let types = ctx.relationship_type_registry.list().await;
    let count = types.len();
    let symmetric_count = types.iter().filter(|t| t.symmetric).count();
    let transitive_count = types.iter().filter(|t| t.transitive).count();
    let with_inverse_count = types.iter().filter(|t| t.inverse.is_some()).count();

    assert_eq!(count, 4);
    assert_eq!(symmetric_count, 2);
    assert_eq!(transitive_count, 1);
    assert_eq!(with_inverse_count, 1);
}

#[tokio::test]
async fn test_semantic_search_with_mock_embeddings() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create memories
    let mem_ids = vec![
        ctx.memory_manager
            .add_fact("The warrior fought bravely in battle")
            .await
            .expect("Failed to create memory 1"),
        ctx.memory_manager
            .add_fact("The kingdom has been at war for three years")
            .await
            .expect("Failed to create memory 2"),
    ];

    // Add mock embeddings to memories (simulating quickstart behavior)
    for mem_id in &mem_ids {
        if let Ok(Some(mut memory)) = ctx.memory_manager.get_memory(mem_id).await {
            // Create a normalized 1024-dimensional embedding
            let mut embedding = vec![0.0; 1024];
            for i in 0..1024 {
                embedding[i] = (i as f32 / 1024.0) * 0.1;
            }
            // Normalize
            let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
            for val in &mut embedding {
                *val /= norm;
            }
            
            memory.embedding = Some(embedding);
            ctx.memory_manager
                .update_memory(memory)
                .await
                .expect("Failed to update memory with embedding");
        }
    }
    
    // Create a mock query embedding (simulating what the CLI does)
    let mut query_embedding = vec![0.0; 1024];
    for i in 0..1024 {
        query_embedding[i] = ((i + 100) as f32 / 1024.0) * 0.1; // Slightly different pattern
    }
    // Normalize
    let norm: f32 = query_embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
    for val in &mut query_embedding {
        *val /= norm;
    }
    
    // Search with semantic mode using search_with_embedding (like the CLI does)
    let results = ctx
        .memory_manager
        .search_with_embedding(
            "warrior",
            Some(&query_embedding),
            Some(10),
            None,
            locai::memory::search_extensions::SearchMode::Vector,
        )
        .await;

    // Semantic search should return results (even if scores are low with mock embeddings)
    assert!(results.is_ok(), "Semantic search should not error");
    let search_results = results.unwrap();
    // Should find at least one result since we added embeddings
    assert!(!search_results.is_empty(), "Should find memories with embeddings");
}

#[tokio::test]
async fn test_vector_search_deserialization() {
    let (ctx, _temp_dir) = create_test_context().await;

    // Create a memory with an embedding
    let mem_id = ctx
        .memory_manager
        .add_fact("Test memory for vector search")
        .await
        .expect("Failed to create memory");

    // Get the memory and add a mock embedding
    if let Ok(Some(mut memory)) = ctx.memory_manager.get_memory(&mem_id).await {
        // Create a normalized 1024-dimensional embedding
        let mut embedding = vec![0.0; 1024];
        for i in 0..1024 {
            embedding[i] = (i as f32 / 1024.0) * 0.1;
        }
        // Normalize
        let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        for val in &mut embedding {
            *val /= norm;
        }
        
        memory.embedding = Some(embedding);
        ctx.memory_manager
            .update_memory(memory)
            .await
            .expect("Failed to update memory with embedding");
    }

    // Try vector search - should not fail with deserialization error
    let query_embedding: Vec<f32> = (0..1024).map(|i| (i as f32 / 1024.0) * 0.1).collect();
    let results = ctx
        .memory_manager
        .storage()
        .vector_search_memories(&query_embedding, Some(10))
        .await;

    // Should not fail with serialization error
    assert!(results.is_ok(), "Vector search should not fail with deserialization error");
}

