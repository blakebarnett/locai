//! Group dynamics and network analysis for relationships

use super::types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Analyzes group dynamics from a set of relationships
pub struct GroupDynamicsAnalyzer;

impl GroupDynamicsAnalyzer {
    /// Analyze group dynamics from a collection of relationships
    pub fn analyze_group_dynamics(
        relationships: &[Relationship],
        entities: &[String],
    ) -> Result<GroupDynamics> {
        let alliances = Self::detect_alliance_patterns(relationships, entities)?;
        let conflicts = Self::identify_conflicts(relationships)?;
        let influence_network = Self::build_influence_network(relationships)?;
        let group_cohesion = Self::calculate_group_cohesion(relationships)?;

        Ok(GroupDynamics {
            alliances,
            conflicts,
            influence_network,
            group_cohesion,
        })
    }

    /// Detect alliance patterns in the group
    fn detect_alliance_patterns(
        relationships: &[Relationship],
        entities: &[String],
    ) -> Result<Vec<AlliancePattern>> {
        let mut alliances = Vec::new();
        let mut processed_entities = std::collections::HashSet::new();

        for entity in entities {
            if processed_entities.contains(entity) {
                continue;
            }

            // Find strong positive relationships for this entity
            let allies = Self::get_strong_allies(entity, relationships)?;

            if !allies.is_empty() {
                // At least one ally to form an alliance
                let alliance_strength =
                    Self::calculate_alliance_strength(entity, &allies, relationships)?;
                let alliance_type = Self::determine_alliance_type(entity, &allies, relationships)?;

                alliances.push(AlliancePattern {
                    leader: entity.clone(),
                    members: allies.clone(),
                    strength: alliance_strength,
                    alliance_type,
                });

                // Mark all members as processed
                processed_entities.insert(entity.clone());
                for ally in &allies {
                    processed_entities.insert(ally.clone());
                }
            }
        }

        Ok(alliances)
    }

    /// Get entities with strong positive relationships to the given entity
    fn get_strong_allies(entity: &str, relationships: &[Relationship]) -> Result<Vec<String>> {
        let mut allies = Vec::new();

        for relationship in relationships {
            if relationship.involves_entity(entity)
                && relationship.intensity > 0.5
                && relationship.trust_level > 0.6
                && let Some(other) = relationship.get_other_entity(entity)
            {
                allies.push(other.to_string());
            }
        }

        Ok(allies)
    }

