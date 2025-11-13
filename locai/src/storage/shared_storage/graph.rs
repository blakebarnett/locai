//! Graph store and traversal implementation for SharedStorage

use async_trait::async_trait;
use std::collections::{HashSet, VecDeque};
use surrealdb::{Connection, RecordId};

use super::base::SharedStorage;
use crate::models::Memory;
use crate::storage::errors::StorageError;
use crate::storage::models::{Entity, MemoryGraph, MemoryPath, Relationship};
use crate::storage::traits::{
    BaseStore, EntityStore, GraphStore, GraphTraversal, MemoryStore, RelationshipStore,
};

#[async_trait]
impl<C> GraphStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn clear_storage(&self) -> Result<(), StorageError> {
        self.clear().await
    }

    fn supports_live_queries(&self) -> bool {
        true // SharedStorage supports live queries
    }

    fn get_live_query_info(&self) -> Option<&'static str> {
        Some("SharedStorage")
    }

    async fn setup_live_queries(
        &self,
    ) -> Result<Option<Box<dyn std::any::Any + Send>>, StorageError> {
        // For now, return None - live queries can be implemented later
        // TODO: Implement live query manager similar to SurrealDB implementation
        Ok(None)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl<C> GraphTraversal for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Get a subgraph of memories and relationships centered on a specific memory
    ///
    /// This method extracts a memory subgraph by traversing the graph through entity
    /// relationships, following the pattern: memory -> contains -> entity -> relates -> entity <- contains <- memory
    async fn get_memory_subgraph(
        &self,
        memory_id: &str,
        depth: u8,
    ) -> Result<MemoryGraph, StorageError> {
        // First verify the central memory exists
        let central_memory = self
            .get_memory(memory_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Memory {} not found", memory_id)))?;

        let mut graph = MemoryGraph::new(memory_id.to_string());
        graph.add_memory(central_memory);

        // Recursively collect connected memories and relationships up to the specified depth
        self.collect_memory_subgraph_recursive(
            &mut graph,
            memory_id,
            depth,
            0,
            &mut HashSet::new(),
        )
        .await?;

        Ok(graph)
    }

    /// Find paths between two memories using breadth-first search through entity relationships
    ///
    /// This method finds paths by traversing through entities:
    /// memory1 -> contains -> entity -> relates -> entity <- contains <- memory2
    async fn find_paths(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Vec<MemoryPath>, StorageError> {
        // Verify both memories exist
        let _from_memory = self.get_memory(from_id).await?.ok_or_else(|| {
            StorageError::NotFound(format!("Source memory {} not found", from_id))
        })?;
        let _to_memory = self
            .get_memory(to_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Target memory {} not found", to_id)))?;

        if from_id == to_id {
            // Self-path: just return the memory itself
            let memory = self.get_memory(from_id).await?.unwrap();
            let mut path = MemoryPath::new(from_id.to_string(), to_id.to_string());
            path.add_memory(memory);
            return Ok(vec![path]);
        }

        // Use breadth-first search to find all paths
        let paths = self.find_paths_bfs(from_id, to_id, max_depth).await?;

        Ok(paths)
    }

    /// Find memories connected to a given memory by following specific relationship types
    ///
    /// This method traverses through entities filtered by relationship type:
    /// memory -> contains -> entity -> relates[relationship_type] -> entity <- contains <- memory
    async fn find_connected_memories(
        &self,
        memory_id: &str,
        relationship_type: Option<&str>,
        max_depth: u8,
    ) -> Result<Vec<Memory>, StorageError> {
        // Verify the source memory exists
        let _source_memory = self
            .get_memory(memory_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(format!("Memory {} not found", memory_id)))?;

        let mut connected_memories = Vec::new();
        let mut visited_memories = HashSet::new();
        let mut visited_entities = HashSet::new();

        // Start BFS from the source memory
        let mut queue = VecDeque::new();
        queue.push_back((memory_id.to_string(), 0u8));
        visited_memories.insert(memory_id.to_string());

        while let Some((current_memory_id, current_depth)) = queue.pop_front() {
            if current_depth >= max_depth {
                continue;
            }

            // First, check for direct memory-to-memory relationships
            let direct_relationships = self.get_direct_memory_relationships(
                &current_memory_id,
                relationship_type,
            ).await?;

            for rel in direct_relationships {
                let target_id = if rel.source_id == current_memory_id {
                    rel.target_id
                } else {
                    rel.source_id
                };

                // Verify target is a memory (not an entity)
                if let Some(target_memory) = self.get_memory(&target_id).await?
                    && !visited_memories.contains(&target_memory.id)
                {
                    visited_memories.insert(target_memory.id.clone());

                    // Don't include the source memory in results
                    if target_memory.id != memory_id {
                        connected_memories.push(target_memory.clone());
                    }

                    // Add to queue for further traversal
                    queue.push_back((target_memory.id, current_depth + 1));
                }
            }

            // Also traverse through entities (entity-mediated relationships)
            let entities = self.get_entities_from_memory(&current_memory_id).await?;

            for entity in entities {
                if visited_entities.contains(&entity.id) {
                    continue;
                }
                visited_entities.insert(entity.id.clone());

                // Find related entities through the specified relationship type (None = all types)
                let related_entities = self
                    .find_related_entities(
                        &entity.id,
                        relationship_type.map(|s| s.to_string()),
                        Some("both".to_string()),
                    )
                    .await?;

                for related_entity in related_entities {
                    // Get memories that contain this related entity
                    let memories = self
                        .get_memories_containing_entity(&related_entity.id)
                        .await?;

                    for memory in memories {
                        if !visited_memories.contains(&memory.id) {
                            visited_memories.insert(memory.id.clone());

                            // Don't include the source memory in results
                            if memory.id != memory_id {
                                connected_memories.push(memory.clone());
                            }

                            // Add to queue for further traversal
                            queue.push_back((memory.id, current_depth + 1));
                        }
                    }
                }
            }
        }

        Ok(connected_memories)
    }

    /// Get entities contained in a memory using the contains edge table
    async fn get_entities_from_memory(&self, memory_id: &str) -> Result<Vec<Entity>, StorageError> {
        let query = r#"
            SELECT VALUE out FROM contains 
            WHERE in = type::thing("memory", $memory_id)
        "#;

        let memory_id_owned = memory_id.to_string();
        let mut response = self
            .client
            .query(query)
            .bind(("memory_id", memory_id_owned))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get entities from memory: {}", e))
            })?;

        let entity_ids: Vec<RecordId> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract entity IDs: {}", e)))?;

        let mut entities = Vec::new();
        for entity_id in entity_ids {
            if let Some(entity) = self.get_entity(&entity_id.key().to_string()).await? {
                entities.push(entity);
            }
        }

        Ok(entities)
    }

    /// Get memories that contain a specific entity using the contains edge table
    async fn get_memories_containing_entity(
        &self,
        entity_id: &str,
    ) -> Result<Vec<Memory>, StorageError> {
        let query = r#"
            SELECT VALUE in FROM contains 
            WHERE out = type::thing("entity", $entity_id)
        "#;

        let entity_id_owned = entity_id.to_string();
        let mut response = self
            .client
            .query(query)
            .bind(("entity_id", entity_id_owned))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get memories containing entity: {}", e))
            })?;

        let memory_ids: Vec<RecordId> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract memory IDs: {}", e)))?;

        let mut memories = Vec::new();
        for memory_id in memory_ids {
            if let Some(memory) = self.get_memory(&memory_id.key().to_string()).await? {
                memories.push(memory);
            }
        }

        Ok(memories)
    }

    /// Get all relationships for an entity using the relates edge table
    async fn get_entity_relationships(
        &self,
        entity_id: &str,
    ) -> Result<Vec<Relationship>, StorageError> {
        let query = r#"
            SELECT * FROM relates 
            WHERE in = type::thing("entity", $entity_id) OR out = type::thing("entity", $entity_id)
        "#;

        let entity_id_owned = entity_id.to_string();
        let mut response = self
            .client
            .query(query)
            .bind(("entity_id", entity_id_owned))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get entity relationships: {}", e))
            })?;

        let relationships: Vec<Relationship> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract relationships: {}", e)))?;

        Ok(relationships)
    }
}

