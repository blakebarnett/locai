//! Data structures and models for storage operations

use crate::models::Memory;
use crate::storage::filters::VectorFilter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Entity model representing a node in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entity {
    /// Unique identifier for the entity
    pub id: String,

    /// Type of entity
    pub entity_type: String,

    /// Properties associated with the entity
    pub properties: serde_json::Value,

    /// When the entity was created
    pub created_at: DateTime<Utc>,

    /// When the entity was last updated
    pub updated_at: DateTime<Utc>,
}

/// Relationship model representing an edge in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    /// Unique identifier for the relationship
    pub id: String,

    /// Type of relationship
    pub relationship_type: String,

    /// Source entity ID
    pub source_id: String,

    /// Target entity ID
    pub target_id: String,

    /// Properties associated with the relationship
    pub properties: serde_json::Value,

    /// When the relationship was created
    pub created_at: DateTime<Utc>,

    /// When the relationship was last updated
    pub updated_at: DateTime<Utc>,
}

/// Version model for representing a snapshot in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// Unique identifier for the version
    pub id: String,

    /// Description of the version
    pub description: String,

    /// When the version was created
    pub created_at: DateTime<Utc>,

    /// Metadata associated with the version
    pub metadata: serde_json::Value,
}

/// Vector model for representing embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    /// Unique identifier for the vector
    pub id: String,

    /// The actual vector data
    pub vector: Vec<f32>,

    /// Dimension of the vector
    pub dimension: usize,

    /// Metadata associated with the vector
    pub metadata: serde_json::Value,

    /// Source reference (e.g., memory ID)
    pub source_id: Option<String>,

    /// When the vector was created
    pub created_at: DateTime<Utc>,
}

/// Parameters for vector search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchParams {
    /// Maximum number of results to return
    pub limit: Option<usize>,

    /// Minimum similarity threshold (0.0 to 1.0)
    pub threshold: Option<f32>,

    /// Optional filter for vector search
    pub filter: Option<VectorFilter>,

    /// Whether to include the vector data in results
    #[serde(default = "default_true")]
    pub include_vectors: bool,

    /// Whether to include metadata in results
    #[serde(default = "default_true")]
    pub include_metadata: bool,

    /// Distance metric to use for vector search
    pub distance_metric: Option<DistanceMetric>,
}

impl Default for VectorSearchParams {
    fn default() -> Self {
        Self {
            limit: Some(10),
            threshold: None,
            filter: None,
            include_vectors: true,
            include_metadata: true,
            distance_metric: Some(DistanceMetric::Cosine),
        }
    }
}

/// Distance metric for vector similarity calculations
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DistanceMetric {
    /// Cosine similarity (default, good for normalized vectors)
    #[default]
    Cosine,
    /// Euclidean distance (L2 norm)
    Euclidean,
    /// Dot product similarity
    DotProduct,
    /// Manhattan distance (L1 norm)
    Manhattan,
}

/// Helper function for serde default values
fn default_true() -> bool {
    true
}

/// Reference to an entity or relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Reference {
    /// Reference to an entity
    Entity(String),

    /// Reference to a relationship
    Relationship(String),
}

/// Connection information for a storage backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Connection string or URL
    pub url: String,

    /// Username for authentication, if applicable
    pub username: Option<String>,

    /// Password for authentication, if applicable
    pub password: Option<String>,

    /// Database name
    pub database: Option<String>,

    /// Additional connection parameters
    pub parameters: HashMap<String, String>,

    /// Connection timeout in seconds
    pub timeout_seconds: Option<u64>,
}

/// A graph representation of memories and their relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryGraph {
    /// Central memory ID
    pub center_id: String,

    /// All memories in the graph
    pub memories: HashMap<String, Memory>,

    /// All relationships between memories
    pub relationships: Vec<Relationship>,
}

impl MemoryGraph {
    /// Create a new memory graph with a central memory
    pub fn new(center_id: String) -> Self {
        Self {
            center_id,
            memories: HashMap::new(),
            relationships: Vec::new(),
        }
    }

    /// Add a memory to the graph
    pub fn add_memory(&mut self, memory: Memory) {
        self.memories.insert(memory.id.clone(), memory);
    }

    /// Add a relationship to the graph
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationships.push(relationship);
    }

    /// Get all memory IDs in the graph
    pub fn memory_ids(&self) -> Vec<String> {
        self.memories.keys().cloned().collect()
    }

    /// Get all memories as a vector
    pub fn memories_vec(&self) -> Vec<Memory> {
        self.memories.values().cloned().collect()
    }
}

/// A path connecting two memories in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPath {
    /// Start memory ID
    pub from_id: String,

    /// End memory ID
    pub to_id: String,

    /// Ordered list of memories on the path
    pub memories: Vec<Memory>,

    /// Ordered list of relationships on the path
    pub relationships: Vec<Relationship>,
}

