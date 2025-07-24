//! Filter types for storage queries

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Filter for memory queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryFilter {
    /// Filter by memory IDs
    pub ids: Option<Vec<String>>,
    
    /// Filter by memory content (substring match)
    pub content: Option<String>,
    
    /// Filter by memory type
    pub memory_type: Option<String>,
    
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    
    /// Filter by source
    pub source: Option<String>,
    
    /// Filter by creation date range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    
    /// Filter by custom properties
    pub properties: Option<HashMap<String, serde_json::Value>>,
    
    /// Custom filter expression (backend-specific)
    pub custom_filter: Option<serde_json::Value>,
}

/// Filter for entity queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityFilter {
    /// Filter by entity IDs
    pub ids: Option<Vec<String>>,
    
    /// Filter by entity type
    pub entity_type: Option<String>,
    
    /// Filter by creation date range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    
    /// Filter by update date range
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    
    /// Filter by entity properties
    pub properties: Option<HashMap<String, serde_json::Value>>,
    
    /// Filter by related entity
    pub related_to: Option<String>,
    
    /// Filter by relationship type when using related_to
    pub related_by: Option<String>,
    
    /// Custom filter expression (backend-specific)
    pub custom_filter: Option<serde_json::Value>,
}

/// Filter for relationship queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelationshipFilter {
    /// Filter by relationship IDs
    pub ids: Option<Vec<String>>,
    
    /// Filter by relationship type
    pub relationship_type: Option<String>,
    
    /// Filter by source entity ID
    pub source_id: Option<String>,
    
    /// Filter by target entity ID
    pub target_id: Option<String>,
    
    /// Filter by creation date range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    
    /// Filter by update date range
    pub updated_after: Option<DateTime<Utc>>,
    pub updated_before: Option<DateTime<Utc>>,
    
    /// Filter by relationship properties
    pub properties: Option<HashMap<String, serde_json::Value>>,
    
    /// Custom filter expression (backend-specific)
    pub custom_filter: Option<serde_json::Value>,
}

/// Filter for vector queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorFilter {
    /// Filter by vector IDs
    pub ids: Option<Vec<String>>,
    
    /// Filter by source reference ID
    pub source_id: Option<String>,
    
    /// Filter by vector dimension
    pub dimension: Option<usize>,
    
    /// Filter by creation date range
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    
    /// Filter by metadata properties
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    
    /// Custom filter expression (backend-specific)
    pub custom_filter: Option<serde_json::Value>,
}

/// Filter for semantic search operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemanticSearchFilter {
    /// Standard memory filter to apply (e.g., by tags, type, date)
    /// This can be used for pre-filtering candidates before vector search 
    /// or post-filtering results.
    pub memory_filter: Option<MemoryFilter>,

    /// Minimum similarity threshold for results (e.g., 0.0 to 1.0)
    /// Only memories with a score above this threshold will be returned.
    pub similarity_threshold: Option<f32>,
    
    // TODO: Consider adding other specific filters relevant to semantic search,
    // e.g., filter by source of embedding, or specific model used for embedding.
}

/// Sort direction for query results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order
    Ascending,
    
    /// Descending order
    Descending,
}

/// Sort order for query results (alias of SortDirection for backwards compatibility)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortOrder {
    /// Ascending order
    Ascending,
    
    /// Descending order
    Descending,
}

impl From<SortDirection> for SortOrder {
    fn from(direction: SortDirection) -> Self {
        match direction {
            SortDirection::Ascending => SortOrder::Ascending,
            SortDirection::Descending => SortOrder::Descending,
        }
    }
}

impl From<SortOrder> for SortDirection {
    fn from(order: SortOrder) -> Self {
        match order {
            SortOrder::Ascending => SortDirection::Ascending,
            SortOrder::Descending => SortDirection::Descending,
        }
    }
}

/// Sort specification for query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    /// Field to sort by
    pub field: String,
    
    /// Sort direction
    pub direction: SortDirection,
}

/// Condition for filter comparisons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterCondition {
    /// Equal to
    Equals(serde_json::Value),
    
    /// Not equal to
    NotEquals(serde_json::Value),
    
    /// Greater than
    GreaterThan(serde_json::Value),
    
    /// Greater than or equal to
    GreaterThanOrEqual(serde_json::Value),
    
    /// Less than
    LessThan(serde_json::Value),
    
    /// Less than or equal to
    LessThanOrEqual(serde_json::Value),
    
    /// Contains (substring or element in array)
    Contains(serde_json::Value),
    
    /// Does not contain
    NotContains(serde_json::Value),
    
    /// Starts with (for strings)
    StartsWith(String),
    
    /// Ends with (for strings)
    EndsWith(String),
    
    /// Value is null
    IsNull,
    
    /// Value is not null
    IsNotNull,
    
    /// Value is in set
    In(Vec<serde_json::Value>),
    
    /// Value is not in set
    NotIn(Vec<serde_json::Value>),
}

/// Helper functions for constructing filters
pub mod helpers {
    use super::*;
    
    /// Create a basic entity filter by type
    pub fn entity_by_type(entity_type: &str) -> EntityFilter {
        EntityFilter {
            entity_type: Some(entity_type.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a basic relationship filter by type
    pub fn relationship_by_type(relationship_type: &str) -> RelationshipFilter {
        RelationshipFilter {
            relationship_type: Some(relationship_type.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a relationship filter between two entities
    pub fn relationship_between(source_id: &str, target_id: &str) -> RelationshipFilter {
        RelationshipFilter {
            source_id: Some(source_id.to_string()),
            target_id: Some(target_id.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a memory filter by type
    pub fn memory_by_type(memory_type: &str) -> MemoryFilter {
        MemoryFilter {
            memory_type: Some(memory_type.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a memory filter by tags
    pub fn memory_by_tags(tags: &[&str]) -> MemoryFilter {
        MemoryFilter {
            tags: Some(tags.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
    
    /// Create a memory filter by source
    pub fn memory_by_source(source: &str) -> MemoryFilter {
        MemoryFilter {
            source: Some(source.to_string()),
            ..Default::default()
        }
    }
} 