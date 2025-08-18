//! Generic relationship data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Generic relationship between two entities (agents, characters, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub entity_a: String,
    pub entity_b: String,
    pub relationship_type: RelationshipType,
    pub intensity: f32,   // -1.0 (hostile) to 1.0 (close)
    pub trust_level: f32, // 0.0 (no trust) to 1.0 (complete trust)
    pub familiarity: f32, // 0.0 (strangers) to 1.0 (very familiar)
    pub history: Vec<RelationshipEvent>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>, // Extensible for application-specific data
}

impl Relationship {
    /// Create a new neutral relationship between two entities
    pub fn new(entity_a: String, entity_b: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            entity_a,
            entity_b,
            relationship_type: RelationshipType::Neutral,
            intensity: 0.0,
            trust_level: 0.5,
            familiarity: 0.1,
            history: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Get the other entity in this relationship
    pub fn get_other_entity(&self, entity: &str) -> Option<&str> {
        if self.entity_a == entity {
            Some(&self.entity_b)
        } else if self.entity_b == entity {
            Some(&self.entity_a)
        } else {
            None
        }
    }

    /// Check if this relationship involves the given entity
    pub fn involves_entity(&self, entity: &str) -> bool {
        self.entity_a == entity || self.entity_b == entity
    }

    /// Add an event to the relationship history
    pub fn add_event(&mut self, event: RelationshipEvent) {
        self.history.push(event);
        self.last_updated = Utc::now();
    }

    /// Get recent events within the specified duration
    pub fn get_recent_events(&self, within_hours: i64) -> Vec<&RelationshipEvent> {
        let cutoff = Utc::now() - chrono::Duration::hours(within_hours);
        self.history
            .iter()
            .filter(|e| e.timestamp > cutoff)
            .collect()
    }
}

/// Types of relationships between entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    Friendship,
    Rivalry,
    Professional,
    Mentorship,
    Family,
    Romance,
    Antagonistic,
    Neutral,
    Alliance,
    Competition,
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipType::Friendship => write!(f, "Friendship"),
            RelationshipType::Rivalry => write!(f, "Rivalry"),
            RelationshipType::Professional => write!(f, "Professional"),
            RelationshipType::Mentorship => write!(f, "Mentorship"),
            RelationshipType::Family => write!(f, "Family"),
            RelationshipType::Romance => write!(f, "Romance"),
            RelationshipType::Antagonistic => write!(f, "Antagonistic"),
            RelationshipType::Neutral => write!(f, "Neutral"),
            RelationshipType::Alliance => write!(f, "Alliance"),
            RelationshipType::Competition => write!(f, "Competition"),
        }
    }
}

/// Events that affect relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEvent {
    pub id: String,
    pub event_type: EventType,
    pub description: String,
    pub impact: RelationshipImpact,
    pub timestamp: DateTime<Utc>,
    pub context: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl RelationshipEvent {
    /// Create a new relationship event
    pub fn new(
        event_type: EventType,
        description: String,
        impact: RelationshipImpact,
        context: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type,
            description,
            impact,
            timestamp: Utc::now(),
            context,
            metadata: HashMap::new(),
        }
    }
}

/// Types of events that can affect relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PositiveInteraction,
    NegativeInteraction,
    SharedExperience,
    Conflict,
    Cooperation,
    Betrayal,
    Sacrifice,
    Discovery,
    Support,
    Achievement,
    Disagreement,
    Collaboration,
    Custom(String), // Extensible for application-specific events
}

/// Impact of an event on a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipImpact {
    pub intensity_change: f32,
    pub trust_change: f32,
    pub familiarity_change: f32,
    pub relationship_type_shift: Option<RelationshipType>,
}

impl RelationshipImpact {
    /// Create a positive interaction impact
    pub fn positive_interaction(magnitude: f32) -> Self {
        Self {
            intensity_change: magnitude * 0.1,
            trust_change: magnitude * 0.05,
            familiarity_change: magnitude * 0.1,
            relationship_type_shift: None,
        }
    }

