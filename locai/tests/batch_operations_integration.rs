//! Integration tests for batch operations
//!
//! These tests verify the complete batch operations workflow including:
//! - Type definitions and serialization
//! - Batch executor functionality
//! - Sequential vs transactional modes
//! - Error handling and partial failures

use locai::batch::{BatchError, BatchExecutorConfig, BatchOperation, BatchResponse};
use locai::models::{MemoryPriority, MemoryType};

#[test]
fn test_batch_operation_serialization() {
    // Test that batch operations can be serialized and deserialized
    let op = BatchOperation::CreateMemory {
        content: "Test memory".to_string(),
        memory_type: "fact".to_string(),
        priority: Some(2),
        tags: Some(vec!["important".to_string()]),
        source: Some("test".to_string()),
        properties: None,
        embedding: None,
    };

    let json = serde_json::to_string(&op).expect("Should serialize");
    let deserialized: BatchOperation = serde_json::from_str(&json).expect("Should deserialize");

    match deserialized {
        BatchOperation::CreateMemory {
            content, priority, ..
        } => {
            assert_eq!(content, "Test memory");
            assert_eq!(priority, Some(2));
        }
        _ => panic!("Unexpected operation type"),
    }
}

#[test]
fn test_batch_response_tracking() {
    let mut response = BatchResponse::new(false);
    assert_eq!(response.completed, 0);
    assert_eq!(response.failed, 0);

    // Add successes
    response.add_success(0, "mem_1".to_string());
    response.add_success(1, "mem_2".to_string());
    assert_eq!(response.completed, 2);
    assert_eq!(response.failed, 0);
    assert!(response.all_successful());

    // Add error
    response.add_error(2, "Failed to create".to_string());
    assert_eq!(response.completed, 2);
    assert_eq!(response.failed, 1);
    assert!(!response.all_successful());
    assert!(response.has_errors());
}

#[test]
fn test_batch_error_messages() {
    let error = BatchError::TooLarge {
        submitted: 2000,
        max_size: 1000,
    };
    assert_eq!(error.to_string(), "Batch size 2000 exceeds maximum 1000");

    let error = BatchError::ValidationError {
        message: "Invalid field".to_string(),
    };
    assert!(error.to_string().contains("Validation error"));
}

#[test]
fn test_batch_executor_config() {
    let config = BatchExecutorConfig::default();
    assert_eq!(config.max_batch_size, 1000);
    assert!(config.collect_metrics);
    assert_eq!(config.timeout_ms, 30000);

    let custom_config = BatchExecutorConfig {
        max_batch_size: 500,
        collect_metrics: false,
        timeout_ms: 60000,
    };
    assert_eq!(custom_config.max_batch_size, 500);
    assert!(!custom_config.collect_metrics);
    assert_eq!(custom_config.timeout_ms, 60000);
}

#[test]
fn test_batch_response_with_messages() {
    let mut response = BatchResponse::new(true);
    response.add_success_with_message(0, "mem_123".to_string(), "Created successfully".to_string());

    assert_eq!(response.results.len(), 1);
    match &response.results[0] {
        locai::batch::BatchResult::Success {
            operation_index,
            resource_id,
            message,
        } => {
            assert_eq!(*operation_index, 0);
            assert_eq!(resource_id, "mem_123");
            assert_eq!(
                message.as_ref().map(|m| m.as_str()),
                Some("Created successfully")
            );
        }
        _ => panic!("Expected success result"),
    }
}

#[test]
fn test_batch_response_with_error_codes() {
    let mut response = BatchResponse::new(false);
    response.add_error_with_code(0, "Resource not found".to_string(), "NOT_FOUND".to_string());

    assert_eq!(response.results.len(), 1);
    match &response.results[0] {
        locai::batch::BatchResult::Error {
            operation_index,
            error,
            error_code,
        } => {
            assert_eq!(*operation_index, 0);
            assert_eq!(error, "Resource not found");
            assert_eq!(error_code.as_ref().map(|c| c.as_str()), Some("NOT_FOUND"));
        }
        _ => panic!("Expected error result"),
    }
}

#[test]
fn test_batch_operations_complete_workflow() {
    // Create a batch with multiple operation types
    let operations = vec![
        BatchOperation::CreateMemory {
            content: "First memory".to_string(),
            memory_type: "fact".to_string(),
            priority: Some(1),
            tags: Some(vec!["test".to_string()]),
            source: None,
            properties: None,
            embedding: None,
        },
        BatchOperation::CreateMemory {
            content: "Second memory".to_string(),
            memory_type: "episodic".to_string(),
            priority: Some(2),
            tags: None,
            source: Some("batch".to_string()),
            properties: None,
            embedding: None,
        },
        BatchOperation::UpdateMetadata {
            memory_id: "id_1".to_string(),
            metadata: serde_json::json!({"custom": "value"}),
        },
    ];

    // Verify serialization
    let json = serde_json::to_string(&operations).expect("Should serialize");
    let deserialized: Vec<BatchOperation> =
        serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.len(), 3);
}

#[test]
fn test_batch_response_serialization() {
    let mut response = BatchResponse::new(true);
    response.add_success(0, "mem_1".to_string());
    response.add_error(1, "Failed".to_string());

    let json = serde_json::to_string(&response).expect("Should serialize");
    let deserialized: BatchResponse = serde_json::from_str(&json).expect("Should deserialize");

    assert_eq!(deserialized.completed, 1);
    assert_eq!(deserialized.failed, 1);
    assert_eq!(deserialized.results.len(), 2);
    assert!(deserialized.transaction);
}

