//! Batch executor for executing multiple operations sequentially or transactionally

use std::sync::Arc;
use tracing::{debug, warn};

use super::types::{BatchError, BatchOperation, BatchResponse};
use crate::models::{Memory, MemoryPriority};
use crate::storage::models::Relationship;
use crate::storage::traits::GraphStore;

/// Configuration for batch execution
#[derive(Debug, Clone)]
pub struct BatchExecutorConfig {
    /// Maximum number of operations allowed in a single batch
    pub max_batch_size: usize,
    /// Whether to collect detailed metrics
    pub collect_metrics: bool,
    /// Timeout for the entire batch in milliseconds
    pub timeout_ms: u64,
}

impl Default for BatchExecutorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            collect_metrics: true,
            timeout_ms: 30000, // 30 seconds
        }
    }
}

/// Executor for batch operations
pub struct BatchExecutor {
    storage: Arc<dyn GraphStore>,
    config: BatchExecutorConfig,
}

impl BatchExecutor {
    /// Create a new batch executor
    pub fn new(storage: Arc<dyn GraphStore>, config: BatchExecutorConfig) -> Self {
        Self { storage, config }
    }

    /// Execute a batch of operations
    ///
    /// # Arguments
    /// * `operations` - Vector of batch operations to execute
    /// * `transaction` - If true, execute all operations as a single transaction
    ///
    /// # Returns
    /// * `Ok(BatchResponse)` - Response with results for each operation
    /// * `Err(BatchError)` - Error if batch validation fails or transaction fails
    pub async fn execute(
        &self,
        operations: Vec<BatchOperation>,
        transaction: bool,
    ) -> Result<BatchResponse, BatchError> {
        // Validate batch size
        if operations.len() > self.config.max_batch_size {
            return Err(BatchError::TooLarge {
                submitted: operations.len(),
                max_size: self.config.max_batch_size,
            });
        }

        debug!(
            "Executing batch of {} operations (transaction={})",
            operations.len(),
            transaction
        );

        if transaction {
            self.execute_transactional(operations).await
        } else {
            self.execute_sequential(operations).await
        }
    }

    /// Execute operations within a SurrealDB transaction
    /// If any operation fails, all operations are rolled back
    async fn execute_transactional(
        &self,
        operations: Vec<BatchOperation>,
    ) -> Result<BatchResponse, BatchError> {
        let mut response = BatchResponse::new(true);

        // Note: SurrealDB doesn't provide a traditional BEGIN/COMMIT API in the Rust SDK yet
        // We'll execute sequentially and track which operations succeeded for rollback
        // A proper implementation would wait for SurrealDB SDK to support transactions

        let mut completed_operations: Vec<(usize, String)> = Vec::new();
        let mut had_error = false;

        for (index, operation) in operations.into_iter().enumerate() {
            match self.execute_operation(index, operation).await {
                Ok(resource_id) => {
                    response.add_success(index, resource_id.clone());
                    completed_operations.push((index, resource_id));
                }
                Err(e) => {
                    warn!("Operation {} failed in transaction: {}", index, e);
                    response.add_error(index, e.to_string());
                    had_error = true;
                    break; // Stop at first error in transaction mode
                }
            }
        }

        // If there was an error, attempt to rollback completed operations
        if had_error {
            warn!(
                "Transaction failed, attempting rollback of {} operations",
                completed_operations.len()
            );

            for (index, resource_id) in completed_operations.iter().rev() {
                // Attempt to delete the created resource
                // This is a best-effort rollback
                if let Err(e) = self.rollback_operation(*index, resource_id).await {
                    warn!("Failed to rollback operation {}: {}", index, e);
                }
            }

            return Err(BatchError::TransactionFailed {
                reason: "Transaction aborted due to operation failure, rollback attempted"
                    .to_string(),
            });
        }

        Ok(response)
    }

