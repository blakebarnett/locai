//! Storage abstractions and implementations
//!
//! This module provides trait definitions and implementations for various
//! storage backends used by Locai, including graph and vector storage.
//!
//! ## Storage Implementations
//!
//! - **SharedStorage**: A unified storage implementation providing full feature
//!   parity with SurrealDB storage, including all traits (BaseStore, MemoryStore,
//!   EntityStore, RelationshipStore, VectorStore, VersionStore, GraphStore, and
//!   GraphTraversal). Recommended for new applications.
//! - **SurrealDB**: Direct SurrealDB integration with comprehensive functionality
//! - **Memory**: Simple in-memory storage for testing and development

pub mod config;
pub mod errors;
pub mod filters;
pub mod lifecycle;
pub mod models;
pub mod shared_storage;
pub mod traits;

// Old surrealdb storage implementation removed - replaced by shared_storage

// Simple in-memory vector store for testing
mod memory_vector_store {
    use crate::storage::errors::StorageError;
    use crate::storage::filters::VectorFilter;
    use crate::storage::models::{Vector, VectorSearchParams};
    use crate::storage::traits::{BaseStore, VectorStore};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Debug)]
    pub struct MemoryVectorStore {
        vectors: RwLock<HashMap<String, Vector>>,
    }

    impl MemoryVectorStore {
        pub fn new() -> Self {
            Self {
                vectors: RwLock::new(HashMap::new()),
            }
        }

        fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
            if a.len() != b.len() {
                return 0.0;
            }

            let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

            if norm_a == 0.0 || norm_b == 0.0 {
                0.0
            } else {
                dot_product / (norm_a * norm_b)
            }
        }
    }

    #[async_trait]
    impl BaseStore for MemoryVectorStore {
        async fn health_check(&self) -> Result<bool, StorageError> {
            Ok(true)
        }

        async fn clear(&self) -> Result<(), StorageError> {
            self.vectors.write().unwrap().clear();
            Ok(())
        }

        async fn get_metadata(&self) -> Result<serde_json::Value, StorageError> {
            let count = self.vectors.read().unwrap().len();
            Ok(serde_json::json!({
                "type": "memory_vector_store",
                "vector_count": count
            }))
        }

        async fn close(&self) -> Result<(), StorageError> {
            Ok(())
        }
    }

    #[async_trait]
    impl VectorStore for MemoryVectorStore {
        async fn add_vector(&self, vector: Vector) -> Result<Vector, StorageError> {
            let mut vectors = self.vectors.write().unwrap();
            if vectors.contains_key(&vector.id) {
                return Err(StorageError::AlreadyExists(format!(
                    "Vector with ID {} already exists",
                    vector.id
                )));
            }
            vectors.insert(vector.id.clone(), vector.clone());
            Ok(vector)
        }

        async fn get_vector(&self, id: &str) -> Result<Option<Vector>, StorageError> {
            let vectors = self.vectors.read().unwrap();
            Ok(vectors.get(id).cloned())
        }

        async fn delete_vector(&self, id: &str) -> Result<bool, StorageError> {
            let mut vectors = self.vectors.write().unwrap();
            Ok(vectors.remove(id).is_some())
        }

        async fn update_vector_metadata(
            &self,
            id: &str,
            metadata: serde_json::Value,
        ) -> Result<Vector, StorageError> {
            let mut vectors = self.vectors.write().unwrap();
            if let Some(vector) = vectors.get_mut(id) {
                vector.metadata = metadata;
                Ok(vector.clone())
            } else {
                Err(StorageError::NotFound(format!(
                    "Vector with ID {} not found",
                    id
                )))
            }
        }

        async fn search_vectors(
            &self,
            query_vector: &[f32],
            params: VectorSearchParams,
        ) -> Result<Vec<(Vector, f32)>, StorageError> {
            let vectors = self.vectors.read().unwrap();
            let mut results: Vec<(Vector, f32)> = vectors
                .values()
                .map(|vector| {
                    let similarity = Self::cosine_similarity(query_vector, &vector.vector);
                    (vector.clone(), similarity)
                })
                .collect();

            // Sort by similarity (descending)
            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Apply limit
            if let Some(limit) = params.limit {
                results.truncate(limit);
            }

            // Apply threshold
            if let Some(threshold) = params.threshold {
                results.retain(|(_, score)| *score >= threshold);
            }

            Ok(results)
        }

        async fn list_vectors(
            &self,
            _filter: Option<VectorFilter>,
            limit: Option<usize>,
            offset: Option<usize>,
        ) -> Result<Vec<Vector>, StorageError> {
            let vectors = self.vectors.read().unwrap();
            let mut all_vectors: Vec<Vector> = vectors.values().cloned().collect();

            // Apply offset
            let start = offset.unwrap_or(0);
            if start >= all_vectors.len() {
                return Ok(vec![]);
            }
            all_vectors = all_vectors.into_iter().skip(start).collect();

            // Apply limit
            if let Some(limit) = limit {
                all_vectors.truncate(limit);
            }

            Ok(all_vectors)
        }

        async fn count_vectors(
            &self,
            _filter: Option<VectorFilter>,
        ) -> Result<usize, StorageError> {
            let vectors = self.vectors.read().unwrap();
            Ok(vectors.len())
        }

        async fn batch_add_vectors(
            &self,
            vectors: Vec<Vector>,
        ) -> Result<Vec<Vector>, StorageError> {
            let mut store_vectors = self.vectors.write().unwrap();
            let mut added_vectors = Vec::new();

            for vector in vectors {
                if store_vectors.contains_key(&vector.id) {
                    return Err(StorageError::AlreadyExists(format!(
                        "Vector with ID {} already exists",
                        vector.id
                    )));
                }
                store_vectors.insert(vector.id.clone(), vector.clone());
                added_vectors.push(vector);
            }

            Ok(added_vectors)
        }

        async fn upsert_vector(&self, vector: Vector) -> Result<(), StorageError> {
            let mut vectors = self.vectors.write().unwrap();
            vectors.insert(vector.id.clone(), vector);
            Ok(())
        }
    }
}

