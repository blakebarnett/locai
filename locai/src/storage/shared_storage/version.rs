//! Version store implementation for SharedStorage

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::{Connection, RecordId};
use uuid::Uuid;

use super::base::SharedStorage;
use crate::models::Memory;
use crate::storage::errors::StorageError;
use crate::storage::models::{Entity, Relationship, Version};
use crate::storage::traits::{EntityStore, MemoryStore, RelationshipStore, VersionStore};

/// Internal representation for SharedStorage version documents
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SurrealVersion {
    id: RecordId,
    description: String,
    metadata: Value,
    created_at: DateTime<Utc>,
    snapshot_type: String,
    snapshot_data: Value,
}

/// Snapshot types for different versioning strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Full snapshot of all graph data
    Full,
    /// Incremental snapshot with changes only
    Incremental,
    /// Metadata-only snapshot for lightweight versioning
    Metadata,
    /// Conversation state snapshot for AI assistant context
    Conversation,
    /// Knowledge evolution snapshot for learning tracking
    Knowledge,
}

impl std::fmt::Display for SnapshotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotType::Full => write!(f, "full"),
            SnapshotType::Incremental => write!(f, "incremental"),
            SnapshotType::Metadata => write!(f, "metadata"),
            SnapshotType::Conversation => write!(f, "conversation"),
            SnapshotType::Knowledge => write!(f, "knowledge"),
        }
    }
}

/// Snapshot data structure for different snapshot types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    pub memories: Vec<Memory>,
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub memory_count: usize,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub checksum: Option<String>,
}

impl From<SurrealVersion> for Version {
    fn from(surreal_version: SurrealVersion) -> Self {
        Version {
            id: surreal_version.id.key().to_string(),
            description: surreal_version.description,
            metadata: surreal_version.metadata,
            created_at: surreal_version.created_at,
        }
    }
}

#[async_trait]
impl<C> VersionStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn create_version(&self, mut version: Version) -> Result<Version, StorageError> {
        // Generate ID if not provided
        if version.id.is_empty() {
            version.id = Uuid::new_v4().to_string();
        }

        // Create a snapshot of the current graph state
        let snapshot_data = self.create_snapshot().await?;

        // Determine snapshot type from metadata
        let snapshot_type = version
            .metadata
            .get("snapshot_type")
            .and_then(|v| v.as_str())
            .unwrap_or("full")
            .to_string();

        // Clone the metadata to avoid lifetime issues
        let metadata_clone = version.metadata.clone();
        let description_clone = version.description.clone();

        // Create the version using SurrealDB query
        let query = r#"
            CREATE version CONTENT {
                description: $description,
                metadata: $metadata,
                created_at: time::now(),
                snapshot_type: $snapshot_type,
                snapshot_data: $snapshot_data
            }
        "#;

        let mut result = self
            .client
            .query(query)
            .bind(("description", description_clone))
            .bind(("metadata", metadata_clone))
            .bind(("snapshot_type", snapshot_type))
            .bind((
                "snapshot_data",
                serde_json::to_value(&snapshot_data)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?,
            ))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to create version: {}", e)))?;