    /// Execute operations sequentially without transaction guarantees
    async fn execute_sequential(
        &self,
        operations: Vec<BatchOperation>,
    ) -> Result<BatchResponse, BatchError> {
        let mut response = BatchResponse::new(false);

        for (index, operation) in operations.into_iter().enumerate() {
            match self.execute_operation(index, operation).await {
                Ok(resource_id) => {
                    response.add_success(index, resource_id);
                }
                Err(e) => {
                    warn!("Operation {} failed: {}", index, e);
                    response.add_error(index, e.to_string());
                    // Continue with remaining operations in non-transactional mode
                }
            }
        }

        Ok(response)
    }

    /// Attempt to rollback a completed operation
    async fn rollback_operation(&self, _index: usize, resource_id: &str) -> Result<(), BatchError> {
        // Determine resource type from ID format and attempt deletion
        // This is a best-effort operation

        // Try to delete as memory
        if let Ok(_) = self.storage.delete_memory(resource_id).await {
            debug!("Rolled back memory {}", resource_id);
            return Ok(());
        }

        // Try to delete as entity
        if let Ok(_) = self.storage.delete_entity(resource_id).await {
            debug!("Rolled back entity {}", resource_id);
            return Ok(());
        }

        // Try to delete as relationship
        if let Ok(_) = self.storage.delete_relationship(resource_id).await {
            debug!("Rolled back relationship {}", resource_id);
            return Ok(());
        }

        Err(BatchError::RollbackFailed {
            resource_id: resource_id.to_string(),
        })
    }

