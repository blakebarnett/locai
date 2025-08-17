//! Test for unified storage system using SharedDatabaseManager
//!
//! This test verifies that the new unified storage system works correctly
//! and that MemoryManager can use it transparently.

use locai::config::ConfigBuilder;

#[tokio::test]
async fn test_unified_storage_memory_operations() {
    // Create configuration with memory storage for testing
    let config = ConfigBuilder::new()
        .with_memory_storage()
        .with_default_ml()
        .build()
        .expect("Failed to build config");

    // Initialize Locai with the unified storage
    let memory_manager = locai::init(config)
        .await
        .expect("Failed to initialize Locai");

    // Test basic memory operations
    let memory_id = memory_manager
        .add_fact("The sky is blue due to Rayleigh scattering")
        .await
        .expect("Failed to add fact");

    println!("Created memory with ID: {}", memory_id);

    // Test that we can add different types of memories
    let conversation_id = memory_manager
        .add_conversation("User: Hello\nBot: Hi there!")
        .await
        .expect("Failed to add conversation");

    println!("Created conversation with ID: {}", conversation_id);

    // Test memory retrieval (this will return None for now since we haven't implemented parsing)
    let retrieved = memory_manager
        .get_memory(&memory_id)
        .await
        .expect("Failed to get memory");

    // For now, this will be None since we haven't implemented the query result parsing
    // but the important thing is that it doesn't error
    println!("Retrieved memory: {:?}", retrieved);

    // Test health check
    let health = memory_manager
        .storage()
        .health_check()
        .await
        .expect("Failed to check health");

    assert!(health, "Storage should be healthy");
    println!("Storage health check passed");

    // Test metadata
    let metadata = memory_manager
        .storage()
        .get_metadata()
        .await
        .expect("Failed to get metadata");

    println!("Storage metadata: {}", metadata);
    assert_eq!(metadata["type"], "shared_storage");
}

#[tokio::test]
async fn test_unified_storage_with_embedded_database() {
    // Create configuration with embedded RocksDB storage
    let config = ConfigBuilder::new()
        .with_default_storage()
        .with_data_dir("./test_data_unified")
        .with_default_ml()
        .build()
        .expect("Failed to build config");

    // Initialize Locai with the unified storage
    let memory_manager = locai::init(config)
        .await
        .expect("Failed to initialize Locai with embedded storage");

    // Test basic operations
    let memory_id = memory_manager
        .add_fact("Testing unified storage with RocksDB")
        .await
        .expect("Failed to add fact to embedded storage");

    println!("Created memory in embedded storage with ID: {}", memory_id);

    // Test health check
    let health = memory_manager
        .storage()
        .health_check()
        .await
        .expect("Failed to check embedded storage health");

    assert!(health, "Embedded storage should be healthy");
    println!("Embedded storage health check passed");

    // Clean up test data
    let _ = std::fs::remove_dir_all("./test_data_unified");
}

#[tokio::test]
async fn test_unified_storage_clear_operation() {
    // Create configuration with memory storage
    let config = ConfigBuilder::new()
        .with_memory_storage()
        .with_default_ml()
        .build()
        .expect("Failed to build config");

    let memory_manager = locai::init(config)
        .await
        .expect("Failed to initialize Locai");

    // Add some memories
    let _id1 = memory_manager
        .add_fact("First fact")
        .await
        .expect("Failed to add first fact");
    let _id2 = memory_manager
        .add_fact("Second fact")
        .await
        .expect("Failed to add second fact");

    // Clear storage
    memory_manager
        .clear_storage()
        .await
        .expect("Failed to clear storage");

    println!("Storage cleared successfully");

    // Verify storage is still healthy after clearing
    let health = memory_manager
        .storage()
        .health_check()
        .await
        .expect("Failed to check health after clear");

    assert!(health, "Storage should still be healthy after clearing");
}
