//! Relationship storage operations
//! 
//! This module handles the low-level CRUD operations for relationships
//! in the graph database, separate from the high-level relationship management.

use crate::storage::models::Relationship;

use crate::storage::traits::GraphStore;
use crate::{LocaiError, Result};
use std::sync::Arc;

/// Low-level relationship storage operations
#[derive(Debug)]
pub struct RelationshipStorage {
    storage: Arc<dyn GraphStore>,
}

impl RelationshipStorage {
    /// Create a new relationship storage handler
    pub fn new(storage: Arc<dyn GraphStore>) -> Self {
        Self { storage }
    }

    // Note: Basic CRUD operations are available directly through self.storage()
    // This module focuses on memory-specific relationship operations and business logic

    /// Create a relationship between two memories
    /// 
    /// # Arguments
    /// * `source_id` - The ID of the source memory
    /// * `target_id` - The ID of the target memory
    /// * `relationship_type` - The type of relationship
    /// 
    /// # Returns
    /// Whether the relationship was created successfully
    pub async fn create_memory_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        tracing::debug!("Creating relationship: {} --[{}]--> {}", 
                       source_id, relationship_type, target_id);
        
        // Create relationship object
        let relationship = Relationship {
            id: format!("{}_{}_{}_{}", source_id, relationship_type, target_id, chrono::Utc::now().timestamp()),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relationship_type: relationship_type.to_string(),
            properties: serde_json::Value::Null,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        match self.storage.create_relationship(relationship).await {
            Ok(result) => {
                tracing::debug!("Successfully created relationship: {} --[{}]--> {} (ID: {})", 
                               source_id, relationship_type, target_id, result.id);
                Ok(result.id.len() > 0)
            },
            Err(e) => {
                tracing::error!("Failed to create relationship {} --[{}]--> {}: {}", 
                               source_id, relationship_type, target_id, e);
                Err(LocaiError::Storage(format!("Failed to create relationship: {}", e)))
            }
        }
    }

    /// Create a bidirectional relationship between two memories
    /// 
    /// # Arguments
    /// * `memory_id1` - The ID of the first memory
    /// * `memory_id2` - The ID of the second memory
    /// * `relationship_type` - The type of relationship
    /// 
    /// # Returns
    /// Whether both relationships were created successfully
    pub async fn create_bidirectional_memory_relationship(
        &self,
        memory_id1: &str,
        memory_id2: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        // Create both directions
        let forward = self.create_memory_relationship(memory_id1, memory_id2, relationship_type).await?;
        let backward = self.create_memory_relationship(memory_id2, memory_id1, relationship_type).await?;
        
        Ok(forward && backward)
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn GraphStore> {
        &self.storage
    }
} 