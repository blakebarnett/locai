//! Trait definitions for storage components in Locai

use async_trait::async_trait;
use std::fmt::Debug;

use crate::models::Memory;
use crate::storage::errors::StorageError;
use crate::storage::filters::{EntityFilter, MemoryFilter, RelationshipFilter, VectorFilter};
use crate::storage::models::{
    Entity, MemoryDiff, MemoryGraph, MemoryPath, MemorySnapshot, MemoryVersionInfo, Relationship,
    RestoreMode, Vector, VectorSearchParams, Version,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Base trait for all storage implementations
#[async_trait]
pub trait BaseStore: Send + Sync + 'static + Debug {
    /// Check if the store is healthy and available
    async fn health_check(&self) -> std::result::Result<bool, StorageError>;

    /// Clear all data in the store
    async fn clear(&self) -> std::result::Result<(), StorageError>;

    /// Get metadata about the store
    async fn get_metadata(&self) -> std::result::Result<serde_json::Value, StorageError>;

    /// Close connections and release resources
    async fn close(&self) -> std::result::Result<(), StorageError>;
}

/// Trait for memory operations
#[async_trait]
pub trait MemoryStore: BaseStore {
    /// Create a new memory
    async fn create_memory(&self, memory: Memory) -> std::result::Result<Memory, StorageError>;

    /// Get a memory by its ID
    async fn get_memory(&self, id: &str) -> std::result::Result<Option<Memory>, StorageError>;

    /// Update an existing memory
    async fn update_memory(&self, memory: Memory) -> std::result::Result<Memory, StorageError>;

    /// Delete a memory by its ID
    async fn delete_memory(&self, id: &str) -> std::result::Result<bool, StorageError>;

