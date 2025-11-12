//! Comprehensive end-to-end integration tests for RFC 001 features
//!
//! These tests verify that all RFC 001 work streams function correctly together:
//! - Work Stream 1: Memory Lifecycle Tracking
//! - Work Stream 2: Relationship Type Registry
//! - Work Stream 3: Hook System
//! - Work Stream 4: Batch Operations
//! - Work Stream 5: Enhanced Search Scoring

use async_trait::async_trait;
use chrono::Utc;
use locai::batch::{BatchOperation, BatchResponse};
use locai::config::LifecycleTrackingConfig;
use locai::hooks::{HookRegistry, HookResult, MemoryHook};
use locai::models::{Memory, MemoryPriority, MemoryType};
use locai::relationships::registry::{RelationshipTypeDef, RelationshipTypeRegistry};
use locai::search::scoring::{DecayFunction, ScoringConfig};
use locai::search::calculator::ScoreCalculator;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Test helper to create a test memory
fn create_test_memory(id: &str, content: &str) -> Memory {
    Memory {
        id: id.to_string(),
        content: content.to_string(),
        memory_type: MemoryType::Fact,
        created_at: Utc::now(),
        last_accessed: None,
        access_count: 0,
        priority: MemoryPriority::Normal,
        tags: vec![],
        source: "test".to_string(),
        expires_at: None,
        properties: serde_json::json!({}),
        related_memories: vec![],
        embedding: None,
    }
}

/// Test hook that counts how many times it was called
#[derive(Debug)]
struct CountingHook {
    create_count: Arc<AtomicU32>,
    access_count: Arc<AtomicU32>,
    update_count: Arc<AtomicU32>,
    delete_count: Arc<AtomicU32>,
}

impl CountingHook {
    fn new() -> Self {
        Self {
            create_count: Arc::new(AtomicU32::new(0)),
            access_count: Arc::new(AtomicU32::new(0)),
            update_count: Arc::new(AtomicU32::new(0)),
            delete_count: Arc::new(AtomicU32::new(0)),
        }
    }
    
    fn get_create_count(&self) -> u32 {
        self.create_count.load(Ordering::SeqCst)
    }
    
    fn get_access_count(&self) -> u32 {
        self.access_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl MemoryHook for CountingHook {
    async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
        self.create_count.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    }
    
    async fn on_memory_accessed(&self, _memory: &Memory) -> HookResult {
        self.access_count.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    }
    
    async fn on_memory_updated(&self, _old: &Memory, _new: &Memory) -> HookResult {
        self.update_count.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    }
    
    async fn before_memory_deleted(&self, _memory: &Memory) -> HookResult {
        self.delete_count.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    }
    
    fn name(&self) -> &str {
        "counting_hook"
    }
    
    fn priority(&self) -> i32 {
        0
    }
}

/// Test hook that vetoes deletion
#[derive(Debug)]
struct VetoHook;

#[async_trait]
impl MemoryHook for VetoHook {
    async fn before_memory_deleted(&self, _memory: &Memory) -> HookResult {
        HookResult::Veto("Deletion not allowed by VetoHook".to_string())
    }
    
