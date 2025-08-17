//! Data Transfer Objects for the API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use locai::models::Memory;
use locai::storage::models::{
    Entity, MemoryGraph, MemoryPath, Relationship, SearchResult, Version,
};

/// Memory DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryDto {
    /// Unique identifier for the memory
    pub id: String,

    /// The actual content of the memory
    pub content: String,

    /// Type of memory
    pub memory_type: String,

    /// When the memory was created
    pub created_at: DateTime<Utc>,

    /// When the memory was last accessed
    pub last_accessed: Option<DateTime<Utc>>,

    /// How many times the memory has been accessed
    pub access_count: u32,

    /// Priority/importance of the memory
    pub priority: String,

    /// Tags associated with the memory
    pub tags: Vec<String>,

    /// Source of the memory
    pub source: String,

    /// When the memory expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional properties as arbitrary JSON
    pub properties: serde_json::Value,

    /// References to related memories by ID
    pub related_memories: Vec<String>,

    /// HATEOAS links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HateoasLinks>,
}

impl From<Memory> for MemoryDto {
    fn from(memory: Memory) -> Self {
        Self {
            id: memory.id.clone(),
            content: memory.content,
            memory_type: memory.memory_type.to_string(),
            created_at: memory.created_at,
            last_accessed: memory.last_accessed,
            access_count: memory.access_count,
            priority: format!("{:?}", memory.priority),
            tags: memory.tags,
            source: memory.source,
            expires_at: memory.expires_at,
            properties: memory.properties,
            related_memories: memory.related_memories,
            links: Some(HateoasLinks::for_memory(&memory.id)),
        }
    }
}

/// Request to create a new memory
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateMemoryRequest {
    /// The content of the memory
    pub content: String,

    /// Type of memory (defaults to "fact")
    #[serde(default = "default_memory_type")]
    pub memory_type: String,

    /// Priority of the memory (defaults to "normal")
    #[serde(default = "default_priority")]
    pub priority: String,

    /// Tags for the memory
    #[serde(default)]
    pub tags: Vec<String>,

    /// Source of the memory (defaults to "api")
    #[serde(default = "default_source")]
    pub source: String,

    /// Expiration date for the memory
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional properties
    #[serde(default)]
    pub properties: serde_json::Value,
}

fn default_source() -> String {
    "api".to_string()
}

fn default_memory_type() -> String {
    "fact".to_string()
}

fn default_priority() -> String {
    "normal".to_string()
}

/// Request to update an existing memory
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateMemoryRequest {
    /// Updated content (optional)
    pub content: Option<String>,

    /// Updated memory type (optional)
    pub memory_type: Option<String>,

    /// Updated priority (optional)
    pub priority: Option<String>,

    /// Updated tags (optional)
    pub tags: Option<Vec<String>>,

    /// Updated source (optional)
    pub source: Option<String>,

    /// Updated expiration date (optional)
    pub expires_at: Option<DateTime<Utc>>,

    /// Updated properties (optional)
    pub properties: Option<serde_json::Value>,
}

/// Entity DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EntityDto {
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

    /// HATEOAS links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HateoasLinks>,
}

impl From<Entity> for EntityDto {
    fn from(entity: Entity) -> Self {
        Self {
            id: entity.id.clone(),
            entity_type: entity.entity_type,
            properties: entity.properties,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
            links: Some(HateoasLinks::for_entity(&entity.id)),
        }
    }
}

/// Request to create a new entity
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEntityRequest {
    /// Type of entity
    pub entity_type: String,

    /// Properties associated with the entity
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// Request to update an existing entity
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateEntityRequest {
    /// Updated entity type (optional)
    pub entity_type: Option<String>,

    /// Updated properties (optional)
    pub properties: Option<serde_json::Value>,
}

/// Relationship DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RelationshipDto {
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

    /// HATEOAS links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HateoasLinks>,
}

impl From<Relationship> for RelationshipDto {
    fn from(relationship: Relationship) -> Self {
        Self {
            id: relationship.id.clone(),
            relationship_type: relationship.relationship_type,
            source_id: relationship.source_id,
            target_id: relationship.target_id,
            properties: relationship.properties,
            created_at: relationship.created_at,
            updated_at: relationship.updated_at,
            links: Some(HateoasLinks::for_relationship(&relationship.id)),
        }
    }
}

/// Request to create a new relationship
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRelationshipRequest {
    /// Type of relationship
    pub relationship_type: String,

    /// Source entity ID
    pub source_id: String,

    /// Target entity ID
    pub target_id: String,

    /// Properties associated with the relationship
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// Version DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VersionDto {
    /// Unique identifier for the version
    pub id: String,

    /// Description of the version
    pub description: String,

    /// When the version was created
    pub created_at: DateTime<Utc>,

    /// Metadata associated with the version
    pub metadata: serde_json::Value,

    /// HATEOAS links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<HateoasLinks>,
}

impl From<Version> for VersionDto {
    fn from(version: Version) -> Self {
        Self {
            id: version.id.clone(),
            description: version.description,
            created_at: version.created_at,
            metadata: version.metadata,
            links: Some(HateoasLinks::for_version(&version.id)),
        }
    }
}

/// Request to create a new version
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateVersionRequest {
    /// Description of the version
    pub description: String,

    /// Metadata associated with the version
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Request to checkout a version
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckoutVersionRequest {
    /// Whether to force checkout even if there are conflicts
    #[serde(default)]
    pub force: bool,
}

/// Memory graph DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryGraphDto {
    /// Central memory ID
    pub center_id: String,

    /// All memories in the graph
    pub memories: Vec<MemoryDto>,

    /// All relationships between memories
    pub relationships: Vec<RelationshipDto>,

    /// Graph metadata
    pub metadata: GraphMetadata,
}

