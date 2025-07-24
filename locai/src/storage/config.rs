//! Configuration structures for storage backends

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::storage::errors::StorageError;

/// Supported graph storage backend types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GraphStorageType {
    /// SurrealDB (embedded or remote graph database)
    SurrealDB,
    /// In-memory graph (for testing)
    Memory,
    /// Custom backend
    Custom,
}

/// Enumeration of vector storage types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VectorStorageType {
    /// SurrealDB (unified graph and vector database)
    SurrealDB,
    /// In-memory vector storage (for testing)
    Memory,
    /// Custom backend
    Custom,
}

/// Configuration for storage connections and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StorageConfig {
    /// SurrealDB storage configuration
    #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
    SurrealDB(SurrealDBConfig),
    
    /// Graph storage configuration
    Graph(GraphStorageConfig),
    
    /// Vector storage configuration
    Vector(VectorStorageConfig),
    
    /// Memory storage configuration
    Memory,
    
    /// File storage configuration
    File(FileStorageConfig),
}

/// SurrealDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBConfig {
    /// SurrealDB engine type
    pub engine: SurrealDBEngine,
    
    /// Connection string for remote or path for embedded
    pub connection: String,
    
    /// Namespace
    pub namespace: String,
    
    /// Database name
    pub database: String,
    
    /// Authentication information
    pub auth: Option<SurrealDBAuth>,
    
    /// Common storage settings
    pub settings: Option<CommonStorageSettings>,
}

/// SurrealDB engine types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurrealDBEngine {
    /// In-memory storage (for testing)
    Memory,
    /// RocksDB on-disk storage (embedded)
    RocksDB,
    /// Remote WebSocket connection
    WebSocket,
    /// Remote HTTP connection
    Http,
}

/// SurrealDB authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDBAuth {
    /// Authentication type
    pub auth_type: SurrealDBAuthType,
    
    /// Username (for root/namespace/database auth)
    pub username: Option<String>,
    
    /// Password (for root/namespace/database auth)
    pub password: Option<String>,
    
    /// Token (for JWT auth)
    pub token: Option<String>,
    
    /// Scope (for scope auth)
    pub scope: Option<String>,
}

/// SurrealDB authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SurrealDBAuthType {
    /// Root user authentication
    Root,
    /// Namespace user authentication
    Namespace,
    /// Database user authentication
    Database,
    /// Scope authentication
    Scope,
    /// JWT token authentication
    Jwt,
}

/// Graph storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStorageConfig {
    /// Graph storage backend type
    pub backend: GraphStorageType,
    
    /// Connection string for the database
    pub connection_string: String,
    
    /// Additional connection parameters
    pub params: HashMap<String, String>,
    
    /// Authentication information
    pub auth: Option<StorageAuth>,
}

/// Vector storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStorageConfig {
    /// Vector storage backend type
    pub backend: VectorStorageType,
    
    /// Connection string for the database
    pub connection_string: String,
    
    /// Authentication information
    pub auth: Option<StorageAuth>,
    
    /// Common storage settings
    pub settings: Option<CommonStorageSettings>,
}

/// Common storage settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonStorageSettings {
    /// Connection pool size
    pub pool_size: Option<usize>,
    
    /// Connection timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Option<Duration>,
    
    /// Additional configuration parameters
    pub params: HashMap<String, String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageAuth {
    /// Username for authentication
    pub username: Option<String>,
    
    /// Password for authentication
    pub password: Option<String>,
    
    /// Access token
    pub token: Option<String>,
}

/// Configuration for file-based storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorageConfig {
    /// Base directory for file storage
    pub base_dir: String,
}

/// Storage type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StorageType {
    /// SurrealDB storage
    SurrealDB,
    
    /// In-memory storage
    Memory,
    
    /// File-based storage
    File,
    
    /// Graph storage
    Graph,
    
    /// Vector storage
    Vector,
}

impl StorageConfig {
    /// Get the storage type for this configuration
    pub fn storage_type(&self) -> StorageType {
        match self {
            #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
            StorageConfig::SurrealDB(_) => StorageType::SurrealDB,
            StorageConfig::Graph(_) => StorageType::Graph,
            StorageConfig::Vector(_) => StorageType::Vector,
            StorageConfig::Memory => StorageType::Memory,
            StorageConfig::File(_) => StorageType::File,
        }
    }

    /// Validate the storage configuration
    pub fn validate(&self) -> Result<(), StorageError> {
        match self {
            #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
            StorageConfig::SurrealDB(config) => {
                if config.connection.is_empty() {
                    return Err(StorageError::Configuration("SurrealDB connection string cannot be empty".to_string()));
                }
                if config.namespace.is_empty() {
                    return Err(StorageError::Configuration("SurrealDB namespace cannot be empty".to_string()));
                }
                if config.database.is_empty() {
                    return Err(StorageError::Configuration("SurrealDB database cannot be empty".to_string()));
                }
                Ok(())
            }
            StorageConfig::Graph(config) => {
                if config.connection_string.is_empty() {
                    return Err(StorageError::Configuration("Graph storage connection string cannot be empty".to_string()));
                }
                Ok(())
            }
            StorageConfig::Vector(config) => {
                if config.connection_string.is_empty() {
                    return Err(StorageError::Configuration("Vector storage connection string cannot be empty".to_string()));
                }
                Ok(())
            }
            StorageConfig::Memory => Ok(()),
            StorageConfig::File(config) => {
                if config.base_dir.is_empty() {
                    return Err(StorageError::Configuration("File storage base directory cannot be empty".to_string()));
                }
                Ok(())
            }
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        #[cfg(any(feature = "surrealdb-embedded", feature = "surrealdb-remote"))]
        {
            StorageConfig::SurrealDB(SurrealDBConfig::default())
        }
        #[cfg(not(any(feature = "surrealdb-embedded", feature = "surrealdb-remote")))]
        {
            StorageConfig::Memory
        }
    }
}

impl Default for SurrealDBConfig {
    fn default() -> Self {
        Self {
            engine: SurrealDBEngine::Memory,
            connection: "memory".to_string(),
            namespace: "test".to_string(),
            database: "test".to_string(),
            auth: None,
            settings: None,
        }
    }
} 