    /// Calculate alliance strength based on relationship metrics
    fn calculate_alliance_strength(
        leader: &str,
        members: &[String],
        relationships: &[Relationship],
    ) -> Result<f32> {
        if members.is_empty() {
            return Ok(0.0);
        }

        let mut total_strength = 0.0;
        let mut relationship_count = 0;

        // Calculate average relationship strength within the alliance
        for member in members {
            if let Some(rel) = Self::find_relationship(leader, member, relationships) {
                total_strength += (rel.intensity + rel.trust_level) / 2.0;
                relationship_count += 1;
            }
        }

        // Also consider inter-member relationships
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                if let Some(rel) = Self::find_relationship(&members[i], &members[j], relationships)
                {
                    total_strength += (rel.intensity + rel.trust_level) / 2.0;
                    relationship_count += 1;
                }
            }
        }

        if relationship_count > 0 {
            Ok(total_strength / relationship_count as f32)
        } else {
            Ok(0.0)
        }
    }

    /// Determine the type of alliance based on relationship characteristics
    fn determine_alliance_type(
        leader: &str,
        members: &[String],
        relationships: &[Relationship],
    ) -> Result<AllianceType> {
        let mut romance_count = 0;
        let mut family_count = 0;
        let mut professional_count = 0;
        let mut friendship_count = 0;

        // Analyze relationship types within the alliance
        for member in members {
            if let Some(rel) = Self::find_relationship(leader, member, relationships) {
                match rel.relationship_type {
                    RelationshipType::Romance => romance_count += 1,
                    RelationshipType::Family => family_count += 1,
                    RelationshipType::Professional => professional_count += 1,
                    RelationshipType::Friendship => friendship_count += 1,
                    _ => {}
                }
            }
        }

        // Determine dominant alliance type
        if romance_count > 0 {
            Ok(AllianceType::Romance)
        } else if family_count > friendship_count && family_count > professional_count {
            Ok(AllianceType::Family)
        } else if professional_count > friendship_count {
            Ok(AllianceType::Professional)
        } else if friendship_count > 0 {
            Ok(AllianceType::Friendship)
        } else {
            Ok(AllianceType::Convenience)
        }
    }

    /// Identify conflict zones in the group
    fn identify_conflicts(relationships: &[Relationship]) -> Result<Vec<ConflictZone>> {
        let mut conflicts = Vec::new();

        for relationship in relationships {
            if relationship.intensity < -0.3
                || (relationship.intensity < 0.0 && relationship.trust_level < 0.4)
            {
                let conflict_intensity =
                    (-relationship.intensity).max(1.0 - relationship.trust_level);
                let source = Self::identify_conflict_source(relationship)?;
                let conflict_type = Self::determine_conflict_type(relationship)?;

                conflicts.push(ConflictZone {
                    characters: vec![relationship.entity_a.clone(), relationship.entity_b.clone()],
                    intensity: conflict_intensity,
                    source,
                    conflict_type,
                });
            }
        }

        Ok(conflicts)
    }

    /// Identify the source of conflict from relationship history
    fn identify_conflict_source(relationship: &Relationship) -> Result<String> {
        // Look for recent negative events
        for event in relationship.history.iter().rev().take(5) {
            match event.event_type {
                EventType::Betrayal => return Ok("Betrayal".to_string()),
                EventType::Conflict => return Ok("Direct conflict".to_string()),
                EventType::NegativeInteraction => return Ok("Negative interaction".to_string()),
                _ => {}
            }
        }

        Ok("Unknown conflict source".to_string())
    }

    /// Determine the type of conflict
    fn determine_conflict_type(relationship: &Relationship) -> Result<ConflictType> {
        // Check metadata for conflict type hints
        if let Some(conflict_type) = relationship.metadata.get("conflict_type")
            && let Some(type_str) = conflict_type.as_str()
        {
            return match type_str.to_lowercase().as_str() {
                "romantic" => Ok(ConflictType::Romantic),
                "resource" => Ok(ConflictType::Resource),
                "ideological" => Ok(ConflictType::Ideological),
                "professional" => Ok(ConflictType::Professional),
                _ => Ok(ConflictType::Personal),
            };
        }

        // Infer from relationship type and history
        match relationship.relationship_type {
            RelationshipType::Professional => Ok(ConflictType::Professional),
            RelationshipType::Romance => Ok(ConflictType::Romantic),
            RelationshipType::Rivalry | RelationshipType::Competition => Ok(ConflictType::Personal),
            _ => Ok(ConflictType::Personal),
        }
    }

    /// Build influence network showing relative influence between entities
    fn build_influence_network(relationships: &[Relationship]) -> Result<InfluenceNetwork> {
        let mut influence_scores = HashMap::new();
        let mut connections = HashMap::new();

        for relationship in relationships {
            let entity_a = &relationship.entity_a;
            let entity_b = &relationship.entity_b;

            // Calculate influence based on relationship metrics
            let a_influence_on_b = Self::calculate_influence_score(relationship, entity_a);
            let b_influence_on_a = Self::calculate_influence_score(relationship, entity_b);

            // Update influence scores
            *influence_scores.entry(entity_a.clone()).or_insert(0.0) += a_influence_on_b;
            *influence_scores.entry(entity_b.clone()).or_insert(0.0) += b_influence_on_a;

            // Track connections
            connections
                .entry(entity_a.clone())
                .or_insert_with(Vec::new)
                .push((entity_b.clone(), a_influence_on_b));
            connections
                .entry(entity_b.clone())
                .or_insert_with(Vec::new)
                .push((entity_a.clone(), b_influence_on_a));
        }

        Ok(InfluenceNetwork {
            influence_scores,
            connections,
        })
    }

    /// Calculate how much influence one entity has over another in a relationship
    fn calculate_influence_score(relationship: &Relationship, _entity: &str) -> f32 {
        // Influence is based on trust level and relationship intensity
        let base_influence = relationship.trust_level * 0.7 + relationship.intensity.abs() * 0.3;

        // Adjust based on relationship type
        let type_multiplier = match relationship.relationship_type {
            RelationshipType::Mentorship => {
                // If this entity is likely the mentor (higher trust), increase influence
                if relationship.trust_level > 0.8 {
                    1.5
                } else {
                    0.8
                }
            }
            RelationshipType::Family => 1.2,
            RelationshipType::Friendship => 1.0,
            RelationshipType::Professional => 0.9,
            RelationshipType::Romance => 1.1,
            _ => 1.0,
        };

        base_influence * type_multiplier
    }

    /// Calculate overall group cohesion
    fn calculate_group_cohesion(relationships: &[Relationship]) -> Result<f32> {
        if relationships.is_empty() {
            return Ok(1.0);
        }

        let total_positive_intensity: f32 =
            relationships.iter().map(|r| r.intensity.max(0.0)).sum();

        let total_negative_intensity: f32 =
            relationships.iter().map(|r| (-r.intensity).max(0.0)).sum();

        let average_trust: f32 =
            relationships.iter().map(|r| r.trust_level).sum::<f32>() / relationships.len() as f32;

        // Cohesion is high when there's more positive than negative intensity
        // and overall trust is high
        let intensity_ratio = if total_negative_intensity > 0.0 {
            total_positive_intensity / (total_positive_intensity + total_negative_intensity)
        } else {
            1.0
        };

        let cohesion = (intensity_ratio * 0.6 + average_trust * 0.4).clamp(0.0, 1.0);
        Ok(cohesion)
    }

    /// Helper function to find a relationship between two entities
    fn find_relationship<'a>(
        entity_a: &str,
        entity_b: &str,
        relationships: &'a [Relationship],
    ) -> Option<&'a Relationship> {
        relationships.iter().find(|r| {
            (r.entity_a == entity_a && r.entity_b == entity_b)
                || (r.entity_a == entity_b && r.entity_b == entity_a)
        })
    }
}

/// Group dynamics analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDynamics {
    pub alliances: Vec<AlliancePattern>,
    pub conflicts: Vec<ConflictZone>,
    pub influence_network: InfluenceNetwork,
    pub group_cohesion: f32,
}

/// Alliance pattern within a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlliancePattern {
    pub leader: String,
    pub members: Vec<String>,
    pub strength: f32,
    pub alliance_type: AllianceType,
}

/// Type of alliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllianceType {
    Friendship,
    Professional,
    Family,
    Convenience,
    Romance,
}

/// Conflict zone between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictZone {
    pub characters: Vec<String>,
    pub intensity: f32,
    pub source: String,
    pub conflict_type: ConflictType,
}

/// Type of conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    Personal,
    Professional,
    Ideological,
    Resource,
    Romantic,
}

/// Influence network showing connections and influence scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceNetwork {
    pub influence_scores: HashMap<String, f32>,
    pub connections: HashMap<String, Vec<(String, f32)>>,
}