    fn name(&self) -> &str {
        "veto_hook"
    }
}

// =============================================================================
// TEST 1: Full Lifecycle Tracking with Hooks
// =============================================================================

#[tokio::test]
async fn test_full_lifecycle_with_hooks() {
    // Setup
    let hook_registry = Arc::new(HookRegistry::new());
    let counting_hook = Arc::new(CountingHook::new());
    
    // Register the hook
    hook_registry.register(counting_hook.clone()).await;
    
    // Create a memory (should trigger on_memory_created hook)
    let memory = create_test_memory("mem_001", "Test memory for lifecycle tracking");
    
    // Simulate hook execution
    hook_registry.execute_on_created(&memory).await.ok();
    
    // Verify hook was called
    assert_eq!(counting_hook.get_create_count(), 1);
    
    // Simulate access (should trigger on_memory_accessed hook and update counts)
    let mut accessed_memory = memory.clone();
    accessed_memory.record_access();
    
    hook_registry.execute_on_accessed(&accessed_memory).await.ok();
    
    // Verify lifecycle metadata updated
    assert_eq!(accessed_memory.access_count, 1);
    assert!(accessed_memory.last_accessed.is_some());
    
    // Verify hook was called
    assert_eq!(counting_hook.get_access_count(), 1);
    
    // Access again
    accessed_memory.record_access();
    hook_registry.execute_on_accessed(&accessed_memory).await.ok();
    
    assert_eq!(accessed_memory.access_count, 2);
    assert_eq!(counting_hook.get_access_count(), 2);
}

// =============================================================================
// TEST 2: Custom Relationship Types
// =============================================================================

#[tokio::test]
async fn test_custom_relationship_types() {
    let registry = RelationshipTypeRegistry::new();
    
    // Register a custom relationship type
    let custom_type = RelationshipTypeDef::new("serves".to_string())
        .unwrap()
        .with_inverse("served_by".to_string())
        .with_custom_metadata(
            "category".to_string(),
            serde_json::json!("military"),
        );
    
    registry.register(custom_type).await.unwrap();
    
    // Verify the type was registered
    let retrieved = registry.get("serves").await;
    assert!(retrieved.is_some());
    
    let type_def = retrieved.unwrap();
    assert_eq!(type_def.name, "serves");
    assert_eq!(type_def.inverse, Some("served_by".to_string()));
    assert!(!type_def.symmetric);
    
    // Register a symmetric type
    let symmetric_type = RelationshipTypeDef::new("married_to".to_string())
        .unwrap()
        .symmetric();
    
    registry.register(symmetric_type).await.unwrap();
    
    let retrieved_symmetric = registry.get("married_to").await.unwrap();
    assert!(retrieved_symmetric.symmetric);
    
    // Verify count
    assert_eq!(registry.count().await, 2);
}

// =============================================================================
// TEST 3: Batch Operations with Transaction Mode
// =============================================================================

#[test]
fn test_batch_operations_with_transaction() {
    // Note: This test documents the EXPECTED behavior once Bug 4.1 is fixed
    // Currently, transaction mode doesn't actually work (see RFC_001_BUG_ANALYSIS.md)
    
    let operations = vec![
        BatchOperation::CreateMemory {
            content: "First memory".to_string(),
            memory_type: "fact".to_string(),
            priority: Some(1),
            tags: None,
            source: None,
            properties: None,
        },
        BatchOperation::CreateMemory {
            content: "Second memory".to_string(),
            memory_type: "episodic".to_string(),
            priority: Some(2),
            tags: Some(vec!["important".to_string()]),
            source: None,
            properties: None,
        },
        BatchOperation::CreateRelationship {
            source: "mem_1".to_string(),
            target: "mem_2".to_string(),
            relationship_type: "references".to_string(),
            properties: None,
            enforce_constraints: Some(false),
        },
    ];
    
    // Verify operations are serializable
    let json = serde_json::to_string(&operations).unwrap();
    let deserialized: Vec<BatchOperation> = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.len(), 3);
    
    // When transaction support is implemented (Bug 4.1 fix):
    // 1. All operations should execute atomically
    // 2. If any operation fails, all should roll back
    // 3. Response should indicate transaction success/failure
}

// =============================================================================
// TEST 4: Enhanced Search Scoring
// =============================================================================

