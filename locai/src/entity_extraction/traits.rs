//! Traits for entity extraction functionality.

use async_trait::async_trait;
use crate::Result;
use super::{ExtractedEntity, EntityType};

/// Trait for extracting entities from text content.
#[async_trait]
pub trait EntityExtractor: Send + Sync + std::fmt::Debug {
    /// Extract entities from the given text content.
    ///
    /// # Arguments
    /// * `content` - The text content to analyze for entities
    ///
    /// # Returns
    /// A vector of extracted entities with their positions and confidence scores
    async fn extract_entities(&self, content: &str) -> Result<Vec<ExtractedEntity>>;

    /// Get the entity types supported by this extractor.
    ///
    /// # Returns
    /// A vector of entity types that this extractor can detect
    fn supported_types(&self) -> Vec<EntityType>;

    /// Get the name of this extractor for identification purposes.
    ///
    /// # Returns
    /// A string identifying this extractor (e.g., "regex", "spacy", "bert")
    fn name(&self) -> &str;

    /// Get the priority of this extractor (higher priority extractors run first).
    ///
    /// # Returns
    /// Priority value (0-255, higher values indicate higher priority)
    fn priority(&self) -> u8 {
        128 // Default medium priority
    }

    /// Check if this extractor is enabled.
    ///
    /// # Returns
    /// True if the extractor should be used, false otherwise
    fn is_enabled(&self) -> bool {
        true
    }
} 