    /// Create a negative interaction impact
    pub fn negative_interaction(magnitude: f32) -> Self {
        Self {
            intensity_change: -magnitude * 0.15,
            trust_change: -magnitude * 0.1,
            familiarity_change: magnitude * 0.05, // Still become more familiar
            relationship_type_shift: None,
        }
    }

    /// Create a shared experience impact
    pub fn shared_experience(magnitude: f32) -> Self {
        Self {
            intensity_change: magnitude * 0.2,
            trust_change: magnitude * 0.1,
            familiarity_change: magnitude * 0.15,
            relationship_type_shift: None,
        }
    }

    /// Create a cooperation impact
    pub fn cooperation(magnitude: f32) -> Self {
        Self {
            intensity_change: magnitude * 0.15,
            trust_change: magnitude * 0.2,
            familiarity_change: magnitude * 0.1,
            relationship_type_shift: None,
        }
    }

    /// Create a conflict impact
    pub fn conflict(magnitude: f32) -> Self {
        Self {
            intensity_change: -magnitude * 0.2,
            trust_change: -magnitude * 0.25,
            familiarity_change: magnitude * 0.05,
            relationship_type_shift: None,
        }
    }

    /// Check if this impact has any relationship effect
    pub fn has_relationship_effect(&self) -> bool {
        self.intensity_change.abs() > 0.01
            || self.trust_change.abs() > 0.01
            || self.familiarity_change.abs() > 0.01
            || self.relationship_type_shift.is_some()
    }
}

/// Complete relationship context including emotional state and recent activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipContext {
    pub relationship: Relationship,
    pub recent_events: Vec<RelationshipEvent>,
    pub emotional_state: EmotionalState,
    pub interaction_style: InteractionStyle,
}

/// Emotional state of a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub current_mood: Mood,
    pub emotional_intensity: f32,
    pub stability_factor: f32,
    pub recent_trend: TrendDirection,
}

/// Current mood in a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Mood {
    Friendly,
    Hostile,
    Neutral,
    Romantic,
    Protective,
    Suspicious,
    Admiring,
    Disappointed,
    Grateful,
    Competitive,
    Respectful,
    Dismissive,
}

impl std::fmt::Display for Mood {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mood::Friendly => write!(f, "Friendly"),
            Mood::Hostile => write!(f, "Hostile"),
            Mood::Neutral => write!(f, "Neutral"),
            Mood::Romantic => write!(f, "Romantic"),
            Mood::Protective => write!(f, "Protective"),
            Mood::Suspicious => write!(f, "Suspicious"),
            Mood::Admiring => write!(f, "Admiring"),
            Mood::Disappointed => write!(f, "Disappointed"),
            Mood::Grateful => write!(f, "Grateful"),
            Mood::Competitive => write!(f, "Competitive"),
            Mood::Respectful => write!(f, "Respectful"),
            Mood::Dismissive => write!(f, "Dismissive"),
        }
    }
}

/// Direction of relationship trends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Declining,
    Stable,
    Volatile,
}

/// Style of interaction between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionStyle {
    Warm,
    Cold,
    Professional,
    Playful,
    Cautious,
    Aggressive,
    Supportive,
    Dismissive,
    Respectful,
    Protective,
}

impl std::fmt::Display for InteractionStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteractionStyle::Warm => write!(f, "Warm"),
            InteractionStyle::Cold => write!(f, "Cold"),
            InteractionStyle::Professional => write!(f, "Professional"),
            InteractionStyle::Playful => write!(f, "Playful"),
            InteractionStyle::Cautious => write!(f, "Cautious"),
            InteractionStyle::Aggressive => write!(f, "Aggressive"),
            InteractionStyle::Supportive => write!(f, "Supportive"),
            InteractionStyle::Dismissive => write!(f, "Dismissive"),
            InteractionStyle::Respectful => write!(f, "Respectful"),
            InteractionStyle::Protective => write!(f, "Protective"),
        }
    }
}