#[test]
fn test_enhanced_search_scoring() {
    // Create memories with different characteristics
    let fresh_memory = create_test_memory("fresh", "Fresh memory");
    
    let old_memory = {
        let mut m = create_test_memory("old", "Old memory");
        m.created_at = Utc::now() - chrono::Duration::days(30);
        m
    };
    
    let frequently_accessed = {
        let mut m = create_test_memory("frequent", "Frequently accessed");
        m.access_count = 50;
        m
    };
    
    let high_priority = {
        let mut m = create_test_memory("priority", "High priority");
        m.priority = MemoryPriority::Critical;
        m
    };
    
    // Test different scoring configurations
    let recency_focused = ScoringConfig::recency_focused();
    let importance_focused = ScoringConfig::importance_focused();
    
    let recency_calc = ScoreCalculator::new(recency_focused);
    let importance_calc = ScoreCalculator::new(importance_focused);
    
    // Base BM25 score for all
    let base_score = 10.0;
    
    // Calculate scores with recency focus
    let fresh_recency_score = recency_calc.calculate_final_score(base_score, None, &fresh_memory);
    let old_recency_score = recency_calc.calculate_final_score(base_score, None, &old_memory);
    
    // Fresh memory should score higher with recency focus
    assert!(fresh_recency_score > old_recency_score);
    
    // Calculate scores with importance focus
    let frequent_importance_score = importance_calc.calculate_final_score(
        base_score,
        None,
        &frequently_accessed,
    );
    let normal_importance_score = importance_calc.calculate_final_score(
        base_score,
        None,
        &fresh_memory,
    );
    
    // Frequently accessed should score higher with importance focus
    assert!(frequent_importance_score > normal_importance_score);
    
    // High priority should get a boost
    let priority_score = importance_calc.calculate_final_score(
        base_score,
        None,
        &high_priority,
    );
    assert!(priority_score > normal_importance_score);
}

// =============================================================================
// TEST 5: Decay Functions
// =============================================================================

#[test]
fn test_decay_functions_behavior() {
    let config_linear = ScoringConfig {
        bm25_weight: 0.0,
        vector_weight: 0.0,
        recency_boost: 10.0,
        access_boost: 0.0,
        priority_boost: 0.0,
        decay_function: DecayFunction::Linear,
        decay_rate: 0.001, // 0.1% per hour - allows decay to be visible over weeks/months
    };
    
    let config_exponential = ScoringConfig {
        decay_function: DecayFunction::Exponential,
        ..config_linear.clone()
    };
    
    let config_logarithmic = ScoringConfig {
        decay_function: DecayFunction::Logarithmic,
        ..config_linear.clone()
    };
    
    let calc_linear = ScoreCalculator::new(config_linear);
    let calc_exp = ScoreCalculator::new(config_exponential);
    let calc_log = ScoreCalculator::new(config_logarithmic);
    
    // Create memories of different ages
    let fresh = create_test_memory("fresh", "Fresh");
    
    let week_old = {
        let mut m = create_test_memory("week", "Week old");
        m.created_at = Utc::now() - chrono::Duration::weeks(1);
        m
    };
    
    let month_old = {
        let mut m = create_test_memory("month", "Month old");
        m.created_at = Utc::now() - chrono::Duration::days(30);
        m
    };
    
    // Calculate scores
    let fresh_linear = calc_linear.calculate_final_score(0.0, None, &fresh);
    let week_linear = calc_linear.calculate_final_score(0.0, None, &week_old);
    let month_linear = calc_linear.calculate_final_score(0.0, None, &month_old);
    
    let fresh_exp = calc_exp.calculate_final_score(0.0, None, &fresh);
    let week_exp = calc_exp.calculate_final_score(0.0, None, &week_old);
    let month_exp = calc_exp.calculate_final_score(0.0, None, &month_old);
    
    let fresh_log = calc_log.calculate_final_score(0.0, None, &fresh);
    let week_log = calc_log.calculate_final_score(0.0, None, &week_old);
    let month_log = calc_log.calculate_final_score(0.0, None, &month_old);
    
    // Verify decay ordering: fresh > week > month for all functions
    assert!(fresh_linear > week_linear);
    assert!(week_linear > month_linear);
    
    assert!(fresh_exp > week_exp);
    assert!(week_exp > month_exp);
    
    assert!(fresh_log > week_log);
    assert!(week_log > month_log);
    
    // Exponential should decay faster than logarithmic
    let _week_decay_linear = (week_linear / fresh_linear) * 100.0;
    let week_decay_exp = (week_exp / fresh_exp) * 100.0;
    let week_decay_log = (week_log / fresh_log) * 100.0;
    
    // Logarithmic should retain more of original score than exponential
    assert!(week_decay_log > week_decay_exp);
}