// Helper methods for graph traversal
impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Get direct memory-to-memory relationships from the relationship table
    async fn get_direct_memory_relationships(
        &self,
        memory_id: &str,
        relationship_type: Option<&str>,
    ) -> Result<Vec<Relationship>, StorageError> {
        let mut query = r#"
            SELECT * FROM relationship 
            WHERE (source_id = $memory_id OR target_id = $memory_id)
        "#
        .to_string();

        if relationship_type.is_some() {
            query.push_str(" AND relationship_type = $relationship_type");
        }

        let memory_id_owned = memory_id.to_string();
        let mut query_builder = self
            .client
            .query(&query)
            .bind(("memory_id", memory_id_owned));

        if let Some(rel_type) = relationship_type {
            query_builder = query_builder.bind(("relationship_type", rel_type.to_string()));
        }

        let mut response = query_builder
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get direct memory relationships: {}", e))
            })?;

        // Deserialize as SurrealRelationship first (with RecordId), then convert to Relationship
        use crate::storage::shared_storage::relationship::SurrealRelationship;
        let surreal_relationships: Vec<SurrealRelationship> = response
            .take(0)
            .map_err(|e| {
                StorageError::Query(format!("Failed to extract memory relationships: {}", e))
            })?;
        
        let relationships: Vec<Relationship> = surreal_relationships
            .into_iter()
            .map(Relationship::from)
            .collect();

        // Filter to only return relationships where both source and target are memories
        let mut memory_to_memory = Vec::new();
        for rel in relationships {
            let source_is_memory = self.get_memory(&rel.source_id).await?.is_some();
            let target_is_memory = self.get_memory(&rel.target_id).await?.is_some();
            
            if source_is_memory && target_is_memory {
                memory_to_memory.push(rel);
            }
        }

        Ok(memory_to_memory)
    }
    /// Recursively collect memory subgraph using depth-limited traversal
    async fn collect_memory_subgraph_recursive(
        &self,
        graph: &mut MemoryGraph,
        memory_id: &str,
        max_depth: u8,
        current_depth: u8,
        visited: &mut HashSet<String>,
    ) -> Result<(), StorageError> {
        if current_depth >= max_depth || visited.contains(memory_id) {
            return Ok(());
        }

        visited.insert(memory_id.to_string());

        // Get entities contained in this memory
        let entities = self.get_entities_from_memory(memory_id).await?;

        for entity in entities {
            // Get relationships for this entity
            let relationships = self.get_entity_relationships(&entity.id).await?;

            for relationship in relationships {
                // Find the other entity in the relationship
                let other_entity_id = if relationship.source_id == entity.id {
                    &relationship.target_id
                } else {
                    &relationship.source_id
                };

                // Get memories that contain the other entity
                let connected_memories =
                    self.get_memories_containing_entity(other_entity_id).await?;

                for connected_memory in connected_memories {
                    // Skip if we already have this memory
                    if graph.memories.contains_key(&connected_memory.id) {
                        continue;
                    }

                    // Add the memory to the graph
                    graph.add_memory(connected_memory.clone());
                    graph.add_relationship(relationship.clone());

                    // Recursively traverse from this memory
                    Box::pin(self.collect_memory_subgraph_recursive(
                        graph,
                        &connected_memory.id,
                        max_depth,
                        current_depth + 1,
                        visited,
                    ))
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Find paths using breadth-first search
    async fn find_paths_bfs(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Vec<MemoryPath>, StorageError> {
        let mut paths = Vec::new();
        let mut queue = VecDeque::new();

        // Initialize with the starting memory
        let start_memory = self.get_memory(from_id).await?.unwrap();
        let mut initial_path = MemoryPath::new(from_id.to_string(), to_id.to_string());
        initial_path.add_memory(start_memory);
        queue.push_back((initial_path, 0u8));

        while let Some((current_path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Get the last memory in the current path
            let last_memory = current_path.memories.last().unwrap();

            // If we've reached the target, add this path to results
            if last_memory.id == to_id {
                paths.push(current_path);
                continue;
            }

            // Get entities from the last memory
            let entities = self.get_entities_from_memory(&last_memory.id).await?;

            for entity in entities {
                // Get relationships for this entity
                let relationships = self.get_entity_relationships(&entity.id).await?;

                for relationship in relationships {
                    // Find the other entity
                    let other_entity_id = if relationship.source_id == entity.id {
                        &relationship.target_id
                    } else {
                        &relationship.source_id
                    };

                    // Get memories containing the other entity
                    let connected_memories =
                        self.get_memories_containing_entity(other_entity_id).await?;

                    for connected_memory in connected_memories {
                        // Skip if this memory is already in the path (avoid cycles)
                        if current_path
                            .memories
                            .iter()
                            .any(|m| m.id == connected_memory.id)
                        {
                            continue;
                        }

                        // Create a new path extending the current one
                        let mut new_path = current_path.clone();
                        new_path.add_memory(connected_memory);
                        new_path.add_relationship(relationship.clone());

                        queue.push_back((new_path, depth + 1));
                    }
                }
            }
        }

        Ok(paths)
    }
}
