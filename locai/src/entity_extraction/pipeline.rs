//! Generic entity extraction pipeline architecture.
//!
//! This module provides a composable pipeline architecture for entity extraction
//! that separates generic extraction logic from domain-specific validation and processing.

use super::{EntityType, ExtractedEntity};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Raw entity extracted by a model before validation and post-processing
#[derive(Debug, Clone)]
pub struct RawEntity {
    /// The text content of the entity
    pub text: String,
    /// Generic entity type classification
    pub entity_type: GenericEntityType,
    /// Starting position in the original text
    pub start_pos: usize,
    /// Ending position in the original text
    pub end_pos: usize,
    /// Raw confidence score from the model (0.0 to 1.0)
    pub confidence: f32,
    /// Additional metadata from the extractor
    pub metadata: HashMap<String, String>,
}

impl RawEntity {
    /// Create a new raw entity
    pub fn new(
        text: String,
        entity_type: GenericEntityType,
        start_pos: usize,
        end_pos: usize,
        confidence: f32,
    ) -> Self {
        Self {
            text,
            entity_type,
            start_pos,
            end_pos,
            confidence,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the raw entity
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Generic entity types that all extractors can produce
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GenericEntityType {
    /// Person names
    Person,
    /// Organizations/institutions
    Organization,
    /// Geographic locations
    Location,
    /// Miscellaneous entities
    Miscellaneous,
}

impl GenericEntityType {
    /// Convert to the specific EntityType
    pub fn to_entity_type(&self) -> EntityType {
        match self {
            GenericEntityType::Person => EntityType::Person,
            GenericEntityType::Organization => EntityType::Organization,
            GenericEntityType::Location => EntityType::Location,
            GenericEntityType::Miscellaneous => EntityType::Custom("MISC".to_string()),
        }
    }

    /// Create from string representation
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PERSON" | "PER" => Some(GenericEntityType::Person),
            "ORGANIZATION" | "ORG" => Some(GenericEntityType::Organization),
            "LOCATION" | "LOC" => Some(GenericEntityType::Location),
            "MISCELLANEOUS" | "MISC" => Some(GenericEntityType::Miscellaneous),
            _ => None,
        }
    }
}

/// Context provided to validators for making validation decisions
#[derive(Debug, Clone)]
pub struct ValidationContext<'a> {
    /// The original text being processed
    pub original_text: &'a str,
    /// Other entities already validated in this text
    pub other_entities: &'a [RawEntity],
    /// Additional context data
    pub metadata: HashMap<String, String>,
}

impl<'a> ValidationContext<'a> {
    /// Create a new validation context
    pub fn new(original_text: &'a str) -> Self {
        Self {
            original_text,
            other_entities: &[],
            metadata: HashMap::new(),
        }
    }

    /// Add other entities for context
    pub fn with_entities(mut self, entities: &'a [RawEntity]) -> Self {
        self.other_entities = entities;
        self
    }

    /// Add metadata for context
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Generic trait for raw entity extraction from text
#[async_trait]
pub trait RawEntityExtractor: Send + Sync + std::fmt::Debug {
    /// Extract raw entities from text without validation
    async fn extract_raw(&self, text: &str) -> Result<Vec<RawEntity>>;

    /// Get the name of this extractor
    fn name(&self) -> &str;

    /// Get supported entity types
    fn supported_types(&self) -> Vec<GenericEntityType>;
}

/// Generic trait for validating extracted entities
pub trait EntityValidator: Send + Sync + std::fmt::Debug {
    /// Validate a raw entity given the context
    fn validate(&self, entity: &RawEntity, context: &ValidationContext) -> bool;

    /// Get the name of this validator
    fn name(&self) -> &str;
}

/// Generic trait for post-processing entities
pub trait EntityPostProcessor: Send + Sync + std::fmt::Debug {
    /// Process a list of validated entities
    fn process(&self, entities: Vec<RawEntity>) -> Vec<RawEntity>;

    /// Get the name of this post-processor
    fn name(&self) -> &str;
}

/// Generic trait for loading models from paths
#[async_trait]
pub trait ModelLoader: Send + Sync {
    /// Load a model from the given path
    async fn load_model(path: &str) -> Result<Self>
    where
        Self: Sized;
}

/// Composable entity extraction pipeline
pub struct EntityExtractionPipeline {
    extractor: Box<dyn RawEntityExtractor>,
    validators: Vec<Box<dyn EntityValidator>>,
    post_processors: Vec<Box<dyn EntityPostProcessor>>,
    extractor_name: String,
}

impl std::fmt::Debug for EntityExtractionPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityExtractionPipeline")
            .field("extractor", &self.extractor_name)
            .field("validators", &self.validators.len())
            .field("post_processors", &self.post_processors.len())
            .finish()
    }
}

impl EntityExtractionPipeline {
    /// Create a new pipeline builder
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::new()
    }

    /// Extract entities using the complete pipeline
    pub async fn extract(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        // Step 1: Extract raw entities
        let raw_entities = self.extractor.extract_raw(text).await?;

        if raw_entities.is_empty() {
            return Ok(Vec::new());
        }

        // Step 2: Validate entities
        let validated_entities = raw_entities
            .into_iter()
            .filter(|entity| {
                let context = ValidationContext::new(text);
                self.validators
                    .iter()
                    .all(|validator| validator.validate(entity, &context))
            })
            .collect();

        // Step 3: Post-process entities
        let processed_entities = self
            .post_processors
            .iter()
            .fold(validated_entities, |entities, processor| {
                processor.process(entities)
            });

        // Step 4: Convert to ExtractedEntity format
        let final_entities = processed_entities
            .into_iter()
            .map(|raw_entity| {
                ExtractedEntity::new(
                    raw_entity.text,
                    raw_entity.entity_type.to_entity_type(),
                    raw_entity.start_pos,
                    raw_entity.end_pos,
                    raw_entity.confidence,
                    self.extractor_name.clone(),
                )
            })
            .collect();

        Ok(final_entities)
    }
}

/// Builder for creating entity extraction pipelines
pub struct PipelineBuilder {
    extractor: Option<Box<dyn RawEntityExtractor>>,
    validators: Vec<Box<dyn EntityValidator>>,
    post_processors: Vec<Box<dyn EntityPostProcessor>>,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new() -> Self {
        Self {
            extractor: None,
            validators: Vec::new(),
            post_processors: Vec::new(),
        }
    }

    /// Set the entity extractor
    pub fn extractor(mut self, extractor: Box<dyn RawEntityExtractor>) -> Self {
        self.extractor = Some(extractor);
        self
    }

    /// Add a validator to the pipeline
    pub fn validator(mut self, validator: Box<dyn EntityValidator>) -> Self {
        self.validators.push(validator);
        self
    }

    /// Add a post-processor to the pipeline
    pub fn post_processor(mut self, post_processor: Box<dyn EntityPostProcessor>) -> Self {
        self.post_processors.push(post_processor);
        self
    }

    /// Build the pipeline
    pub fn build(self) -> Result<EntityExtractionPipeline> {
        let extractor = self.extractor.ok_or_else(|| {
            crate::LocaiError::Entity("Pipeline requires an extractor".to_string())
        })?;

        let extractor_name = extractor.name().to_string();

        Ok(EntityExtractionPipeline {
            extractor,
            validators: self.validators,
            post_processors: self.post_processors,
            extractor_name,
        })
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