// =============================================================================
// TEST 6: Hook Priority Ordering
// =============================================================================

#[tokio::test]
async fn test_hook_priority_ordering() {
    #[derive(Debug)]
    struct OrderedHook {
        priority: i32,
        name: String,
        execution_order: Arc<tokio::sync::Mutex<Vec<String>>>,
    }
    
    #[async_trait]
    impl MemoryHook for OrderedHook {
        async fn on_memory_created(&self, _memory: &Memory) -> HookResult {
            let mut order = self.execution_order.lock().await;
            order.push(self.name.clone());
            HookResult::Continue
        }
        
        fn priority(&self) -> i32 {
            self.priority
        }
        
        fn name(&self) -> &str {
            &self.name
        }
    }
    
    let execution_order = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let registry = HookRegistry::new();
    
    // Register hooks in random order
    let hook_low = Arc::new(OrderedHook {
        priority: 1,
        name: "low".to_string(),
        execution_order: execution_order.clone(),
    });
    
    let hook_high = Arc::new(OrderedHook {
        priority: 10,
        name: "high".to_string(),
        execution_order: execution_order.clone(),
    });
    
    let hook_medium = Arc::new(OrderedHook {
        priority: 5,
        name: "medium".to_string(),
        execution_order: execution_order.clone(),
    });
    
    registry.register(hook_low).await;
    registry.register(hook_high).await;
    registry.register(hook_medium).await;
    
    // Execute hooks
    let memory = create_test_memory("test", "Test");
    registry.execute_on_created(&memory).await.ok();
    
    // Verify execution order: high -> medium -> low
    let order = execution_order.lock().await;
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], "high");
    assert_eq!(order[1], "medium");
    assert_eq!(order[2], "low");
}

// =============================================================================
// TEST 7: Hook Veto Mechanism
// =============================================================================

#[tokio::test]
async fn test_hook_veto_deletion() {
    let registry = HookRegistry::new();
    let veto_hook = Arc::new(VetoHook);
    
    registry.register(veto_hook).await;
    
    let memory = create_test_memory("protected", "Protected memory");
    
    // Attempt deletion
    let can_delete = registry.execute_before_deleted(&memory).await.unwrap();
    
    // Deletion should be vetoed
    assert!(!can_delete);
}

// =============================================================================
// TEST 8: Batch Operation Serialization
// =============================================================================

