//! Entity types and structures for entity extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of entities that can be extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// Person names (e.g., "John Smith", "Dr. Jane Doe")
    Person,
    /// Organizations (e.g., "Google", "Harvard University")
    Organization,
    /// Locations (e.g., "New York", "San Francisco Bay Area")
    Location,
    /// Dates (e.g., "January 15, 2024", "2023-01-01")
    Date,
    /// Times (e.g., "3:30 PM", "14:45")
    Time,
    /// Money amounts (e.g., "$100", "€50", "¥1000")
    Money,
    /// Email addresses (e.g., "user@example.com")
    Email,
    /// URLs (e.g., "https://example.com", "http://test.org")
    Url,
    /// Phone numbers (e.g., "+1-555-123-4567", "(555) 123-4567")
    PhoneNumber,
    /// Medical terms and entities
    Medical,
    /// Legal terms and entities
    Legal,
    /// Technical terms and entities
    Technical,
    /// Custom entity type with user-defined name
    Custom(String),
}

impl EntityType {
    /// Get a string representation of the entity type.
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Person => "person",
            EntityType::Organization => "organization",
            EntityType::Location => "location",
            EntityType::Date => "date",
            EntityType::Time => "time",
            EntityType::Money => "money",
            EntityType::Email => "email",
            EntityType::Url => "url",
            EntityType::PhoneNumber => "phone_number",
            EntityType::Medical => "medical",
            EntityType::Legal => "legal",
            EntityType::Technical => "technical",
            EntityType::Custom(name) => name,
        }
    }
}

/// An entity extracted from text content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// The text content of the entity
    pub text: String,
    /// The type of entity detected
    pub entity_type: EntityType,
    /// Starting position in the original text
    pub start_pos: usize,
    /// Ending position in the original text
    pub end_pos: usize,
    /// Confidence score (0.0 to 1.0) indicating how certain the extractor is
    pub confidence: f32,
    /// Source extractor that detected this entity (e.g., "regex", "spacy", "bert")
    pub extractor_source: String,
    /// Additional metadata about the entity
    pub metadata: HashMap<String, String>,
}

impl ExtractedEntity {
    /// Create a new extracted entity.
    pub fn new(
        text: String,
        entity_type: EntityType,
        start_pos: usize,
        end_pos: usize,
        confidence: f32,
        extractor_source: String,
    ) -> Self {
        Self {
            text,
            entity_type,
            start_pos,
            end_pos,
            confidence,
            extractor_source,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the entity.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the length of the entity text.
    pub fn len(&self) -> usize {
        self.end_pos - self.start_pos
    }

    /// Check if the entity text is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a formatted string representation of the entity.
    pub fn format(&self) -> String {
        format!(
            "{} [{}] (confidence: {:.2}, source: {})",
            self.text, 
            self.entity_type.as_str(), 
            self.confidence, 
            self.extractor_source
        )
    }
} 