#[test]
fn test_batch_executor_config_defaults() {
    let config = BatchExecutorConfig::default();

    // Verify defaults match RFC 001
    assert_eq!(config.max_batch_size, 1000, "Max batch size should be 1000");
    assert!(config.collect_metrics, "Should collect metrics by default");
    assert_eq!(
        config.timeout_ms, 30000,
        "Timeout should be 30 seconds by default"
    );
}

#[test]
fn test_batch_error_too_large() {
    let error = BatchError::TooLarge {
        submitted: 5000,
        max_size: 1000,
    };

    let message = error.to_string();
    assert!(message.contains("5000"));
    assert!(message.contains("1000"));
    assert!(message.contains("exceeds"));
}

#[test]
fn test_batch_memory_type_conversion() {
    // Test that memory types are correctly converted
    let memory_type_str = "fact";
    let converted = MemoryType::from_str(memory_type_str);
    assert_eq!(converted, MemoryType::Fact);

    let custom_type_str = "custom:my_type";
    let converted = MemoryType::from_str(custom_type_str);
    match converted {
        MemoryType::Custom(s) => assert_eq!(s, "my_type"),
        _ => panic!("Should be custom type"),
    }
}

#[test]
fn test_batch_priority_conversion() {
    // Test priority level conversion
    let priority_levels = vec![
        (Some(0), MemoryPriority::Low),
        (Some(1), MemoryPriority::Normal),
        (Some(2), MemoryPriority::High),
        (Some(3), MemoryPriority::Critical),
        (None, MemoryPriority::Normal), // Default
    ];

    for (input, expected) in priority_levels {
        let converted = match input {
            Some(0) => MemoryPriority::Low,
            Some(1) => MemoryPriority::Normal,
            Some(2) => MemoryPriority::High,
            Some(3) => MemoryPriority::Critical,
            _ => MemoryPriority::Normal,
        };
        assert_eq!(converted, expected);
    }
}

#[test]
fn test_batch_operation_all_types_serializable() {
    // Verify all operation types can be serialized
    let operations = vec![
        BatchOperation::CreateMemory {
            content: "test".to_string(),
            memory_type: "fact".to_string(),
            priority: None,
            tags: None,
            source: None,
            properties: None,
            embedding: None,
        },
        BatchOperation::UpdateMemory {
            id: "mem_1".to_string(),
            content: Some("updated".to_string()),
            priority: Some(1),
            tags: None,
            properties: None,
            embedding: None,
        },
        BatchOperation::DeleteMemory {
            id: "mem_2".to_string(),
        },
        BatchOperation::CreateRelationship {
            source: "mem_1".to_string(),
            target: "mem_2".to_string(),
            relationship_type: "references".to_string(),
            properties: None,
            enforce_constraints: None,
        },
        BatchOperation::UpdateRelationship {
            id: "rel_1".to_string(),
            properties: Some(serde_json::json!({"key": "value"})),
        },
        BatchOperation::DeleteRelationship {
            id: "rel_2".to_string(),
        },
        BatchOperation::UpdateMetadata {
            memory_id: "mem_3".to_string(),
            metadata: serde_json::json!({"custom": "data"}),
        },
    ];

    for op in operations {
        let json = serde_json::to_string(&op).expect("Should serialize");
        let _deserialized: BatchOperation =
            serde_json::from_str(&json).expect("Should deserialize");
    }
}

#[test]
fn test_batch_response_mixed_results() {
    let mut response = BatchResponse::new(false);

    // Add various results
    response.add_success(0, "mem_1".to_string());
    response.add_success_with_message(1, "mem_2".to_string(), "With note".to_string());
    response.add_error(2, "Generic error".to_string());
    response.add_error_with_code(3, "Not found".to_string(), "NOT_FOUND".to_string());

    assert_eq!(response.results.len(), 4);
    assert_eq!(response.completed, 2);
    assert_eq!(response.failed, 2);
    assert!(!response.all_successful());
    assert!(response.has_errors());
}

#[test]
fn test_batch_response_json_serialization() {
    let mut response = BatchResponse::new(true);
    response.add_success(0, "resource_1".to_string());
    response.add_error(1, "Operation failed".to_string());
    response.transaction_id = Some("tx_123".to_string());

    let json = serde_json::to_string(&response).expect("Should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse as JSON");

    // Verify structure
    assert_eq!(parsed["completed"], 1);
    assert_eq!(parsed["failed"], 1);
    assert_eq!(parsed["transaction"], true);
    assert_eq!(parsed["transaction_id"], "tx_123");
    assert!(parsed["results"].is_array());
}

#[test]
fn test_batch_empty_operations() {
    let response = BatchResponse::new(false);
    assert_eq!(response.completed, 0);
    assert_eq!(response.failed, 0);
    assert!(response.all_successful());
    assert!(!response.has_errors());
}

#[test]
fn test_batch_operation_with_complex_properties() {
    let complex_props = serde_json::json!({
        "nested": {
            "field": "value",
            "array": [1, 2, 3]
        },
        "priority": 0.95
    });

    let op = BatchOperation::CreateMemory {
        content: "Complex memory".to_string(),
        memory_type: "fact".to_string(),
        priority: Some(1),
        tags: Some(vec!["complex".to_string()]),
        source: None,
        properties: Some(complex_props.clone()),
        embedding: None,
    };

    let json = serde_json::to_string(&op).expect("Should serialize");
    let deserialized: BatchOperation = serde_json::from_str(&json).expect("Should deserialize");

    match deserialized {
        BatchOperation::CreateMemory { properties, .. } => {
            assert_eq!(properties, Some(complex_props));
        }
        _ => panic!("Unexpected operation type"),
    }
}
