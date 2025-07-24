//! Data structures and models for storage operations

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::models::Memory;
use crate::storage::filters::VectorFilter;

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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistanceMetric {
    /// Cosine similarity (default, good for normalized vectors)
    Cosine,
    /// Euclidean distance (L2 norm)
    Euclidean,
    /// Dot product similarity
    DotProduct,
    /// Manhattan distance (L1 norm)
    Manhattan,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Cosine
    }
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