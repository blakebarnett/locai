//! Graph-based memory operations
//!
//! This module handles graph traversal, path finding, and relationship
//! navigation for memories and entities.

use crate::models::Memory;
use crate::relationships::storage::RelationshipStorage;
use crate::storage::filters::RelationshipFilter;
use crate::storage::models::{MemoryGraph, MemoryPath, Relationship};
use crate::storage::traits::{GraphStore, GraphTraversal};
use crate::{LocaiError, Result};
use std::sync::Arc;

/// Graph-based operations for memories
#[derive(Debug)]
pub struct GraphOperations {
    storage: Arc<dyn GraphStore>,
    relationship_storage: RelationshipStorage,
}

impl GraphOperations {
    /// Create a new graph operations handler
    pub fn new(storage: Arc<dyn GraphStore>) -> Self {
        let relationship_storage = RelationshipStorage::new(Arc::clone(&storage));
        Self {
            storage,
            relationship_storage,
        }
    }

    /// Retrieve a memory with its graph context
    ///
    /// # Arguments
    /// * `id` - The ID of the memory to retrieve
    /// * `depth` - How many levels of relationships to traverse
    ///
    /// # Returns
    /// A memory graph containing the memory and its relationships
    pub async fn get_memory_graph(&self, id: &str, depth: u8) -> Result<MemoryGraph> {
        GraphTraversal::get_memory_subgraph(&*self.storage, id, depth)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memory graph: {}", e)))
    }

