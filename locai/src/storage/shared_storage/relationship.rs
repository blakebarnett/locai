//! Relationship storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use surrealdb::{Connection, RecordId};

use super::base::SharedStorage;
use crate::storage::errors::StorageError;
use crate::storage::filters::RelationshipFilter;
use crate::storage::models::{Entity, Relationship};
use crate::storage::traits::{EntityStore, MemoryStore, RelationshipStore};

/// Internal representation of a Relationship record for SurrealDB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SurrealRelationship {
    id: RecordId,
    relationship_type: String,
    source_id: String,
    target_id: String,
    properties: Value,
    owner: RecordId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Struct for creating relationships (without generated fields)
#[derive(Debug, Clone, serde::Serialize)]
struct CreateRelationship {
    relationship_type: String,
    source_id: String,
    target_id: String,
    properties: Value,
    owner: RecordId,
}

impl From<Relationship> for SurrealRelationship {
    fn from(relationship: Relationship) -> Self {
        Self {
            id: RecordId::from(("relationship", relationship.id.as_str())),
            relationship_type: relationship.relationship_type,
            source_id: relationship.source_id,
            target_id: relationship.target_id,
            properties: relationship.properties,
            owner: RecordId::from(("user", "system")),
            created_at: relationship.created_at,
            updated_at: relationship.updated_at,
        }
    }
}

impl From<SurrealRelationship> for Relationship {
    fn from(surreal_relationship: SurrealRelationship) -> Self {
        Self {
            id: surreal_relationship.id.key().to_string(),
            relationship_type: surreal_relationship.relationship_type,
            source_id: surreal_relationship.source_id,
            target_id: surreal_relationship.target_id,
            properties: surreal_relationship.properties,
            created_at: surreal_relationship.created_at,
            updated_at: surreal_relationship.updated_at,
        }
    }
}

