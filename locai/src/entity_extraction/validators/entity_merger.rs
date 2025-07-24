//! Entity merger for handling overlapping entities.

use crate::entity_extraction::pipeline::{EntityPostProcessor, RawEntity};
use std::cmp;

/// Generic entity merger that handles overlapping entities
#[derive(Debug, Clone)]
pub struct EntityMerger {
    name: String,
}

impl EntityMerger {
    /// Create a new entity merger
    pub fn new() -> Self {
        Self {
            name: "entity_merger".to_string(),
        }
    }

    /// Check if two entities overlap
    fn overlaps(a: &RawEntity, b: &RawEntity) -> bool {
        a.start_pos < b.end_pos && a.end_pos > b.start_pos
    }

    /// Merge two overlapping entities, keeping the one with higher confidence
    fn merge_entities(a: RawEntity, b: RawEntity) -> RawEntity {
        if a.confidence >= b.confidence {
            // Keep entity A, but extend the span if B extends beyond it
            RawEntity {
                text: if b.end_pos > a.end_pos {
                    // Extend the text to include B's text
                    format!("{} {}", a.text.trim(), b.text.trim()).trim().to_string()
                } else {
                    a.text
                },
                entity_type: a.entity_type,
                start_pos: cmp::min(a.start_pos, b.start_pos),
                end_pos: cmp::max(a.end_pos, b.end_pos),
                confidence: a.confidence,
                metadata: a.metadata,
            }
        } else {
            // Keep entity B, but extend the span if A extends beyond it
            RawEntity {
                text: if a.end_pos > b.end_pos {
                    // Extend the text to include A's text
                    format!("{} {}", b.text.trim(), a.text.trim()).trim().to_string()
                } else {
                    b.text
                },
                entity_type: b.entity_type,
                start_pos: cmp::min(a.start_pos, b.start_pos),
                end_pos: cmp::max(a.end_pos, b.end_pos),
                confidence: b.confidence,
                metadata: b.metadata,
            }
        }
    }
}

impl Default for EntityMerger {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityPostProcessor for EntityMerger {
    fn process(&self, mut entities: Vec<RawEntity>) -> Vec<RawEntity> {
        if entities.len() <= 1 {
            return entities;
        }

        // Sort entities by start position
        entities.sort_by_key(|e| e.start_pos);

        let mut result = Vec::new();
        let mut entities_iter = entities.into_iter();
        let mut current_entity = entities_iter.next().unwrap();

        for next_entity in entities_iter {
            if Self::overlaps(&current_entity, &next_entity) {
                // Merge overlapping entities
                current_entity = Self::merge_entities(current_entity, next_entity);
            } else {
                // No overlap, add current entity to result and move to next
                result.push(current_entity);
                current_entity = next_entity;
            }
        }

        // Don't forget the last entity
        result.push(current_entity);

        result
    }

    fn name(&self) -> &str {
        &self.name
    }
} 