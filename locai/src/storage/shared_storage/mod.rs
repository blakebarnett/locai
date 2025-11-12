//! Shared Storage - Official Implementation
//!
//! This is the official shared storage implementation for Locai, providing
//! a unified interface for all storage operations across multiple agents.
//!
//! **SharedStorage provides full feature parity with SurrealDB storage**, implementing
//! all storage traits including BaseStore, MemoryStore, EntityStore, RelationshipStore,
//! VectorStore, VersionStore, GraphStore, and GraphTraversal. It serves as a drop-in
//! replacement with identical functionality and performance characteristics.
//!
//! It uses proven patterns from existing SurrealDB implementations.

use surrealdb::Surreal;

use crate::storage::config::{SurrealDBAuth, SurrealDBAuthType, SurrealDBConfig, SurrealDBEngine};
use crate::storage::errors::StorageError;
use crate::storage::traits::GraphStore;

pub mod base;
pub mod config;
pub mod entity;
pub mod graph;
pub mod intelligence;
pub mod live_query;
pub mod memory;
pub mod relationship;
pub mod schema;
pub mod vector;
pub mod version;

pub use base::*;
pub use config::*;
pub use intelligence::*;

/// Type alias for embedded shared storage
pub type EmbeddedSharedStorage = SharedStorage<surrealdb::engine::local::Db>;

/// Create an embedded shared storage instance
pub async fn create_embedded_shared_storage(
    path: &str,
    config: SharedStorageConfig,
) -> Result<EmbeddedSharedStorage, StorageError> {
    use surrealdb::engine::local::RocksDb;

    let client = Surreal::new::<RocksDb>(path).await.map_err(|e| {
        StorageError::Connection(format!("Failed to create embedded database: {}", e))
    })?;

    SharedStorage::new(client, config).await
}

/// Create a shared storage instance from configuration
pub async fn create_shared_store(
    config: SurrealDBConfig,
) -> Result<Box<dyn GraphStore>, StorageError> {
    match config.engine {
        SurrealDBEngine::Memory => {
            tracing::info!("Creating SharedStorage in-memory store");
            let client = Surreal::new::<surrealdb::engine::local::Mem>(())
                .await
                .map_err(|e| {
                    StorageError::Connection(format!("Failed to create memory client: {}", e))
                })?;

            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
            };
            let store = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(store))
        }
        SurrealDBEngine::RocksDB => {
            tracing::info!(
                "Creating SharedStorage RocksDB store at {}",
                config.connection
            );
            let client = Surreal::new::<surrealdb::engine::local::RocksDb>(&config.connection)
                .await
                .map_err(|e| {
                    StorageError::Connection(format!("Failed to create RocksDB client: {}", e))
                })?;

            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
            };
            let store = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(store))
        }
        #[cfg(feature = "surrealdb-remote")]
        SurrealDBEngine::WebSocket => {
            tracing::info!(
                "Creating SharedStorage WebSocket connection to {}",
                config.connection
            );
            let client = Surreal::new::<surrealdb::engine::remote::ws::Ws>(&config.connection)
                .await
                .map_err(|e| {
                    StorageError::Connection(format!("Failed to create WebSocket client: {}", e))
                })?;

            // Handle authentication if provided
            if let Some(auth) = &config.auth {
                authenticate_client(&client, auth, &config).await?;
            }

            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
            };
            let store = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(store))
        }
        #[cfg(not(feature = "surrealdb-remote"))]
        SurrealDBEngine::WebSocket => Err(StorageError::Configuration(
            "WebSocket engine requires 'surrealdb-remote' feature to be enabled".to_string(),
        )),
        #[cfg(feature = "surrealdb-remote")]
        SurrealDBEngine::Http => {
            tracing::info!(
                "Creating SharedStorage HTTP connection to {}",
                config.connection
            );
            let client = Surreal::new::<surrealdb::engine::remote::http::Http>(&config.connection)
                .await
                .map_err(|e| {
                    StorageError::Connection(format!("Failed to create HTTP client: {}", e))
                })?;

            // Handle authentication if provided
            if let Some(auth) = &config.auth {
                authenticate_client(&client, auth, &config).await?;
            }

            let shared_config = SharedStorageConfig {
                namespace: config.namespace.clone(),
                database: config.database.clone(),
                lifecycle_tracking: Default::default(),
            };
            let store = SharedStorage::new(client, shared_config).await?;
            Ok(Box::new(store))
        }
        #[cfg(not(feature = "surrealdb-remote"))]
        SurrealDBEngine::Http => Err(StorageError::Configuration(
            "HTTP engine requires 'surrealdb-remote' feature to be enabled".to_string(),
        )),
    }
}

