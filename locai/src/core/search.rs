//! Search functionality for Locai
//!
//! This module provides unified search capabilities across memories, entities, and graphs.

use crate::models::{Memory, MemoryType};
use crate::storage::models::{Entity, MemoryGraph};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// All search results are returned in this unified format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique identifier for the result
    pub id: String,

    /// Type of the result
    pub result_type: SearchResultType,

    /// The actual content/data
    pub content: SearchContent,

    /// Relevance score (0.0 to 1.0)
    pub score: f32,

    /// Why this result matched
    pub match_info: MatchInfo,

    /// Related entities and memories
    pub context: SearchContext,

    /// Metadata about the result
    pub metadata: SearchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchResultType {
    Memory,
    Entity,
    Graph,
    Relationship,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchContent {
    Memory(Memory),
    Entity(Entity),
    Graph(MemoryGraph),
    Relationship(RelationshipInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipInfo {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub relationship_type: String,
    pub properties: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInfo {
    /// Primary reason for match
    pub reason: String,

    /// Detailed match explanations
    pub details: Vec<String>,

    /// Highlighted snippets
    pub highlights: Vec<Highlight>,

    /// Match path (for graph results)
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    /// The highlighted text
    pub text: String,

    /// Start position in the original text
    pub start: usize,

    /// End position in the original text
    pub end: usize,

    /// Type of highlight (e.g., "exact_match", "semantic_match")
    pub highlight_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    /// Related entities
    pub entities: Vec<EntityRef>,

    /// Related memories
    pub memories: Vec<MemoryRef>,

    /// Relevant relationships
    pub relationships: Vec<RelationshipRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRef {
    pub id: String,
    pub memory_type: MemoryType,
    pub summary: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRef {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub relationship_type: String,
    pub relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// When this was created
    pub created_at: DateTime<Utc>,

    /// Last accessed/updated
    pub last_accessed: Option<DateTime<Utc>>,

    /// Source of the data
    pub source: String,

    /// Any tags
    pub tags: Vec<String>,

    /// Custom properties
    pub properties: serde_json::Value,
}

/// Search options for customizing search behavior
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,

    /// Search strategy
    pub strategy: SearchStrategy,

    /// Result types to include
    pub include_types: SearchTypeFilter,

    /// Time range filter
    pub time_range: Option<TimeRange>,

    /// Minimum relevance score
    pub min_score: Option<f32>,

    /// Expand results with context
    pub include_context: bool,

    /// Graph traversal depth
    pub graph_depth: u8,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 20,
            strategy: SearchStrategy::Auto,
            include_types: SearchTypeFilter::all(),
            time_range: None,
            min_score: None,
            include_context: true,
            graph_depth: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SearchStrategy {
    /// Automatically determine best strategy
    Auto,
    /// Prefer semantic/embedding search
    Semantic,
    /// Prefer keyword/text search
    Keyword,
    /// Prefer graph traversal
    Graph,
    /// Use all strategies and merge
    Hybrid,
}

#[derive(Debug, Clone)]
pub struct SearchTypeFilter {
    pub memories: bool,
    pub entities: bool,
    pub graphs: bool,
    pub relationships: bool,
}

impl SearchTypeFilter {
    pub fn all() -> Self {
        Self {
            memories: true,
            entities: true,
            graphs: true,
            relationships: true,
        }
    }

    pub fn memories_only() -> Self {
        Self {
            memories: true,
            entities: false,
            graphs: false,
            relationships: false,
        }
    }

    pub fn entities_only() -> Self {
        Self {
            memories: false,
            entities: true,
            graphs: false,
            relationships: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl SearchResult {
    /// Get a human-readable summary of the result
    pub fn summary(&self) -> String {
        match &self.content {
            SearchContent::Memory(memory) => {
                format!(
                    "Memory: {}",
                    memory.content.chars().take(100).collect::<String>()
                )
            }
            SearchContent::Entity(entity) => {
                let entity_name = entity
                    .properties
                    .get("name")
                    .or_else(|| entity.properties.get("text"))
                    .or_else(|| entity.properties.get("value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(&entity.id);
                format!("Entity: {} [{}]", entity_name, entity.entity_type)
            }
            SearchContent::Graph(graph) => {
                format!(
                    "Graph: {} memories, {} relationships",
                    graph.memories.len(),
                    graph.relationships.len()
                )
            }
            SearchContent::Relationship(rel) => {
                format!(
                    "Relationship: {} -> {} [{}]",
                    rel.from_id, rel.to_id, rel.relationship_type
                )
            }
        }
    }

    /// Get the primary match reason
    pub fn match_reason(&self) -> &str {
        &self.match_info.reason
    }

    /// Convert from UniversalSearchResult to SearchResult
    pub fn from_universal(result: crate::memory::search_extensions::UniversalSearchResult) -> Self {
        match result {
            crate::memory::search_extensions::UniversalSearchResult::Memory {
                memory,
                score,
                match_reason,
            } => {
                SearchResult {
                    id: memory.id.clone(),
                    result_type: SearchResultType::Memory,
                    content: SearchContent::Memory(memory.clone()),
                    score: score.unwrap_or(0.0).min(1.0), // Normalize BM25 scores to 0-1 range
                    match_info: MatchInfo {
                        reason: match_reason,
                        details: vec![],
                        highlights: vec![],
                        path: None,
                    },
                    context: SearchContext {
                        entities: vec![],
                        memories: vec![],
                        relationships: vec![],
                    },
                    metadata: SearchMetadata {
                        created_at: memory.created_at,
                        last_accessed: memory.last_accessed,
                        source: memory.source.clone(),
                        tags: memory.tags,
                        properties: memory.properties,
                    },
                }
            }
            crate::memory::search_extensions::UniversalSearchResult::Entity {
                entity,
                score,
                match_reason,
                related_memories,
            } => {
                SearchResult {
                    id: entity.id.clone(),
                    result_type: SearchResultType::Entity,
                    content: SearchContent::Entity(entity.clone()),
                    score: score.unwrap_or(0.0).min(1.0), // Normalize entity scores to 0-1 range
                    match_info: MatchInfo {
                        reason: match_reason,
                        details: vec![format!("Entity type: {}", entity.entity_type)],
                        highlights: vec![],
                        path: None,
                    },
                    context: SearchContext {
                        entities: vec![],
                        memories: related_memories
                            .into_iter()
                            .map(|id| MemoryRef {
                                id,
                                memory_type: MemoryType::Episodic, // Default, would need to fetch actual type
                                summary: "Related memory".to_string(),
                                relevance_score: 0.5,
                            })
                            .collect(),
                        relationships: vec![],
                    },
                    metadata: SearchMetadata {
                        created_at: entity.created_at,
                        last_accessed: None,
                        source: "entity_extraction".to_string(),
                        tags: vec![entity.entity_type.clone()],
                        properties: entity.properties.clone(),
                    },
                }
            }
            crate::memory::search_extensions::UniversalSearchResult::Graph {
                center_id,
                center_type,
                graph,
                score,
                match_reason,
            } => {
                // Extract entities and relationships from the graph for context
                let context_entities = vec![];
                let mut context_memories = vec![];
                let mut context_relationships = vec![];

                // Add memories from the graph as context
                for (memory_id, memory) in &graph.memories {
                    if memory_id != &center_id {
                        // Don't include the center as context
                        context_memories.push(MemoryRef {
                            id: memory.id.clone(),
                            memory_type: memory.memory_type.clone(),
                            summary: memory.content.chars().take(100).collect::<String>(),
                            relevance_score: 0.7,
                        });
                    }
                }

                // Add relationships from the graph as context
                for relationship in &graph.relationships {
                    context_relationships.push(RelationshipRef {
                        id: relationship.id.clone(),
                        from_id: relationship.source_id.clone(),
                        to_id: relationship.target_id.clone(),
                        relationship_type: relationship.relationship_type.clone(),
                        relevance_score: 1.0, // Default strength
                    });
                }

                SearchResult {
                    id: center_id.clone(),
                    result_type: SearchResultType::Graph,
                    content: SearchContent::Graph(graph),
                    score: score.unwrap_or(0.0).min(1.0), // Normalize graph scores to 0-1 range
                    match_info: MatchInfo {
                        reason: match_reason,
                        details: vec![format!("Graph centered on {} {}", center_type, center_id)],
                        highlights: vec![],
                        path: None,
                    },
                    context: SearchContext {
                        entities: context_entities,
                        memories: context_memories,
                        relationships: context_relationships,
                    },
                    metadata: SearchMetadata {
                        created_at: Utc::now(),
                        last_accessed: None,
                        source: "graph_search".to_string(),
                        tags: vec!["graph".to_string(), center_type],
                        properties: serde_json::Value::Object(serde_json::Map::new()),
                    },
                }
            }
        }
    }
}