#[test]
fn test_batch_operation_serialization() {
    let operations = vec![
        BatchOperation::CreateMemory {
            content: "Test".to_string(),
            memory_type: "fact".to_string(),
            priority: Some(1),
            tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
            source: Some("test".to_string()),
            properties: Some(serde_json::json!({"custom": "value"})),
        },
        BatchOperation::UpdateMemory {
            id: "mem_123".to_string(),
            content: Some("Updated".to_string()),
            priority: Some(2),
            tags: None,
            properties: None,
        },
        BatchOperation::DeleteMemory {
            id: "mem_456".to_string(),
        },
        BatchOperation::CreateRelationship {
            source: "entity_1".to_string(),
            target: "entity_2".to_string(),
            relationship_type: "custom_type".to_string(),
            properties: Some(serde_json::json!({"strength": 0.8})),
            enforce_constraints: Some(true),
        },
        BatchOperation::UpdateRelationship {
            id: "rel_789".to_string(),
            properties: Some(serde_json::json!({"updated": true})),
        },
        BatchOperation::DeleteRelationship {
            id: "rel_012".to_string(),
        },
        BatchOperation::UpdateMetadata {
            memory_id: "mem_345".to_string(),
            metadata: serde_json::json!({"key": "value"}),
        },
    ];
    
    // Serialize
    let json = serde_json::to_string_pretty(&operations).unwrap();
    
    // Deserialize
    let deserialized: Vec<BatchOperation> = serde_json::from_str(&json).unwrap();
    
    // Verify all operations survived round-trip
    assert_eq!(deserialized.len(), 7);
    
    // Verify specific operation types
    match &deserialized[0] {
        BatchOperation::CreateMemory { content, tags, .. } => {
            assert_eq!(content, "Test");
            assert_eq!(tags.as_ref().unwrap().len(), 2);
        }
        _ => panic!("Expected CreateMemory operation"),
    }
    
    match &deserialized[3] {
        BatchOperation::CreateRelationship { relationship_type, enforce_constraints, .. } => {
            assert_eq!(relationship_type, "custom_type");
            assert_eq!(enforce_constraints, &Some(true));
        }
        _ => panic!("Expected CreateRelationship operation"),
    }
}

// =============================================================================
// TEST 9: Lifecycle Configuration Validation
// =============================================================================

#[test]
fn test_lifecycle_config_validation() {
    // Valid config
    let valid_config = LifecycleTrackingConfig {
        enabled: true,
        update_on_get: true,
        update_on_search: false,
        update_on_list: false,
        blocking: false,
        batched: true,
        flush_interval_secs: 60,
        flush_threshold_count: 100,
    };
    
    assert!(valid_config.validate().is_ok());
    
    // Invalid config: zero flush interval
    let invalid_config_1 = LifecycleTrackingConfig {
        flush_interval_secs: 0,
        ..valid_config.clone()
    };
    
    assert!(invalid_config_1.validate().is_err());
    
    // Invalid config: zero threshold
    let invalid_config_2 = LifecycleTrackingConfig {
        flush_threshold_count: 0,
        ..valid_config.clone()
    };
    
    assert!(invalid_config_2.validate().is_err());
}

// =============================================================================
// TEST 10: Scoring Configuration Validation
// =============================================================================

#[test]
fn test_scoring_config_validation() {
    // Valid config
    let valid_config = ScoringConfig::default();
    assert!(valid_config.validate().is_ok());
    
    // Invalid: negative weight
    let invalid_config = ScoringConfig {
        bm25_weight: -1.0,
        ..ScoringConfig::default()
    };
    assert!(invalid_config.validate().is_err());
    
    // Invalid: zero decay rate
    let invalid_decay = ScoringConfig {
        decay_rate: 0.0,
        ..ScoringConfig::default()
    };
    assert!(invalid_decay.validate().is_err());
}

// =============================================================================
// TEST 11: Relationship Type Name Validation
// =============================================================================

#[tokio::test]
async fn test_relationship_type_name_validation() {
    // Valid names
    assert!(RelationshipTypeDef::new("valid_name".to_string()).is_ok());
    assert!(RelationshipTypeDef::new("valid-name".to_string()).is_ok());
    assert!(RelationshipTypeDef::new("ValidName123".to_string()).is_ok());
    
    // Invalid names
    assert!(RelationshipTypeDef::new("".to_string()).is_err());
    assert!(RelationshipTypeDef::new("invalid@name".to_string()).is_err());
    assert!(RelationshipTypeDef::new("invalid name".to_string()).is_err());
}

// =============================================================================
// TEST 12: Integration - Lifecycle + Hooks + Search
// =============================================================================