    /// Find paths between two memories
    ///
    /// # Arguments
    /// * `from_id` - The ID of the starting memory
    /// * `to_id` - The ID of the target memory
    /// * `max_depth` - Maximum path length to consider
    ///
    /// # Returns
    /// A list of paths between the memories
    pub async fn find_paths(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Vec<MemoryPath>> {
        GraphTraversal::find_paths(&*self.storage, from_id, to_id, max_depth)
            .await
            .map_err(|e| {
                LocaiError::Storage(format!("Failed to find paths between memories: {}", e))
            })
    }

    /// Find the shortest path between two memories
    ///
    /// # Arguments
    /// * `from_id` - The ID of the starting memory
    /// * `to_id` - The ID of the target memory
    /// * `max_depth` - Maximum path length to consider
    ///
    /// # Returns
    /// The shortest path if one exists
    pub async fn find_shortest_path(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Option<MemoryPath>> {
        let paths = self.find_paths(from_id, to_id, max_depth).await?;

        // Return the shortest path if any exist
        Ok(paths.into_iter().min_by_key(|path| path.memories.len()))
    }

    /// Find all memories connected to this memory by a specific relationship
    ///
    /// # Arguments
    /// * `id` - The ID of the memory to start from
    /// * `relationship_type` - The type of relationship to follow
    /// * `max_depth` - Maximum depth to traverse
    ///
    /// # Returns
    /// A list of connected memories
    pub async fn find_connected_memories(
        &self,
        id: &str,
        relationship_type: &str,
        max_depth: u8,
    ) -> Result<Vec<Memory>> {
        GraphTraversal::find_connected_memories(&*self.storage, id, relationship_type, max_depth)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to find connected memories: {}", e)))
    }

    /// Create a relationship between two memories
    ///
    /// # Arguments
    /// * `source_id` - The ID of the source memory
    /// * `target_id` - The ID of the target memory
    /// * `relationship_type` - The type of relationship
    ///
    /// # Returns
    /// Whether the relationship was created successfully
    pub async fn create_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        self.relationship_storage
            .create_memory_relationship(source_id, target_id, relationship_type)
            .await
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
    pub async fn create_bidirectional_relationship(
        &self,
        memory_id1: &str,
        memory_id2: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        self.relationship_storage
            .create_bidirectional_memory_relationship(memory_id1, memory_id2, relationship_type)
            .await
    }

    /// Add a related memory in one step
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the parent memory
    /// * `content` - The content of the new memory
    /// * `relationship_type` - The type of relationship
    /// * `memory_type` - Optional memory type (defaults to Fact)
    /// * `memory_operations` - Reference to memory operations for creating the memory
    ///
    /// # Returns
    /// The ID of the new memory
    pub async fn add_related_memory<S: Into<String>>(
        &self,
        parent_id: &str,
        content: S,
        relationship_type: &str,
        memory_type: Option<crate::models::MemoryType>,
        memory_operations: &crate::memory::operations::MemoryOperations,
    ) -> Result<String> {
        use crate::models::MemoryBuilder;

        // Create the new memory
        let memory = MemoryBuilder::new_with_content(content)
            .memory_type(memory_type.unwrap_or(crate::models::MemoryType::Fact))
            .build();

        // Store the memory
        let memory_id = memory_operations.store_memory(memory).await?;

        // Create the relationship
        self.create_relationship(parent_id, &memory_id, relationship_type)
            .await?;

        Ok(memory_id)
    }

    /// Create a bidirectional related memory
    ///
    /// Creates a new memory and establishes bidirectional relationships with an existing memory
    ///
    /// # Arguments
    /// * `parent_id` - The ID of the existing memory
    /// * `content` - The content of the new memory
    /// * `relationship_type` - The type of relationship
    /// * `memory_type` - Optional memory type (defaults to Fact)
    /// * `memory_operations` - Reference to memory operations for creating the memory
    ///
    /// # Returns
    /// The ID of the new memory
    pub async fn add_bidirectional_related_memory<S: Into<String>>(
        &self,
        parent_id: &str,
        content: S,
        relationship_type: &str,
        memory_type: Option<crate::models::MemoryType>,
        memory_operations: &crate::memory::operations::MemoryOperations,
    ) -> Result<String> {
        use crate::models::MemoryBuilder;

        // Create the new memory
        let memory = MemoryBuilder::new_with_content(content)
            .memory_type(memory_type.unwrap_or(crate::models::MemoryType::Fact))
            .build();

        // Store the memory
        let memory_id = memory_operations.store_memory(memory).await?;

        // Create the bidirectional relationship
        self.create_bidirectional_relationship(parent_id, &memory_id, relationship_type)
            .await?;

        Ok(memory_id)
    }

    /// Get related memories by relationship type
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory to find relationships for
    /// * `relationship_type` - The type of relationship to look for
    /// * `direction` - The relationship direction ("outgoing", "incoming", or "both")
    ///
    /// # Returns
    /// A vector of related memories
    pub async fn get_related_memories(
        &self,
        memory_id: &str,
        relationship_type: Option<&str>,
        direction: &str,
    ) -> Result<Vec<Memory>> {
        // Strategy 1: Direct memory-to-memory relationships
        let mut results = Vec::new();

        // Get direct relationships where this memory is involved
        let relationship_filter = RelationshipFilter {
            source_id: if direction == "outgoing" || direction == "both" {
                Some(memory_id.to_string())
            } else {
                None
            },
            target_id: if direction == "incoming" || direction == "both" {
                Some(memory_id.to_string())
            } else {
                None
            },
            relationship_type: relationship_type.map(|s| s.to_string()),
            ..Default::default()
        };

        if let Ok(relationships) = self
            .storage
            .list_relationships(Some(relationship_filter), Some(100), None)
            .await
        {
            for relationship in relationships {
                // Get the other memory in the relationship
                let other_memory_id = if relationship.source_id == memory_id {
                    &relationship.target_id
                } else {
                    &relationship.source_id
                };

                // Try to get the memory
                if let Ok(Some(memory)) = self.storage.get_memory(other_memory_id).await {
                    results.push(memory);
                }
            }
        }

        // Strategy 2: Entity-mediated relationships (memory -> entity -> entity -> memory)
        // This is the main graph traversal approach
        if let Ok(entities_in_memory) = self.storage.get_entities_from_memory(memory_id).await {
            for entity in entities_in_memory {
                // Find entities related to this entity
                if let Ok(related_entities) = self
                    .storage
                    .find_related_entities(
                        &entity.id,
                        relationship_type.map(|s| s.to_string()),
                        Some(direction.to_string()),
                    )
                    .await
                {
                    for related_entity in related_entities {
                        // Get memories that contain the related entity
                        if let Ok(related_memory_ids) =
                            self.get_memories_for_entity(&related_entity.id).await
                        {
                            for related_memory_id in related_memory_ids {
                                // Don't include the original memory
                                if related_memory_id != memory_id
                                    && let Ok(Some(memory)) =
                                        self.storage.get_memory(&related_memory_id).await
                                    && !results.iter().any(|m| m.id == memory.id)
                                {
                                    results.push(memory);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Strategy 3: If no specific relationship type, also look for memories that share entities
        if relationship_type.is_none() && results.is_empty() {
            // Find memories that share entities with this memory (co-occurrence)
            if let Ok(entities_in_memory) = self.storage.get_entities_from_memory(memory_id).await {
                for entity in entities_in_memory.into_iter().take(5) {
                    // Limit to avoid too many results
                    if let Ok(related_memory_ids) = self.get_memories_for_entity(&entity.id).await {
                        for related_memory_id in related_memory_ids.into_iter().take(10) {
                            if related_memory_id != memory_id
                                && let Ok(Some(memory)) =
                                    self.storage.get_memory(&related_memory_id).await
                                && !results.iter().any(|m| m.id == memory.id)
                            {
                                results.push(memory);
                            }
                        }
                    }
                }
            }
        }

        // Limit results to avoid overwhelming responses
        results.truncate(50);

        Ok(results)
    }

    /// Get memory IDs that contain a specific entity
    async fn get_memories_for_entity(&self, entity_id: &str) -> Result<Vec<String>> {
        // First try to use relationship graph to find connected memories
        let relationship_filter = RelationshipFilter {
            source_id: None,
            target_id: Some(entity_id.to_string()),
            relationship_type: Some("contains".to_string()),
            ..Default::default()
        };

        let relationships = self
            .storage
            .list_relationships(Some(relationship_filter), Some(100), None)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to query relationships: {}", e)))?;

        let mut related_memory_ids: Vec<String> =
            relationships.into_iter().map(|rel| rel.source_id).collect();

        // If no relationships found, fall back to content-based search
        if related_memory_ids.is_empty() {
            let entity = self
                .storage
                .get_entity(entity_id)
                .await
                .map_err(|e| LocaiError::Storage(format!("Failed to get entity: {}", e)))?
                .ok_or_else(|| LocaiError::Storage("Entity not found".to_string()))?;

            // Get entity name from properties (same logic as in search)
            let entity_name = entity
                .properties
                .get("name")
                .or_else(|| entity.properties.get("text"))
                .or_else(|| entity.properties.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or(&entity.id);

            // Search memories that contain this entity's name
            let memory_filter = crate::storage::filters::MemoryFilter {
                content: Some(entity_name.to_string()),
                ..Default::default()
            };

            let memories = self
                .storage
                .list_memories(Some(memory_filter), Some(100), None)
                .await
                .map_err(|e| LocaiError::Storage(format!("Failed to search memories: {}", e)))?;

            related_memory_ids = memories.into_iter().map(|m| m.id).collect();
        }

        Ok(related_memory_ids)
    }

    /// Query cross-process relationships (enabled by shared database)
    ///
    /// # Arguments
    /// * `process_id` - ID of the process to query interactions for
    ///
    /// # Returns
    /// List of relationships involving the process
    pub async fn get_process_interactions(&self, process_id: &str) -> Result<Vec<Relationship>> {
        // Get entity relationships for the process
        self.storage
            .find_related_entities(
                &format!("process:{}", process_id),
                None,
                Some("both".to_string()),
            )
            .await
            .map(|_| Vec::new()) // Simplified for now - would need proper relationship querying
            .map_err(|e| LocaiError::Storage(format!("Failed to get process interactions: {}", e)))
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn GraphStore> {
        &self.storage
    }

    /// Get access to the relationship storage
    pub fn relationship_storage(&self) -> &RelationshipStorage {
        &self.relationship_storage
    }
}
