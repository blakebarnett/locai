//! Core memory operations module
//!
//! This module contains the fundamental CRUD operations for memories,
//! including storage, retrieval, updating, and deletion.

use crate::config::LocaiConfig;
use crate::entity_extraction::{
    AutomaticRelationshipCreator, BasicEntityExtractor, EntityExtractor, EntityResolver,
    ExtractorType,
};
use crate::ml::model_manager::EmbeddingManager;
use crate::models::Memory;
use crate::storage::filters::MemoryFilter;
use crate::storage::traits::GraphStore;

use crate::{LocaiError, Result};
use std::sync::Arc;

/// Core memory operations handler
#[derive(Debug, Clone)]
pub struct MemoryOperations {
    storage: Arc<dyn GraphStore>,
    ml_service: Option<Arc<EmbeddingManager>>,
    config: LocaiConfig,
    entity_extractors: Vec<Arc<dyn EntityExtractor>>,
    entity_resolver: Option<EntityResolver>,
    relationship_creator: Option<AutomaticRelationshipCreator>,
}

impl MemoryOperations {
    /// Create a new memory operations handler
    pub fn new(
        storage: Arc<dyn GraphStore>,
        ml_service: Option<Arc<EmbeddingManager>>,
        config: LocaiConfig,
    ) -> Self {
        // Initialize entity extractors if enabled
        let mut entity_extractors: Vec<Arc<dyn EntityExtractor>> = Vec::new();

        if config.entity_extraction.enabled {
            for extractor_config in &config.entity_extraction.extractors {
                if extractor_config.enabled {
                    match &extractor_config.extractor_type {
                        ExtractorType::Regex => {
                            let basic_extractor = BasicEntityExtractor::with_config(
                                config.entity_extraction.confidence_threshold,
                            );
                            entity_extractors
                                .push(Arc::new(basic_extractor) as Arc<dyn EntityExtractor>);
                            tracing::info!(
                                "🔧 Initialized BasicEntityExtractor for structured data"
                            );
                        }
                        ExtractorType::Hybrid { .. } => {
                            // Hybrid extractor requires async initialization - defer to new_with_ml
                            tracing::info!(
                                "🔧 Hybrid extractor initialization deferred - requires async context"
                            );
                        }
                        ExtractorType::Pipeline { .. } => {
                            // Pipeline extractor requires async initialization - defer to new_with_ml
                            tracing::info!(
                                "🔧 Pipeline extractor initialization deferred - requires async context"
                            );
                        }
                        ExtractorType::Spacy { .. } => {
                            // spaCy extractor requires async initialization - defer to new_with_ml
                            tracing::info!(
                                "🔧 spaCy extractor initialization deferred - requires async context"
                            );
                        }
                        ExtractorType::HuggingFace { .. } => {
                            // Transformer extractor requires async initialization - defer to new_with_ml
                            tracing::info!(
                                "🔧 Transformer extractor initialization deferred - requires async context"
                            );
                        }

                        ExtractorType::Llm { .. } => {
                            tracing::warn!("🔧 LLM extractor not yet implemented");
                        }
                    }
                }
            }
        }

        // Initialize entity resolver for Phase 2
        let entity_resolver =
            if config.entity_extraction.enabled && config.entity_extraction.deduplicate_entities {
                Some(EntityResolver::new(
                    config.entity_extraction.resolution.clone(),
                ))
            } else {
                None
            };

        // Initialize automatic relationship creator for Phase 2
        let relationship_creator = if config.entity_extraction.automatic_relationships.enabled {
            Some(AutomaticRelationshipCreator::new(
                config.entity_extraction.automatic_relationships.clone(),
            ))
        } else {
            None
        };

        Self {
            storage,
            ml_service,
            config,
            entity_extractors,
            entity_resolver,
            relationship_creator,
        }
    }

    /// Create a new MemoryOperations with ML extractors initialized asynchronously
    pub async fn new_with_ml(
        storage: Arc<dyn GraphStore>,
        ml_service: Option<Arc<EmbeddingManager>>,
        config: LocaiConfig,
    ) -> Result<Self> {
        // First create with basic extractors
        let mut ops = Self::new(storage, ml_service, config);

        // Then initialize ML extractors
        ops.initialize_ml_extractors().await?;

        Ok(ops)
    }

