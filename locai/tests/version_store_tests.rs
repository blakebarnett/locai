//! Integration tests for SharedStorage VersionStore implementation

use chrono::Utc;
use locai::models::{Memory, MemoryPriority, MemoryType};
use locai::storage::models::{Entity, Version};
use locai::storage::shared_storage::{SharedStorage, SharedStorageConfig};
use locai::storage::traits::{EntityStore, MemoryStore, VersionStore};
use serde_json::json;

/// Creates a test store for version operations
async fn create_test_store() -> SharedStorage<surrealdb::engine::local::Db> {
    let config = SharedStorageConfig {
        namespace: "test_version".to_string(),
        database: "test_version".to_string(),
    };

    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
        .await
        .unwrap();
    SharedStorage::new(client, config).await.unwrap()
}

fn create_test_memory(id: &str, content: &str) -> Memory {
    let now = Utc::now();
    Memory {
        id: id.to_string(),
        content: content.to_string(),
        memory_type: MemoryType::Episodic,
        created_at: now,
        last_accessed: Some(now),
        access_count: 0,
        priority: MemoryPriority::Normal,
        tags: vec!["test".to_string()],
        source: "test".to_string(),
        expires_at: None,
        properties: json!({}),
        related_memories: vec![],
        embedding: None,
    }
}

fn create_test_entity(id: &str, entity_type: &str) -> Entity {
    let now = Utc::now();
    Entity {
        id: id.to_string(),
        entity_type: entity_type.to_string(),
        properties: json!({"name": format!("Test {}", id)}),
        created_at: now,
        updated_at: now,
    }
}

fn create_test_version(description: &str) -> Version {
    Version {
        id: "".to_string(), // Will be auto-generated
        description: description.to_string(),
        metadata: json!({"test": true}),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_create_version_basic() {
    let store = create_test_store().await;

    let version = create_test_version("Test version creation");
    let created_version = store.create_version(version).await.unwrap();

    assert!(!created_version.id.is_empty());
    assert_eq!(created_version.description, "Test version creation");
    assert_eq!(created_version.metadata["test"], true);
}

#[tokio::test]
async fn test_get_version() {
    let store = create_test_store().await;

    let version = create_test_version("Test get version");
    let created_version = store.create_version(version).await.unwrap();

    let retrieved_version = store.get_version(&created_version.id).await.unwrap();

    assert!(retrieved_version.is_some());
    let retrieved = retrieved_version.unwrap();
    assert_eq!(retrieved.id, created_version.id);
    assert_eq!(retrieved.description, "Test get version");
}

#[tokio::test]
async fn test_list_versions() {
    let store = create_test_store().await;

    // Create multiple versions
    let version1 = create_test_version("Version 1");
    let version2 = create_test_version("Version 2");
    let version3 = create_test_version("Version 3");

    store.create_version(version1).await.unwrap();
    store.create_version(version2).await.unwrap();
    store.create_version(version3).await.unwrap();

    let versions = store.list_versions(None, None).await.unwrap();

    assert_eq!(versions.len(), 3);
    // Should be ordered by created_at DESC
    assert_eq!(versions[0].description, "Version 3");
    assert_eq!(versions[1].description, "Version 2");
    assert_eq!(versions[2].description, "Version 1");
}

#[tokio::test]
async fn test_checkout_version() {
    let store = create_test_store().await;

    // Create initial data
    let memory1 = create_test_memory("memory1", "Initial content");
    let entity1 = create_test_entity("entity1", "person");
    store.create_memory(memory1).await.unwrap();
    store.create_entity(entity1).await.unwrap();

    // Create a version
    let version = create_test_version("Initial state");
    let created_version = store.create_version(version).await.unwrap();

    // Add more data
    let memory2 = create_test_memory("memory2", "New content");
    store.create_memory(memory2).await.unwrap();

    // Verify we have 2 memories
    let memories_before = store.list_memories(None, None, None).await.unwrap();
    assert_eq!(memories_before.len(), 2);

    // Checkout the version (restore to initial state)
    let checkout_result = store.checkout_version(&created_version.id).await.unwrap();
    assert!(checkout_result);

    // Verify we're back to 1 memory
    let memories_after = store.list_memories(None, None, None).await.unwrap();
    assert_eq!(memories_after.len(), 1);
    assert_eq!(memories_after[0].content, "Initial content");
}

#[tokio::test]
async fn test_conversation_version() {
    let store = create_test_store().await;

    let conversation_version = store
        .create_conversation_version("conv-123", "Conversation about AI development")
        .await
        .unwrap();

    assert!(!conversation_version.id.is_empty());
    assert_eq!(
        conversation_version.description,
        "Conversation about AI development"
    );
    assert_eq!(
        conversation_version.metadata["snapshot_type"],
        "conversation"
    );
    assert_eq!(conversation_version.metadata["conversation_id"], "conv-123");
    assert_eq!(
        conversation_version.metadata["context_type"],
        "ai_assistant"
    );
}

#[tokio::test]
async fn test_knowledge_version() {
    let store = create_test_store().await;

    let knowledge_version = store
        .create_knowledge_version("machine_learning", "Learned about neural networks")
        .await
        .unwrap();

    assert!(!knowledge_version.id.is_empty());
    assert_eq!(
        knowledge_version.description,
        "Learned about neural networks"
    );
    assert_eq!(knowledge_version.metadata["snapshot_type"], "knowledge");
    assert_eq!(knowledge_version.metadata["topic"], "machine_learning");
    assert_eq!(
        knowledge_version.metadata["learning_context"],
        "evolution_tracking"
    );
}

// AI Assistant Context Management Tests
#[tokio::test]
async fn test_ai_assistant_conversation_tracking() {
    let store = create_test_store().await;

    // Simulate a conversation with the AI assistant
    let memory1 = create_test_memory("msg1", "User: What is machine learning?");
    let memory2 = create_test_memory(
        "msg2",
        "AI: Machine learning is a subset of artificial intelligence...",
    );

    store.create_memory(memory1).await.unwrap();
    store.create_memory(memory2).await.unwrap();

    // Create a conversation checkpoint
    let conv_version = store
        .create_conversation_version("session-123", "ML discussion checkpoint")
        .await
        .unwrap();

    // Continue conversation
    let memory3 = create_test_memory("msg3", "User: Can you give me an example?");
    store.create_memory(memory3).await.unwrap();

    // Verify we can restore to conversation checkpoint
    let checkout_result = store.checkout_version(&conv_version.id).await.unwrap();
    assert!(checkout_result);

    let memories = store.list_memories(None, None, None).await.unwrap();
    assert_eq!(memories.len(), 2); // Back to the checkpoint state
}
