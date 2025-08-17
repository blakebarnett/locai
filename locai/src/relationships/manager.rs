//! Generic relationship management system

use super::analyzer::RelationshipAnalyzer;
use super::types::*;
use crate::core::MemoryManager;
use crate::models::MemoryType;
use anyhow::{Result, anyhow};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Callback function type for enriching relationship events with custom analysis
/// Parameters: (action, context, other_entity) -> custom_data
pub type RelationshipEnrichmentFn =
    Box<dyn Fn(&str, &str, &str) -> HashMap<String, serde_json::Value> + Send + Sync>;

/// Manages relationships between entities using the memory system
pub struct RelationshipManager {
    memory_manager: Arc<MemoryManager>,
    analyzer: Arc<RelationshipAnalyzer>,
    #[allow(dead_code)]
    entity_registry: HashMap<String, String>, // entity_name -> entity_id
    enrichment_callback: Option<RelationshipEnrichmentFn>,
}

impl RelationshipManager {
    /// Create a new relationship manager
    pub async fn new(memory_manager: Arc<MemoryManager>) -> Result<Self> {
        let analyzer = Arc::new(RelationshipAnalyzer::new(Arc::clone(&memory_manager)));

        let manager = Self {
            memory_manager,
            analyzer,
            entity_registry: HashMap::new(),
            enrichment_callback: None,
        };

        info!("🤝 Generic RelationshipManager initialized");
        Ok(manager)
    }