    /// Execute a single operation
    async fn execute_operation(
        &self,
        _index: usize,
        operation: BatchOperation,
    ) -> Result<String, BatchError> {
        match operation {
            BatchOperation::CreateMemory {
                content,
                memory_type,
                priority,
                tags,
                source,
                properties,
            } => {
                let priority_enum = match priority {
                    Some(0) => MemoryPriority::Low,
                    Some(1) => MemoryPriority::Normal,
                    Some(2) => MemoryPriority::High,
                    Some(3) => MemoryPriority::Critical,
                    _ => MemoryPriority::Normal,
                };

                let memory = Memory {
                    id: uuid::Uuid::new_v4().to_string(),
                    content,
                    memory_type: crate::models::MemoryType::from_str(&memory_type),
                    created_at: chrono::Utc::now(),
                    last_accessed: None,
                    access_count: 0,
                    priority: priority_enum,
                    tags: tags.unwrap_or_default(),
                    source: source.unwrap_or_else(|| "batch".to_string()),
                    expires_at: None,
                    properties: properties.unwrap_or(serde_json::json!({})),
                    related_memories: Vec::new(),
                    embedding: None,
                };

                let created = self.storage.create_memory(memory).await.map_err(|e| {
                    BatchError::StorageError {
                        message: e.to_string(),
                    }
                })?;

                Ok(created.id)
            }

            BatchOperation::UpdateMemory {
                id,
                content,
                priority,
                tags,
                properties,
            } => {
                // Get existing memory
                let mut memory = self
                    .storage
                    .get_memory(&id)
                    .await
                    .map_err(|e| BatchError::StorageError {
                        message: e.to_string(),
                    })?
                    .ok_or_else(|| BatchError::StorageError {
                        message: format!("Memory {} not found", id),
                    })?;

                // Update fields
                if let Some(new_content) = content {
                    memory.content = new_content;
                }
                if let Some(new_priority) = priority {
                    memory.priority = match new_priority {
                        0 => MemoryPriority::Low,
                        1 => MemoryPriority::Normal,
                        2 => MemoryPriority::High,
                        3 => MemoryPriority::Critical,
                        _ => MemoryPriority::Normal,
                    };
                }
                if let Some(new_tags) = tags {
                    memory.tags = new_tags;
                }
                if let Some(new_properties) = properties {
                    memory.properties = new_properties;
                }

                let updated = self.storage.update_memory(memory).await.map_err(|e| {
                    BatchError::StorageError {
                        message: e.to_string(),
                    }
                })?;

                Ok(updated.id)
            }

            BatchOperation::DeleteMemory { id } => {
                let success = self.storage.delete_memory(&id).await.map_err(|e| {
                    BatchError::StorageError {
                        message: e.to_string(),
                    }
                })?;

                if success {
                    Ok(id)
                } else {
                    Err(BatchError::StorageError {
                        message: format!("Failed to delete memory {}", id),
                    })
                }
            }

            BatchOperation::CreateRelationship {
                source,
                target,
                relationship_type,
                properties,
                enforce_constraints: _enforce_constraints,
            } => {
                let relationship = Relationship {
                    id: uuid::Uuid::new_v4().to_string(),
                    source_id: source,
                    target_id: target,
                    relationship_type,
                    properties: properties.unwrap_or(serde_json::json!({})),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };

                let created = self
                    .storage
                    .create_relationship(relationship)
                    .await
                    .map_err(|e| BatchError::StorageError {
                        message: e.to_string(),
                    })?;

                Ok(created.id)
            }

            BatchOperation::UpdateRelationship { id, properties } => {
                let mut relationship = self
                    .storage
                    .get_relationship(&id)
                    .await
                    .map_err(|e| BatchError::StorageError {
                        message: e.to_string(),
                    })?
                    .ok_or_else(|| BatchError::StorageError {
                        message: format!("Relationship {} not found", id),
                    })?;

                if let Some(new_properties) = properties {
                    relationship.properties = new_properties;
                }

                let updated = self
                    .storage
                    .update_relationship(relationship)
                    .await
                    .map_err(|e| BatchError::StorageError {
                        message: e.to_string(),
                    })?;

                Ok(updated.id)
            }

            BatchOperation::DeleteRelationship { id } => {
                let success = self.storage.delete_relationship(&id).await.map_err(|e| {
                    BatchError::StorageError {
                        message: e.to_string(),
                    }
                })?;

                if success {
                    Ok(id)
                } else {
                    Err(BatchError::StorageError {
                        message: format!("Failed to delete relationship {}", id),
                    })
                }
            }

            BatchOperation::UpdateMetadata {
                memory_id,
                metadata,
            } => {
                let mut memory = self
                    .storage
                    .get_memory(&memory_id)
                    .await
                    .map_err(|e| BatchError::StorageError {
                        message: e.to_string(),
                    })?
                    .ok_or_else(|| BatchError::StorageError {
                        message: format!("Memory {} not found", memory_id),
                    })?;

                // Merge metadata into properties
                if let serde_json::Value::Object(ref mut props) = memory.properties {
                    if let serde_json::Value::Object(new_props) = metadata {
                        for (k, v) in new_props {
                            props.insert(k, v);
                        }
                    }
                }

                let updated = self.storage.update_memory(memory).await.map_err(|e| {
                    BatchError::StorageError {
                        message: e.to_string(),
                    }
                })?;

                Ok(updated.id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_executor_config_default() {
        let config = BatchExecutorConfig::default();
        assert_eq!(config.max_batch_size, 1000);
        assert!(config.collect_metrics);
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_batch_error_too_large() {
        let error = BatchError::TooLarge {
            submitted: 2000,
            max_size: 1000,
        };
        assert_eq!(error.to_string(), "Batch size 2000 exceeds maximum 1000");
    }

    #[test]
    fn test_batch_error_transaction_failed() {
        let error = BatchError::TransactionFailed {
            reason: "Database connection lost".to_string(),
        };
        assert!(error.to_string().contains("Transaction failed"));
    }

    #[test]
    fn test_batch_error_invalid_operation() {
        let error = BatchError::InvalidOperation {
            index: 5,
            reason: "Missing required field".to_string(),
        };
        assert!(error.to_string().contains("Operation 5 invalid"));
    }
}
