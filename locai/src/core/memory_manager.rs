//! Memory Manager interface for Locai
//!
//! This module provides the primary interface for interacting with the Locai memory system.
//! It orchestrates the various memory management components.

use crate::config::LocaiConfig;
use crate::ml::model_manager::EmbeddingManager;
use crate::models::{Memory, MemoryBuilder, MemoryPriority, MemoryType};
use crate::storage::filters::{
    EntityFilter, MemoryFilter, RelationshipFilter, SemanticSearchFilter,
};
use crate::storage::models::{Entity, MemoryGraph, MemoryPath, Relationship, SearchResult};
use crate::{LocaiError, Result};
use std::sync::Arc;

// Import the new modules
use crate::memory::{
    builders::MemoryBuilders,
    entity_operations::EntityOperations,
    graph_operations::GraphOperations,
    messaging::MessagingIntegration,
    operations::MemoryOperations,
    search_extensions::{
        SearchExtensions, SearchMode, UniversalSearchOptions, UniversalSearchResult,
    },
};
use crate::relationships::storage::RelationshipStorage;

/// The primary interface for interacting with Locai's memory system.
///
/// `MemoryManager` orchestrates various memory management components and provides
/// a unified API for memory operations.
#[derive(Debug)]
pub struct MemoryManager {
    /// Core memory operations (CRUD, entity extraction)
    memory_ops: MemoryOperations,

    /// Memory builder convenience methods
    builders: MemoryBuilders,

    /// Advanced search functionality
    search: SearchExtensions,

    /// Graph-based operations
    graph: GraphOperations,

    /// Entity management operations
    entities: EntityOperations,

    /// Messaging system integration
    messaging: MessagingIntegration,

    /// Relationship storage operations
    relationships: RelationshipStorage,

    /// Configuration for the memory manager
    config: LocaiConfig,
}

impl MemoryManager {
    /// Create a new memory manager with the provided storage and configuration
    #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
    pub fn new(
        storage: Arc<dyn crate::storage::traits::GraphStore>,
        ml_service: Option<Arc<EmbeddingManager>>,
        config: LocaiConfig,
    ) -> Self {
        // Initialize all the component modules
        let memory_ops =
            MemoryOperations::new(Arc::clone(&storage), ml_service.clone(), config.clone());
        let builders = MemoryBuilders::new(Arc::new(memory_ops.clone()));
        let search = SearchExtensions::new(Arc::clone(&storage));
        let graph = GraphOperations::new(Arc::clone(&storage));
        let entities = EntityOperations::new(Arc::clone(&storage));
        let messaging = MessagingIntegration::new(Arc::clone(&storage));
        let relationships = RelationshipStorage::new(Arc::clone(&storage));

        Self {
            memory_ops,
            builders,
            search,
            graph,
            entities,
            messaging,
            relationships,
            config,
        }
    }

    /// Create a new memory manager with ML extractors initialized asynchronously
    #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
    pub async fn new_with_ml(
        storage: Arc<dyn crate::storage::traits::GraphStore>,
        ml_service: Option<Arc<EmbeddingManager>>,
        config: LocaiConfig,
    ) -> Result<Self> {
        // Initialize memory operations with ML extractors
        let memory_ops =
            MemoryOperations::new_with_ml(Arc::clone(&storage), ml_service.clone(), config.clone())
                .await?;
        let builders = MemoryBuilders::new(Arc::new(memory_ops.clone()));
        let search = SearchExtensions::new(Arc::clone(&storage));
        let graph = GraphOperations::new(Arc::clone(&storage));
        let entities = EntityOperations::new(Arc::clone(&storage));
        let messaging = MessagingIntegration::new(Arc::clone(&storage));
        let relationships = RelationshipStorage::new(Arc::clone(&storage));

        Ok(Self {
            memory_ops,
            builders,
            search,
            graph,
            entities,
            messaging,
            relationships,
            config,
        })
    }

    /// Initialize ML extractors asynchronously after construction (deprecated - use new_with_ml instead)
    pub async fn initialize_ml_extractors(&mut self) -> Result<()> {
        tracing::info!(
            "🤖 ML extractor initialization called - this is a no-op, use MemoryManager::new_with_ml() instead"
        );
        Ok(())
    }

    // =============================================================================
    // Core Memory Operations (delegated to MemoryOperations)
    // =============================================================================

    /// Store a new memory
    pub async fn store_memory(&self, memory: Memory) -> Result<String> {
        self.memory_ops.store_memory(memory).await
    }

    /// Retrieve a memory by ID
    pub async fn get_memory(&self, id: &str) -> Result<Option<Memory>> {
        self.memory_ops.get_memory(id).await
    }

    /// Update an existing memory
    pub async fn update_memory(&self, memory: Memory) -> Result<bool> {
        self.memory_ops.update_memory(memory).await
    }