impl MemoryPath {
    /// Create a new path between two memories
    pub fn new(from_id: String, to_id: String) -> Self {
        Self {
            from_id,
            to_id,
            memories: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Add a memory to the path
    pub fn add_memory(&mut self, memory: Memory) {
        self.memories.push(memory);
    }

    /// Add a relationship to the path
    pub fn add_relationship(&mut self, relationship: Relationship) {
        self.relationships.push(relationship);
    }

    /// Get the length of the path (number of relationships)
    pub fn length(&self) -> usize {
        self.relationships.len()
    }
}

/// Represents a single result from a semantic search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The memory object that matched the search query.
    pub memory: Memory,

    /// The relevance score of this memory to the search query.
    /// The nature of this score (e.g., cosine similarity) depends on the
    /// underlying vector store and embedding model.
    /// This will be `None` for keyword-only searches.
    pub score: Option<f32>,
    // TODO: Consider adding other metadata, e.g., distance if different from score,
    // or explainability features if supported.
}

// Memory Versioning Models

/// Information about a memory version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersionInfo {
    /// Unique version identifier
    pub version_id: String,
    /// Memory ID this version belongs to
    pub memory_id: String,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Preview of version content (first 100 chars)
    pub content_preview: String,
    /// Size of version in bytes
    pub size_bytes: usize,
    /// Whether this version is stored as a delta
    pub is_delta: bool,
    /// Parent version ID (for delta versions)
    pub parent_version_id: Option<String>,
}

/// Diff between two memory versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDiff {
    /// Memory ID
    pub memory_id: String,
    /// Old version ID
    pub old_version_id: String,
    /// New version ID
    pub new_version_id: String,
    /// List of changes
    pub changes: Vec<Change>,
    /// Type of diff
    pub diff_type: DiffType,
}

/// A single change in a memory version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Change {
    /// Content was changed
    ContentChanged {
        /// Old content
        old_content: String,
        /// New content
        new_content: String,
        /// Diff hunks showing the changes
        diff_hunks: Vec<DiffHunk>,
    },
    /// Metadata was changed
    MetadataChanged {
        /// Key that changed
        key: String,
        /// Old value (None if added)
        old_value: Option<serde_json::Value>,
        /// New value (None if removed)
        new_value: Option<serde_json::Value>,
    },
    /// Memory was deleted
    Deleted,
    /// Memory was created
    Created,
}

/// A hunk of diff lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Starting line in old version
    pub old_start_line: usize,
    /// Number of lines in old version
    pub old_line_count: usize,
    /// Starting line in new version
    pub new_start_line: usize,
    /// Number of lines in new version
    pub new_line_count: usize,
    /// Lines in this hunk
    pub lines: Vec<DiffLine>,
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    /// Unchanged context line
    Context(String),
    /// Removed line
    Removed(String),
    /// Added line
    Added(String),
}

/// Type of diff
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiffType {
    /// Full diff (complete content)
    Full,
    /// Delta diff (only changes)
    Delta,
}

/// Snapshot of memory state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Unique snapshot identifier
    pub snapshot_id: String,
    /// When snapshot was created
    pub created_at: DateTime<Utc>,
    /// Number of memories in snapshot
    pub memory_count: usize,
    /// List of memory IDs in snapshot
    pub memory_ids: Vec<String>,
    /// Map from memory_id to version_id
    pub version_map: HashMap<String, String>,
    /// Snapshot metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Size of snapshot in bytes
    pub size_bytes: usize,
}

/// Mode for restoring snapshots
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RestoreMode {
    /// Overwrite existing memories
    Overwrite,
    /// Skip existing memories
    SkipExisting,
    /// Create new versions instead of overwriting
    CreateVersions,
}

/// Versioning statistics for a memory or all memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningStats {
    /// Total number of versions
    pub total_versions: usize,
    /// Number of delta versions
    pub total_delta_versions: usize,
    /// Number of full copy versions
    pub total_full_versions: usize,
    /// Total storage size in bytes
    pub storage_size_bytes: usize,
    /// Storage savings from deltas in bytes
    pub storage_savings_bytes: usize,
    /// Number of compressed versions
    pub compressed_versions: usize,
    /// Average versions per memory
    pub average_versions_per_memory: f64,
    /// Memory ID (if stats for specific memory)
    pub memory_id: Option<String>,
}

/// Version integrity issue found during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionIntegrityIssue {
    /// Memory ID
    pub memory_id: String,
    /// Version ID (if specific version)
    pub version_id: Option<String>,
    /// Issue type
    pub issue_type: IntegrityIssueType,
    /// Issue description
    pub description: String,
}

/// Type of integrity issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrityIssueType {
    /// Missing parent version
    MissingParent,
    /// Broken delta chain
    BrokenDeltaChain,
    /// Corrupted delta data
    CorruptedDelta,
    /// Missing base version
    MissingBase,
    /// Orphaned version (no parent chain to base)
    OrphanedVersion,
}

/// Repair report from version repair operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairReport {
    /// Number of versions repaired
    pub versions_repaired: usize,
    /// Number of versions that could not be repaired
    pub versions_failed: usize,
    /// Details of repairs
    pub repair_details: Vec<String>,
}
