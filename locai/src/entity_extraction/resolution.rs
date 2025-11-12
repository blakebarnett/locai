//! Entity resolution and disambiguation functionality.
//!
//! This module provides Phase 2 capabilities for entity resolution, including:
//! - Fuzzy matching using Levenshtein distance
//! - Property-based matching for unique identifiers
//! - Context-based disambiguation
//! - Intelligent entity merging strategies

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::Result;
use crate::models::Memory;
use crate::storage::{GraphStore, filters::EntityFilter, models::Entity};

use super::{EntityType, ExtractedEntity};

/// Configuration for entity resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityResolutionConfig {
    /// Whether entity resolution is enabled
    pub enabled: bool,
    /// Strategy for merging entities
    pub merge_strategy: MergeStrategy,
    /// Similarity threshold for entity matching
    pub similarity_threshold: f32,
    /// Minimum confidence required to merge entities
    pub min_confidence_for_merge: f32,
    /// Disambiguation configuration
    pub disambiguation: DisambiguationConfig,
    /// Entity type-specific rules
    pub type_rules: EntityTypeRules,
}

impl Default for EntityResolutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            merge_strategy: MergeStrategy::Balanced,
            similarity_threshold: 0.8,
            min_confidence_for_merge: 0.7,
            disambiguation: DisambiguationConfig::default(),
            type_rules: EntityTypeRules::default(),
        }
    }
}

/// Strategy for merging entities when conflicts arise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Keep existing, only add new properties
    Conservative,
    /// Merge all properties, prefer higher confidence
    Balanced,
    /// Always update with new information
    Aggressive,
}

/// Configuration for entity disambiguation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisambiguationConfig {
    /// Whether disambiguation is enabled
    pub enabled: bool,
    /// Minimum confidence for merging
    pub min_confidence_for_merge: f32,
    /// Context window size (characters around entity mention)
    pub context_window: usize,
    /// Whether to check unique identifiers
    pub check_unique_identifiers: bool,
    /// Whether to check entity co-occurrence
    pub check_cooccurrence: bool,
    /// Whether to check temporal proximity
    pub check_temporal_proximity: bool,
    /// Weights for different confidence factors
    pub weights: ConfidenceWeights,
}

impl Default for DisambiguationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_confidence_for_merge: 0.7,
            context_window: 100,
            check_unique_identifiers: true,
            check_cooccurrence: true,
            check_temporal_proximity: true,
            weights: ConfidenceWeights::default(),
        }
    }
}

/// Weights for combining different confidence factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceWeights {
    pub identifiers: f32,
    pub context: f32,
    pub cooccurrence: f32,
    pub temporal: f32,
}

impl Default for ConfidenceWeights {
    fn default() -> Self {
        Self {
            identifiers: 0.4,
            context: 0.3,
            cooccurrence: 0.2,
            temporal: 0.1,
        }
    }
}

/// Rules for different entity types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeRules {
    /// Entity types that are likely globally unique
    pub globally_unique_types: HashSet<String>,
    /// Entity types that require context for disambiguation
    pub context_required_types: HashSet<String>,
    /// Entity types that are always local to a context
    pub always_local_types: HashSet<String>,
}

impl Default for EntityTypeRules {
    fn default() -> Self {
        let mut globally_unique = HashSet::new();
        globally_unique.insert("email".to_string());
        globally_unique.insert("url".to_string());
        globally_unique.insert("phone_number".to_string());

        let mut context_required = HashSet::new();
        context_required.insert("person".to_string());
        context_required.insert("organization".to_string());

        let mut always_local = HashSet::new();
        always_local.insert("event".to_string());
        always_local.insert("date".to_string());

        Self {
            globally_unique_types: globally_unique,
            context_required_types: context_required,
            always_local_types: always_local,
        }
    }
}

/// Entity resolver for finding and merging similar entities
#[derive(Debug, Clone)]
pub struct EntityResolver {
    config: EntityResolutionConfig,
}

impl EntityResolver {
    /// Create a new entity resolver
    pub fn new(config: EntityResolutionConfig) -> Self {
        Self { config }
    }

