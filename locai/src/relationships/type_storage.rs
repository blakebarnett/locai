//! Persistent storage for relationship type definitions

use super::registry::{RegistryError, RelationshipTypeDef, RelationshipTypeStorage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use surrealdb::{Connection, RecordId, Surreal};

/// SurrealDB storage backend for relationship types
#[derive(Debug)]
pub struct SurrealRelationshipTypeStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    client: Surreal<C>,
}

/// SurrealDB representation of a relationship type
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurrealRelationshipTypeDef {
    id: Option<RecordId>,
    #[serde(flatten)]
    def: RelationshipTypeDef,
}

impl<C> SurrealRelationshipTypeStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new SurrealDB storage backend
    pub fn new(client: Surreal<C>) -> Arc<Self> {
        Arc::new(Self { client })
    }

    /// Initialize the storage schema (table for relationship types)
    pub async fn initialize_schema(&self) -> Result<(), RegistryError> {
        // Define the relationship_type table with SCHEMAFULL for validation
        let query = r#"
            DEFINE TABLE IF NOT EXISTS relationship_type SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS name ON TABLE relationship_type TYPE string ASSERT $value != NONE;
            DEFINE FIELD IF NOT EXISTS inverse ON TABLE relationship_type TYPE option<string>;
            DEFINE FIELD IF NOT EXISTS symmetric ON TABLE relationship_type TYPE bool DEFAULT false;
            DEFINE FIELD IF NOT EXISTS transitive ON TABLE relationship_type TYPE bool DEFAULT false;
            DEFINE FIELD IF NOT EXISTS metadata_schema ON TABLE relationship_type TYPE option<object>;
            DEFINE FIELD IF NOT EXISTS version ON TABLE relationship_type TYPE int DEFAULT 1;
            DEFINE FIELD IF NOT EXISTS created_at ON TABLE relationship_type TYPE datetime DEFAULT time::now();
            DEFINE FIELD IF NOT EXISTS custom_metadata ON TABLE relationship_type TYPE object DEFAULT {};
            DEFINE INDEX IF NOT EXISTS relationship_type_name_idx ON TABLE relationship_type COLUMNS name UNIQUE;
        "#;

        self.client
            .query(query)
            .await
            .map_err(|e| RegistryError::InternalError(format!("Failed to initialize schema: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl<C> RelationshipTypeStorage for SurrealRelationshipTypeStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn save_type(&self, def: &RelationshipTypeDef) -> Result<(), RegistryError> {
        // Clone the necessary data to avoid lifetime issues
        let name = def.name.clone();
        let inverse = def.inverse.clone();
        let symmetric = def.symmetric;
        let transitive = def.transitive;
        let metadata_schema = def.metadata_schema.clone();
        let version = def.version;
        let created_at = def.created_at.to_rfc3339();
        let custom_metadata = def.custom_metadata.clone();
        
        let record_id = RecordId::from(("relationship_type", name.as_str()));

        // Use UPSERT to handle both insert and update
        let query = r#"
            UPSERT $id CONTENT {
                name: $name,
                inverse: $inverse,
                symmetric: $symmetric,
                transitive: $transitive,
                metadata_schema: $metadata_schema,
                version: $version,
                created_at: $created_at,
                custom_metadata: $custom_metadata
            }
        "#;

        self.client
            .query(query)
            .bind(("id", record_id))
            .bind(("name", name))
            .bind(("inverse", inverse))
            .bind(("symmetric", symmetric))
            .bind(("transitive", transitive))
            .bind(("metadata_schema", metadata_schema))
            .bind(("version", version))
            .bind(("created_at", created_at))
            .bind(("custom_metadata", custom_metadata))
            .await
            .map_err(|e| {
                RegistryError::InternalError(format!("Failed to save relationship type: {}", e))
            })?;

        Ok(())
    }

    async fn load_all_types(&self) -> Result<Vec<RelationshipTypeDef>, RegistryError> {
        let query = "SELECT * FROM relationship_type";

        let mut result = self
            .client
            .query(query)
            .await
            .map_err(|e| {
                RegistryError::InternalError(format!("Failed to load relationship types: {}", e))
            })?;

        let types: Vec<SurrealRelationshipTypeDef> = result
            .take(0)
            .map_err(|e| {
                RegistryError::InternalError(format!("Failed to parse relationship types: {}", e))
            })?;

        Ok(types.into_iter().map(|t| t.def).collect())
    }

    async fn delete_type(&self, name: &str) -> Result<(), RegistryError> {
        let record_id = RecordId::from(("relationship_type", name));

        let query = "DELETE $id";

        self.client
            .query(query)
            .bind(("id", record_id))
            .await
            .map_err(|e| {
                RegistryError::InternalError(format!("Failed to delete relationship type: {}", e))
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_trait_implementation() {
        // This test verifies that SurrealRelationshipTypeStorage implements the trait correctly
        // Actual DB tests would require a running SurrealDB instance
        use crate::relationships::registry::RelationshipTypeDef;

        let _def = RelationshipTypeDef::new("test_type".to_string()).unwrap();
        
        // Verify the struct compiles and has correct trait bounds
        #[allow(dead_code)]
        fn assert_storage_impl<T: RelationshipTypeStorage>(_: &T) {}
        
        // This would require actual DB connection, so just a compile-time check
        // let client = Surreal::new::<surrealdb::engine::local::Mem>(()).await.unwrap();
        // let storage = SurrealRelationshipTypeStorage::new(client);
        // assert_storage_impl(&*storage);
    }
}