    /// Delete a memory by ID
    pub async fn delete_memory(&self, id: &str) -> Result<bool> {
        self.memory_ops.delete_memory(id).await
    }

    /// Filter memories using various criteria
    pub async fn filter_memories(
        &self,
        filter: MemoryFilter,
        _sort_by: Option<&str>,
        _sort_order: Option<crate::storage::filters::SortOrder>,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        self.memory_ops.filter_memories(filter, limit).await
    }

    /// Count memories with optional filtering
    pub async fn count_memories(&self, filter: Option<MemoryFilter>) -> Result<usize> {
        self.memory_ops.count_memories(filter).await
    }

    /// Tag a memory
    pub async fn tag_memory(&self, memory_id: &str, tag: &str) -> Result<bool> {
        self.memory_ops.tag_memory(memory_id, tag).await
    }

    // =============================================================================
    // Memory Builder Methods (delegated to MemoryBuilders)
    // =============================================================================

    /// Add a fact memory (convenience method)
    pub async fn add_fact<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_fact(content).await
    }

    /// Add a conversation memory (convenience method)
    pub async fn add_conversation<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_conversation(content).await
    }

    /// Add a procedural memory (convenience method)
    pub async fn add_procedural<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_procedural(content).await
    }

    /// Add an episodic memory (convenience method)
    pub async fn add_episodic<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_episodic(content).await
    }

    /// Add an identity memory (convenience method)
    pub async fn add_identity<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_identity(content).await
    }

    /// Add a world memory (convenience method)
    pub async fn add_world<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_world(content).await
    }

    /// Add an action memory (convenience method)
    pub async fn add_action<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_action(content).await
    }

    /// Add an event memory (convenience method)
    pub async fn add_event<S: Into<String>>(&self, content: S) -> Result<String> {
        self.builders.add_event(content).await
    }

    /// Add a memory with a specific type
    pub async fn add_memory<S: Into<String>>(
        &self,
        content: S,
        memory_type: MemoryType,
    ) -> Result<String> {
        self.builders.add_memory(content, memory_type).await
    }

    /// Add a memory with customization options
    pub async fn add_memory_with_options<S, F>(&self, content: S, options: F) -> Result<String>
    where
        S: Into<String>,
        F: FnOnce(MemoryBuilder) -> MemoryBuilder,
    {
        self.builders
            .add_memory_with_options(content, options)
            .await
    }

    /// Add a memory with priority
    pub async fn add_memory_with_priority<S: Into<String>>(
        &self,
        content: S,
        memory_type: MemoryType,
        priority: MemoryPriority,
    ) -> Result<String> {
        self.builders
            .add_memory_with_priority(content, memory_type, priority)
            .await
    }

    // =============================================================================
    // Search Operations (delegated to SearchExtensions)
    // =============================================================================

    /// Perform a search for memories using the specified mode
    pub async fn search(
        &self,
        query_text: &str,
        limit: Option<usize>,
        filter: Option<SemanticSearchFilter>,
        search_mode: SearchMode,
    ) -> Result<Vec<SearchResult>> {
        self.search
            .search(query_text, limit, filter, search_mode)
            .await
    }

    /// Perform a search for memories with optional query embedding (BYOE approach)
    ///
    /// This method supports vector and hybrid search when a query embedding is provided.
    /// For BYOE (Bring Your Own Embeddings), users provide embeddings from their preferred
    /// provider (OpenAI, Cohere, Voyage, etc.).
    ///
    /// # Arguments
    /// * `query_text` - The natural language query string
    /// * `query_embedding` - Optional query embedding from user's provider
    /// * `limit` - Maximum number of results to return
    /// * `filter` - Optional filters to apply
    /// * `search_mode` - Search mode (Text, Vector, or Hybrid)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use locai::memory::SearchMode;
    ///
    /// // This example shows the concept - you would use your actual embedding provider
    /// async fn example_usage() -> locai::Result<()> {
    ///     let embedding = vec![0.1, 0.2, 0.3]; // Mock embedding
    ///     
    ///     // You would typically get the manager from a configured Locai instance:
    ///     // let locai = Locai::new().await?;
    ///     // let results = locai.manager().search_with_embedding(
    ///     //     "search query",
    ///     //     Some(&embedding),
    ///     //     Some(10),
    ///     //     None,
    ///     //     SearchMode::Vector
    ///     // ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn search_with_embedding(
        &self,
        query_text: &str,
        query_embedding: Option<&[f32]>,
        limit: Option<usize>,
        filter: Option<SemanticSearchFilter>,
        search_mode: SearchMode,
    ) -> Result<Vec<SearchResult>> {
        self.search
            .search_with_embedding(query_text, query_embedding, limit, filter, search_mode)
            .await
    }

    /// Legacy method for backward compatibility - use search() instead
    #[deprecated(note = "Use search() instead")]
    pub async fn semantic_search(
        &self,
        query_text: &str,
        limit: Option<usize>,
        filter: Option<SemanticSearchFilter>,
        search_mode: SearchMode,
    ) -> Result<Vec<SearchResult>> {
        self.search
            .search(query_text, limit, filter, search_mode)
            .await
    }

    /// Search memories by character name or general query
    pub async fn search_memories(&self, query: &str, limit: Option<usize>) -> Result<Vec<Memory>> {
        self.search.search_memories(query, limit).await
    }

    /// Enhanced search that removes the restrictive "fact" filter
    pub async fn enhanced_search(&self, query: &str, limit: Option<usize>) -> Result<Vec<Memory>> {
        self.search.enhanced_search(query, limit).await
    }

    /// Search memories for a character/entity with advanced options
    pub async fn search_memories_with_options<F>(
        &self,
        character_name: &str,
        filter_options: F,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>>
    where
        F: FnOnce(MemoryFilter) -> MemoryFilter,
    {
        self.search
            .search_memories_with_options(character_name, filter_options, limit)
            .await
    }

    /// Universal search across all data types (memories, entities, graphs)
    pub async fn universal_search(
        &self,
        query: &str,
        limit: Option<usize>,
        options: Option<UniversalSearchOptions>,
    ) -> Result<Vec<UniversalSearchResult>> {
        self.search.universal_search(query, limit, options).await
    }

    // =============================================================================
    // Graph Operations (delegated to GraphOperations)
    // =============================================================================

    /// Retrieve a memory with its graph context
    pub async fn get_memory_graph(&self, id: &str, depth: u8) -> Result<MemoryGraph> {
        self.graph.get_memory_graph(id, depth).await
    }

    /// Find paths between two memories
    pub async fn find_paths(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Vec<MemoryPath>> {
        self.graph.find_paths(from_id, to_id, max_depth).await
    }

    /// Find the shortest path between two memories
    pub async fn find_shortest_path(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> Result<Option<MemoryPath>> {
        self.graph
            .find_shortest_path(from_id, to_id, max_depth)
            .await
    }

    /// Find all memories connected to this memory by a specific relationship
    pub async fn find_connected_memories(
        &self,
        id: &str,
        relationship_type: &str,
        max_depth: u8,
    ) -> Result<Vec<Memory>> {
        self.graph
            .find_connected_memories(id, relationship_type, max_depth)
            .await
    }

    /// Create a relationship between two memories
    pub async fn create_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        self.graph
            .create_relationship(source_id, target_id, relationship_type)
            .await
    }

    /// Create a bidirectional relationship between two memories
    pub async fn create_bidirectional_relationship(
        &self,
        memory_id1: &str,
        memory_id2: &str,
        relationship_type: &str,
    ) -> Result<bool> {
        self.graph
            .create_bidirectional_relationship(memory_id1, memory_id2, relationship_type)
            .await
    }

    /// Add a related memory in one step
    pub async fn add_related_memory<S: Into<String>>(
        &self,
        parent_id: &str,
        content: S,
        relationship_type: &str,
        memory_type: Option<MemoryType>,
    ) -> Result<String> {
        self.graph
            .add_related_memory(
                parent_id,
                content,
                relationship_type,
                memory_type,
                &self.memory_ops,
            )
            .await
    }

    /// Create a bidirectional related memory
    pub async fn add_bidirectional_related_memory<S: Into<String>>(
        &self,
        parent_id: &str,
        content: S,
        relationship_type: &str,
        memory_type: Option<MemoryType>,
    ) -> Result<String> {
        self.graph
            .add_bidirectional_related_memory(
                parent_id,
                content,
                relationship_type,
                memory_type,
                &self.memory_ops,
            )
            .await
    }

    /// Get related memories by relationship type
    pub async fn get_related_memories(
        &self,
        memory_id: &str,
        relationship_type: Option<&str>,
        direction: &str,
    ) -> Result<Vec<Memory>> {
        self.graph
            .get_related_memories(memory_id, relationship_type, direction)
            .await
    }

    /// Query cross-process relationships (enabled by shared database)
    pub async fn get_process_interactions(&self, process_id: &str) -> Result<Vec<Relationship>> {
        self.graph.get_process_interactions(process_id).await
    }

    // =============================================================================
    // Entity Operations (delegated to EntityOperations)
    // =============================================================================

    /// Create a new entity
    pub async fn create_entity(&self, entity: Entity) -> Result<Entity> {
        self.entities.create_entity(entity).await
    }

    /// Get an entity by ID
    pub async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        self.entities.get_entity(id).await
    }

    /// Update an existing entity
    pub async fn update_entity(&self, entity: Entity) -> Result<Entity> {
        self.entities.update_entity(entity).await
    }

    /// Delete an entity by ID
    pub async fn delete_entity(&self, id: &str) -> Result<bool> {
        self.entities.delete_entity(id).await
    }

    /// List entities with optional filtering
    pub async fn list_entities(
        &self,
        filter: Option<EntityFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Entity>> {
        self.entities.list_entities(filter, limit, offset).await
    }

    /// Count entities with optional filtering
    pub async fn count_entities(&self, filter: Option<EntityFilter>) -> Result<usize> {
        self.entities.count_entities(filter).await
    }

    /// Find related entities
    pub async fn find_related_entities(
        &self,
        entity_id: &str,
        relationship_type: Option<String>,
        direction: Option<String>,
    ) -> Result<Vec<Entity>> {
        self.entities
            .find_related_entities(entity_id, relationship_type, direction)
            .await
    }

    /// Get memories by priority level
    pub async fn get_memories_by_priority(
        &self,
        priority: MemoryPriority,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        self.entities
            .get_memories_by_priority(priority, limit)
            .await
    }

    /// Get memories by type
    pub async fn get_memories_by_type(
        &self,
        memory_type: MemoryType,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        self.entities.get_memories_by_type(memory_type, limit).await
    }

    /// Find memories by tag
    pub async fn find_memories_by_tag(
        &self,
        tag: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        self.entities.find_memories_by_tag(tag, limit).await
    }

    /// Get recent memories
    pub async fn get_recent_memories(&self, limit: usize) -> Result<Vec<Memory>> {
        self.entities.get_recent_memories(limit).await
    }

    // =============================================================================
    // Relationship Operations (delegated to RelationshipStorage)
    // =============================================================================

    /// Create a new relationship entity
    pub async fn create_relationship_entity(
        &self,
        relationship: Relationship,
    ) -> Result<Relationship> {
        self.relationships
            .storage()
            .create_relationship(relationship)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to create relationship: {}", e)))
    }

    /// Get a relationship by ID
    pub async fn get_relationship(&self, id: &str) -> Result<Option<Relationship>> {
        self.relationships
            .storage()
            .get_relationship(id)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to get relationship: {}", e)))
    }

    /// Update an existing relationship
    pub async fn update_relationship(&self, relationship: Relationship) -> Result<Relationship> {
        self.relationships
            .storage()
            .update_relationship(relationship)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to update relationship: {}", e)))
    }

    /// Delete a relationship by ID
    pub async fn delete_relationship(&self, id: &str) -> Result<bool> {
        self.relationships
            .storage()
            .delete_relationship(id)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to delete relationship: {}", e)))
    }

    /// List relationships with optional filtering
    pub async fn list_relationships(
        &self,
        filter: Option<RelationshipFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Relationship>> {
        self.relationships
            .storage()
            .list_relationships(filter, limit, offset)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to list relationships: {}", e)))
    }

    /// Count relationships with optional filtering
    pub async fn count_relationships(&self, filter: Option<RelationshipFilter>) -> Result<usize> {
        self.relationships
            .storage()
            .count_relationships(filter)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to count relationships: {}", e)))
    }

    // =============================================================================
    // Messaging Operations (delegated to MessagingIntegration)
    // =============================================================================

    /// Subscribe to memory changes with live queries (for messaging system)
    pub async fn subscribe_to_memory_changes(
        &self,
        filter: MemoryFilter,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<Memory>> + Send>>> {
        self.messaging.subscribe_to_memory_changes(filter).await
    }

    /// Store a message as a memory record (specialized method for messaging)
    pub async fn store_message(
        &self,
        message: &crate::messaging::types::Message,
    ) -> Result<String> {
        self.messaging
            .store_message(message, &self.memory_ops)
            .await
    }

    /// Get message history (specialized method for messaging)
    pub async fn get_message_history(
        &self,
        filter: &crate::messaging::types::MessageFilter,
        limit: Option<usize>,
    ) -> Result<Vec<crate::messaging::types::Message>> {
        self.messaging.get_message_history(filter, limit).await
    }

    // =============================================================================
    // Configuration and Utility Methods
    // =============================================================================

    /// Get the configuration for this memory manager
    pub fn config(&self) -> &LocaiConfig {
        &self.config
    }

    /// Check if ML service is available for semantic search
    pub fn has_ml_service(&self) -> bool {
        self.memory_ops.has_ml_service()
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn crate::storage::traits::GraphStore> {
        self.memory_ops.storage()
    }

    /// Clear all data from the storage
    pub async fn clear_storage(&self) -> Result<()> {
        self.memory_ops
            .storage()
            .clear_storage()
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to clear storage: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_memory_manager_creation() {
        // This is a placeholder test
        assert!(true);
    }
}
