//! Basic pattern-based entity extractor using regular expressions.

use async_trait::async_trait;
use regex::Regex;

use lazy_static::lazy_static;
use crate::Result;
use super::{EntityExtractor, ExtractedEntity, EntityType};

/// Basic pattern-based entity extractor using regular expressions.
#[derive(Debug)]
pub struct BasicEntityExtractor {
    /// Name of this extractor
    name: String,
    /// Supported entity types
    supported_types: Vec<EntityType>,
    /// Confidence threshold
    confidence_threshold: f32,
}

impl BasicEntityExtractor {
    /// Create a new basic entity extractor.
    /// Focuses on structured data patterns, not named entities (use ML extractor for those).
    pub fn new() -> Self {
        Self {
            name: "basic".to_string(),
            supported_types: vec![
                // Only handle structured data - let ML extractor handle named entities
                EntityType::Email,
                EntityType::Url,
                EntityType::PhoneNumber,
                EntityType::Date,
                EntityType::Time,
                EntityType::Money,
            ],
            confidence_threshold: 0.7,
        }
    }

    /// Create a new basic entity extractor with custom configuration.
    /// Focuses on structured data patterns, not named entities (use ML extractor for those).
    pub fn with_config(confidence_threshold: f32) -> Self {
        Self {
            name: "basic".to_string(),
            supported_types: vec![
                // Only handle structured data - let ML extractor handle named entities
                EntityType::Email,
                EntityType::Url,
                EntityType::PhoneNumber,
                EntityType::Date,
                EntityType::Time,
                EntityType::Money,
            ],
            confidence_threshold,
        }
    }

    /// Extract email addresses from text.
    fn extract_emails(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref EMAIL_REGEX: Regex = Regex::new(
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b"
            ).unwrap();
        }

        EMAIL_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::Email,
                    m.start(),
                    m.end(),
                    0.95, // High confidence for email regex
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Extract URLs from text.
    fn extract_urls(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref URL_REGEX: Regex = Regex::new(
                r"https?://(?:[-\w.])+(?:[:\d]+)?(?:/(?:[\w/_.])*(?:\?(?:[\w&=%.])*)?(?:#(?:[\w.])*)?)?|www\.(?:[-\w.])+(?:[:\d]+)?(?:/(?:[\w/_.])*(?:\?(?:[\w&=%.])*)?(?:#(?:[\w.])*)?)?|(?:[-\w.])+\.(?:com|org|net|edu|gov|mil|int|co|uk|ca|de|jp|fr|au|us|ru|ch|it|nl|se|no|es|mil)(?:[:\d]+)?(?:/(?:[\w/_.])*(?:\?(?:[\w&=%.])*)?(?:#(?:[\w.])*)?)?(?:\b|$)"
            ).unwrap();
        }

        URL_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::Url,
                    m.start(),
                    m.end(),
                    0.90, // High confidence for URL regex
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Extract phone numbers from text.
    fn extract_phone_numbers(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref PHONE_REGEX: Regex = Regex::new(
                r"\+?1?[-.\s]?\(?[2-9]\d{2}\)?[-.\s]?\d{3}[-.\s]?\d{4}|\+\d{1,3}[-.\s]?\d{3,4}[-.\s]?\d{3}[-.\s]?\d{4}"
            ).unwrap();
        }

        PHONE_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::PhoneNumber,
                    m.start(),
                    m.end(),
                    0.85, // Good confidence for phone regex
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Extract dates from text.
    fn extract_dates(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref DATE_REGEX: Regex = Regex::new(
                r"\b(?:(?:0?[1-9]|1[0-2])[\/\-.](?:0?[1-9]|[12]\d|3[01])[\/\-.](?:19|20)\d{2}|\b(?:January|February|March|April|May|June|July|August|September|October|November|December|Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\.?\s+(?:0?[1-9]|[12]\d|3[01])(?:st|nd|rd|th)?,?\s+(?:19|20)\d{2}|\b(?:19|20)\d{2}[-/.](?:0?[1-9]|1[0-2])[-/.](?:0?[1-9]|[12]\d|3[01])\b)"
            ).unwrap();
        }

        DATE_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::Date,
                    m.start(),
                    m.end(),
                    0.80, // Good confidence for date patterns
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Extract times from text.
    fn extract_times(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref TIME_REGEX: Regex = Regex::new(
                r"\b(?:[01]?[0-9]|2[0-3]):[0-5][0-9](?:\s?(?:AM|PM|am|pm))?\b|\b(?:[1-9]|1[0-2])(?:\s?(?:AM|PM|am|pm))\b"
            ).unwrap();
        }