    /// Find existing entities that match the extracted entity
    pub async fn find_matches(
        &self,
        extracted: &ExtractedEntity,
        storage: &dyn GraphStore,
    ) -> Result<Vec<(Entity, f32)>> {
        let mut matches = Vec::new();

        // 1. Exact name match - filter by entity type and search in properties
        let filter = EntityFilter {
            entity_type: Some(self.entity_type_to_string(&extracted.entity_type)),
            ..Default::default()
        };

        if let Ok(exact_matches) = storage.list_entities(Some(filter), None, None).await {
            for entity in exact_matches {
                if let Some(name) = self.extract_entity_name(&entity)
                    && name == extracted.text
                    && self.entity_types_compatible(&extracted.entity_type, &entity.entity_type)
                {
                    matches.push((entity, 1.0));
                }
            }
        }

        // 2. Fuzzy name match using edit distance
        if matches.is_empty() {
            let fuzzy_matches = self.find_fuzzy_matches(extracted, storage).await?;
            matches.extend(fuzzy_matches);
        }

        // 3. Property overlap (same email, phone, etc.)
        let property_matches = self.find_property_matches(extracted, storage).await?;
        matches.extend(property_matches);

        // 4. Context similarity (if enabled)
        if self.config.disambiguation.enabled {
            let context_matches = self.find_context_matches(extracted, storage).await?;
            matches.extend(context_matches);
        }

        // Sort by confidence and deduplicate
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches.dedup_by(|a, b| a.0.id == b.0.id);

        Ok(matches)
    }

    /// Convert EntityType to string for storage filtering
    fn entity_type_to_string(&self, entity_type: &EntityType) -> String {
        match entity_type {
            EntityType::Person => "person".to_string(),
            EntityType::Organization => "organization".to_string(),
            EntityType::Location => "location".to_string(),
            EntityType::Date => "date".to_string(),
            EntityType::Time => "time".to_string(),
            EntityType::Money => "money".to_string(),
            EntityType::Email => "email".to_string(),
            EntityType::PhoneNumber => "phone_number".to_string(),
            EntityType::Url => "url".to_string(),
            EntityType::Medical => "medical".to_string(),
            EntityType::Legal => "legal".to_string(),
            EntityType::Technical => "technical".to_string(),
            EntityType::Custom(name) => name.clone(),
        }
    }

