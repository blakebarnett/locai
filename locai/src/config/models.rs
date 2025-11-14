//! Configuration model definitions.
//!
//! This module contains the configuration structures for all Locai components.

use crate::storage::config::SurrealDBConfig;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// Main configuration structure for Locai.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LocaiConfig {
    /// Storage configuration
    pub storage: StorageConfig,

    /// Machine learning configuration
    pub ml: MLConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Entity extraction configuration
    pub entity_extraction: crate::entity_extraction::EntityExtractionConfig,

    /// Memory lifecycle tracking configuration
    pub lifecycle_tracking: LifecycleTrackingConfig,

    /// Memory versioning configuration
    pub versioning: VersioningConfig,
}

/// Configuration for automatic memory lifecycle tracking.
///
/// Lifecycle tracking automatically updates memory metadata (access_count, last_accessed)
/// when memories are retrieved or accessed. This enables forgetting curves, importance
/// calculation, and other time-based memory dynamics.
///
/// # Performance Considerations
///
/// - `enabled: true` means every memory retrieval becomes a write operation
/// - Use `batched: true` to defer updates, reducing write load
/// - Set `flush_interval_secs` and `flush_threshold_count` to balance consistency vs. performance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LifecycleTrackingConfig {
    /// Whether lifecycle tracking is enabled globally
    pub enabled: bool,

    /// Whether to update access count and last_accessed on get_memory()
    pub update_on_get: bool,

    /// Whether to update access count on search operations
    /// (Usually false - searching shouldn't count as accessing)
    pub update_on_search: bool,

    /// Whether to update access count on list operations
    /// (Usually false - browsing shouldn't count as accessing)
    pub update_on_list: bool,

    /// Whether lifecycle updates should block the get_memory() call
    /// If false, updates are queued asynchronously
    pub blocking: bool,

    /// Whether to batch lifecycle updates instead of updating immediately
    pub batched: bool,

    /// Time interval (in seconds) between batch flushes when batched=true
    pub flush_interval_secs: u64,

    /// Number of pending updates before force-flushing the batch
    pub flush_threshold_count: usize,
}

impl Default for LifecycleTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_on_get: true,
            update_on_search: false,
            update_on_list: false,
            blocking: false,
            batched: true,
            flush_interval_secs: 60,
            flush_threshold_count: 100,
        }
    }
}

impl LifecycleTrackingConfig {
    /// Validate the configuration, returning an error if invalid
    pub fn validate(&self) -> Result<(), String> {
        if self.flush_interval_secs == 0 {
            return Err("flush_interval_secs must be greater than 0".to_string());
        }
        if self.flush_threshold_count == 0 {
            return Err("flush_threshold_count must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Configuration for storage components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    /// Base directory for storage
    pub data_dir: PathBuf,

    /// Graph storage configuration
    pub graph: GraphStorageConfig,

    /// Vector storage configuration
    pub vector: VectorStorageConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let data_dir = directories::ProjectDirs::from("org", "locai", "locai")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./data"));

        Self {
            data_dir,
            graph: GraphStorageConfig::default(),
            vector: VectorStorageConfig::default(),
        }
    }
}

/// Graph storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphStorageConfig {
    /// Type of graph storage to use
    pub storage_type: GraphStorageType,

    /// Path to store graph data (relative to data_dir)
    pub path: PathBuf,

    /// SurrealDB-specific configuration
    pub surrealdb: SurrealDBConfig,
}

impl Default for GraphStorageConfig {
    fn default() -> Self {
        Self {
            storage_type: GraphStorageType::SurrealDB,
            path: PathBuf::from("graph"),
            surrealdb: SurrealDBConfig::default(),
        }
    }
}

/// Vector storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorStorageConfig {
    /// Type of vector storage to use
    pub storage_type: VectorStorageType,

    /// Path to store vector data (relative to data_dir)
    pub path: PathBuf,
}

impl Default for VectorStorageConfig {
    fn default() -> Self {
        Self {
            storage_type: VectorStorageType::SurrealDB,
            path: PathBuf::from("vectors"),
        }
    }
}

/// Graph storage type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GraphStorageType {
    /// SurrealDB graph database (recommended)
    SurrealDB,
}

/// Vector storage type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VectorStorageType {
    /// SurrealDB vector database (unified graph and vector storage)
    SurrealDB,

    /// In-memory vector storage
    Memory,
}