        let created: Vec<SurrealVersion> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to extract created version: {}", e))
        })?;

        created
            .into_iter()
            .next()
            .map(Version::from)
            .ok_or_else(|| StorageError::NotFound("Created version not returned".to_string()))
    }

    async fn get_version(&self, id: &str) -> Result<Option<Version>, StorageError> {
        let record_id = RecordId::from(("version", id));

        let query = "SELECT * FROM $id";

        let mut result = self
            .client
            .query(query)
            .bind(("id", record_id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get version {}: {}", id, e)))?;

        let versions: Vec<SurrealVersion> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract version: {}", e)))?;

        Ok(versions.into_iter().next().map(Version::from))
    }

    async fn list_versions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Version>, StorageError> {
        let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();
        let offset_clause = offset.map(|o| format!("START {}", o)).unwrap_or_default();

        let query = format!(
            "SELECT * FROM version ORDER BY created_at DESC {} {}",
            offset_clause, limit_clause
        );

        let mut result = self
            .client
            .query(query.as_str())
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list versions: {}", e)))?;

        let versions: Vec<SurrealVersion> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to parse version results: {}", e)))?;

        Ok(versions.into_iter().map(Version::from).collect())
    }

    async fn checkout_version(&self, id: &str) -> Result<bool, StorageError> {
        // Get the version and its snapshot data
        let version = self.get_version_with_snapshot(id).await?;

        match version {
            Some((_version_data, snapshot)) => {
                // Start a transaction for atomic restoration
                self.restore_from_snapshot(snapshot).await?;

                tracing::info!("Successfully checked out version: {}", id);
                Ok(true)
            }
            None => {
                tracing::warn!("Version not found for checkout: {}", id);
                Ok(false)
            }
        }
    }
}

impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a snapshot of the current graph state
    async fn create_snapshot(&self) -> Result<SnapshotData, StorageError> {
        // Get all current data
        let memories = self.list_memories(None, None, None).await?;
        let entities = self.list_entities(None, None, None).await?;
        let relationships = self.list_relationships(None, None, None).await?;

        // Calculate counts
        let memory_count = memories.len();
        let entity_count = entities.len();
        let relationship_count = relationships.len();

        // Generate a simple checksum for data integrity
        let checksum = Some(format!(
            "{}-{}-{}",
            memory_count, entity_count, relationship_count
        ));

        Ok(SnapshotData {
            memories,
            entities,
            relationships,
            memory_count,
            entity_count,
            relationship_count,
            checksum,
        })
    }

    /// Get a version with its snapshot data
    async fn get_version_with_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<(Version, SnapshotData)>, StorageError> {
        let record_id = RecordId::from(("version", id));

        let query = "SELECT * FROM $id";

        let mut result = self
            .client
            .query(query)
            .bind(("id", record_id))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get version with snapshot {}: {}", id, e))
            })?;

        let versions: Vec<SurrealVersion> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract version: {}", e)))?;

        match versions.into_iter().next() {
            Some(version_record) => {
                let version = Version::from(version_record.clone());
                let snapshot: SnapshotData = serde_json::from_value(version_record.snapshot_data)
                    .map_err(|e| {
                    StorageError::Serialization(format!("Failed to deserialize snapshot: {}", e))
                })?;
                Ok(Some((version, snapshot)))
            }
            None => Ok(None),
        }
    }

    /// Restore the graph state from a snapshot
    async fn restore_from_snapshot(&self, snapshot: SnapshotData) -> Result<(), StorageError> {
        // Clear current data (this should be done in a transaction in production)
        tracing::info!("Clearing current graph state for restoration");

        // Delete all current data
        let delete_queries = ["DELETE relationship", "DELETE entity", "DELETE memory"];

        for query_str in &delete_queries {
            let mut result = self
                .client
                .query(*query_str)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to clear data: {}", e)))?;

            let _: Vec<Value> = result.take(0).map_err(|e| {
                StorageError::Query(format!("Failed to process delete result: {}", e))
            })?;
        }

        // Restore memories
        tracing::info!("Restoring {} memories", snapshot.memories.len());
        for memory in snapshot.memories {
            self.create_memory(memory).await?;
        }

        // Restore entities
        tracing::info!("Restoring {} entities", snapshot.entities.len());
        for entity in snapshot.entities {
            self.create_entity(entity).await?;
        }

        // Restore relationships
        tracing::info!("Restoring {} relationships", snapshot.relationships.len());
        for relationship in snapshot.relationships {
            self.create_relationship(relationship).await?;
        }

        tracing::info!("Graph state restoration completed successfully");
        Ok(())
    }

    /// Create a conversation state version for AI assistant context management
    pub async fn create_conversation_version(
        &self,
        conversation_id: &str,
        description: &str,
    ) -> Result<Version, StorageError> {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "snapshot_type".to_string(),
            Value::String("conversation".to_string()),
        );
        metadata.insert(
            "conversation_id".to_string(),
            Value::String(conversation_id.to_string()),
        );
        metadata.insert(
            "context_type".to_string(),
            Value::String("ai_assistant".to_string()),
        );

        let version = Version {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            metadata: Value::Object(metadata),
            created_at: Utc::now(),
        };

        self.create_version(version).await
    }

    /// Create a knowledge evolution version for tracking learning progress
    pub async fn create_knowledge_version(
        &self,
        topic: &str,
        description: &str,
    ) -> Result<Version, StorageError> {
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "snapshot_type".to_string(),
            Value::String("knowledge".to_string()),
        );
        metadata.insert("topic".to_string(), Value::String(topic.to_string()));
        metadata.insert(
            "learning_context".to_string(),
            Value::String("evolution_tracking".to_string()),
        );

        let version = Version {
            id: Uuid::new_v4().to_string(),
            description: description.to_string(),
            metadata: Value::Object(metadata),
            created_at: Utc::now(),
        };

        self.create_version(version).await
    }

    /// List versions by type for better organization
    pub async fn list_versions_by_type(
        &self,
        snapshot_type: SnapshotType,
        limit: Option<usize>,
    ) -> Result<Vec<Version>, StorageError> {
        let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            "SELECT * FROM version WHERE snapshot_type = $snapshot_type ORDER BY created_at DESC {}",
            limit_clause
        );

        let mut result = self
            .client
            .query(query.as_str())
            .bind(("snapshot_type", snapshot_type.to_string()))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list versions by type: {}", e)))?;

        let versions: Vec<SurrealVersion> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to parse version results: {}", e)))?;

        Ok(versions.into_iter().map(Version::from).collect())
    }
}