    /// List memories with optional filtering
    async fn list_memories(
        &self,
        filter: Option<MemoryFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> std::result::Result<Vec<Memory>, StorageError>;

    /// Count memories with optional filtering
    async fn count_memories(
        &self,
        filter: Option<MemoryFilter>,
    ) -> std::result::Result<usize, StorageError>;

    /// Batch create multiple memories
    async fn batch_create_memories(
        &self,
        memories: Vec<Memory>,
    ) -> std::result::Result<Vec<Memory>, StorageError>;

    /// Full-text search using BM25 scoring with highlights
    async fn bm25_search_memories(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> std::result::Result<Vec<(Memory, f32, String)>, StorageError>;

    /// Fuzzy search for typo tolerance
    async fn fuzzy_search_memories(
        &self,
        query: &str,
        similarity_threshold: Option<f32>,
        limit: Option<usize>,
    ) -> std::result::Result<Vec<(Memory, f32)>, StorageError>;

    /// Vector similarity search on memories using their embeddings (BYOE approach)
    ///
    /// Searches memories that have embeddings using vector similarity to the provided query embedding.
    /// This supports the BYOE (Bring Your Own Embeddings) approach where users provide embeddings
    /// from their preferred provider (OpenAI, Cohere, Voyage, etc.).
    ///
    /// # Arguments
    /// * `query_vector` - The query embedding vector from user's provider
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of tuples containing (Memory, similarity_score, highlight)
    /// where similarity_score is between 0.0 and 1.0 (higher = more similar)
    async fn vector_search_memories(
        &self,
        query_vector: &[f32],
        limit: Option<usize>,
    ) -> std::result::Result<Vec<(Memory, f32, String)>, StorageError>;

    /// Search memories with configurable multi-factor scoring
    ///
    /// Combines BM25 keyword matching, vector similarity (if available), and
    /// memory lifecycle metadata (recency, access count, priority) to produce
    /// comprehensive relevance rankings.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `scoring` - Optional scoring configuration. If None, uses default
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of (Memory, final_score) tuples, sorted by score (highest first)
    async fn search_memories_with_scoring(
        &self,
        query: &str,
        scoring: Option<crate::search::ScoringConfig>,
        limit: Option<usize>,
    ) -> std::result::Result<Vec<(Memory, f32)>, StorageError>;
}

/// Trait for entity operations
#[async_trait]
pub trait EntityStore: BaseStore {
    /// Create a new entity
    async fn create_entity(&self, entity: Entity) -> std::result::Result<Entity, StorageError>;

    /// Get an entity by its ID
    async fn get_entity(&self, id: &str) -> std::result::Result<Option<Entity>, StorageError>;

    /// Update an existing entity
    async fn update_entity(&self, entity: Entity) -> std::result::Result<Entity, StorageError>;

    /// Delete an entity by its ID
    async fn delete_entity(&self, id: &str) -> std::result::Result<bool, StorageError>;

    /// List entities with optional filtering
    async fn list_entities(
        &self,
        filter: Option<EntityFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> std::result::Result<Vec<Entity>, StorageError>;

    /// Count entities with optional filtering
    async fn count_entities(
        &self,
        filter: Option<EntityFilter>,
    ) -> std::result::Result<usize, StorageError>;
}

/// Trait for relationship operations
#[async_trait]
pub trait RelationshipStore: BaseStore {
    /// Create a new relationship
    async fn create_relationship(
        &self,
        relationship: Relationship,
    ) -> std::result::Result<Relationship, StorageError>;

    /// Get a relationship by its ID
    async fn get_relationship(
        &self,
        id: &str,
    ) -> std::result::Result<Option<Relationship>, StorageError>;

    /// Update an existing relationship
    async fn update_relationship(
        &self,
        relationship: Relationship,
    ) -> std::result::Result<Relationship, StorageError>;

    /// Delete a relationship by its ID
    async fn delete_relationship(&self, id: &str) -> std::result::Result<bool, StorageError>;

    /// List relationships with optional filtering
    async fn list_relationships(
        &self,
        filter: Option<RelationshipFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> std::result::Result<Vec<Relationship>, StorageError>;

    /// Count relationships with optional filtering
    async fn count_relationships(
        &self,
        filter: Option<RelationshipFilter>,
    ) -> std::result::Result<usize, StorageError>;

    /// Get a relationship by source and target entities
    async fn get_relationship_by_entities(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> std::result::Result<Option<Relationship>, StorageError>;

    /// Get properties of a relationship
    async fn get_relationship_properties(
        &self,
        id: &str,
    ) -> std::result::Result<serde_json::Value, StorageError>;

    /// Find relationships between two entities
    async fn find_relationships(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: Option<String>,
    ) -> std::result::Result<Vec<Relationship>, StorageError>;

    /// Find entities related to the given entity
    async fn find_related_entities(
        &self,
        entity_id: &str,
        relationship_type: Option<String>,
        direction: Option<String>,
    ) -> std::result::Result<Vec<Entity>, StorageError>;
}

/// Trait for versioning operations
#[async_trait]
pub trait VersionStore: BaseStore {
    /// Create a new version
    async fn create_version(&self, version: Version) -> std::result::Result<Version, StorageError>;

    /// Get a version by its ID
    async fn get_version(&self, id: &str) -> std::result::Result<Option<Version>, StorageError>;

    /// List versions with optional filtering
    async fn list_versions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> std::result::Result<Vec<Version>, StorageError>;

    /// Checkout a specific version, making it the active state
    async fn checkout_version(&self, id: &str) -> std::result::Result<bool, StorageError>;
}

/// Combined trait for all graph operations
#[async_trait]
pub trait GraphStore:
    MemoryStore + EntityStore + RelationshipStore + VersionStore + VectorStore + GraphTraversal
{
    /// Clear all data from the storage
    async fn clear_storage(&self) -> std::result::Result<(), StorageError>;

    /// Check if this store supports live queries
    fn supports_live_queries(&self) -> bool {
        false
    }

    /// Get live query setup information if supported
    /// Returns the type name for casting purposes
    fn get_live_query_info(&self) -> Option<&'static str> {
        None
    }

    /// Setup live queries if supported
    /// Returns a receiver for database events
    async fn setup_live_queries(
        &self,
    ) -> std::result::Result<Option<Box<dyn std::any::Any + Send>>, StorageError> {
        Ok(None)
    }

    /// Get a reference to the underlying store as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Trait for vector operations
#[async_trait]
pub trait VectorStore: BaseStore {
    /// Add a vector with metadata
    async fn add_vector(&self, vector: Vector) -> std::result::Result<Vector, StorageError>;

    /// Get a vector by its ID
    async fn get_vector(&self, id: &str) -> std::result::Result<Option<Vector>, StorageError>;

    /// Delete a vector by its ID
    async fn delete_vector(&self, id: &str) -> std::result::Result<bool, StorageError>;

    /// Update vector metadata
    async fn update_vector_metadata(
        &self,
        id: &str,
        metadata: serde_json::Value,
    ) -> std::result::Result<Vector, StorageError>;

    /// Search for similar vectors
    async fn search_vectors(
        &self,
        query_vector: &[f32],
        params: VectorSearchParams,
    ) -> std::result::Result<Vec<(Vector, f32)>, StorageError>;

    /// List vectors with optional filtering
    async fn list_vectors(
        &self,
        filter: Option<VectorFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> std::result::Result<Vec<Vector>, StorageError>;

    /// Count vectors with optional filtering
    async fn count_vectors(
        &self,
        filter: Option<VectorFilter>,
    ) -> std::result::Result<usize, StorageError>;

    /// Batch add multiple vectors
    async fn batch_add_vectors(
        &self,
        vectors: Vec<Vector>,
    ) -> std::result::Result<Vec<Vector>, StorageError>;

    /// Add or update a vector (Upsert)
    async fn upsert_vector(&self, vector: Vector) -> std::result::Result<(), StorageError>;
}

/// Graph traversal operations for memory graphs
#[async_trait]
pub trait GraphTraversal: Send + Sync + 'static {
    /// Get a subgraph of memories and relationships centered on a specific memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the central memory
    /// * `depth` - How many relationship hops to traverse
    ///
    /// # Returns
    /// A graph structure containing memories and relationships
    async fn get_memory_subgraph(
        &self,
        memory_id: &str,
        depth: u8,
    ) -> std::result::Result<MemoryGraph, StorageError>;

    /// Find paths between two memories
    ///
    /// # Arguments
    /// * `from_id` - The ID of the starting memory
    /// * `to_id` - The ID of the destination memory
    /// * `max_depth` - Maximum path length to consider
    ///
    /// # Returns
    /// A vector of paths (each containing memories and relationships)
    async fn find_paths(
        &self,
        from_id: &str,
        to_id: &str,
        max_depth: u8,
    ) -> std::result::Result<Vec<MemoryPath>, StorageError>;

    /// Find memories connected to a given memory by a specific relationship type
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the starting memory
    /// * `relationship_type` - The relationship type to follow (None for all types)
    /// * `max_depth` - Maximum traversal depth
    ///
    /// # Returns
    /// A vector of connected memories
    async fn find_connected_memories(
        &self,
        memory_id: &str,
        relationship_type: Option<&str>,
        max_depth: u8,
    ) -> std::result::Result<Vec<Memory>, StorageError>;

    /// Get entities contained in a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// A vector of entities contained in the memory
    async fn get_entities_from_memory(
        &self,
        memory_id: &str,
    ) -> std::result::Result<Vec<Entity>, StorageError>;

    /// Get memories that contain a specific entity
    ///
    /// # Arguments
    /// * `entity_id` - The ID of the entity
    ///
    /// # Returns
    /// A vector of memories that contain the entity
    async fn get_memories_containing_entity(
        &self,
        entity_id: &str,
    ) -> std::result::Result<Vec<Memory>, StorageError>;

    /// Get all relationships for an entity
    ///
    /// # Arguments
    /// * `entity_id` - The ID of the entity
    ///
    /// # Returns
    /// A vector of relationships involving the entity
    async fn get_entity_relationships(
        &self,
        entity_id: &str,
    ) -> std::result::Result<Vec<Relationship>, StorageError>;
}

/// Trait for memory versioning operations
#[async_trait]
pub trait MemoryVersionStore: BaseStore {
    /// Create a new version of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory to version
    /// * `content` - The new content for this version
    /// * `metadata` - Optional metadata for this version
    ///
    /// # Returns
    /// The version ID of the created version
    async fn create_memory_version(
        &self,
        memory_id: &str,
        content: &str,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<String, StorageError>;

    /// Get a specific version of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `version_id` - The ID of the version to retrieve
    ///
    /// # Returns
    /// The memory at the specified version, or None if not found
    async fn get_memory_version(
        &self,
        memory_id: &str,
        version_id: &str,
    ) -> std::result::Result<Option<Memory>, StorageError>;

    /// Get the current (latest) version of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// The current version of the memory, or None if not found
    async fn get_memory_current_version(
        &self,
        memory_id: &str,
    ) -> std::result::Result<Option<Memory>, StorageError>;

    /// List all versions of a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// A list of version information, ordered by creation time
    async fn list_memory_versions(
        &self,
        memory_id: &str,
    ) -> std::result::Result<Vec<MemoryVersionInfo>, StorageError>;

    /// Get memory as it existed at a specific time
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `at_time` - The timestamp to query
    ///
    /// # Returns
    /// The memory as it existed at that time, or None if not found
    async fn get_memory_at_time(
        &self,
        memory_id: &str,
        at_time: DateTime<Utc>,
    ) -> std::result::Result<Option<Memory>, StorageError>;

    /// Delete a specific version (or all versions if version_id is None)
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `version_id` - The ID of the version to delete, or None to delete all versions
    ///
    /// # Returns
    /// Ok(()) on success
    async fn delete_memory_version(
        &self,
        memory_id: &str,
        version_id: Option<&str>,
    ) -> std::result::Result<(), StorageError>;

    /// Compute diff between two versions
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `old_version_id` - The ID of the old version
    /// * `new_version_id` - The ID of the new version
    ///
    /// # Returns
    /// A diff structure showing the changes
    async fn diff_memory_versions(
        &self,
        memory_id: &str,
        old_version_id: &str,
        new_version_id: &str,
    ) -> std::result::Result<MemoryDiff, StorageError>;

    /// Create a snapshot
    ///
    /// # Arguments
    /// * `memory_ids` - Optional list of memory IDs to include (None = all memories)
    /// * `metadata` - Optional metadata for the snapshot
    ///
    /// # Returns
    /// The created snapshot
    async fn create_snapshot(
        &self,
        memory_ids: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<MemorySnapshot, StorageError>;

    /// Restore from snapshot
    ///
    /// # Arguments
    /// * `snapshot` - The snapshot to restore
    /// * `restore_mode` - How to handle existing memories
    ///
    /// # Returns
    /// Ok(()) on success
    async fn restore_snapshot(
        &self,
        snapshot: &MemorySnapshot,
        restore_mode: RestoreMode,
    ) -> std::result::Result<(), StorageError>;

    /// Search memories in a snapshot state
    ///
    /// # Arguments
    /// * `snapshot` - The snapshot to search
    /// * `query` - The search query
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// A list of memories matching the query as they existed in the snapshot
    async fn search_snapshot(
        &self,
        snapshot: &MemorySnapshot,
        query: &str,
        limit: Option<usize>,
    ) -> std::result::Result<Vec<Memory>, StorageError>;

    /// Get a memory from a snapshot
    ///
    /// # Arguments
    /// * `snapshot` - The snapshot
    /// * `memory_id` - The ID of the memory to retrieve
    ///
    /// # Returns
    /// The memory as it existed in the snapshot, or None if not found
    async fn get_memory_from_snapshot(
        &self,
        snapshot: &MemorySnapshot,
        memory_id: &str,
    ) -> std::result::Result<Option<Memory>, StorageError>;

    /// Get versioning statistics
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to get stats for (None = all memories)
    ///
    /// # Returns
    /// Versioning statistics
    async fn get_versioning_stats(
        &self,
        memory_id: Option<&str>,
    ) -> std::result::Result<crate::storage::models::VersioningStats, StorageError>;

    /// Compact versions by removing old versions
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to compact (None = all memories)
    /// * `keep_count` - Number of most recent versions to keep
    /// * `older_than_days` - Remove versions older than this many days
    ///
    /// # Returns
    /// Number of versions removed
    async fn compact_versions(
        &self,
        memory_id: Option<&str>,
        keep_count: Option<usize>,
        older_than_days: Option<u64>,
    ) -> std::result::Result<usize, StorageError>;

    /// Validate version integrity
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to validate (None = all memories)
    ///
    /// # Returns
    /// List of integrity issues found
    async fn validate_versions(
        &self,
        memory_id: Option<&str>,
    ) -> std::result::Result<Vec<crate::storage::models::VersionIntegrityIssue>, StorageError>;

    /// Repair corrupted versions
    ///
    /// # Arguments
    /// * `memory_id` - Optional memory ID to repair (None = all memories)
    ///
    /// # Returns
    /// Repair report
    async fn repair_versions(
        &self,
        memory_id: Option<&str>,
    ) -> std::result::Result<crate::storage::models::RepairReport, StorageError>;

    /// Promote a delta version to full copy
    ///
    /// # Arguments
    /// * `memory_id` - The memory ID
    /// * `version_id` - The version ID to promote
    ///
    /// # Returns
    /// Ok(()) on success
    async fn promote_version_to_full_copy(
        &self,
        memory_id: &str,
        version_id: &str,
    ) -> std::result::Result<(), StorageError>;
}