    /// Extract the name/text of an entity from its properties
    fn extract_entity_name(&self, entity: &Entity) -> Option<String> {
        // Try to extract name from common property fields
        if let Some(obj) = entity.properties.as_object() {
            if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                return Some(name.to_string());
            }
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                return Some(text.to_string());
            }
            if let Some(value) = obj.get("value").and_then(|v| v.as_str()) {
                return Some(value.to_string());
            }
        }
        None
    }

    /// Check if two entity types are compatible for merging
    fn entity_types_compatible(&self, type1: &EntityType, type2: &str) -> bool {
        self.entity_type_to_string(type1) == type2
    }

    /// Find matches using fuzzy string matching
    async fn find_fuzzy_matches(
        &self,
        extracted: &ExtractedEntity,
        storage: &dyn GraphStore,
    ) -> Result<Vec<(Entity, f32)>> {
        let mut matches = Vec::new();

        // Get all entities of the same type
        let filter = EntityFilter {
            entity_type: Some(self.entity_type_to_string(&extracted.entity_type)),
            ..Default::default()
        };

        if let Ok(entities) = storage.list_entities(Some(filter), None, None).await {
            for entity in entities {
                if let Some(name) = self.extract_entity_name(&entity) {
                    let similarity = self.calculate_string_similarity(&extracted.text, &name);
                    if similarity >= self.config.similarity_threshold {
                        matches.push((entity, similarity));
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Calculate string similarity using normalized edit distance
    fn calculate_string_similarity(&self, str1: &str, str2: &str) -> f32 {
        let distance = self.levenshtein_distance(str1, str2);
        let max_len = str1.len().max(str2.len());

        if max_len == 0 {
            1.0
        } else {
            1.0 - (distance as f32 / max_len as f32)
        }
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(&self, str1: &str, str2: &str) -> usize {
        let chars1: Vec<char> = str1.chars().collect();
        let chars2: Vec<char> = str2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();

        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        // Initialize first row and column
        #[allow(clippy::needless_range_loop)]
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        // Fill the matrix
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }

    /// Find matches based on property overlap
    async fn find_property_matches(
        &self,
        extracted: &ExtractedEntity,
        storage: &dyn GraphStore,
    ) -> Result<Vec<(Entity, f32)>> {
        let mut matches = Vec::new();

        // Check for unique identifiers in metadata
        for (key, value) in &extracted.metadata {
            if self.is_unique_identifier(key) {
                // Search for entities with matching property values
                let mut props = HashMap::new();
                props.insert(key.clone(), serde_json::Value::String(value.clone()));

                let filter = EntityFilter {
                    properties: Some(props),
                    entity_type: Some(self.entity_type_to_string(&extracted.entity_type)),
                    ..Default::default()
                };

                if let Ok(entities) = storage.list_entities(Some(filter), None, None).await {
                    for entity in entities {
                        if self.entity_types_compatible(&extracted.entity_type, &entity.entity_type)
                        {
                            matches.push((entity, 0.95)); // High confidence for unique identifiers
                        }
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Check if a property key represents a unique identifier
    fn is_unique_identifier(&self, key: &str) -> bool {
        matches!(
            key.to_lowercase().as_str(),
            "email" | "phone" | "url" | "id" | "username"
        )
    }

    /// Find matches based on context similarity
    async fn find_context_matches(
        &self,
        _extracted: &ExtractedEntity,
        _storage: &dyn GraphStore,
    ) -> Result<Vec<(Entity, f32)>> {
        // This would implement semantic context matching
        // For now, return empty - this would require embedding-based similarity
        Ok(Vec::new())
    }

    /// Merge extracted entity with existing entity
    pub fn merge_entities(&self, existing: Entity, extracted: ExtractedEntity) -> Result<Entity> {
        let mut merged = existing.clone();

        // Update the updated_at timestamp
        merged.updated_at = Utc::now();

        match self.config.merge_strategy {
            MergeStrategy::Conservative => {
                // Only add new properties, don't overwrite existing ones
                for (key, value) in extracted.metadata {
                    if let Some(props) = merged.properties.as_object_mut()
                        && !props.contains_key(&key)
                    {
                        props.insert(key, serde_json::Value::String(value));
                    }
                }
            }
            MergeStrategy::Balanced => {
                // Merge properties, prefer higher confidence
                for (key, value) in extracted.metadata {
                    if let Some(props) = merged.properties.as_object_mut() {
                        // Check if we should update based on confidence
                        if extracted.confidence > 0.8 || !props.contains_key(&key) {
                            props.insert(key, serde_json::Value::String(value));
                        }
                    }
                }

                // Update confidence in properties if extracted entity has higher confidence
                if let Some(props) = merged.properties.as_object_mut() {
                    if let Some(existing_confidence) =
                        props.get("confidence").and_then(|v| v.as_f64())
                    {
                        if extracted.confidence > existing_confidence as f32 {
                            props.insert(
                                "confidence".to_string(),
                                serde_json::Value::Number(
                                    serde_json::Number::from_f64(extracted.confidence as f64)
                                        .unwrap_or(serde_json::Number::from(0)),
                                ),
                            );
                        }
                    } else {
                        props.insert(
                            "confidence".to_string(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(extracted.confidence as f64)
                                    .unwrap_or(serde_json::Number::from(0)),
                            ),
                        );
                    }
                }
            }
            MergeStrategy::Aggressive => {
                // Always update with new information
                for (key, value) in extracted.metadata {
                    if let Some(props) = merged.properties.as_object_mut() {
                        props.insert(key, serde_json::Value::String(value));
                    }
                }

                // Update confidence
                if let Some(props) = merged.properties.as_object_mut() {
                    props.insert(
                        "confidence".to_string(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(extracted.confidence as f64)
                                .unwrap_or(serde_json::Number::from(0)),
                        ),
                    );
                }

                // Update name if different and higher confidence
                let existing_name = self.extract_entity_name(&merged).unwrap_or_default();
                let existing_confidence = merged
                    .properties
                    .as_object()
                    .and_then(|props| props.get("confidence"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                if extracted.text != existing_name
                    && extracted.confidence > existing_confidence as f32
                    && let Some(props) = merged.properties.as_object_mut()
                {
                    props.insert(
                        "name".to_string(),
                        serde_json::Value::String(extracted.text),
                    );
                }
            }
        }

        Ok(merged)
    }
}

/// Entity disambiguator for determining if entities are the same
#[derive(Debug, Clone)]
pub struct EntityDisambiguator {
    config: DisambiguationConfig,
}

impl EntityDisambiguator {
    /// Create a new entity disambiguator
    pub fn new(config: DisambiguationConfig) -> Self {
        Self { config }
    }

    /// Determine if an extracted entity and existing entity are the same
    pub async fn are_same_entity(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        memory_context: &Memory,
        storage: &dyn GraphStore,
    ) -> Result<(bool, f32)> {
        let mut confidence_scores = Vec::new();

        // 1. Check unique identifiers
        if self.config.check_unique_identifiers
            && let Some(score) = self.check_unique_identifiers(extracted, existing)
        {
            confidence_scores.push(("identifiers", score));
        }

        // 2. Analyze context overlap
        if self.config.check_cooccurrence {
            let context_score = self
                .analyze_context_overlap(extracted, existing, memory_context, storage)
                .await?;
            confidence_scores.push(("context", context_score));
        }

        // 3. Check entity co-occurrence
        if self.config.check_cooccurrence {
            let cooccurrence_score = self
                .check_entity_cooccurrence(extracted, existing, storage)
                .await?;
            confidence_scores.push(("cooccurrence", cooccurrence_score));
        }

        // 4. Check temporal proximity
        if self.config.check_temporal_proximity {
            let temporal_score = self
                .check_temporal_proximity(memory_context, existing, storage)
                .await?;
            confidence_scores.push(("temporal", temporal_score));
        }

        // Calculate weighted final score
        let final_score = self.calculate_weighted_score(&confidence_scores);
        let is_same = final_score >= self.config.min_confidence_for_merge;

        Ok((is_same, final_score))
    }

    /// Check for matching unique identifiers
    fn check_unique_identifiers(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
    ) -> Option<f32> {
        for (key, value) in &extracted.metadata {
            if self.is_unique_identifier_key(key)
                && let Some(existing_props) = existing.properties.as_object()
                && let Some(existing_value) = existing_props.get(key).and_then(|v| v.as_str())
                && value == existing_value
            {
                return Some(1.0); // Perfect match for unique identifier
            }
        }
        None
    }

    /// Check if a key represents a unique identifier
    fn is_unique_identifier_key(&self, key: &str) -> bool {
        matches!(
            key.to_lowercase().as_str(),
            "email" | "phone" | "url" | "id" | "username" | "social_security_number" | "passport"
        )
    }

    /// Analyze context overlap between entities
    async fn analyze_context_overlap(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        memory_context: &Memory,
        storage: &dyn GraphStore,
    ) -> Result<f32> {
        let mut overlap_score = 0.0;
        let mut total_factors = 0;

        // 1. Check surrounding words in current context
        let context_score =
            self.calculate_local_context_similarity(extracted, existing, memory_context);
        overlap_score += context_score;
        total_factors += 1;

        // 2. Check co-occurring entities in memories that mention the existing entity
        let cooccurrence_score = self
            .calculate_entity_cooccurrence_score(extracted, existing, storage)
            .await?;
        overlap_score += cooccurrence_score;
        total_factors += 1;

        // 3. Check domain/topic consistency
        let domain_score = self.calculate_domain_consistency(extracted, existing, memory_context);
        overlap_score += domain_score;
        total_factors += 1;

        if total_factors > 0 {
            Ok(overlap_score / total_factors as f32)
        } else {
            Ok(0.5)
        }
    }

    /// Calculate similarity based on local context words
    fn calculate_local_context_similarity(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        memory_context: &Memory,
    ) -> f32 {
        let content = &memory_context.content;
        let window_size = self.config.context_window;

        // Extract context around the entity mention
        let entity_start = extracted.start_pos;
        let entity_end = extracted.end_pos;

        let context_start = entity_start.saturating_sub(window_size);
        let context_end = (entity_end + window_size).min(content.len());

        if context_start < context_end && context_end <= content.len() {
            let context_text = &content[context_start..context_end];

            // Extract context words (simple whitespace tokenization)
            let context_words: Vec<&str> = context_text
                .split_whitespace()
                .filter(|word| word.len() > 2)
                .collect();

            // Check if existing entity has context information
            if let Some(existing_context) = existing
                .properties
                .as_object()
                .and_then(|props| props.get("typical_context"))
                .and_then(|v| v.as_str())
            {
                let existing_words: Vec<&str> = existing_context
                    .split_whitespace()
                    .filter(|word| word.len() > 2)
                    .collect();

                // Calculate word overlap
                let overlap_count = context_words
                    .iter()
                    .filter(|word| existing_words.contains(word))
                    .count();

                let total_unique_words = context_words.len().max(existing_words.len());

                if total_unique_words > 0 {
                    return overlap_count as f32 / total_unique_words as f32;
                }
            }
        }

        0.3 // Default neutral score
    }

    /// Calculate entity co-occurrence score
    async fn calculate_entity_cooccurrence_score(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        storage: &dyn GraphStore,
    ) -> Result<f32> {
        // Find memories that mention the existing entity
        let existing_memories = match self
            .find_memories_mentioning_entity(&existing.id, storage)
            .await
        {
            Ok(memories) => memories,
            Err(_) => return Ok(0.3), // Default score if query fails
        };

        let mut cooccurrence_score = 0.0;
        let mut total_memories = 0;

        for memory in existing_memories.iter().take(10) {
            // Limit to recent/relevant memories
            // Simple approach: check if the extracted entity name appears in these memories
            let content_lower = memory.content.to_lowercase();
            let entity_name_lower = extracted.text.to_lowercase();

            if content_lower.contains(&entity_name_lower) {
                cooccurrence_score += 1.0;
            }
            total_memories += 1;
        }

        if total_memories > 0 {
            Ok(cooccurrence_score / total_memories as f32)
        } else {
            Ok(0.3)
        }
    }

    /// Calculate domain consistency score
    fn calculate_domain_consistency(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        memory_context: &Memory,
    ) -> f32 {
        // Check if entity types are consistent
        let extracted_type = &extracted.entity_type;
        let existing_type = &existing.entity_type;

        if self.entity_types_are_compatible(extracted_type, existing_type) {
            let mut score: f32 = 0.7; // Base score for type compatibility

            // Boost score if tags/topics overlap
            let memory_tags = &memory_context.tags;
            if let Some(entity_tags) = existing
                .properties
                .as_object()
                .and_then(|props| props.get("tags"))
                .and_then(|v| v.as_array())
            {
                let entity_tag_strings: Vec<String> = entity_tags
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();

                let overlap_count = memory_tags
                    .iter()
                    .filter(|tag| entity_tag_strings.contains(tag))
                    .count();

                if overlap_count > 0 {
                    score += 0.2; // Boost for tag overlap
                }
            }

            score.min(1.0)
        } else {
            0.1 // Low score for type mismatch
        }
    }

    /// Check if entity types are compatible
    fn entity_types_are_compatible(&self, type1: &EntityType, type2: &str) -> bool {
        match type1 {
            EntityType::Person => type2 == "person",
            EntityType::Organization => type2 == "organization",
            EntityType::Location => type2 == "location",
            EntityType::Custom(custom) => custom == type2,
            _ => false,
        }
    }

    /// Find memories that mention a specific entity
    async fn find_memories_mentioning_entity(
        &self,
        entity_id: &str,
        storage: &dyn GraphStore,
    ) -> Result<Vec<Memory>> {
        // This would use a proper relationship query in production
        // For now, we'll implement a simple fallback

        // Try to get relationships where this entity is involved
        let relationships = storage
            .list_relationships(None, None, None)
            .await
            .unwrap_or_default();

        let mut memory_ids = Vec::new();
        for rel in relationships {
            // Check if the relationship is entity-memory type and involves our entity
            if rel.relationship_type == "entity_mentions"
                || rel.relationship_type == "contains_entity"
            {
                if rel.source_id == entity_id {
                    // Entity -> Memory relationship
                    memory_ids.push(rel.target_id.clone());
                } else if rel.target_id == entity_id {
                    // Memory -> Entity relationship
                    memory_ids.push(rel.source_id.clone());
                }
            } else if rel.source_id == entity_id || rel.target_id == entity_id {
                // For other relationship types, check if it's connected to this entity
                // and try to find memories on the other end
                let other_id = if rel.source_id == entity_id {
                    &rel.target_id
                } else {
                    &rel.source_id
                };

                // Check if the other end might be a memory (simple heuristic)
                if other_id.starts_with("mem_") || other_id.contains("memory") {
                    memory_ids.push(other_id.clone());
                }
            }
        }

        // Get the actual memories
        let mut memories = Vec::new();
        for memory_id in memory_ids.into_iter().take(20) {
            // Limit results
            if let Ok(Some(memory)) = storage.get_memory(&memory_id).await {
                memories.push(memory);
            }
        }

        Ok(memories)
    }

    /// Check entity co-occurrence patterns
    async fn check_entity_cooccurrence(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
        storage: &dyn GraphStore,
    ) -> Result<f32> {
        // This implementation focuses on how often these entities appear together
        let cooccurrence_score = self
            .calculate_entity_cooccurrence_score(extracted, existing, storage)
            .await?;
        Ok(cooccurrence_score)
    }

    /// Check temporal proximity of entity mentions
    async fn check_temporal_proximity(
        &self,
        memory_context: &Memory,
        existing: &Entity,
        storage: &dyn GraphStore,
    ) -> Result<f32> {
        // Get entity type from existing entity for proper checking
        let entity_type = self.parse_entity_type(&existing.entity_type);

        if !self.should_check_temporal_proximity(&entity_type) {
            return Ok(0.0);
        }

        // Find recent memories containing the existing entity using our existing method
        let recent_memories = self
            .find_memories_mentioning_entity(&existing.id, storage)
            .await?;

        if recent_memories.is_empty() {
            return Ok(0.0);
        }

        let current_time = memory_context.created_at;
        let mut best_score: f32 = 0.0;

        for memory in recent_memories.iter().take(10) {
            let time_diff = (current_time - memory.created_at).num_seconds().abs();

            let temporal_score = if time_diff <= 3600 {
                // Within 1 hour
                0.9
            } else if time_diff <= 24 * 3600 {
                // Within 24 hours
                0.8
            } else if time_diff <= 7 * 24 * 3600 {
                // Within a week
                0.6
            } else if time_diff <= 30 * 24 * 3600 {
                // Within a month
                0.3
            } else {
                0.1
            };

            best_score = best_score.max(temporal_score);
        }

        Ok(best_score)
    }

    /// Parse entity type string to EntityType enum
    fn parse_entity_type(&self, type_str: &str) -> EntityType {
        match type_str.to_lowercase().as_str() {
            "person" => EntityType::Person,
            "organization" => EntityType::Organization,
            "location" => EntityType::Location,
            _ => EntityType::Custom(type_str.to_string()),
        }
    }

    /// Check if temporal proximity should be considered for this entity type
    fn should_check_temporal_proximity(&self, entity_type: &EntityType) -> bool {
        matches!(
            entity_type,
            EntityType::Person | EntityType::Organization | EntityType::Location
        )
    }

    /// Calculate weighted confidence score
    fn calculate_weighted_score(&self, scores: &[(&str, f32)]) -> f32 {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for (score_type, score) in scores {
            let weight = match *score_type {
                "identifiers" => self.config.weights.identifiers,
                "context" => self.config.weights.context,
                "cooccurrence" => self.config.weights.cooccurrence,
                "temporal" => self.config.weights.temporal,
                _ => 0.0,
            };

            weighted_sum += score * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        }
    }
}

/// Decision about what to do with an extracted entity
#[derive(Debug, Clone)]
pub struct EntityCreationDecision {
    pub action: EntityAction,
    pub confidence: f32,
    pub reasoning: String,
}

/// Possible actions for entity creation
#[derive(Debug, Clone)]
pub enum EntityAction {
    /// Create new entity (different from existing)
    CreateNew { suggested_id_suffix: String },
    /// Merge with existing entity
    MergeWith { entity_id: String },
    /// Create alias/reference to existing
    CreateAlias { primary_entity_id: String },
    /// Skip - too ambiguous
    Skip { reason: String },
}

impl EntityResolver {
    /// Decide what action to take for an extracted entity
    pub async fn decide_entity_action(
        &self,
        extracted: &ExtractedEntity,
        candidates: Vec<(Entity, f32)>,
        context: &Memory,
    ) -> Result<EntityCreationDecision> {
        if candidates.is_empty() {
            return Ok(EntityCreationDecision {
                action: EntityAction::CreateNew {
                    suggested_id_suffix: self.generate_id_suffix(extracted, context),
                },
                confidence: 1.0,
                reasoning: "No similar entities found".to_string(),
            });
        }

        let (best_match, confidence) = &candidates[0];

        if *confidence >= 0.95 {
            // Very high confidence - merge
            Ok(EntityCreationDecision {
                action: EntityAction::MergeWith {
                    entity_id: best_match.id.clone(),
                },
                confidence: *confidence,
                reasoning: format!("High confidence match ({})", confidence),
            })
        } else if *confidence >= 0.8 {
            // Good confidence - check for unique identifiers
            if self.has_matching_unique_identifier(extracted, best_match) {
                Ok(EntityCreationDecision {
                    action: EntityAction::MergeWith {
                        entity_id: best_match.id.clone(),
                    },
                    confidence: *confidence,
                    reasoning: "Matching unique identifier found".to_string(),
                })
            } else {
                Ok(EntityCreationDecision {
                    action: EntityAction::CreateAlias {
                        primary_entity_id: best_match.id.clone(),
                    },
                    confidence: *confidence,
                    reasoning: "Good match but no unique identifier - creating alias".to_string(),
                })
            }
        } else if *confidence >= 0.6 {
            // Moderate confidence - create new with disambiguation
            Ok(EntityCreationDecision {
                action: EntityAction::CreateNew {
                    suggested_id_suffix: self.generate_disambiguated_suffix(extracted, context),
                },
                confidence: *confidence,
                reasoning: "Moderate similarity but not confident enough to merge".to_string(),
            })
        } else {
            // Low confidence - create new
            Ok(EntityCreationDecision {
                action: EntityAction::CreateNew {
                    suggested_id_suffix: self.generate_id_suffix(extracted, context),
                },
                confidence: *confidence,
                reasoning: "Low similarity to existing entities".to_string(),
            })
        }
    }

    /// Generate a suffix for entity ID
    fn generate_id_suffix(&self, extracted: &ExtractedEntity, _context: &Memory) -> String {
        let clean_text = extracted
            .text
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();

        format!(
            "{}_{}",
            self.entity_type_to_string(&extracted.entity_type),
            clean_text
        )
    }

    /// Generate a disambiguated suffix when there are similar entities
    fn generate_disambiguated_suffix(
        &self,
        extracted: &ExtractedEntity,
        context: &Memory,
    ) -> String {
        let base_suffix = self.generate_id_suffix(extracted, context);
        let memory_suffix = context.id.chars().take(8).collect::<String>();

        format!("{}_mem_{}", base_suffix, memory_suffix)
    }

    /// Check if there's a matching unique identifier
    fn has_matching_unique_identifier(
        &self,
        extracted: &ExtractedEntity,
        existing: &Entity,
    ) -> bool {
        for (key, value) in &extracted.metadata {
            if self.is_unique_identifier(key)
                && let Some(existing_props) = existing.properties.as_object()
                && let Some(existing_value) = existing_props.get(key).and_then(|v| v.as_str())
                && value == existing_value
            {
                return true;
            }
        }
        false
    }
}