/// Authenticate with SurrealDB client
pub async fn authenticate_client<C>(
    client: &Surreal<C>,
    auth: &SurrealDBAuth,
    config: &SurrealDBConfig,
) -> Result<(), StorageError>
where
    C: surrealdb::Connection,
{
    match auth.auth_type {
        SurrealDBAuthType::Root => {
            tracing::debug!("Authenticating as root user");
            if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                let root = surrealdb::opt::auth::Root { username, password };
                client.signin(root).await.map_err(|e| {
                    StorageError::Authentication(format!("Root auth failed: {}", e))
                })?;
            }
        }
        SurrealDBAuthType::Namespace => {
            tracing::debug!("Authenticating as namespace user");
            if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                let ns_auth = surrealdb::opt::auth::Namespace {
                    namespace: &config.namespace,
                    username,
                    password,
                };
                client.signin(ns_auth).await.map_err(|e| {
                    StorageError::Authentication(format!("Namespace auth failed: {}", e))
                })?;
            }
        }
        SurrealDBAuthType::Database => {
            tracing::debug!("Authenticating as database user");
            if let (Some(username), Some(password)) = (&auth.username, &auth.password) {
                let db_auth = surrealdb::opt::auth::Database {
                    namespace: &config.namespace,
                    database: &config.database,
                    username,
                    password,
                };
                client.signin(db_auth).await.map_err(|e| {
                    StorageError::Authentication(format!("Database auth failed: {}", e))
                })?;
            }
        }
        SurrealDBAuthType::Scope => {
            tracing::debug!("Scope authentication not yet fully implemented");
            return Err(StorageError::Configuration(
                "Scope authentication requires additional implementation".to_string(),
            ));
        }
        SurrealDBAuthType::Jwt => {
            tracing::debug!("JWT authentication not yet fully implemented");
            if let Some(token) = &auth.token {
                client
                    .authenticate(token.clone())
                    .await
                    .map_err(|e| StorageError::Authentication(format!("JWT auth failed: {}", e)))?;
            }
        }
    }
    Ok(())
}

/// Helper function to check if a GraphStore is a SharedStorage store and supports live queries
pub fn supports_live_queries(store: &dyn GraphStore) -> bool {
    store.supports_live_queries() && store.get_live_query_info() == Some("SharedStorage")
}

/// Helper function to setup live queries for a SharedStorage store
/// This function uses unsafe downcasting, so it should only be called after verifying
/// the store type with supports_live_queries()
pub async fn setup_live_queries_for_store(
    store: &dyn GraphStore,
) -> Result<
    (
        Box<dyn std::any::Any + Send>,
        tokio::sync::broadcast::Receiver<live_query::DbEvent>,
    ),
    StorageError,
> {
    if !supports_live_queries(store) {
        return Err(StorageError::Configuration(
            "Store does not support live queries".to_string(),
        ));
    }

    // This is a workaround since we can't downcast trait objects directly
    // In a real implementation, you might want to use a different approach
    // such as adding the live query setup directly to the GraphStore trait
    Err(StorageError::Configuration(
        "Live query setup requires direct access to SurrealDB client".to_string(),
    ))
}