impl From<MemoryGraph> for MemoryGraphDto {
    fn from(graph: MemoryGraph) -> Self {
        let memories: Vec<MemoryDto> = graph.memories.into_values().map(MemoryDto::from).collect();
        let relationships: Vec<RelationshipDto> = graph
            .relationships
            .into_iter()
            .map(RelationshipDto::from)
            .collect();

        Self {
            center_id: graph.center_id,
            memories,
            relationships,
            metadata: GraphMetadata::default(),
        }
    }
}

/// Memory path DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MemoryPathDto {
    /// Start memory ID
    pub from_id: String,

    /// End memory ID
    pub to_id: String,

    /// Ordered list of memories on the path
    pub memories: Vec<MemoryDto>,

    /// Ordered list of relationships on the path
    pub relationships: Vec<RelationshipDto>,

    /// Path length (number of relationships)
    pub length: usize,
}

impl From<MemoryPath> for MemoryPathDto {
    fn from(path: MemoryPath) -> Self {
        let memories: Vec<MemoryDto> = path.memories.into_iter().map(MemoryDto::from).collect();
        let relationships: Vec<RelationshipDto> = path
            .relationships
            .into_iter()
            .map(RelationshipDto::from)
            .collect();
        let length = relationships.len();

        Self {
            from_id: path.from_id,
            to_id: path.to_id,
            memories,
            relationships,
            length,
        }
    }
}

/// Search request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchRequest {
    /// Search query text
    pub query: String,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Search mode (semantic or keyword)
    #[serde(default)]
    pub mode: SearchMode,

    /// Similarity threshold for semantic search
    pub threshold: Option<f32>,

    /// Memory type filter
    pub memory_type: Option<String>,

    /// Tags filter
    pub tags: Option<Vec<String>>,

    /// Priority filter
    pub priority: Option<String>,
}

fn default_limit() -> usize {
    50
}

/// Search mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// BM25 full-text search (always available)
    Text,
    /// Vector similarity search (requires ML service configuration)
    Vector,
    /// Hybrid BM25 + vector search (requires ML service configuration)
    Hybrid,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Text
    }
}

/// Search result DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResultDto {
    /// The memory that matched the search
    pub memory: MemoryDto,

    /// Relevance score (for semantic search)
    pub score: Option<f32>,
}

impl From<SearchResult> for SearchResultDto {
    fn from(result: SearchResult) -> Self {
        Self {
            memory: MemoryDto::from(result.memory),
            score: result.score,
        }
    }
}

/// Graph query request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GraphQueryRequest {
    /// Graph pattern query
    pub pattern: String,

    /// Maximum number of results
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Graph metrics DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphMetricsDto {
    /// Total number of memories
    pub memory_count: usize,

    /// Total number of relationships
    pub relationship_count: usize,

    /// Average degree (connections per memory)
    pub average_degree: f64,

    /// Graph density
    pub density: f64,

    /// Number of connected components
    pub connected_components: usize,

    /// Most central memories
    pub central_memories: Vec<CentralMemoryDto>,
}

/// Central memory DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CentralMemoryDto {
    /// Memory ID
    pub memory_id: String,

    /// Centrality score
    pub centrality_score: f64,

    /// Memory content preview
    pub content_preview: String,
}

/// Pagination parameters
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginationParams {
    /// Page number (0-based)
    #[serde(default)]
    pub page: usize,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub size: usize,

    /// Sort field
    pub sort_by: Option<String>,

    /// Sort direction
    #[serde(default)]
    pub sort_direction: SortDirection,
}

fn default_page_size() -> usize {
    20
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl Default for SortDirection {
    fn default() -> Self {
        Self::Desc
    }
}

/// Error response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error type
    pub error: String,

    /// Error message
    pub message: String,

    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// HATEOAS links for resource discovery
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HateoasLinks {
    /// Link to self
    #[serde(rename = "self")]
    pub self_link: String,

    /// Related links
    #[serde(flatten)]
    pub related: std::collections::HashMap<String, String>,
}

impl HateoasLinks {
    /// Create HATEOAS links for a memory
    pub fn for_memory(id: &str) -> Self {
        let mut related = std::collections::HashMap::new();
        related.insert("graph".to_string(), format!("/api/memories/{}/graph", id));
        related.insert(
            "relationships".to_string(),
            format!("/api/memories/{}/relationships", id),
        );

        Self {
            self_link: format!("/api/memories/{}", id),
            related,
        }
    }

    /// Create HATEOAS links for an entity
    pub fn for_entity(id: &str) -> Self {
        let mut related = std::collections::HashMap::new();
        related.insert(
            "memories".to_string(),
            format!("/api/entities/{}/memories", id),
        );
        related.insert("graph".to_string(), format!("/api/entities/{}/graph", id));
        related.insert(
            "related_entities".to_string(),
            format!("/api/entities/{}/related_entities", id),
        );

        Self {
            self_link: format!("/api/entities/{}", id),
            related,
        }
    }

    /// Create HATEOAS links for a relationship
    pub fn for_relationship(id: &str) -> Self {
        Self {
            self_link: format!("/api/relationships/{}", id),
            related: std::collections::HashMap::new(),
        }
    }

    /// Create HATEOAS links for a version
    pub fn for_version(id: &str) -> Self {
        let mut related = std::collections::HashMap::new();
        related.insert(
            "checkout".to_string(),
            format!("/api/versions/{}/checkout", id),
        );

        Self {
            self_link: format!("/api/versions/{}", id),
            related,
        }
    }
}

/// Graph metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct GraphMetadata {
    /// Number of nodes in the graph
    pub node_count: usize,

    /// Number of edges in the graph
    pub edge_count: usize,

    /// Maximum depth traversed
    pub max_depth: u8,
}
