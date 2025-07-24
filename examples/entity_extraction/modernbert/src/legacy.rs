//! Legacy API wrapper for backwards compatibility
//!
//! This provides a compatibility wrapper for the old ModernBertNerExtractor API

use async_trait::async_trait;
use locai::{Result, prelude::{EntityType, ExtractedEntity, EntityExtractor}};
use locai::entity_extraction::RawEntityExtractor;
use locai::ml::ModelManager;
use crate::extractor::ModernBertExtractor;
use std::sync::Arc;

/// Legacy wrapper for backwards compatibility
pub struct LegacyModernBertNerExtractor {
    inner: ModernBertExtractor,
}

impl std::fmt::Debug for LegacyModernBertNerExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LegacyModernBertNerExtractor")
            .field("inner", &self.inner)
            .finish()
    }
}

impl LegacyModernBertNerExtractor {
    /// Create from model path using ModelManager
    pub async fn from_path(model_path: &str) -> Result<Self> {
        // Create a model manager for this legacy extractor
        let model_manager = Arc::new(ModelManager::new("./model_cache"));
        
        let inner = ModernBertExtractor::from_manager_with_path(
            model_manager, 
            "modernbert-ner", 
            model_path
        ).await
            .map_err(|e| locai::LocaiError::Entity(e.to_string()))?;
        
        Ok(Self { inner })
    }
    
    /// Set max length
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.inner = self.inner.with_max_length(max_length);
        self
    }
}

#[async_trait]
impl EntityExtractor for LegacyModernBertNerExtractor {
    async fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let raw_entities = self.inner.extract_raw(text).await?;
        
        // Convert raw entities to legacy format
        let entities = raw_entities.into_iter()
            .map(|raw| {
                let entity_type = match raw.entity_type {
                    locai::entity_extraction::GenericEntityType::Person => EntityType::Person,
                    locai::entity_extraction::GenericEntityType::Organization => EntityType::Organization,
                    locai::entity_extraction::GenericEntityType::Location => EntityType::Location,
                    locai::entity_extraction::GenericEntityType::Miscellaneous => EntityType::Custom("MISC".to_string()),
                };
                
                ExtractedEntity::new(
                    raw.text.clone(),
                    entity_type,
                    raw.start_pos,
                    raw.end_pos,
                    raw.confidence,
                    "ModernBertNerExtractor".to_string(),
                )
            })
            .collect();
            
        Ok(entities)
    }
    
    fn name(&self) -> &str {
        "ModernBertNerExtractor"
    }
    
    fn supported_types(&self) -> Vec<EntityType> {
        vec![
            EntityType::Person,
            EntityType::Organization,
            EntityType::Location,
            EntityType::Custom("MISC".to_string()),
        ]
    }
} 