//! Automatic relationship creation between memories based on shared entities and other criteria
//! 
//! This module provides functionality to automatically create relationships between memories
//! that share entities, are temporally related, or have topic overlap.

use crate::models::Memory;
use crate::storage::{models::{Entity, Relationship}, traits::GraphStore};
use crate::{LocaiError, Result};
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Configuration for automatic relationship creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticRelationshipConfig {
    /// Whether automatic relationship creation is enabled
    pub enabled: bool,
    /// Methods to use for relationship creation
    pub methods: Vec<RelationshipMethod>,
    /// Minimum confidence threshold for creating relationships
    pub min_confidence: f32,
    /// Maximum number of relationships to create per memory
    pub max_relationships_per_memory: Option<usize>,
}

impl Default for AutomaticRelationshipConfig {
    fn default() -> Self {
        Self {
            enabled: true, // Enable by default for rich graph connections
            methods: vec![
                RelationshipMethod::EntityCoreference {
                    min_entity_confidence: 0.7, // Slightly lower for more connections
                },
                RelationshipMethod::TemporalProximity {
                    max_time_gap: Duration::minutes(30), // Longer time window
                    same_source_only: false, // Allow cross-source relationships
                },
                RelationshipMethod::TopicOverlap {
                    min_overlap_ratio: 0.3, // Add topic-based relationships
                },
            ],
            min_confidence: 0.6, // Lower threshold for more connections
            max_relationships_per_memory: Some(15), // Allow more relationships
        }
    }
}

/// Methods for automatic relationship creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipMethod {
    /// Same entities mentioned in different memories
    EntityCoreference {
        min_entity_confidence: f32,
    },
    /// Memories close in time from same source
    TemporalProximity {
        max_time_gap: Duration,
        same_source_only: bool,
    },
    /// Memories with overlapping tags/topics
    TopicOverlap {
        min_overlap_ratio: f32,
    },
}

/// Automatic relationship creator
#[derive(Debug, Clone)]
pub struct AutomaticRelationshipCreator {
    config: AutomaticRelationshipConfig,
}

impl AutomaticRelationshipCreator {
    /// Create a new automatic relationship creator
    pub fn new(config: AutomaticRelationshipConfig) -> Self {
        Self { config }
    }