        TIME_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::Time,
                    m.start(),
                    m.end(),
                    0.85, // Good confidence for time patterns
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Extract money amounts from text.
    fn extract_money(&self, content: &str) -> Vec<ExtractedEntity> {
        lazy_static! {
            static ref MONEY_REGEX: Regex = Regex::new(
                r"(?:\$\d{1,3}(?:,\d{3})*(?:\.\d{2})?|€\d{1,3}(?:,\d{3})*(?:\.\d{2})?|£\d{1,3}(?:,\d{3})*(?:\.\d{2})?|¥\d{1,3}(?:,\d{3})*(?:\.\d{2})?|\b\d{1,3}(?:,\d{3})*(?:\.\d{2})?\s?(?:USD|EUR|GBP|JPY|dollars?|euros?|pounds?|yen)\b)"
            ).unwrap();
        }

        MONEY_REGEX
            .find_iter(content)
            .map(|m| {
                ExtractedEntity::new(
                    m.as_str().to_string(),
                    EntityType::Money,
                    m.start(),
                    m.end(),
                    0.90, // High confidence for money patterns
                    self.name.clone(),
                )
            })
            .collect()
    }

    /// Remove overlapping entities, keeping the ones with higher priority/confidence
    fn remove_overlaps(&self, mut entities: Vec<ExtractedEntity>) -> Vec<ExtractedEntity> {
        // Sort by start position first
        entities.sort_by_key(|e| e.start_pos);
        
        let mut result = Vec::new();
        
        for entity in entities {
            let mut should_add = true;
            let mut indices_to_remove = Vec::new();
            
            // Check for overlaps with already added entities
            for (idx, existing) in result.iter().enumerate() {
                if self.entities_overlap(&entity, existing) {
                    // If overlapping, prefer the entity with higher priority
                    let entity_priority = self.get_entity_priority(&entity.entity_type);
                    let existing_priority = self.get_entity_priority(&existing.entity_type);
                    
                    if entity_priority > existing_priority || 
                       (entity_priority == existing_priority && entity.confidence > existing.confidence) {
                        // Mark the existing entity for removal
                        indices_to_remove.push(idx);
                    } else {
                        // Keep the existing entity, don't add the new one
                        should_add = false;
                        break;
                    }
                }
            }
            
            // Remove overlapping entities (in reverse order to maintain indices)
            for &idx in indices_to_remove.iter().rev() {
                result.remove(idx);
            }
            
            if should_add {
                result.push(entity);
            }
        }
        
        result
    }
    
    /// Check if two entities overlap
    fn entities_overlap(&self, entity1: &ExtractedEntity, entity2: &ExtractedEntity) -> bool {
        entity1.start_pos < entity2.end_pos && entity1.end_pos > entity2.start_pos
    }
    
    /// Get priority for entity types (higher priority entities are preferred in case of overlap)
    fn get_entity_priority(&self, entity_type: &EntityType) -> u8 {
        match entity_type {
            EntityType::Email => 100,      // Highest priority - emails are very specific
            EntityType::PhoneNumber => 90, // High priority - phone numbers are specific
            EntityType::Money => 80,       // High priority - money amounts are specific
            EntityType::Date => 70,        // Medium-high priority
            EntityType::Time => 60,        // Medium priority
            EntityType::Url => 50,         // Lower priority - URLs can be part of emails
            _ => 40,                       // Default priority
        }
    }
}

#[async_trait]
impl EntityExtractor for BasicEntityExtractor {
    async fn extract_entities(&self, content: &str) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();

        // Extract structured data types only - named entities handled by ML extractor
        entities.extend(self.extract_emails(content));
        entities.extend(self.extract_urls(content));
        entities.extend(self.extract_phone_numbers(content));
        entities.extend(self.extract_dates(content));
        entities.extend(self.extract_times(content));
        entities.extend(self.extract_money(content));

        // Filter entities based on confidence threshold
        entities.retain(|entity| entity.confidence >= self.confidence_threshold);

        // Remove overlapping entities (prefer higher priority/confidence ones)
        entities = self.remove_overlaps(entities);

        // Sort by position in text for consistent ordering
        entities.sort_by_key(|entity| entity.start_pos);

        Ok(entities)
    }

    fn supported_types(&self) -> Vec<EntityType> {
        self.supported_types.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        100 // Basic extractor has high priority since it's fast
    }
}

impl Default for BasicEntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

 