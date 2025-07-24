//! Generic relationship analysis and emotional state calculation

use super::types::*;
use crate::core::MemoryManager;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::sync::Arc;

/// Relationship analysis and sentiment detection
pub struct RelationshipAnalyzer {
    #[allow(dead_code)]
    memory_manager: Arc<MemoryManager>,
}

impl RelationshipAnalyzer {
    /// Create a new relationship analyzer
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            memory_manager,
        }
    }
    
    /// Determine relationship type based on metrics
    pub fn determine_relationship_type(&self, relationship: &Relationship) -> Result<RelationshipType> {
        match (relationship.intensity, relationship.trust_level, relationship.familiarity) {
            // Strong positive relationships
            (i, t, f) if i > 0.7 && t > 0.8 && f > 0.7 => Ok(RelationshipType::Friendship),
            (i, t, f) if i > 0.8 && t > 0.9 && f > 0.9 => Ok(RelationshipType::Romance),
            
            // Negative relationships
            (i, t, _) if i < -0.4 && t < 0.3 => Ok(RelationshipType::Antagonistic),
            (i, t, f) if i < -0.2 && t > 0.4 && f > 0.5 => Ok(RelationshipType::Rivalry),
            
            // Specialized relationships
            (i, t, _f) if i > 0.5 && t > 0.6 && (i - t).abs() > 0.3 => Ok(RelationshipType::Mentorship),
            (i, t, f) if f > 0.7 && i.abs() < 0.3 && t > 0.5 => Ok(RelationshipType::Professional),
            (i, t, _f) if i > 0.8 && t > 0.9 && self.has_family_markers(relationship) => Ok(RelationshipType::Family),
            (i, t, _) if i > 0.4 && t > 0.6 && i > 0.0 => Ok(RelationshipType::Alliance),
            (i, t, f) if i < 0.0 && t > 0.3 && f > 0.5 => Ok(RelationshipType::Competition),
            
            // Default to neutral
            _ => Ok(RelationshipType::Neutral),
        }
    }
    
    /// Analyze emotional state between entities
    pub fn analyze_emotional_state(&self, relationship: &Relationship) -> Result<EmotionalState> {
        let recent_trend = self.calculate_recent_trend(&relationship.history)?;
        let stability = self.calculate_stability(&relationship.history)?;
        let current_mood = self.determine_mood(relationship)?;
        
        Ok(EmotionalState {
            current_mood,
            emotional_intensity: relationship.intensity.abs(),
            stability_factor: stability,
            recent_trend,
        })
    }
    
    /// Determine current mood based on relationship state
    fn determine_mood(&self, relationship: &Relationship) -> Result<Mood> {
        // Check recent events first
        if let Some(recent_event) = relationship.history.last() {
            let time_since = Utc::now() - recent_event.timestamp;
            if time_since < Duration::hours(1) {
                return Ok(self.mood_from_recent_event(recent_event, relationship));
            }
        }
        
        // Fall back to general relationship state
        match (relationship.intensity, relationship.trust_level, &relationship.relationship_type) {
            (i, _, RelationshipType::Romance) if i > 0.5 => Ok(Mood::Romantic),
            (i, _, RelationshipType::Antagonistic) if i < -0.3 => Ok(Mood::Hostile),
            (i, t, _) if i > 0.6 && t > 0.7 => Ok(Mood::Friendly),
            (i, t, _) if i < -0.2 && t < 0.4 => Ok(Mood::Suspicious),
            (i, t, _) if i > 0.4 && t > 0.8 => Ok(Mood::Protective),
            (i, t, _) if i > 0.3 && t > 0.6 => Ok(Mood::Admiring),
            (i, t, _) if i < 0.0 && t > 0.5 => Ok(Mood::Disappointed),
            (i, _t, RelationshipType::Rivalry | RelationshipType::Competition) if i.abs() > 0.3 => Ok(Mood::Competitive),
            (i, t, RelationshipType::Professional) if i > 0.0 && t > 0.5 => Ok(Mood::Respectful),
            (i, t, _) if i < -0.3 && t < 0.3 => Ok(Mood::Dismissive),
            _ => Ok(Mood::Neutral),
        }
    }
    
    /// Determine mood from a recent event
    fn mood_from_recent_event(&self, event: &RelationshipEvent, relationship: &Relationship) -> Mood {
        match event.event_type {
            EventType::PositiveInteraction | EventType::Support => {
                if relationship.intensity > 0.5 { Mood::Grateful } else { Mood::Friendly }
            },
            EventType::NegativeInteraction => Mood::Hostile,
            EventType::Betrayal => Mood::Suspicious,
            EventType::Sacrifice => Mood::Grateful,
            EventType::Conflict => {
                if relationship.relationship_type == RelationshipType::Rivalry ||
                   relationship.relationship_type == RelationshipType::Competition {
                    Mood::Competitive
                } else {
                    Mood::Hostile
                }
            },
            EventType::Cooperation | EventType::Collaboration => Mood::Friendly,
            EventType::Achievement => Mood::Admiring,
            EventType::Custom(_) => self.determine_mood(relationship).unwrap_or(Mood::Neutral),
            _ => self.determine_mood(relationship).unwrap_or(Mood::Neutral),
        }
    }
    
    /// Calculate recent trend in relationship
    fn calculate_recent_trend(&self, history: &[RelationshipEvent]) -> Result<TrendDirection> {
        if history.len() < 2 {
            return Ok(TrendDirection::Stable);
        }
        
        // Look at last 5 events or events from last 24 hours
        let recent_cutoff = Utc::now() - Duration::hours(24);
        let recent_events: Vec<_> = history.iter()
            .rev()
            .take(5)
            .filter(|e| e.timestamp > recent_cutoff)
            .collect();
        
        if recent_events.len() < 2 {
            return Ok(TrendDirection::Stable);
        }
        
        // Calculate average changes
        let total_intensity_change: f32 = recent_events.iter()
            .map(|e| e.impact.intensity_change)
            .sum();
        
        let total_trust_change: f32 = recent_events.iter()
            .map(|e| e.impact.trust_change)
            .sum();
        
        // Check for volatility (large swings)
        let intensity_variance: f32 = recent_events.iter()
            .map(|e| e.impact.intensity_change)
            .map(|change| (change - (total_intensity_change / recent_events.len() as f32)).powi(2))
            .sum();
        
        if intensity_variance > 0.1 {
            Ok(TrendDirection::Volatile)
        } else if total_intensity_change > 0.1 || total_trust_change > 0.1 {
            Ok(TrendDirection::Improving)
        } else if total_intensity_change < -0.1 || total_trust_change < -0.1 {
            Ok(TrendDirection::Declining)
        } else {
            Ok(TrendDirection::Stable)
        }
    }
    
    /// Calculate relationship stability
    fn calculate_stability(&self, history: &[RelationshipEvent]) -> Result<f32> {
        if history.len() < 3 {
            return Ok(1.0); // New relationships are considered stable
        }
        
        // Calculate variance in intensity changes
        let intensity_changes: Vec<f32> = history.iter()
            .map(|e| e.impact.intensity_change)
            .collect();
        
        let mean_change = intensity_changes.iter().sum::<f32>() / intensity_changes.len() as f32;
        let variance = intensity_changes.iter()
            .map(|&change| (change - mean_change).powi(2))
            .sum::<f32>() / intensity_changes.len() as f32;
        
        // Convert variance to stability factor (lower variance = higher stability)
        let stability = 1.0 / (1.0 + variance * 10.0);
        Ok(stability.clamp(0.0, 1.0))
    }
    
    /// Determine interaction style based on relationship state
    pub fn determine_interaction_style(&self, relationship: &Relationship) -> Result<InteractionStyle> {
        match (relationship.intensity, relationship.trust_level, &relationship.relationship_type) {
            (i, t, _) if i > 0.6 && t > 0.7 => Ok(InteractionStyle::Warm),
            (i, t, _) if i < -0.3 && t < 0.4 => Ok(InteractionStyle::Cold),
            (i, t, RelationshipType::Professional) if i > 0.0 && t > 0.5 => Ok(InteractionStyle::Professional),
            (i, t, _) if i > 0.4 && t > 0.6 => Ok(InteractionStyle::Supportive),
            (i, t, _) if i < 0.0 && t < 0.5 => Ok(InteractionStyle::Cautious),
            (i, _t, RelationshipType::Rivalry | RelationshipType::Competition) if i.abs() > 0.3 => Ok(InteractionStyle::Aggressive),
            (i, t, _) if i > 0.5 && t > 0.8 => Ok(InteractionStyle::Protective),
            (i, _t, _) if i < -0.4 => Ok(InteractionStyle::Dismissive),
            (i, t, _) if i > 0.2 && t > 0.7 => Ok(InteractionStyle::Respectful),
            (i, t, RelationshipType::Romance) if i > 0.3 && t > 0.5 => Ok(InteractionStyle::Playful),
            _ => Ok(InteractionStyle::Professional),
        }
    }
    
    /// Check for family relationship markers
    fn has_family_markers(&self, relationship: &Relationship) -> bool {
        // Check metadata for family indicators
        if let Some(family_marker) = relationship.metadata.get("family_relation") {
            return family_marker.as_bool().unwrap_or(false);
        }
        
        // Check event history for family-related events
        relationship.history.iter().any(|event| {
            event.description.to_lowercase().contains("family") ||
            event.description.to_lowercase().contains("sibling") ||
            event.description.to_lowercase().contains("parent") ||
            event.description.to_lowercase().contains("child")
        })
    }
    

} 