#[async_trait]
impl<C> RelationshipStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new relationship
    async fn create_relationship(
        &self,
        relationship: Relationship,
    ) -> Result<Relationship, StorageError> {
        // First ensure system user exists
        self.ensure_system_user().await?;

        // Validate source and target based on relationship type
        match relationship.relationship_type.as_str() {
            "contains" | "mentions" | "references_entity" | "has_entity" => {
                // For memory->entity relationships: memory -> entity
                // Verify source is a memory
                if self.get_memory(&relationship.source_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Source memory with ID {} not found",
                        relationship.source_id
                    )));
                }

                // Verify target is an entity
                if self.get_entity(&relationship.target_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Target entity with ID {} not found",
                        relationship.target_id
                    )));
                }
            }
            "references" => {
                // For references relationships: memory -> relationship
                // Verify source is a memory
                if self.get_memory(&relationship.source_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Source memory with ID {} not found",
                        relationship.source_id
                    )));
                }

                // Verify target is a relationship
                if self
                    .get_relationship(&relationship.target_id)
                    .await?
                    .is_none()
                {
                    return Err(StorageError::NotFound(format!(
                        "Target relationship with ID {} not found",
                        relationship.target_id
                    )));
                }
            }
            "entity_coreference" | "temporal_sequence" | "topic_similarity" => {
                // For memory->memory relationships (automatic relationships): memory -> memory
                // Verify source is a memory
                if self.get_memory(&relationship.source_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Source memory with ID {} not found",
                        relationship.source_id
                    )));
                }

                // Verify target is a memory
                if self.get_memory(&relationship.target_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Target memory with ID {} not found",
                        relationship.target_id
                    )));
                }
            }
            _ => {
                // For all other relationships (like "relates", "emphasizes", etc.): entity -> entity
                // Verify that both source and target entities exist
                if self.get_entity(&relationship.source_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Source entity with ID {} not found",
                        relationship.source_id
                    )));
                }

                if self.get_entity(&relationship.target_id).await?.is_none() {
                    return Err(StorageError::NotFound(format!(
                        "Target entity with ID {} not found",
                        relationship.target_id
                    )));
                }
            }
        }

        // Create a struct for creation (timestamps handled by SurrealDB)
        let create_relationship = CreateRelationship {
            relationship_type: relationship.relationship_type.clone(),
            source_id: relationship.source_id.clone(),
            target_id: relationship.target_id.clone(),
            properties: relationship.properties.clone(),
            owner: RecordId::from(("user", "system")),
        };

        // Let SurrealDB auto-generate the ID
        let created: Option<SurrealRelationship> = self
            .client
            .create("relationship")
            .content(create_relationship)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to create relationship: {}", e)))?;

        let created_relationship = created
            .map(Relationship::from)
            .ok_or_else(|| StorageError::Internal("No relationship created".to_string()))?;

        // Create appropriate edge table entries based on relationship type
        match relationship.relationship_type.as_str() {
            "contains" | "mentions" | "references_entity" | "has_entity" => {
                // Create memory->contains->entity edge
                let edge_query = r#"
                    RELATE $source_memory->contains->$target_entity CONTENT {
                        relationship_type: $relationship_type,
                        properties: $properties,
                        confidence: 1.0
                    }
                "#;

                let source_id = relationship.source_id.clone();
                let target_id = relationship.target_id.clone();
                let rel_type = relationship.relationship_type.clone();
                let props = relationship.properties.clone();

                let _edge_result = self
                    .client
                    .query(edge_query)
                    .bind((
                        "source_memory",
                        RecordId::from(("memory", source_id.as_str())),
                    ))
                    .bind((
                        "target_entity",
                        RecordId::from(("entity", target_id.as_str())),
                    ))
                    .bind(("relationship_type", rel_type))
                    .bind(("properties", props))
                    .await
                    .map_err(|e| {
                        StorageError::Query(format!("Failed to create contains edge: {}", e))
                    })?;
            }
            "references" => {
                // Create memory->references->relationship edge
                let edge_query = r#"
                    RELATE $source_memory->references->$target_relationship CONTENT {
                        relationship_type: $relationship_type,
                        properties: $properties,
                        confidence: 1.0
                    }
                "#;

                let source_id = relationship.source_id.clone();
                let target_id = relationship.target_id.clone();
                let rel_type = relationship.relationship_type.clone();
                let props = relationship.properties.clone();

                let _edge_result = self
                    .client
                    .query(edge_query)
                    .bind((
                        "source_memory",
                        RecordId::from(("memory", source_id.as_str())),
                    ))
                    .bind((
                        "target_relationship",
                        RecordId::from(("relationship", target_id.as_str())),
                    ))
                    .bind(("relationship_type", rel_type))
                    .bind(("properties", props))
                    .await
                    .map_err(|e| {
                        StorageError::Query(format!("Failed to create references edge: {}", e))
                    })?;
            }
            _ => {
                // Create entity->relates->entity edge for other relationship types
                let edge_query = r#"
                    RELATE $source_entity->relates->$target_entity CONTENT {
                        relationship_type: $relationship_type,
                        properties: $properties,
                        confidence: 1.0
                    }
                "#;

                let source_id = relationship.source_id.clone();
                let target_id = relationship.target_id.clone();
                let rel_type = relationship.relationship_type.clone();
                let props = relationship.properties.clone();

                let _edge_result = self
                    .client
                    .query(edge_query)
                    .bind((
                        "source_entity",
                        RecordId::from(("entity", source_id.as_str())),
                    ))
                    .bind((
                        "target_entity",
                        RecordId::from(("entity", target_id.as_str())),
                    ))
                    .bind(("relationship_type", rel_type))
                    .bind(("properties", props))
                    .await
                    .map_err(|e| {
                        StorageError::Query(format!("Failed to create relates edge: {}", e))
                    })?;
            }
        }

        Ok(created_relationship)
    }

    /// Get a relationship by its ID
    async fn get_relationship(&self, id: &str) -> Result<Option<Relationship>, StorageError> {
        let relationship: Option<SurrealRelationship> = self
            .client
            .select(("relationship", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get relationship: {}", e)))?;

        Ok(relationship.map(Relationship::from))
    }

    /// Update an existing relationship
    async fn update_relationship(
        &self,
        relationship: Relationship,
    ) -> Result<Relationship, StorageError> {
        // Use MERGE to update specific fields while preserving created_at
        let merge_query = r#"
            UPDATE $record_id MERGE {
                relationship_type: $relationship_type,
                source_id: $source_id,
                target_id: $target_id,
                properties: $properties,
                owner: $owner,
                updated_at: time::now()
            }
        "#;

        let mut response = self
            .client
            .query(merge_query)
            .bind((
                "record_id",
                RecordId::from(("relationship", relationship.id.as_str())),
            ))
            .bind(("relationship_type", relationship.relationship_type.clone()))
            .bind(("source_id", relationship.source_id.clone()))
            .bind(("target_id", relationship.target_id.clone()))
            .bind(("properties", relationship.properties.clone()))
            .bind(("owner", RecordId::from(("user", "system"))))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to update relationship: {}", e)))?;

        let updated: Option<SurrealRelationship> = response.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to extract updated relationship: {}", e))
        })?;

        let updated_relationship = updated.map(Relationship::from).ok_or_else(|| {
            StorageError::NotFound(format!(
                "Relationship with id {} not found",
                relationship.id
            ))
        })?;

        // Also update edge table entry - just run the update without extracting result
        let edge_update_query = r#"
            UPDATE relates 
            SET relationship_type = $relationship_type,
                properties = $properties
            WHERE in = type::thing("entity", $source_id) 
            AND out = type::thing("entity", $target_id)
        "#;

        let source_id = relationship.source_id.clone();
        let target_id = relationship.target_id.clone();
        let rel_type = relationship.relationship_type.clone();
        let props = relationship.properties.clone();

        let _edge_result = self
            .client
            .query(edge_update_query)
            .bind(("source_id", source_id))
            .bind(("target_id", target_id))
            .bind(("relationship_type", rel_type))
            .bind(("properties", props))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to update edge: {}", e)))?;

        Ok(updated_relationship)
    }

    /// Delete a relationship by its ID
    async fn delete_relationship(&self, id: &str) -> Result<bool, StorageError> {
        // Get the relationship first to get source and target IDs for edge cleanup
        let relationship = match self.get_relationship(id).await? {
            Some(rel) => rel,
            None => return Ok(false), // Already doesn't exist
        };

        // Delete from relationship table
        let deleted: Option<SurrealRelationship> = self
            .client
            .delete(("relationship", id))
            .await
            .map_err(|e| {
            StorageError::Query(format!("Failed to delete relationship: {}", e))
        })?;

        if deleted.is_some() {
            // Also delete from edge table
            let edge_delete_query = r#"
                DELETE relates 
                WHERE in = type::thing("entity", $source_id) 
                AND out = type::thing("entity", $target_id)
                AND relationship_type = $relationship_type
            "#;

            let source_id = relationship.source_id.clone();
            let target_id = relationship.target_id.clone();
            let rel_type = relationship.relationship_type.clone();

            let _edge_result: Option<Value> = self
                .client
                .query(edge_delete_query)
                .bind(("source_id", source_id))
                .bind(("target_id", target_id))
                .bind(("relationship_type", rel_type))
                .await
                .map_err(|e| StorageError::Query(format!("Failed to delete edge: {}", e)))?
                .take(0)
                .map_err(|e| {
                    StorageError::Query(format!("Failed to extract edge delete result: {}", e))
                })?;
        }

        Ok(deleted.is_some())
    }

    /// List relationships with optional filtering
    async fn list_relationships(
        &self,
        filter: Option<RelationshipFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Relationship>, StorageError> {
        // If no filters, use simple SDK select
        if filter.is_none() && limit.is_none() && offset.is_none() {
            let relationships: Vec<SurrealRelationship> =
                self.client.select("relationship").await.map_err(|e| {
                    StorageError::Query(format!("Failed to list relationships: {}", e))
                })?;

            return Ok(relationships.into_iter().map(Relationship::from).collect());
        }

        // For complex filtering, use raw queries
        let mut query = "SELECT * FROM relationship".to_string();
        let mut conditions = Vec::new();

        // Add filter conditions
        if let Some(f) = &filter {
            if let Some(ids) = &f.ids {
                if !ids.is_empty() {
                    let id_list = ids
                        .iter()
                        .map(|id| format!("relationship:{}", id))
                        .collect::<Vec<_>>()
                        .join(", ");
                    conditions.push(format!("id IN [{}]", id_list));
                }
            }

            if let Some(relationship_type) = &f.relationship_type {
                conditions.push(format!("relationship_type = '{}'", relationship_type));
            }

            if let Some(source_id) = &f.source_id {
                conditions.push(format!("source_id = '{}'", source_id));
            }

            if let Some(target_id) = &f.target_id {
                conditions.push(format!("target_id = '{}'", target_id));
            }

            if let Some(created_after) = &f.created_after {
                conditions.push(format!("created_at > d'{}'", created_after.to_rfc3339()));
            }

            if let Some(created_before) = &f.created_before {
                conditions.push(format!("created_at < d'{}'", created_before.to_rfc3339()));
            }

            if let Some(updated_after) = &f.updated_after {
                conditions.push(format!("updated_at > d'{}'", updated_after.to_rfc3339()));
            }

            if let Some(updated_before) = &f.updated_before {
                conditions.push(format!("updated_at < d'{}'", updated_before.to_rfc3339()));
            }

            // Handle properties filtering
            if let Some(properties) = &f.properties {
                for (key, value) in properties {
                    let value_str = match value {
                        Value::String(s) => format!("'{}'", s),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => format!("'{}'", value),
                    };
                    conditions.push(format!("properties.{} = {}", key, value_str));
                }
            }
        }

        // Add WHERE clause if we have conditions
        if !conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        // Add LIMIT and OFFSET
        if let Some(limit_val) = limit {
            query.push_str(&format!(" LIMIT {}", limit_val));
        }

        if let Some(offset_val) = offset {
            query.push_str(&format!(" START {}", offset_val));
        }

        let mut response = self
            .client
            .query(&query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list relationships: {}", e)))?;

        let relationships: Vec<SurrealRelationship> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract relationships: {}", e)))?;

        Ok(relationships.into_iter().map(Relationship::from).collect())
    }

    /// Count relationships with optional filtering
    async fn count_relationships(
        &self,
        filter: Option<RelationshipFilter>,
    ) -> Result<usize, StorageError> {
        // Simple approach: get all relationships matching the filter and count them
        let relationships = self.list_relationships(filter, None, None).await?;
        Ok(relationships.len())
    }

    /// Get a relationship by source and target entities
    async fn get_relationship_by_entities(
        &self,
        source_id: &str,
        target_id: &str,
    ) -> Result<Option<Relationship>, StorageError> {
        let query = r#"
            SELECT * FROM relationship 
            WHERE source_id = $source_id AND target_id = $target_id
            LIMIT 1
        "#;

        let source_id = source_id.to_string();
        let target_id = target_id.to_string();

        let mut response = self
            .client
            .query(query)
            .bind(("source_id", source_id))
            .bind(("target_id", target_id))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get relationship by entities: {}", e))
            })?;

        let relationships: Vec<SurrealRelationship> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract relationship: {}", e)))?;

        Ok(relationships.into_iter().next().map(Relationship::from))
    }

    /// Get properties of a relationship
    async fn get_relationship_properties(&self, id: &str) -> Result<Value, StorageError> {
        let relationship = self.get_relationship(id).await?;

        match relationship {
            Some(rel) => Ok(rel.properties),
            None => Err(StorageError::NotFound(format!(
                "Relationship with ID {} not found",
                id
            ))),
        }
    }

    /// Find relationships between two entities
    async fn find_relationships(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: Option<String>,
    ) -> Result<Vec<Relationship>, StorageError> {
        let mut query = r#"
            SELECT * FROM relationship 
            WHERE source_id = $source_id AND target_id = $target_id
        "#
        .to_string();

        if relationship_type.is_some() {
            query.push_str(" AND relationship_type = $relationship_type");
        }

        let source_id = source_id.to_string();
        let target_id = target_id.to_string();

        let mut query_builder = self
            .client
            .query(&query)
            .bind(("source_id", source_id))
            .bind(("target_id", target_id));

        if let Some(rel_type) = relationship_type {
            query_builder = query_builder.bind(("relationship_type", rel_type));
        }

        let mut response = query_builder
            .await
            .map_err(|e| StorageError::Query(format!("Failed to find relationships: {}", e)))?;

        let relationships: Vec<SurrealRelationship> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract relationships: {}", e)))?;

        Ok(relationships.into_iter().map(Relationship::from).collect())
    }

    /// Find entities related to the given entity
    async fn find_related_entities(
        &self,
        entity_id: &str,
        relationship_type: Option<String>,
        direction: Option<String>,
    ) -> Result<Vec<Entity>, StorageError> {
        let direction = direction.as_deref().unwrap_or("both");

        // For "both" direction, we need to run two separate queries and combine results
        let mut all_entity_ids: Vec<RecordId> = Vec::new();

        if direction == "outgoing" || direction == "both" {
            let outgoing_query = if relationship_type.is_some() {
                r#"
                    SELECT VALUE out FROM relates 
                    WHERE in = type::thing("entity", $entity_id) 
                    AND relationship_type = $relationship_type
                "#
            } else {
                r#"
                    SELECT VALUE out FROM relates 
                    WHERE in = type::thing("entity", $entity_id)
                "#
            };

            let entity_id_str = entity_id.to_string();
            let mut query_builder = self
                .client
                .query(outgoing_query)
                .bind(("entity_id", entity_id_str));

            if let Some(rel_type) = &relationship_type {
                query_builder = query_builder.bind(("relationship_type", rel_type.clone()));
            }

            let mut response = query_builder.await.map_err(|e| {
                StorageError::Query(format!("Failed to find outgoing related entities: {}", e))
            })?;

            if let Ok(ids) = response.take::<Vec<RecordId>>(0) {
                all_entity_ids.extend(ids);
            }
        }

        if direction == "incoming" || direction == "both" {
            let incoming_query = if relationship_type.is_some() {
                r#"
                    SELECT VALUE in FROM relates 
                    WHERE out = type::thing("entity", $entity_id) 
                    AND relationship_type = $relationship_type
                "#
            } else {
                r#"
                    SELECT VALUE in FROM relates 
                    WHERE out = type::thing("entity", $entity_id)
                "#
            };

            let entity_id_str = entity_id.to_string();
            let mut query_builder = self
                .client
                .query(incoming_query)
                .bind(("entity_id", entity_id_str));

            if let Some(rel_type) = &relationship_type {
                query_builder = query_builder.bind(("relationship_type", rel_type.clone()));
            }

            let mut response = query_builder.await.map_err(|e| {
                StorageError::Query(format!("Failed to find incoming related entities: {}", e))
            })?;

            if let Ok(ids) = response.take::<Vec<RecordId>>(0) {
                all_entity_ids.extend(ids);
            }
        }

        // Remove duplicates
        all_entity_ids.sort_by_key(|a| a.key().to_string());
        all_entity_ids.dedup_by(|a, b| a.key() == b.key());

        // Now get the actual entities
        let mut all_entities = Vec::new();
        for entity_id in all_entity_ids {
            if let Some(entity) = self.get_entity(&entity_id.key().to_string()).await? {
                all_entities.push(entity);
            }
        }

        Ok(all_entities)
    }
}