/// Machine learning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MLConfig {
    /// Embedding model configuration
    pub embedding: EmbeddingConfig,

    /// Directory to cache models
    pub model_cache_dir: PathBuf,
}

impl Default for MLConfig {
    fn default() -> Self {
        let cache_dir = directories::ProjectDirs::from("org", "locai", "locai")
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("./cache"));

        Self {
            embedding: EmbeddingConfig::default(),
            model_cache_dir: cache_dir,
        }
    }
}

/// Embedding model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EmbeddingConfig {
    /// Model type for embeddings
    pub model_type: EmbeddingModelType,

    /// Model name or path
    pub model_name: String,

    /// Local or remote embedding service
    pub service_type: EmbeddingServiceType,

    /// Remote service URL (if using remote)
    pub service_url: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_type: EmbeddingModelType::OpenAI,
            model_name: "text-embedding-3-small".to_string(),
            service_type: EmbeddingServiceType::Remote,
            service_url: Some("https://api.openai.com/v1".to_string()),
        }
    }
}

/// Embedding model type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingModelType {
    /// OpenAI compatible API
    OpenAI,

    /// Cohere API
    Cohere,

    /// Custom model/provider
    Custom,
}

/// Embedding service type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingServiceType {
    /// Local embedding service
    Local,

    /// Remote embedding service
    Remote,
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,

    /// Log format
    pub format: LogFormat,

    /// File to log to (if any)
    pub file: Option<PathBuf>,

    /// Whether to log to stdout
    pub stdout: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Default,
            file: None,
            stdout: true,
        }
    }
}

/// Log level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level
    Trace,

    /// Debug level
    Debug,

    /// Info level
    Info,

    /// Warn level
    Warn,

    /// Error level
    Error,
}

// Implement Display for LogLevel
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

// Implement FromStr for LogLevel
impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

/// Log format.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Default format
    Default,

    /// JSON format
    Json,

    /// Compact format
    Compact,

    /// Pretty format
    Pretty,
}

/// Configuration for memory versioning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VersioningConfig {
    /// Whether versioning is enabled
    pub enabled: bool,

    /// Number of most recent versions to keep as full copies (before converting to deltas)
    pub delta_threshold: usize,

    /// Maximum delta chain length before promoting to full copy
    pub max_delta_chain_length: usize,

    /// Promotion: Access count threshold for auto-promotion
    pub promotion_access_threshold: u32,

    /// Promotion: Time window in hours for access tracking
    pub promotion_time_window_hours: u64,

    /// Promotion: Cost threshold in milliseconds (promote if reconstruction slower)
    pub promotion_cost_threshold_ms: u64,

    /// Cache size (max cached versions)
    pub cache_size: usize,

    /// Cache TTL in seconds (server mode only)
    pub cache_ttl_seconds: u64,

    /// Cache strategy (auto-detect, server, or embedded)
    pub cache_strategy: CacheStrategy,

    /// Enable auto-promotion based on heuristics
    pub enable_auto_promotion: bool,

    /// Enable reconstruction cache
    pub enable_reconstruction_cache: bool,

    /// Override server mode detection (None = auto-detect)
    pub server_mode: Option<bool>,

    /// Enable compression for old versions
    pub enable_compression: bool,

    /// Compression threshold: compress versions older than N days
    pub compression_threshold_days: u64,

    /// Maximum versions per memory (None = unlimited)
    pub max_versions_per_memory: Option<usize>,
}

impl Default for VersioningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            delta_threshold: 10,
            max_delta_chain_length: 50,
            promotion_access_threshold: 5,
            promotion_time_window_hours: 24,
            promotion_cost_threshold_ms: 50,
            cache_size: 1000,
            cache_ttl_seconds: 3600,
            cache_strategy: CacheStrategy::Auto,
            enable_auto_promotion: true,
            enable_reconstruction_cache: true,
            server_mode: None,
            enable_compression: true,
            compression_threshold_days: 30,
            max_versions_per_memory: None,
        }
    }
}

/// Cache strategy for version reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CacheStrategy {
    /// Auto-detect based on runtime context
    Auto,
    /// Force server mode (LRU cache)
    Server,
    /// Force embedded mode (simple cache)
    Embedded,
}