    /// Find and create relationships for a newly stored memory
    pub async fn create_relationships_for_memory(
        &self,
        memory_id: &str,
        storage: &dyn GraphStore,
    ) -> Result<Vec<String>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }

        let mut created_relationships = Vec::new();
        let mut relationship_count = 0;

        for method in &self.config.methods {
            if let Some(max_rels) = self.config.max_relationships_per_memory {
                if relationship_count >= max_rels {
                    break;
                }
            }

            let relationships = match method {
                RelationshipMethod::EntityCoreference { min_entity_confidence } => {
                    self.find_entity_coreferences(memory_id, *min_entity_confidence, storage).await?
                }
                RelationshipMethod::TemporalProximity { max_time_gap, same_source_only } => {
                    self.find_temporal_relationships(memory_id, *max_time_gap, *same_source_only, storage).await?
                }
                RelationshipMethod::TopicOverlap { min_overlap_ratio } => {
                    self.find_topic_relationships(memory_id, *min_overlap_ratio, storage).await?
                }
            };

            for rel in relationships {
                if rel.confidence >= self.config.min_confidence {
                    if let Some(max_rels) = self.config.max_relationships_per_memory {
                        if relationship_count >= max_rels {
                            break;
                        }
                    }

                    let relationship = self.create_relationship_record(rel)?;
                    match storage.create_relationship(relationship).await {
                        Ok(created_rel) => {
                            created_relationships.push(created_rel.id);
                            relationship_count += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to create automatic relationship: {}", e);
                        }
                    }
                }
            }
        }

        Ok(created_relationships)
    }

    /// Find entity coreference relationships
    async fn find_entity_coreferences(
        &self,
        memory_id: &str,
        min_entity_confidence: f32,
        storage: &dyn GraphStore,
    ) -> Result<Vec<PotentialRelationship>> {
        let mut relationships = Vec::new();

        // Get the memory
        let memory = storage.get_memory(memory_id).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memory: {}", e)))?
            .ok_or_else(|| LocaiError::Memory(format!("Memory {} not found", memory_id)))?;

        // Find entities connected to this memory
        let entities = self.find_entities_for_memory(memory_id, storage).await?;

        for entity in entities {
            // Skip low-confidence entities
            let entity_confidence = entity.properties
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;

            if entity_confidence < min_entity_confidence {
                continue;
            }

            // Find other memories that mention this entity
            let related_memories = self.find_memories_mentioning_entity(&entity.id, storage).await?;

            for related_memory in related_memories {
                if related_memory.id != memory_id {
                    let confidence = self.calculate_entity_coreference_confidence(
                        &memory,
                        &related_memory,
                        &entity,
                    );

                    relationships.push(PotentialRelationship {
                        source_id: memory_id.to_string(),
                        target_id: related_memory.id,
                        relationship_type: "entity_coreference".to_string(),
                        confidence,
                        evidence: RelationshipEvidence {
                            description: format!("Both memories mention entity: {}", 
                                entity.properties.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")),
                            supporting_data: serde_json::json!({
                                "entity_id": entity.id,
                                "entity_type": entity.entity_type,
                                "entity_confidence": entity_confidence
                            }),
                        },
                        generation_method: GenerationMethod::EntityCoreference {
                            entity_id: entity.id.clone(),
                        },
                    });
                }
            }
        }

        Ok(relationships)
    }

    /// Find temporal proximity relationships
    async fn find_temporal_relationships(
        &self,
        memory_id: &str,
        max_time_gap: Duration,
        same_source_only: bool,
        storage: &dyn GraphStore,
    ) -> Result<Vec<PotentialRelationship>> {
        let mut relationships = Vec::new();

        // Get the memory
        let memory = storage.get_memory(memory_id).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memory: {}", e)))?
            .ok_or_else(|| LocaiError::Memory(format!("Memory {} not found", memory_id)))?;

        // Find memories within the time window
        let time_window_start = memory.created_at - max_time_gap;
        let time_window_end = memory.created_at + max_time_gap;

        let nearby_memories = self.find_memories_in_time_range(
            time_window_start,
            time_window_end,
            if same_source_only { Some(memory.source.as_str()) } else { None },
            storage,
        ).await?;

        for nearby_memory in nearby_memories {
            if nearby_memory.id != memory_id {
                let time_diff = (memory.created_at - nearby_memory.created_at).abs();
                let confidence = self.calculate_temporal_confidence(time_diff, max_time_gap);

                relationships.push(PotentialRelationship {
                    source_id: memory_id.to_string(),
                    target_id: nearby_memory.id,
                    relationship_type: "temporal_sequence".to_string(),
                    confidence,
                    evidence: RelationshipEvidence {
                        description: format!("Memories created within {} of each other", 
                            format_duration(time_diff)),
                        supporting_data: serde_json::json!({
                            "time_gap_seconds": time_diff.num_seconds(),
                            "max_gap_seconds": max_time_gap.num_seconds(),
                            "same_source": memory.source == nearby_memory.source
                        }),
                    },
                    generation_method: GenerationMethod::TemporalSequence {
                        time_gap: time_diff,
                    },
                });
            }
        }

        Ok(relationships)
    }

    /// Find topic overlap relationships
    async fn find_topic_relationships(
        &self,
        memory_id: &str,
        min_overlap_ratio: f32,
        storage: &dyn GraphStore,
    ) -> Result<Vec<PotentialRelationship>> {
        let mut relationships = Vec::new();

        // Get the memory
        let memory = storage.get_memory(memory_id).await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memory: {}", e)))?
            .ok_or_else(|| LocaiError::Memory(format!("Memory {} not found", memory_id)))?;

        if memory.tags.is_empty() {
            return Ok(relationships);
        }

        // Find memories with overlapping tags
        let memories_with_tags = self.find_memories_with_overlapping_tags(&memory.tags, storage).await?;

        for other_memory in memories_with_tags {
            if other_memory.id != memory_id {
                let overlap_ratio = self.calculate_tag_overlap_ratio(&memory.tags, &other_memory.tags);
                
                if overlap_ratio >= min_overlap_ratio {
                    let common_tags: Vec<_> = memory.tags.iter()
                        .filter(|tag| other_memory.tags.contains(tag))
                        .cloned()
                        .collect();

                    relationships.push(PotentialRelationship {
                        source_id: memory_id.to_string(),
                        target_id: other_memory.id,
                        relationship_type: "topic_similarity".to_string(),
                        confidence: overlap_ratio,
                        evidence: RelationshipEvidence {
                            description: format!("Memories share {} common tags", common_tags.len()),
                            supporting_data: serde_json::json!({
                                "common_tags": common_tags,
                                "overlap_ratio": overlap_ratio
                            }),
                        },
                        generation_method: GenerationMethod::TopicSimilarity {
                            common_tags,
                        },
                    });
                }
            }
        }

        Ok(relationships)
    }

    /// Find entities connected to a memory
    async fn find_entities_for_memory(
        &self,
        memory_id: &str,
        storage: &dyn GraphStore,
    ) -> Result<Vec<Entity>> {
        use crate::storage::filters::RelationshipFilter;

        // Find "contains" relationships where the memory is the source
        let filter = RelationshipFilter {
            source_id: Some(memory_id.to_string()),
            relationship_type: Some("contains".to_string()),
            ..Default::default()
        };

        let relationships = storage.list_relationships(Some(filter), None, None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find entity relationships: {}", e)))?;

        let mut entities = Vec::new();
        for rel in relationships {
            if let Ok(Some(entity)) = storage.get_entity(&rel.target_id).await {
                entities.push(entity);
            }
        }

        Ok(entities)
    }

    /// Find memories that mention a specific entity
    async fn find_memories_mentioning_entity(
        &self,
        entity_id: &str,
        storage: &dyn GraphStore,
    ) -> Result<Vec<Memory>> {
        use crate::storage::filters::RelationshipFilter;

        // Find "contains" relationships where the entity is the target
        let filter = RelationshipFilter {
            target_id: Some(entity_id.to_string()),
            relationship_type: Some("contains".to_string()),
            ..Default::default()
        };

        let relationships = storage.list_relationships(Some(filter), None, None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find memory relationships: {}", e)))?;

        let mut memories = Vec::new();
        for rel in relationships {
            if let Ok(Some(memory)) = storage.get_memory(&rel.source_id).await {
                memories.push(memory);
            }
        }

        Ok(memories)
    }

    /// Find memories within a time range
    async fn find_memories_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        source_filter: Option<&str>,
        storage: &dyn GraphStore,
    ) -> Result<Vec<Memory>> {
        use crate::storage::filters::MemoryFilter;

        let mut filter = MemoryFilter {
            created_after: Some(start),
            created_before: Some(end),
            ..Default::default()
        };

        if let Some(source) = source_filter {
            filter.source = Some(source.to_string());
        }

        storage.list_memories(Some(filter), Some(100), None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find memories in time range: {}", e)))
    }

    /// Find memories with overlapping tags
    async fn find_memories_with_overlapping_tags(
        &self,
        tags: &[String],
        storage: &dyn GraphStore,
    ) -> Result<Vec<Memory>> {
        use crate::storage::filters::MemoryFilter;

        let filter = MemoryFilter {
            tags: Some(tags.to_vec()),
            ..Default::default()
        };

        storage.list_memories(Some(filter), Some(100), None).await
            .map_err(|e| LocaiError::Storage(format!("Failed to find memories with tags: {}", e)))
    }

    /// Calculate confidence for entity coreference
    pub fn calculate_entity_coreference_confidence(
        &self,
        _memory1: &Memory,
        _memory2: &Memory,
        entity: &Entity,
    ) -> f32 {
        // Base confidence on entity confidence
        let entity_confidence = entity.properties
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        // TODO: Add more sophisticated confidence calculation
        // - Consider entity type (unique identifiers get higher confidence)
        // - Consider context similarity
        // - Consider temporal proximity

        entity_confidence * 0.9 // Slight reduction for automatic relationship
    }

    /// Calculate confidence for temporal relationships
    pub fn calculate_temporal_confidence(&self, time_diff: Duration, max_gap: Duration) -> f32 {
        let ratio = time_diff.num_seconds() as f32 / max_gap.num_seconds() as f32;
        (1.0 - ratio).max(0.0)
    }

    /// Calculate tag overlap ratio
    pub fn calculate_tag_overlap_ratio(&self, tags1: &[String], tags2: &[String]) -> f32 {
        if tags1.is_empty() || tags2.is_empty() {
            return 0.0;
        }

        let common_count = tags1.iter()
            .filter(|tag| tags2.contains(tag))
            .count();

        let total_unique = tags1.len() + tags2.len() - common_count;
        if total_unique == 0 {
            1.0
        } else {
            common_count as f32 / total_unique as f32
        }
    }

    /// Create a relationship record from a potential relationship
    fn create_relationship_record(&self, potential: PotentialRelationship) -> Result<Relationship> {
        let mut properties = serde_json::Map::new();
        properties.insert("auto_generated".to_string(), serde_json::Value::Bool(true));
        properties.insert("confidence".to_string(), 
            serde_json::Value::Number(serde_json::Number::from_f64(potential.confidence as f64).unwrap()));
        
        let generation_method_value = serde_json::to_value(&potential.generation_method)
            .map_err(|e| LocaiError::Other(format!("Failed to serialize generation method: {}", e)))?;
        properties.insert("generation_method".to_string(), generation_method_value);
        
        let evidence_value = serde_json::to_value(&potential.evidence)
            .map_err(|e| LocaiError::Other(format!("Failed to serialize evidence: {}", e)))?;
        properties.insert("evidence".to_string(), evidence_value);

        Ok(Relationship {
            id: Uuid::new_v4().to_string(),
            source_id: potential.source_id,
            target_id: potential.target_id,
            relationship_type: potential.relationship_type,
            properties: serde_json::Value::Object(properties),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
}

/// A potential relationship that could be created
#[derive(Debug, Clone)]
struct PotentialRelationship {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub evidence: RelationshipEvidence,
    pub generation_method: GenerationMethod,
}

/// Evidence supporting a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvidence {
    pub description: String,
    pub supporting_data: serde_json::Value,
}

/// Method used to generate a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationMethod {
    Manual,
    EntityCoreference { entity_id: String },
    TemporalSequence { time_gap: Duration },
    TopicSimilarity { common_tags: Vec<String> },
}

/// Format a duration for human readability
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds().abs();
    
    if total_seconds < 60 {
        format!("{} seconds", total_seconds)
    } else if total_seconds < 3600 {
        format!("{} minutes", total_seconds / 60)
    } else if total_seconds < 86400 {
        format!("{} hours", total_seconds / 3600)
    } else {
        format!("{} days", total_seconds / 86400)
    }
} 