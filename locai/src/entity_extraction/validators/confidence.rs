//! Confidence-based entity validator.

use crate::entity_extraction::pipeline::{EntityValidator, RawEntity, ValidationContext};

/// Generic confidence-based validator
#[derive(Debug, Clone)]
pub struct ConfidenceValidator {
    threshold: f32,
    name: String,
}

impl ConfidenceValidator {
    /// Create a new confidence validator with the given threshold
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            name: format!("confidence_validator_{:.2}", threshold),
        }
    }

    /// Create a permissive validator (low threshold)
    pub fn permissive() -> Self {
        Self::new(0.3)
    }

    /// Create a balanced validator (medium threshold)
    pub fn balanced() -> Self {
        Self::new(0.5)
    }

    /// Create a strict validator (high threshold)
    pub fn strict() -> Self {
        Self::new(0.8)
    }
}

impl EntityValidator for ConfidenceValidator {
    fn validate(&self, entity: &RawEntity, _context: &ValidationContext) -> bool {
        entity.confidence >= self.threshold
    }

    fn name(&self) -> &str {
        &self.name
    }
}