    /// Initialize ML extractors asynchronously after construction
    pub async fn initialize_ml_extractors(&mut self) -> Result<()> {
        if !self.config.entity_extraction.enabled {
            tracing::debug!("🔍 Entity extraction disabled, skipping ML extractor initialization");
            return Ok(());
        }

        // If no extractors are configured, add default extractors
        if self.config.entity_extraction.extractors.is_empty() {
            // Add basic extractor for structured data
            let basic_extractor = crate::entity_extraction::BasicEntityExtractor::new();
            self.entity_extractors
                .push(Arc::new(basic_extractor) as Arc<dyn EntityExtractor>);
        } else {
            // Process configured extractors
            for extractor_config in &self.config.entity_extraction.extractors {
                if extractor_config.enabled {
                    match &extractor_config.extractor_type {
                        ExtractorType::Hybrid {
                            config: hybrid_config,
                        } => {
                            // Initialize basic extractor first if enabled
                            if hybrid_config.enable_basic {
                                let basic_extractor =
                                    crate::entity_extraction::BasicEntityExtractor::new();
                                self.entity_extractors
                                    .push(Arc::new(basic_extractor) as Arc<dyn EntityExtractor>);
                            }
                        }
                        ExtractorType::Regex => {
                            let basic_extractor =
                                crate::entity_extraction::BasicEntityExtractor::new();
                            self.entity_extractors
                                .push(Arc::new(basic_extractor) as Arc<dyn EntityExtractor>);
                        }
                        ExtractorType::HuggingFace { model: _model } => {
                            // HuggingFace extractors not included in default build
                        }
                        ExtractorType::Pipeline {
                            extractors: _pipeline_extractors,
                            min_confidence: _min_confidence,
                        } => {
                            // Pipeline extractors not included in default build
                        }
                        ExtractorType::Spacy { model: _model } => {
                            // SpaCy extractors not included in default build
                        }
                        ExtractorType::Llm {
                            provider: _provider,
                            model: _model,
                        } => {
                            // LLM extractors not included in default build
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Store a new memory
    ///
    /// # Arguments
    /// * `memory` - The memory to store
    ///
    /// # Returns
    /// The ID of the stored memory
    pub async fn store_memory(&self, memory: Memory) -> Result<String> {
        // BYOE approach: Users provide their own embeddings via Memory.with_embedding()
        // No automatic embedding generation - embeddings are provided by the user when needed

        // Store the memory first
        let created = self
            .storage
            .create_memory(memory)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to store memory: {}", e)))?;

        // Create Vector record if memory has an embedding
        if let Some(embedding) = &created.embedding {
            let vector = crate::storage::models::Vector {
                id: format!("mem_{}", created.id),
                vector: embedding.clone(),
                dimension: embedding.len(),
                metadata: serde_json::json!({
                    "type": "memory",
                    "memory_id": created.id,
                    "memory_type": created.memory_type.to_string(),
                    "content_preview": created.content.chars().take(100).collect::<String>(),
                    "source": created.source,
                    "tags": created.tags
                }),
                source_id: Some(created.id.clone()),
                created_at: created.created_at,
            };

            // Add vector to storage, but don't fail memory storage if this fails
            match self.storage.add_vector(vector).await {
                Ok(_) => {
                    tracing::debug!("Created vector record for memory {}", created.id);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create vector record for memory {}: {}",
                        created.id,
                        e
                    );
                    // Continue - memory storage should not fail because of vector creation failure
                }
            }
        }

        // Extract entities if entity extraction is enabled
        if self.config.entity_extraction.enabled && !self.entity_extractors.is_empty() {
            let mut all_extracted_entities = Vec::new();

            // Run all extractors and collect results
            for extractor in &self.entity_extractors {
                match extractor.extract_entities(&created.content).await {
                    Ok(extracted_entities) => {
                        all_extracted_entities.extend(extracted_entities);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Extractor '{}' failed to extract entities from memory {}: {}",
                            extractor.name(),
                            created.id,
                            e
                        );
                        // Continue with other extractors even if one fails
                    }
                }
            }

            // Process each extracted entity with Phase 2 resolution
            for extracted in all_extracted_entities {
                if extracted.confidence >= self.config.entity_extraction.confidence_threshold {
                    match self
                        .process_extracted_entity_with_resolution(&created.id, &extracted)
                        .await
                    {
                        Ok(_) => {
                            tracing::debug!(
                                "Successfully processed entity: {}",
                                extracted.format()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to process entity '{}': {}", extracted.text, e);
                            // Continue processing other entities even if one fails
                        }
                    }
                } else {
                    tracing::debug!(
                        "Skipping entity '{}' due to low confidence: {:.2} < {:.2}",
                        extracted.text,
                        extracted.confidence,
                        self.config.entity_extraction.confidence_threshold
                    );
                }
            }
        }

        // Create automatic relationships (Phase 2)
        if let Some(relationship_creator) = &self.relationship_creator {
            match relationship_creator
                .create_relationships_for_memory(&created.id, self.storage.as_ref())
                .await
            {
                Ok(relationship_ids) => {
                    tracing::debug!(
                        "Created {} automatic relationships for memory {}",
                        relationship_ids.len(),
                        created.id
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create automatic relationships for memory {}: {}",
                        created.id,
                        e
                    );
                    // Don't fail the memory storage if relationship creation fails
                }
            }
        }

        Ok(created.id)
    }

    /// Process an extracted entity with Phase 2 resolution and deduplication
    async fn process_extracted_entity_with_resolution(
        &self,
        memory_id: &str,
        extracted: &crate::entity_extraction::ExtractedEntity,
    ) -> Result<()> {
        let entity = if let Some(resolver) = &self.entity_resolver {
            // Phase 2: Find potential matches and resolve duplicates
            let matches = resolver
                .find_matches(extracted, self.storage.as_ref())
                .await?;

            if let Some((existing_entity, confidence)) = matches.first() {
                if confidence
                    >= &self
                        .config
                        .entity_extraction
                        .resolution
                        .min_confidence_for_merge
                {
                    // Merge with existing entity
                    tracing::debug!(
                        "Merging entity '{}' with existing entity {} (confidence: {:.2})",
                        extracted.text,
                        existing_entity.id,
                        confidence
                    );

                    let merged_entity =
                        resolver.merge_entities(existing_entity.clone(), extracted.clone())?;
                    self.storage
                        .update_entity(merged_entity.clone())
                        .await
                        .map_err(|e| {
                            LocaiError::Storage(format!("Failed to update merged entity: {}", e))
                        })?;

                    merged_entity
                } else {
                    // Create new entity (no good matches found)
                    self.create_new_entity(extracted).await?
                }
            } else {
                // No matches found, create new entity
                self.create_new_entity(extracted).await?
            }
        } else {
            // Phase 1 behavior: check for exact matches only
            self.find_or_create_entity(extracted).await?
        };

        // Create the "contains" edge: memory -> contains -> entity
        self.create_contains_edge(memory_id, &entity.id).await?;

        Ok(())
    }

    /// Create a new entity from extracted entity data
    async fn create_new_entity(
        &self,
        extracted: &crate::entity_extraction::ExtractedEntity,
    ) -> Result<crate::storage::models::Entity> {
        use crate::storage::models::Entity;
        use std::collections::HashMap;

        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            serde_json::Value::String(extracted.text.clone()),
        );
        properties.insert(
            "confidence".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(extracted.confidence as f64).unwrap(),
            ),
        );
        properties.insert(
            "extractor_source".to_string(),
            serde_json::Value::String(extracted.extractor_source.clone()),
        );
        properties.insert(
            "start_pos".to_string(),
            serde_json::Value::Number(serde_json::Number::from(extracted.start_pos)),
        );
        properties.insert(
            "end_pos".to_string(),
            serde_json::Value::Number(serde_json::Number::from(extracted.end_pos)),
        );

        // Add extracted metadata
        for (key, value) in &extracted.metadata {
            properties.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        let new_entity = Entity {
            id: uuid::Uuid::new_v4().to_string(),
            entity_type: extracted.entity_type.as_str().to_string(),
            properties: serde_json::Value::Object(properties.into_iter().collect()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created_entity = self
            .storage
            .create_entity(new_entity)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to create entity: {}", e)))?;

        let entity_name = created_entity
            .properties
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        tracing::debug!(
            "Created new entity: {} ({})",
            entity_name,
            created_entity.id
        );

        Ok(created_entity)
    }

    /// Process an extracted entity by creating or finding the entity and linking it to the memory.
    #[allow(dead_code)]
    async fn process_extracted_entity(
        &self,
        memory_id: &str,
        extracted: &crate::entity_extraction::ExtractedEntity,
    ) -> Result<()> {
        // Check if entity already exists or create it
        let existing_entity = self.find_or_create_entity(extracted).await?;

        tracing::debug!(
            "Processing entity {} (ID: {}) for memory {}",
            extracted.text,
            existing_entity.id,
            memory_id
        );

        // Create the "contains" edge: memory -> contains -> entity
        match self
            .create_contains_edge(memory_id, &existing_entity.id)
            .await
        {
            Ok(_) => {
                tracing::debug!(
                    "Successfully created contains edge: memory {} -> contains -> entity {}",
                    memory_id,
                    existing_entity.id
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create 'contains' edge between memory {} and entity {}: {}",
                    memory_id,
                    existing_entity.id,
                    e
                );
                // Don't fail the entire process if edge creation fails
            }
        }

        Ok(())
    }

    /// Find an existing entity or create a new one from extracted entity data.
    async fn find_or_create_entity(
        &self,
        extracted: &crate::entity_extraction::ExtractedEntity,
    ) -> Result<crate::storage::models::Entity> {
        use crate::storage::filters::EntityFilter;

        // First, try to find an existing entity with the same name and type
        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "name".to_string(),
            serde_json::Value::String(extracted.text.clone()),
        );

        let filter = EntityFilter {
            entity_type: Some(extracted.entity_type.as_str().to_string()),
            properties: Some(properties),
            ..Default::default()
        };

        let existing_entities = self
            .storage
            .list_entities(Some(filter), Some(1), None)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to list entities: {}", e)))?;

        if let Some(existing) = existing_entities.first() {
            // Entity already exists, return it
            let entity_name = existing
                .properties
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            tracing::debug!("Found existing entity: {} ({})", entity_name, existing.id);
            Ok(existing.clone())
        } else {
            // Create new entity
            self.create_new_entity(extracted).await
        }
    }

    /// Create a "mentions" edge between a memory and an entity
    async fn create_contains_edge(&self, memory_id: &str, entity_id: &str) -> Result<bool> {
        use crate::storage::models::Relationship;

        tracing::debug!(
            "Creating mentions relationship: memory {} -> mentions -> entity {}",
            memory_id,
            entity_id
        );

        // Create a relationship record for memory->contains->entity
        let relationship = Relationship {
            id: format!("contains_{}_{}", memory_id, entity_id),
            source_id: memory_id.to_string(),
            target_id: entity_id.to_string(),
            relationship_type: "mentions".to_string(),
            properties: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        match self.storage.create_relationship(relationship).await {
            Ok(_) => {
                tracing::debug!(
                    "Successfully created contains relationship: memory {} -> entity {}",
                    memory_id,
                    entity_id
                );
                Ok(true)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create contains relationship: memory {} -> entity {}: {}",
                    memory_id,
                    entity_id,
                    e
                );
                // Don't fail completely - this is not critical
                Ok(false)
            }
        }
    }

    /// Retrieve a memory by ID
    ///
    /// # Arguments
    /// * `id` - The ID of the memory to retrieve
    ///
    /// # Returns
    /// The memory if found, None otherwise
    pub async fn get_memory(&self, id: &str) -> Result<Option<Memory>> {
        self.storage
            .get_memory(id)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to get memory: {}", e)))
    }

    /// Update an existing memory
    ///
    /// # Arguments
    /// * `memory` - The updated memory
    ///
    /// # Returns
    /// Whether the update was successful
    pub async fn update_memory(&self, memory: Memory) -> Result<bool> {
        let updated = self
            .storage
            .update_memory(memory)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to update memory: {}", e)))?;

        // Update or create vector record if memory has an embedding
        if let Some(embedding) = &updated.embedding {
            let vector_id = format!("mem_{}", updated.id);

            // Check if vector already exists
            match self.storage.get_vector(&vector_id).await {
                Ok(Some(existing_vector)) => {
                    // Update existing vector - preserve original creation time
                    let new_vector = crate::storage::models::Vector {
                        id: vector_id.clone(),
                        vector: embedding.clone(),
                        dimension: embedding.len(),
                        metadata: serde_json::json!({
                            "type": "memory",
                            "memory_id": updated.id,
                            "memory_type": updated.memory_type.to_string(),
                            "content_preview": updated.content.chars().take(100).collect::<String>(),
                            "source": updated.source,
                            "tags": updated.tags
                        }),
                        source_id: Some(updated.id.clone()),
                        created_at: existing_vector.created_at, // Preserve original vector creation time
                    };

                    match self.storage.upsert_vector(new_vector).await {
                        Ok(_) => {
                            tracing::debug!("Updated vector record for memory {}", updated.id);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to update vector record for memory {}: {}",
                                updated.id,
                                e
                            );
                        }
                    }
                }
                Ok(None) => {
                    // Create new vector
                    let vector = crate::storage::models::Vector {
                        id: vector_id,
                        vector: embedding.clone(),
                        dimension: embedding.len(),
                        metadata: serde_json::json!({
                            "type": "memory",
                            "memory_id": updated.id,
                            "memory_type": updated.memory_type.to_string(),
                            "content_preview": updated.content.chars().take(100).collect::<String>(),
                            "source": updated.source,
                            "tags": updated.tags
                        }),
                        source_id: Some(updated.id.clone()),
                        created_at: updated.created_at,
                    };

                    match self.storage.add_vector(vector).await {
                        Ok(_) => {
                            tracing::debug!(
                                "Created vector record for updated memory {}",
                                updated.id
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to create vector record for updated memory {}: {}",
                                updated.id,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check for existing vector for memory {}: {}",
                        updated.id,
                        e
                    );
                }
            }
        } else {
            // Memory no longer has an embedding, delete the vector if it exists
            let vector_id = format!("mem_{}", updated.id);
            match self.storage.delete_vector(&vector_id).await {
                Ok(deleted) => {
                    if deleted {
                        tracing::debug!(
                            "Deleted vector record for memory {} (no longer has embedding)",
                            updated.id
                        );
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "No vector to delete for memory {} (or deletion failed): {}",
                        updated.id,
                        e
                    );
                }
            }
        }

        Ok(true) // If we got here, the update was successful
    }

    /// Delete a memory by ID
    ///
    /// # Arguments
    /// * `id` - The ID of the memory to delete
    ///
    /// # Returns
    /// Whether the deletion was successful
    pub async fn delete_memory(&self, id: &str) -> Result<bool> {
        // Delete associated vector first (if it exists)
        let vector_id = format!("mem_{}", id);
        match self.storage.delete_vector(&vector_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::debug!("Deleted vector record for memory {}", id);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to delete vector record for memory {}: {}", id, e);
                // Continue with memory deletion even if vector deletion fails
            }
        }

        // Delete the memory
        self.storage
            .delete_memory(id)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to delete memory: {}", e)))
    }

    /// Filter memories using various criteria
    ///
    /// # Arguments
    /// * `filter` - The filter to apply
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// A vector of memories matching the filter criteria
    pub async fn filter_memories(
        &self,
        filter: MemoryFilter,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>> {
        self.storage
            .list_memories(Some(filter), limit, None)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to filter memories: {}", e)))
    }

    /// Count memories with optional filtering
    ///
    /// # Arguments
    /// * `filter` - Optional filter to apply
    ///
    /// # Returns
    /// The number of memories matching the filter
    pub async fn count_memories(&self, filter: Option<MemoryFilter>) -> Result<usize> {
        self.storage
            .count_memories(filter)
            .await
            .map_err(|e| LocaiError::Storage(format!("Failed to count memories: {}", e)))
    }

    /// Tag a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory to tag
    /// * `tag` - The tag to add
    ///
    /// # Returns
    /// Whether the operation was successful
    pub async fn tag_memory(&self, memory_id: &str, tag: &str) -> Result<bool> {
        // Get the memory
        let mut memory = match self.get_memory(memory_id).await? {
            Some(m) => m,
            None => {
                return Err(LocaiError::Memory(format!(
                    "Memory with ID {} not found",
                    memory_id
                )));
            }
        };

        // Add the tag
        memory.add_tag(tag);

        // Update the memory
        self.update_memory(memory).await
    }

    /// Get access to the underlying storage service
    pub fn storage(&self) -> &Arc<dyn GraphStore> {
        &self.storage
    }

    /// Get the configuration
    pub fn config(&self) -> &LocaiConfig {
        &self.config
    }

    /// Check if ML service is available
    pub fn has_ml_service(&self) -> bool {
        self.ml_service.is_some()
    }

    /// Get ML service reference
    pub fn ml_service(&self) -> Option<&Arc<EmbeddingManager>> {
        self.ml_service.as_ref()
    }
}
