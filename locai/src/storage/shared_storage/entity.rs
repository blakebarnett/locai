//! Entity storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use surrealdb::{Connection, RecordId};

use super::base::SharedStorage;
use crate::storage::errors::StorageError;
use crate::storage::filters::EntityFilter;
use crate::storage::models::Entity;
use crate::storage::traits::EntityStore;

/// Internal representation of an Entity record for SurrealDB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SurrealEntity {
    id: RecordId,
    entity_type: String,
    properties: Value,
    owner: RecordId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Struct for creating entities (without generated fields)
#[derive(Debug, Clone, serde::Serialize)]
struct CreateEntity {
    entity_type: String,
    properties: Value,
    owner: RecordId,
}

impl From<Entity> for SurrealEntity {
    fn from(entity: Entity) -> Self {
        Self {
            id: RecordId::from(("entity", entity.id.as_str())),
            entity_type: entity.entity_type,
            properties: entity.properties,
            owner: RecordId::from(("user", "system")),
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}

impl From<SurrealEntity> for Entity {
    fn from(surreal_entity: SurrealEntity) -> Self {
        // Extract the key string from RecordId
        // RecordId::key() returns a RecordId, and to_string() includes brackets ⟨⟩
        // We need to extract just the key part without brackets
        let key_record = surreal_entity.id.key();
        let key_string = key_record.to_string();
        // Remove angle brackets if present: ⟨key⟩ -> key
        let clean_id = key_string
            .strip_prefix('⟨')
            .and_then(|s| s.strip_suffix('⟩'))
            .unwrap_or(&key_string)
            .to_string();
        
        Self {
            id: clean_id,
            entity_type: surreal_entity.entity_type,
            properties: surreal_entity.properties,
            created_at: surreal_entity.created_at,
            updated_at: surreal_entity.updated_at,
        }
    }
}

#[async_trait]
impl<C> EntityStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new entity
    async fn create_entity(&self, entity: Entity) -> Result<Entity, StorageError> {
        // First ensure system user exists
        self.ensure_system_user().await?;

        // Create a struct for creation (timestamps handled by SurrealDB)
        let create_entity = CreateEntity {
            entity_type: entity.entity_type.clone(),
            properties: entity.properties.clone(),
            owner: RecordId::from(("user", "system")),
        };

        // Use the provided ID if available, otherwise let SurrealDB generate one
        let created: Option<SurrealEntity> = if !entity.id.is_empty() {
            self.client
                .create(("entity", entity.id.as_str()))
                .content(create_entity)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to create entity: {}", e)))?
        } else {
            self.client
                .create("entity")
                .content(create_entity)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to create entity: {}", e)))?
        };

        created
            .map(Entity::from)
            .ok_or_else(|| StorageError::Internal("No entity created".to_string()))
    }

    /// Get an entity by its ID
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>, StorageError> {
        // Use the SDK's select method
        let entity: Option<SurrealEntity> = self
            .client
            .select(("entity", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get entity: {}", e)))?;

        Ok(entity.map(Entity::from))
    }

    /// Update an existing entity
    async fn update_entity(&self, entity: Entity) -> Result<Entity, StorageError> {
        // Use a query to update, letting SurrealDB handle updated_at automatically
        let update_query = r#"
            UPDATE $record_id MERGE {
                entity_type: $entity_type,
                properties: $properties,
                updated_at: time::now()
            }
        "#;

        let mut response = self
            .client
            .query(update_query)
            .bind(("record_id", RecordId::from(("entity", entity.id.as_str()))))
            .bind(("entity_type", entity.entity_type.clone()))
            .bind(("properties", entity.properties.clone()))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to update entity: {}", e)))?;

        let updated: Option<SurrealEntity> = response
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract updated entity: {}", e)))?;

        updated.map(Entity::from).ok_or_else(|| {
            StorageError::NotFound(format!("Entity with id {} not found", entity.id))
        })
    }

    /// Delete an entity by its ID
    async fn delete_entity(&self, id: &str) -> Result<bool, StorageError> {

        // Use the SDK's delete method for the entity record
        // Note: SurrealDB will handle cascade deletion of related records automatically if configured
        let deleted: Option<SurrealEntity> = self
            .client
            .delete(("entity", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to delete entity: {}", e)))?;

        Ok(deleted.is_some())
    }

    /// List entities with optional filtering
    async fn list_entities(
        &self,
        filter: Option<EntityFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Entity>, StorageError> {
        // If no filters, use simple SDK select
        if filter.is_none() && limit.is_none() && offset.is_none() {
            let entities: Vec<SurrealEntity> = self
                .client
                .select("entity")
                .await
                .map_err(|e| StorageError::Query(format!("Failed to list entities: {}", e)))?;

            return Ok(entities.into_iter().map(Entity::from).collect());
        }

        // For complex filtering, still use raw queries but with better syntax
        let mut query = "SELECT * FROM entity".to_string();
        let mut conditions = Vec::new();

        // Add filter conditions
        if let Some(f) = &filter {
            if let Some(ids) = &f.ids
                && !ids.is_empty()
            {
                let id_list = ids
                    .iter()
                    .map(|id| format!("entity:{}", id))
                    .collect::<Vec<_>>()
                    .join(", ");
                conditions.push(format!("id IN [{}]", id_list));
            }

            if let Some(entity_type) = &f.entity_type {
                conditions.push(format!("entity_type = '{}'", entity_type));
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

            // Handle property filtering
            if let Some(properties) = &f.properties {
                for (key, value) in properties {
                    match value {
                        Value::String(s) => {
                            conditions.push(format!("properties.{} = '{}'", key, s));
                        }
                        Value::Number(n) => {
                            conditions.push(format!("properties.{} = {}", key, n));
                        }
                        Value::Bool(b) => {
                            conditions.push(format!("properties.{} = {}", key, b));
                        }
                        _ => {
                            conditions.push(format!("properties.{} = {}", key, value));
                        }
                    }
                }
            }
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" START {}", offset));
        }

        let mut result = self
            .client
            .query(&query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list entities: {}", e)))?;

        let entities: Vec<SurrealEntity> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract entities: {}", e)))?;

        Ok(entities.into_iter().map(Entity::from).collect())
    }

    /// Count entities with optional filtering
    async fn count_entities(&self, filter: Option<EntityFilter>) -> Result<usize, StorageError> {
        // Simple approach: get all entities matching the filter and count them
        let entities = self.list_entities(filter, None, None).await?;
        Ok(entities.len())
    }
}
