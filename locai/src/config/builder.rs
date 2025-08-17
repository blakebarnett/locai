//! Configuration builder.
//!
//! This module provides a builder pattern API for creating configurations.

use super::{Result, models::*, validation};
use crate::storage::config::{SurrealDBConfig, SurrealDBEngine};
use std::path::{Path, PathBuf};

/// Builder for creating LocaiConfig instances.
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    config: LocaiConfig,
}

impl ConfigBuilder {
    /// Create a new configuration builder with default values.
    pub fn new() -> Self {
        Self {
            config: LocaiConfig::default(),
        }
    }

    /// Set the base data directory.
    pub fn with_data_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.storage.data_dir = path.as_ref().to_path_buf();
        self
    }

    /// Configure graph storage type
    pub fn with_graph_storage_type(mut self, storage_type: GraphStorageType) -> Self {
        self.config.storage.graph.storage_type = storage_type;
        self
    }

    /// Configure vector storage type
    pub fn with_vector_storage_type(mut self, storage_type: VectorStorageType) -> Self {
        self.config.storage.vector.storage_type = storage_type;
        self
    }

    /// Use default storage configuration (SurrealDB for both graph and vector)
    pub fn with_default_storage(mut self) -> Self {
        // Set data directory to a default location if not already set
        if self.config.storage.data_dir == PathBuf::from("./data") {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            self.config.storage.data_dir = home_dir.join(".locai").join("data");
        }

        // Configure graph storage with SurrealDB
        self.config.storage.graph.storage_type = GraphStorageType::SurrealDB;
        self.config.storage.graph.surrealdb = SurrealDBConfig {
            engine: SurrealDBEngine::RocksDB,
            connection: self
                .config
                .storage
                .data_dir
                .join("graph")
                .to_string_lossy()
                .to_string(),
            namespace: "locai".to_string(),
            database: "main".to_string(),
            auth: None,
            settings: None,
        };

        // Configure vector storage with SurrealDB (unified storage)
        self.config.storage.vector.storage_type = VectorStorageType::SurrealDB;

        self
    }

    /// Use in-memory storage for both graph and vector storage (good for testing)
    pub fn with_memory_storage(mut self) -> Self {
        // Configure graph storage with SurrealDB/Memory
        self.config.storage.graph.storage_type = GraphStorageType::SurrealDB;
        self.config.storage.graph.surrealdb.engine = SurrealDBEngine::Memory;

        // Configure vector storage with SurrealDB in-memory backend
        self.config.storage.vector.storage_type = VectorStorageType::SurrealDB;

        self
    }

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model_name: impl Into<String>) -> Self {
        self.config.ml.embedding.model_name = model_name.into();
        self
    }

    /// Configure to use a local embedding model.
    pub fn with_local_embeddings(mut self) -> Self {
        self.config.ml.embedding.service_type = EmbeddingServiceType::Local;
        self.config.ml.embedding.service_url = None;
        self
    }

    /// Configure to use a remote embedding service.
    pub fn with_remote_embeddings(mut self, url: impl Into<String>) -> Self {
        self.config.ml.embedding.service_type = EmbeddingServiceType::Remote;
        self.config.ml.embedding.service_url = Some(url.into());
        self
    }

    /// Set the model cache directory.
    pub fn with_model_cache_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.ml.model_cache_dir = path.as_ref().to_path_buf();
        self
    }

    /// Use default ML configuration (local embeddings with a standard model)
    pub fn with_default_ml(mut self) -> Self {
        // Set cache directory to a default location if not set
        if self.config.ml.model_cache_dir == PathBuf::from("./model_cache") {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            self.config.ml.model_cache_dir = home_dir.join(".locai").join("models");
        }

        // Configure embedding service to use a default model locally
        self.config.ml.embedding.service_type = EmbeddingServiceType::Local;
        self.config.ml.embedding.model_name = "BAAI/bge-m3".to_string(); // Use BGE-M3 for best performance, multilingual support, and 1024 dimensions
        self.config.ml.embedding.service_url = None;

        self
    }

    /// Set the log level.
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.config.logging.level = level;
        self
    }

    /// Configure logging to a file.
    pub fn with_log_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.logging.file = Some(path.as_ref().to_path_buf());
        self
    }

    /// Use default logging configuration (console output at Info level)
    pub fn with_default_logging(mut self) -> Self {
        self.config.logging.level = LogLevel::Info;
        self.config.logging.format = LogFormat::Json;
        self.config.logging.file = None; // Console only by default

        self
    }

    /// Disable entity extraction and automatic relationship creation.
    ///
    /// This can be useful for specialized use cases where you want minimal overhead
    /// or have custom entity management logic. By default, entity extraction is
    /// enabled to provide rich graph connections automatically.
    pub fn without_entity_extraction(mut self) -> Self {
        self.config.entity_extraction.enabled = false;
        self.config
            .entity_extraction
            .automatic_relationships
            .enabled = false;
        self
    }

    /// Configure entity extraction settings.
    pub fn with_entity_extraction_config(
        mut self,
        config: crate::entity_extraction::EntityExtractionConfig,
    ) -> Self {
        self.config.entity_extraction = config;
        self
    }

    /// Create a configuration for development with in-memory databases.
    ///
    /// This creates a configuration suitable for development with:
    /// - In-memory storage for fast operations without persistence
    /// - Default ML configuration
    /// - Debug-level logging
    pub fn development() -> Self {
        Self::new()
            .with_memory_storage()
            .with_default_ml()
            .with_log_level(LogLevel::Debug)
    }

    /// Create a configuration for testing.
    ///
    /// This creates a configuration suitable for automated testing with:
    /// - In-memory storage by default
    /// - Test-specific data directories
    /// - Default ML configuration with test cache directory
    pub fn testing() -> Self {
        Self::development()
            .with_data_dir(PathBuf::from("./test_data"))
            .with_model_cache_dir(PathBuf::from("./test_cache"))
    }

    /// Create a production-ready configuration with persistent storage.
    ///
    /// This creates a configuration suitable for production use with:
    /// - Persistent storage for both graph and vector data
    /// - Default ML configuration with a standard embedding model
    /// - Standard logging at Info level
    /// - Default HTTP server configuration
    pub fn production() -> Self {
        Self::new()
            .with_default_storage()
            .with_default_ml()
            .with_default_logging()
    }

    /// Create a fully default configuration suitable for most uses
    ///
    /// This is equivalent to `production()` and provides a complete
    /// configuration ready for real-world use
    pub fn defaults() -> Self {
        Self::production().with_remote_surrealdb_if_configured()
    }

    /// Configure SurrealDB to use remote connection if environment variables are set
    pub fn with_remote_surrealdb_if_configured(mut self) -> Self {
        if let Ok(connection_url) = std::env::var("SURREALDB_URL") {
            tracing::info!(
                "Configuring SurrealDB remote connection to: {}",
                connection_url
            );

            // Determine engine type from URL
            #[allow(clippy::if_same_then_else)]
            let engine =
                if connection_url.starts_with("ws://") || connection_url.starts_with("wss://") {
                    SurrealDBEngine::WebSocket
                } else if connection_url.starts_with("http://")
                    || connection_url.starts_with("https://")
                {
                    SurrealDBEngine::Http
                } else {
                    SurrealDBEngine::Http // Default to HTTP for remote connections
                };

            let namespace =
                std::env::var("SURREALDB_NAMESPACE").unwrap_or_else(|_| "locai".to_string());
            let database =
                std::env::var("SURREALDB_DATABASE").unwrap_or_else(|_| "main".to_string());

            // Set up authentication if provided
            let auth = if let (Ok(username), Ok(password)) = (
                std::env::var("SURREALDB_USERNAME"),
                std::env::var("SURREALDB_PASSWORD"),
            ) {
                Some(crate::storage::config::SurrealDBAuth {
                    auth_type: crate::storage::config::SurrealDBAuthType::Root,
                    username: Some(username),
                    password: Some(password),
                    token: None,
                    scope: None,
                })
            } else {
                None
            };

            // Configure graph storage to use remote SurrealDB
            self.config.storage.graph.surrealdb = SurrealDBConfig {
                engine,
                connection: connection_url,
                namespace,
                database,
                auth,
                settings: None,
            };
        }

        self
    }

    /// Create a minimal configuration for quick testing and prototyping
    ///
    /// This creates a configuration with:
    /// - In-memory storage (no persistence)
    /// - Default ML configuration
    /// - Minimal logging
    /// - No HTTP server configuration
    pub fn minimal() -> Self {
        Self::new()
            .with_memory_storage()
            .with_default_ml()
            .with_log_level(LogLevel::Info)
    }

    /// Build the configuration, validating it in the process.
    pub fn build(self) -> Result<LocaiConfig> {
        // Validate the configuration
        validation::validate_config(&self.config)?;

        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