    /// Add a callback function for enriching relationship events with custom analysis
    pub fn with_enrichment_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, &str, &str) -> HashMap<String, serde_json::Value> + Send + Sync + 'static,
    {
        self.enrichment_callback = Some(Box::new(callback));
        self
    }

    /// Initialize a relationship between two entities
    pub async fn initialize_relationship(&self, entity_a: &str, entity_b: &str) -> Result<String> {
        // Check if relationship already exists
        if let Ok(existing) = self.get_relationship_from_memory(entity_a, entity_b).await {
            debug!(
                "Relationship between {} and {} already exists: {}",
                entity_a, entity_b, existing.id
            );
            return Ok(existing.id);
        }

        let relationship = Relationship::new(entity_a.to_string(), entity_b.to_string());

        // Store relationship in memory system
        let memory_id = self
            .memory_manager
            .add_memory_with_options(
                &format!(
                    "Relationship between {} and {}: {} (intensity: {:.2}, trust: {:.2})",
                    entity_a,
                    entity_b,
                    relationship.relationship_type,
                    relationship.intensity,
                    relationship.trust_level
                ),
                |builder| {
                    let mut properties = HashMap::new();
                    properties.insert(
                        "relationship_data",
                        serde_json::to_value(&relationship).unwrap_or_default(),
                    );
                    properties.insert(
                        "relationship_id",
                        serde_json::Value::String(relationship.id.clone()),
                    );
                    properties.insert("entity_a", serde_json::Value::String(entity_a.to_string()));
                    properties.insert("entity_b", serde_json::Value::String(entity_b.to_string()));

                    builder
                        .memory_type(MemoryType::Fact)
                        .source("relationship_manager")
                        .tags(vec![
                            "relationship",
                            entity_a,
                            entity_b,
                            &format!(
                                "type_{}",
                                relationship.relationship_type.to_string().to_lowercase()
                            ),
                        ])
                        .properties(properties)
                },
            )
            .await?;

        info!(
            "🤝 Initialized relationship between {} and {} (ID: {})",
            entity_a, entity_b, relationship.id
        );
        Ok(memory_id)
    }

    /// Update relationship based on an event
    pub async fn update_relationship(
        &self,
        entity_a: &str,
        entity_b: &str,
        event: RelationshipEvent,
    ) -> Result<()> {
        // Get existing relationship (initialize if doesn't exist)
        let mut relationship = match self.get_relationship_from_memory(entity_a, entity_b).await {
            Ok(rel) => rel,
            Err(_) => {
                self.initialize_relationship(entity_a, entity_b).await?;
                self.get_relationship_from_memory(entity_a, entity_b)
                    .await?
            }
        };

        let old_intensity = relationship.intensity;
        let old_trust = relationship.trust_level;

        // Apply event impact with bounds checking
        relationship.intensity =
            (relationship.intensity + event.impact.intensity_change).clamp(-1.0, 1.0);
        relationship.trust_level =
            (relationship.trust_level + event.impact.trust_change).clamp(0.0, 1.0);
        relationship.familiarity =
            (relationship.familiarity + event.impact.familiarity_change).clamp(0.0, 1.0);

        // Update relationship type based on new metrics
        if let Some(ref new_type) = event.impact.relationship_type_shift {
            relationship.relationship_type = new_type.clone();
        } else {
            // Let the analyzer determine if type should change
            if let Ok(suggested_type) = self.analyzer.determine_relationship_type(&relationship) {
                if suggested_type != relationship.relationship_type {
                    debug!(
                        "Relationship type evolved from {:?} to {:?} for {} and {}",
                        relationship.relationship_type, suggested_type, entity_a, entity_b
                    );
                    relationship.relationship_type = suggested_type;
                }
            }
        }

        // Add event to history
        relationship.add_event(event);

        // Store updated relationship
        self.store_relationship_update(&relationship).await?;

        info!(
            "🤝 Updated relationship between {} and {} (intensity: {:.2} → {:.2}, trust: {:.2} → {:.2})",
            entity_a,
            entity_b,
            old_intensity,
            relationship.intensity,
            old_trust,
            relationship.trust_level
        );

        Ok(())
    }

    /// Get relationship context including emotional state
    pub async fn get_relationship_context(
        &self,
        entity_a: &str,
        entity_b: &str,
    ) -> Result<RelationshipContext> {
        let relationship = self
            .get_relationship_from_memory(entity_a, entity_b)
            .await?;
        let recent_events = relationship
            .get_recent_events(24)
            .into_iter()
            .cloned()
            .collect();
        let emotional_state = self.analyzer.analyze_emotional_state(&relationship)?;
        let interaction_style = self.analyzer.determine_interaction_style(&relationship)?;

        Ok(RelationshipContext {
            relationship,
            recent_events,
            emotional_state,
            interaction_style,
        })
    }

    /// Get all relationships for a specific entity
    pub async fn get_entity_relationships(&self, entity: &str) -> Result<Vec<Relationship>> {
        let memories = self
            .memory_manager
            .search_memories(&format!("relationship {}", entity), None)
            .await?;

        let mut relationships = Vec::new();
        for memory in memories {
            if let Some(relationship_data) = memory.properties.get("relationship_data") {
                if let Ok(relationship) =
                    serde_json::from_value::<Relationship>(relationship_data.clone())
                {
                    if relationship.involves_entity(entity) {
                        relationships.push(relationship);
                    }
                }
            }
        }

        Ok(relationships)
    }

    /// Process an action that might affect relationships
    pub async fn process_entity_action(
        &self,
        entity: &str,
        action: &str,
        other_entities: &[String],
        context: &str,
    ) -> Result<()> {
        // Calculate basic action magnitude
        let magnitude = self.calculate_action_magnitude(action);

        // Create appropriate relationship events for affected entities
        for other_entity in other_entities {
            if entity != other_entity {
                // Get enrichment data from callback if available
                let enrichment_data = if let Some(ref callback) = self.enrichment_callback {
                    callback(action, context, other_entity)
                } else {
                    HashMap::new()
                };

                let event =
                    self.create_event_from_action(action, magnitude, context, enrichment_data);
                self.update_relationship(entity, other_entity, event)
                    .await?;
            }
        }

        Ok(())
    }

    /// Get a summary of the relationship between two entities
    pub async fn get_relationship_summary(&self, entity_a: &str, entity_b: &str) -> Result<String> {
        let context = self.get_relationship_context(entity_a, entity_b).await?;
        let rel = &context.relationship;

        let summary = format!(
            "{} and {} have a {} relationship (intensity: {:.2}, trust: {:.2}). \
             Current mood: {}, interaction style: {}. Recent trend: {:?}",
            entity_a,
            entity_b,
            rel.relationship_type,
            rel.intensity,
            rel.trust_level,
            context.emotional_state.current_mood,
            context.interaction_style,
            context.emotional_state.recent_trend
        );

        Ok(summary)
    }

    /// Private helper methods
    async fn get_relationship_from_memory(
        &self,
        entity_a: &str,
        entity_b: &str,
    ) -> Result<Relationship> {
        let search_query = format!("relationship {} {}", entity_a, entity_b);
        let memories = self
            .memory_manager
            .search_memories(&search_query, None)
            .await?;

        for memory in memories {
            if let Some(relationship_data) = memory.properties.get("relationship_data") {
                if let Ok(relationship) =
                    serde_json::from_value::<Relationship>(relationship_data.clone())
                {
                    if (relationship.entity_a == entity_a && relationship.entity_b == entity_b)
                        || (relationship.entity_a == entity_b && relationship.entity_b == entity_a)
                    {
                        return Ok(relationship);
                    }
                }
            }
        }

        Err(anyhow!(
            "Relationship not found between {} and {}",
            entity_a,
            entity_b
        ))
    }

    async fn store_relationship_update(&self, relationship: &Relationship) -> Result<()> {
        // Find and update existing memory
        let search_query = format!(
            "relationship {} {}",
            relationship.entity_a, relationship.entity_b
        );
        let memories = self
            .memory_manager
            .search_memories(&search_query, None)
            .await?;

        for memory in memories {
            if let Some(stored_id) = memory.properties.get("relationship_id") {
                if let Some(id_str) = stored_id.as_str() {
                    if id_str == relationship.id {
                        // Update the memory with new relationship data
                        let mut updated_memory = memory.clone();
                        if let serde_json::Value::Object(ref mut map) = updated_memory.properties {
                            map.insert(
                                "relationship_data".to_string(),
                                serde_json::to_value(relationship).unwrap_or_default(),
                            );
                        }

                        // Update memory content
                        updated_memory.content = format!(
                            "Relationship between {} and {}: {} (intensity: {:.2}, trust: {:.2})",
                            relationship.entity_a,
                            relationship.entity_b,
                            relationship.relationship_type,
                            relationship.intensity,
                            relationship.trust_level
                        );

                        self.memory_manager.update_memory(updated_memory).await?;
                        return Ok(());
                    }
                }
            }
        }

        warn!(
            "Could not find memory to update for relationship {}",
            relationship.id
        );
        Ok(())
    }

    fn calculate_action_magnitude(&self, action: &str) -> f32 {
        // Simple magnitude calculation based on action intensity keywords
        let action_lower = action.to_lowercase();

        if action_lower.contains("attack")
            || action_lower.contains("betray")
            || action_lower.contains("destroy")
        {
            0.9
        } else if action_lower.contains("help")
            || action_lower.contains("save")
            || action_lower.contains("support")
        {
            0.8
        } else if action_lower.contains("argue")
            || action_lower.contains("disagree")
            || action_lower.contains("conflict")
        {
            0.6
        } else if action_lower.contains("cooperate")
            || action_lower.contains("collaborate")
            || action_lower.contains("assist")
        {
            0.7
        } else if action_lower.contains("talk")
            || action_lower.contains("discuss")
            || action_lower.contains("chat")
        {
            0.3
        } else {
            0.4 // Default magnitude
        }
    }

    fn create_event_from_action(
        &self,
        action: &str,
        magnitude: f32,
        context: &str,
        enrichment_data: HashMap<String, serde_json::Value>,
    ) -> RelationshipEvent {
        // Determine event type based on enrichment data first, then fall back to keywords
        let event_type = if let Some(event_type_value) = enrichment_data.get("event_type") {
            // User provided event type
            if let Ok(event_type_str) = serde_json::from_value::<String>(event_type_value.clone()) {
                match event_type_str.as_str() {
                    "support" => EventType::Support,
                    "cooperation" => EventType::Cooperation,
                    "betrayal" => EventType::Betrayal,
                    "conflict" => EventType::Conflict,
                    "positive" => EventType::PositiveInteraction,
                    "negative" => EventType::NegativeInteraction,
                    _ => self.determine_event_type_from_keywords(action),
                }
            } else {
                self.determine_event_type_from_keywords(action)
            }
        } else {
            self.determine_event_type_from_keywords(action)
        };

        // Determine impact based on enrichment data or fall back to keywords
        let impact = if let Some(sentiment_value) = enrichment_data.get("sentiment") {
            // User provided sentiment analysis
            if let Ok(sentiment_str) = serde_json::from_value::<String>(sentiment_value.clone()) {
                match sentiment_str.as_str() {
                    "positive" => RelationshipImpact::positive_interaction(magnitude * 0.7),
                    "negative" => RelationshipImpact::negative_interaction(magnitude * 0.7),
                    _ => RelationshipImpact::shared_experience(magnitude * 0.5),
                }
            } else if let Ok(sentiment_score) =
                serde_json::from_value::<f32>(sentiment_value.clone())
            {
                // Numeric sentiment score
                if sentiment_score > 0.0 {
                    RelationshipImpact::positive_interaction(magnitude * sentiment_score.abs())
                } else if sentiment_score < 0.0 {
                    RelationshipImpact::negative_interaction(magnitude * sentiment_score.abs())
                } else {
                    RelationshipImpact::shared_experience(magnitude * 0.5)
                }
            } else {
                self.determine_impact_from_keywords(action, magnitude)
            }
        } else {
            self.determine_impact_from_keywords(action, magnitude)
        };

        // Create event with enrichment data as metadata
        let mut event =
            RelationshipEvent::new(event_type, action.to_string(), impact, context.to_string());

        // Add enrichment data to event metadata
        for (key, value) in enrichment_data {
            event.metadata.insert(key, value);
        }

        event
    }

    fn determine_event_type_from_keywords(&self, action: &str) -> EventType {
        let action_lower = action.to_lowercase();
        if action_lower.contains("help") || action_lower.contains("support") {
            EventType::Support
        } else if action_lower.contains("cooperate") || action_lower.contains("collaborate") {
            EventType::Cooperation
        } else if action_lower.contains("betray") {
            EventType::Betrayal
        } else if action_lower.contains("attack") || action_lower.contains("fight") {
            EventType::Conflict
        } else if action_lower.contains("love")
            || action_lower.contains("praise")
            || action_lower.contains("thank")
        {
            EventType::PositiveInteraction
        } else if action_lower.contains("insult")
            || action_lower.contains("mock")
            || action_lower.contains("hurt")
        {
            EventType::NegativeInteraction
        } else {
            EventType::SharedExperience
        }
    }

    fn determine_impact_from_keywords(&self, action: &str, magnitude: f32) -> RelationshipImpact {
        let action_lower = action.to_lowercase();
        if action_lower.contains("help")
            || action_lower.contains("support")
            || action_lower.contains("love")
            || action_lower.contains("praise")
        {
            RelationshipImpact::positive_interaction(magnitude * 0.7)
        } else if action_lower.contains("attack")
            || action_lower.contains("betray")
            || action_lower.contains("insult")
            || action_lower.contains("hurt")
        {
            RelationshipImpact::negative_interaction(magnitude * 0.7)
        } else {
            RelationshipImpact::shared_experience(magnitude * 0.5)
        }
    }
}