#[tokio::test]
async fn test_integration_lifecycle_hooks_search() {
    // This test demonstrates how all features work together
    
    // 1. Setup hook system
    let hook_registry = Arc::new(HookRegistry::new());
    let counting_hook = Arc::new(CountingHook::new());
    hook_registry.register(counting_hook.clone()).await;
    
    // 2. Create memories
    let mut memory1 = create_test_memory("mem_001", "First memory about cats");
    let mut memory2 = create_test_memory("mem_002", "Second memory about dogs");
    let mut memory3 = create_test_memory("mem_003", "Third memory about cats and dogs");
    
    // Simulate creation hooks
    hook_registry.execute_on_created(&memory1).await.ok();
    hook_registry.execute_on_created(&memory2).await.ok();
    hook_registry.execute_on_created(&memory3).await.ok();
    
    assert_eq!(counting_hook.get_create_count(), 3);
    
    // 3. Simulate different access patterns
    memory1.record_access();
    memory1.record_access();
    memory1.record_access(); // Most accessed
    
    memory2.record_access(); // Least accessed
    
    memory3.record_access();
    memory3.record_access(); // Medium accessed
    
    // 4. Apply different scoring strategies
    let importance_config = ScoringConfig::importance_focused();
    let calc = ScoreCalculator::new(importance_config);
    
    // Base scores (simulating BM25 keyword matching)
    let base_score = 10.0;
    
    let score1 = calc.calculate_final_score(base_score, None, &memory1);
    let score2 = calc.calculate_final_score(base_score, None, &memory2);
    let score3 = calc.calculate_final_score(base_score, None, &memory3);
    
    // With importance-focused scoring, most accessed should score highest
    assert!(score1 > score3);
    assert!(score3 > score2);
}

// =============================================================================
// TEST 13: Batch Response Tracking
// =============================================================================

#[test]
fn test_batch_response_tracking() {
    let mut response = BatchResponse::new(false);
    
    // Add various results
    response.add_success(0, "mem_1".to_string());
    response.add_success(1, "mem_2".to_string());
    response.add_error(2, "Failed to create".to_string());
    response.add_success(3, "rel_1".to_string());
    
    assert_eq!(response.completed, 3);
    assert_eq!(response.failed, 1);
    assert!(!response.all_successful());
    assert!(response.has_errors());
    
    // Verify serialization
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: BatchResponse = serde_json::from_str(&json).unwrap();
    
    assert_eq!(deserialized.completed, 3);
    assert_eq!(deserialized.failed, 1);
}

// =============================================================================
// TEST 14: Hook Registry Cleanup
// =============================================================================

#[tokio::test]
async fn test_hook_registry_cleanup() {
    let registry = HookRegistry::new();
    
    // Register hooks
    let hook1 = Arc::new(CountingHook::new());
    let hook2 = Arc::new(CountingHook::new());
    let hook3 = Arc::new(CountingHook::new());
    
    registry.register(hook1).await;
    registry.register(hook2).await;
    registry.register(hook3).await;
    
    assert_eq!(registry.hook_count().await, 3);
    
    // Clear all hooks
    registry.clear().await;
    
    assert_eq!(registry.hook_count().await, 0);
}

// =============================================================================
// TEST 15: Seed Common Relationship Types
// =============================================================================

#[tokio::test]
async fn test_seed_common_relationship_types() {
    let registry = RelationshipTypeRegistry::new();
    
    // Seed common types
    registry.seed_common_types().await.unwrap();
    
    // Verify standard types exist
    assert!(registry.exists("friendship").await);
    assert!(registry.exists("rivalry").await);
    assert!(registry.exists("mentorship").await);
    
    // Verify symmetric types are marked correctly
    let friendship = registry.get("friendship").await.unwrap();
    assert!(friendship.symmetric);
    
    let mentorship = registry.get("mentorship").await.unwrap();
    assert!(!mentorship.symmetric);
    assert_eq!(mentorship.inverse, Some("mentee".to_string()));
    
    // Verify count
    assert!(registry.count().await >= 10);
}






