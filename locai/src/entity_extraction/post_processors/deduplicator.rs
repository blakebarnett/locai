//! Entity deduplicator for removing duplicate entities.

use crate::entity_extraction::pipeline::{EntityPostProcessor, RawEntity};
use std::collections::HashSet;

/// Generic entity deduplicator that removes duplicate entities
#[derive(Debug, Clone)]
pub struct EntityDeduplicator {
    name: String,
    case_sensitive: bool,
}

impl EntityDeduplicator {
    /// Create a new entity deduplicator
    pub fn new() -> Self {
        Self {
            name: "entity_deduplicator".to_string(),
            case_sensitive: false,
        }
    }

    /// Create a case-sensitive deduplicator
    pub fn case_sensitive() -> Self {
        Self {
            name: "entity_deduplicator_case_sensitive".to_string(),
            case_sensitive: true,
        }
    }

    /// Create a case-insensitive deduplicator
    pub fn case_insensitive() -> Self {
        Self {
            name: "entity_deduplicator_case_insensitive".to_string(),
            case_sensitive: false,
        }
    }

    /// Get the normalized text for comparison
    fn normalize_text(&self, text: &str) -> String {
        if self.case_sensitive {
            text.trim().to_string()
        } else {
            text.trim().to_lowercase()
        }
    }

    /// Create a key for deduplication
    fn create_key(&self, entity: &RawEntity) -> String {
        format!(
            "{}:{:?}",
            self.normalize_text(&entity.text),
            entity.entity_type
        )
    }
}

impl Default for EntityDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityPostProcessor for EntityDeduplicator {
    fn process(&self, entities: Vec<RawEntity>) -> Vec<RawEntity> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();

        for entity in entities {
            let key = self.create_key(&entity);

            if !seen.contains(&key) {
                seen.insert(key);
                result.push(entity);
            }
        }

        result
    }

    fn name(&self) -> &str {
        &self.name
    }
}