// Re-export common types for convenience
pub use config::{
    CommonStorageSettings, GraphStorageConfig, GraphStorageType, StorageConfig,
    VectorStorageConfig, VectorStorageType,
};
pub use errors::StorageError;
pub use filters::{
    EntityFilter, FilterCondition, MemoryFilter, RelationshipFilter, SortDirection, SortOrder,
    VectorFilter,
};
pub use models::{Entity, Relationship, Vector, VectorSearchParams, Version};
pub use traits::{
    BaseStore, EntityStore, GraphStore, MemoryStore, RelationshipStore, VectorStore, VersionStore,
};

pub use shared_storage::{
    EmbeddedSharedStorage, SharedStorage, SharedStorageConfig, create_embedded_shared_storage,
};

// Backwards compatibility type aliases
// (no need for dyn since these are just for import compatibility, not actual usage)
pub use traits::GraphStore as GraphStorage;
pub use traits::VectorStore as VectorStorage;

// Old surrealdb storage re-exports removed - use shared_storage instead

/// Create a graph storage backend based on configuration
///
/// **Deprecated**: Use `create_storage_service` instead for unified storage.
/// This function is maintained for backward compatibility.
pub async fn create_graph_storage(
    config: &StorageConfig,
) -> Result<Box<dyn GraphStore>, errors::StorageError> {
    match config {
        StorageConfig::SurrealDB(config) => {
            // Create SharedStorage as the new default
            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
                versioning: Default::default(),
            };

            match config.engine {
                crate::storage::config::SurrealDBEngine::Memory => {
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                        .await
                        .map_err(|e| {
                            errors::StorageError::Connection(format!(
                                "Failed to create memory client: {}",
                                e
                            ))
                        })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                crate::storage::config::SurrealDBEngine::RocksDB => {
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::RocksDb>(
                        &config.connection,
                    )
                    .await
                    .map_err(|e| {
                        errors::StorageError::Connection(format!(
                            "Failed to create RocksDB client: {}",
                            e
                        ))
                    })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                #[cfg(feature = "surrealdb-remote")]
                _ => {
                    // For remote connections, use the memory fallback for now
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                        .await
                        .map_err(|e| {
                            errors::StorageError::Connection(format!(
                                "Failed to create memory client: {}",
                                e
                            ))
                        })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                #[cfg(not(feature = "surrealdb-remote"))]
                _ => Err(errors::StorageError::Configuration(
                    "Remote engines require 'surrealdb-remote' feature to be enabled".to_string(),
                )),
            }
        }
        StorageConfig::Memory => {
            // Use SharedStorage with memory engine for memory configuration
            let shared_config = SharedStorageConfig {
                namespace: "memory".to_string(),
                database: "main".to_string(),
                lifecycle_tracking: Default::default(),
                versioning: Default::default(),
            };
            let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                .await
                .map_err(|e| {
                    errors::StorageError::Connection(format!(
                        "Failed to create memory client: {}",
                        e
                    ))
                })?;
            let shared_storage = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(shared_storage))
        }
        _ => Err(errors::StorageError::UnsupportedStorageType),
    }
}

/// Create a vector storage backend based on configuration
///
/// **Deprecated**: Use `create_storage_service` instead for unified storage.
/// This function is maintained for backward compatibility and now uses SharedStorage.
pub async fn create_vector_storage(
    config: &StorageConfig,
) -> Result<Box<dyn VectorStore>, errors::StorageError> {
    match config {
        StorageConfig::SurrealDB(config) => {
            // Create SharedStorage as the new default (which implements VectorStore)
            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
                versioning: Default::default(),
            };

            match config.engine {
                crate::storage::config::SurrealDBEngine::Memory => {
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                        .await
                        .map_err(|e| {
                            errors::StorageError::Connection(format!(
                                "Failed to create memory client: {}",
                                e
                            ))
                        })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                crate::storage::config::SurrealDBEngine::RocksDB => {
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::RocksDb>(
                        &config.connection,
                    )
                    .await
                    .map_err(|e| {
                        errors::StorageError::Connection(format!(
                            "Failed to create RocksDB client: {}",
                            e
                        ))
                    })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                #[cfg(feature = "surrealdb-remote")]
                _ => {
                    // For remote connections, use the memory fallback for now
                    let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                        .await
                        .map_err(|e| {
                            errors::StorageError::Connection(format!(
                                "Failed to create memory client: {}",
                                e
                            ))
                        })?;
                    let shared_storage = SharedStorage::new(client, shared_config).await?;
                    Ok(Box::new(shared_storage))
                }
                #[cfg(not(feature = "surrealdb-remote"))]
                _ => Err(errors::StorageError::Configuration(
                    "Remote engines require 'surrealdb-remote' feature to be enabled".to_string(),
                )),
            }
        }
        StorageConfig::Memory => {
            let store = memory_vector_store::MemoryVectorStore::new();
            Ok(Box::new(store))
        }
        _ => Err(errors::StorageError::UnsupportedStorageType),
    }
}

/// Create a storage backend based on configuration (defaults to graph storage)
///
/// This function is provided for backward compatibility.
/// For specific storage types, use `create_graph_storage` or `create_vector_storage`.
pub async fn create_storage(
    config: &StorageConfig,
) -> Result<Box<dyn GraphStore>, errors::StorageError> {
    create_graph_storage(config).await
}

/// Create a unified storage service using SharedStorage
///
/// This function creates a unified storage service that handles both graph and
/// messaging operations through a single SurrealDB instance, eliminating
/// RocksDB locking conflicts in embedded mode.
///
/// # Arguments
/// * `config` - The Locai configuration
///
/// # Returns
/// A storage service backed by SharedStorage
pub async fn create_storage_service(
    config: &crate::config::LocaiConfig,
) -> Result<Box<dyn crate::storage::traits::GraphStore>, errors::StorageError> {
    let shared_config = SharedStorageConfig {
        namespace: config.storage.graph.surrealdb.namespace.clone(),
        database: config.storage.graph.surrealdb.database.clone(),
        lifecycle_tracking: config.lifecycle_tracking.clone(),
        versioning: config.versioning.clone(),
    };

    // Create SharedStorage based on engine type
    match config.storage.graph.surrealdb.engine {
        crate::storage::config::SurrealDBEngine::Memory => {
            tracing::info!("Creating SharedStorage with in-memory engine");
            let client = surrealdb::Surreal::new::<surrealdb::engine::local::Mem>(())
                .await
                .map_err(|e| {
                    errors::StorageError::Connection(format!(
                        "Failed to create memory client: {}",
                        e
                    ))
                })?;
            let shared_storage = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(shared_storage))
        }
        crate::storage::config::SurrealDBEngine::RocksDB => {
            tracing::info!(
                "Creating SharedStorage with RocksDB engine at {}",
                config.storage.graph.surrealdb.connection
            );
            let shared_storage = create_embedded_shared_storage(
                &config.storage.graph.surrealdb.connection,
                shared_config,
            )
            .await?;
            Ok(Box::new(shared_storage))
        }
        #[cfg(feature = "surrealdb-remote")]
        crate::storage::config::SurrealDBEngine::WebSocket => {
            tracing::info!(
                "Creating SharedStorage with WebSocket connection to {}",
                config.storage.graph.surrealdb.connection
            );
            let client = surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(
                &config.storage.graph.surrealdb.connection,
            )
            .await
            .map_err(|e| {
                errors::StorageError::Connection(format!(
                    "Failed to create WebSocket client: {}",
                    e
                ))
            })?;
            let shared_storage = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(shared_storage))
        }
        #[cfg(feature = "surrealdb-remote")]
        crate::storage::config::SurrealDBEngine::Http => {
            tracing::info!(
                "Creating SharedStorage with HTTP connection to {}",
                config.storage.graph.surrealdb.connection
            );
            let client = surrealdb::Surreal::new::<surrealdb::engine::remote::http::Http>(
                &config.storage.graph.surrealdb.connection,
            )
            .await
            .map_err(|e| {
                errors::StorageError::Connection(format!("Failed to create HTTP client: {}", e))
            })?;
            let shared_storage = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(shared_storage))
        }
        #[cfg(not(feature = "surrealdb-remote"))]
        _ => Err(errors::StorageError::Configuration(
            "Remote engines require 'surrealdb-remote' feature to be enabled".to_string(),
        )),
    }
}
