//! Entity management operations
//! 
//! This module handles entity CRUD operations, entity queries,
//! and entity-memory relationships.

use crate::storage::models::Entity;
use crate::storage::filters::EntityFilter;
use crate::storage::traits::GraphStore;
use crate::models::{Memory, MemoryType, MemoryPriority};
use crate::{LocaiError, Result};
use std::sync::Arc;

/// Entity management operations
#[derive(Debug)]
pub struct EntityOperations {
    storage: Arc<dyn GraphStore>,
}

impl EntityOperations {
    /// Create a new entity operations handler
    pub fn new(storage: Arc<dyn GraphStore>) -> Self {
        Self { storage }
    }

    /// Create a new entity
    /// 
    /// # Arguments
    /// * `entity` - The entity to create
    /// 
    /// # Returns
    /// The created entity
    pub async fn create_entity(&self, entity: Entity) -> Result<Entity> {
        self.storage.create_entity(entity).await
            .map_err(|e| LocaiError::Storage(format!("Failed to create entity: {}", e)))
    }

    /// Get an entity by ID
    /// 
    /// # Arguments
    /// * `id` - The ID of the entity to retrieve
    /// 
    /// # Returns
    /// The entity if found, None otherwise
    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        self.storage.get_entity(id).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get entity: {}", e)))
    }

    /// Update an existing entity
    /// 
    /// # Arguments
    /// * `entity` - The updated entity
    /// 
    /// # Returns
    /// The updated entity
    pub async fn update_entity(&self, entity: Entity) -> Result<Entity> {
        self.storage.update_entity(entity).await
            .map_err(|e| LocaiError::Storage(format!("Failed to update entity: {}", e)))
    }

    /// Delete an entity by ID
    /// 
    /// # Arguments
    /// * `id` - The ID of the entity to delete
    /// 
    /// # Returns
    /// Whether the deletion was successful
    pub async fn delete_entity(&self, id: &str) -> Result<bool> {
        self.storage.delete_entity(id).await
            .map_err(|e| LocaiError::Storage(format!("Failed to delete entity: {}", e)))
    }

    /// List entities with optional filtering
    /// 
    /// # Arguments
    /// * `filter` - Optional filter to apply
    /// * `limit` - Maximum number of results to return
    /// * `offset` - Number of results to skip
    /// 
    /// # Returns
    /// A vector of entities matching the filter criteria
    pub async fn list_entities(
        &self,
        filter: Option<EntityFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Entity>> {
        self.storage.list_entities(filter, limit, offset).await
            .map_err(|e| LocaiError::Storage(format!("Failed to list entities: {}", e)))
    }

    /// Count entities with optional filtering
    /// 
    /// # Arguments
    /// * `filter` - Optional filter to apply
    /// 
    /// # Returns
    /// The number of entities matching the filter
    pub async fn count_entities(&self, filter: Option<EntityFilter>) -> Result<usize> {
        self.storage.count_entities(filter).await
            .map_err(|e| LocaiError::Storage(format!("Failed to count entities: {}", e)))
    }

    /// Find related entities
    /// 
    /// # Arguments
    /// * `entity_id` - The ID of the entity to find relationships for
    /// * `relationship_type` - Optional relationship type filter
    /// * `direction` - Optional direction filter
    /// 
    /// # Returns
    /// A vector of related entities
    pub async fn find_related_entities(
        &self,
        entity_id: &str,
        relationship_type: Option<String>,
        direction: Option<String>,
    ) -> Result<Vec<Entity>> {
        self.storage.find_related_entities(entity_id, relationship_type, direction).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find related entities: {}", e)))
    }

    /// Get memories by priority level
    ///
    /// # Arguments
    /// * `priority` - The priority level to filter by
    /// * `limit` - Maximum number of memories to return
    ///
    /// # Returns
    /// A vector of memories with the specified priority
    pub async fn get_memories_by_priority(
        &self, 
        priority: MemoryPriority, 
        limit: Option<usize>
    ) -> Result<Vec<Memory>> {
        // Since MemoryFilter doesn't have a priority field, we need to first get all memories
        // and then filter them manually.
        // Using list_memories to get all memories (or a reasonable large number)
        let all_memories = self.storage.list_memories(None, Some(10000), None).await // Increased limit, adjust as needed
            .map_err(|e| LocaiError::Storage(format!("Failed to list memories for priority filter: {}", e)))?;
        
        // Filter by priority
        let filtered_memories: Vec<Memory> = all_memories.into_iter()
            .filter(|m| m.priority == priority)
            .take(limit.unwrap_or(usize::MAX))
            .collect();
        
        Ok(filtered_memories)
    }

    /// Get memories by type
    ///
    /// # Arguments
    /// * `memory_type` - The memory type to filter by
    /// * `limit` - Maximum number of memories to return
    ///
    /// # Returns
    /// A vector of memories of the specified type
    pub async fn get_memories_by_type(
        &self, 
        memory_type: MemoryType, 
        limit: Option<usize>
    ) -> Result<Vec<Memory>> {
        use crate::storage::filters::MemoryFilter;
        
        let mut filter = MemoryFilter::default();
        
        // Convert MemoryType enum to string representation
        filter.memory_type = Some(memory_type.to_string());
        
        self.storage.list_memories(Some(filter), limit, None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memories by type: {}", e)))
    }

    /// Find memories by tag
    ///
    /// # Arguments
    /// * `tag` - The tag to search for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of memories with the specified tag
    pub async fn find_memories_by_tag(&self, tag: &str, limit: Option<usize>) -> Result<Vec<Memory>> {
        use crate::storage::filters::MemoryFilter;
        
        let mut filter = MemoryFilter::default();
        filter.tags = Some(vec![tag.to_string()]);
        
        self.storage.list_memories(Some(filter), limit, None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find memories by tag: {}", e)))
    }

    /// Get recent memories
    ///
    /// # Arguments
    /// * `limit` - Maximum number of memories to return
    ///
    /// # Returns
    /// A vector of recently created memories
    pub async fn get_recent_memories(&self, limit: usize) -> Result<Vec<Memory>> {
        // MemoryFilter doesn't support sorting by created_at directly in list_memories for all backends.
        // We fetch a reasonable number of memories and sort them manually.
        // Consider adding sorting to MemoryStore trait and implementations for more efficiency.
        let all_memories = self.storage.list_memories(None, Some(1000), None).await // Fetch a decent number
            .map_err(|e| LocaiError::Storage(format!("Failed to list memories for recency: {}", e)))?;
        
        // Sort by created_at (most recent first)
        let mut sorted_memories = all_memories;
        sorted_memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // Take only what we need
        let limited_memories = sorted_memories.into_iter()
            .take(limit)
            .collect();
        
        Ok(limited_memories)
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn GraphStore> {
        &self.storage
    }
} 