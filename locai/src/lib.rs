//! # Locai
//!
//! Advanced memory management system for AI agents and applications, providing
//! persistent storage and intelligent retrieval through professional-grade BM25 search,
//! optional embeddings via BYOE (Bring Your Own Embeddings), and graph relationships.
//!
//! ## Quick Start
//!
//! ```rust
//! use locai::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Initialize with defaults - just BM25 search + storage
//!     let locai = Locai::new().await?;
//!
//!     // Store memories - BM25 search works immediately
//!     locai.remember("The sky is blue").await?;
//!     locai.remember_fact("Water boils at 100°C").await?;
//!     
//!     // Search memories using professional BM25
//!     let results = locai.search("sky color").await?;
//!     
//!     // Add embeddings for hybrid search (BYOE approach)
//!     // This example shows the concept - you would use your actual embedding provider
//!     let embedding = vec![0.1, 0.2, 0.3]; // Mock embedding from your provider
//!     let memory = MemoryBuilder::new_with_content("text")
//!         .embedding(embedding)  // ← You provide the embedding
//!         .build();
//!     locai.manager().store_memory(memory).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## BYOE (Bring Your Own Embeddings)
//!
//! Locai follows a BYOE approach - you choose your embedding provider:
//!
//! - **OpenAI**: text-embedding-3-small/large
//! - **Cohere**: embed-english-v3.0  
//! - **Azure**: OpenAI-compatible endpoints
//! - **Local**: fastembed, Ollama, sentence-transformers
//! - **Custom**: Any provider via simple API
//!
//! This gives you maximum flexibility, cost control, and always access to the latest models.
//!
//! ## Architecture
//!
//! - **Core**: BM25 search, memory storage, graph relationships (always available)
//! - **Optional**: Local ML models for advanced users (candle-embeddings feature)
//! - **BYOE**: User-provided embeddings for vector/hybrid search
//!
//! This crate provides the core library functionality that can be used directly
//! in Rust applications or through the separate service crate.

pub mod config;
pub mod core;
pub mod entity_extraction;
pub mod logging;
pub mod memory;
pub mod messaging;
pub mod ml;
pub mod models;
pub mod relationships;
pub mod runtime;
pub mod simple;
pub mod storage;

/// The prelude re-exports commonly used types for convenience
pub mod prelude {
    // Re-export the simplified API (recommended for new users)
    pub use crate::simple::{Locai, LocaiBuilder, RememberBuilder, SearchBuilder};

    // Re-export core initialization functions
    pub use crate::{init, init_with_defaults};

    // Re-export config types
    pub use crate::config::{
        ConfigBuilder, EmbeddingConfig, GraphStorageType, LocaiConfig, LogLevel, MLConfig,
        StorageConfig, VectorStorageType,
    };

    // Re-export entity extraction types
    pub use crate::entity_extraction::{
        BasicEntityExtractor, EntityExtractionConfig, EntityExtractor, EntityType, ExtractedEntity,
        ExtractorConfig, ExtractorType,
    };

    // Re-export model types
    pub use crate::models::{Memory, MemoryBuilder, MemoryPriority, MemoryType};

    // Re-export core types for advanced usage
    pub use crate::core::{
        MemoryManager, SearchOptions, SearchResult, SearchStrategy, SearchTypeFilter,
    };

    // Re-export storage types for advanced usage
    pub use crate::storage::{
        StorageError,
        models::{MemoryGraph, MemoryPath},
    };

    // Re-export essential result type
    pub use crate::{LocaiError, Result};
}

/// Current library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Error type for Locai operations with helpful recovery suggestions
#[derive(Debug, thiserror::Error)]
pub enum LocaiError {
    /// Error during storage operations
    #[error("Storage error: {0}")]
    Storage(String),

    /// Error during ML operations
    #[error("ML error: {0}")]
    ML(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Logging error
    #[error("Logging error: {0}")]
    Logging(#[from] crate::logging::LogError),

    /// Errors related to memory operations
    #[error("Memory error: {0}")]
    Memory(String),

    /// Errors related to entity operations
    #[error("Entity error: {0}")]
    Entity(String),

    /// Errors related to relationship operations
    #[error("Relationship error: {0}")]
    Relationship(String),

    /// Errors related to versioning operations
    #[error("Version error: {0}")]
    Version(String),

    /// ML service not configured (with helpful guidance)
    #[error(
        "ML service not configured. To use semantic search, initialize with: Locai::builder().with_defaults().build().await or use ConfigBuilder::new().with_default_ml()"
    )]
    MLNotConfigured,

    /// Storage directory not accessible
    #[error(
        "Storage directory not accessible: {path}. Ensure the directory exists and has write permissions, or try using in-memory storage with Locai::for_testing()"
    )]
    StorageNotAccessible { path: String },

    /// Invalid embedding model
    #[error(
        "Invalid embedding model '{model}'. Try using a supported model like 'BAAI/bge-m3' or initialize with ConfigBuilder::new().with_default_ml()"
    )]
    InvalidEmbeddingModel { model: String },

    /// Connection errors (for remote messaging)
    #[error("Connection error: {0}. Check your network connection and server availability")]
    Connection(String),

    /// Authentication errors (for remote messaging)
    #[error("Authentication error: {0}. Verify your credentials and permissions")]
    Authentication(String),

    /// Protocol errors (for remote messaging)
    #[error("Protocol error: {0}. Ensure client and server versions are compatible")]
    Protocol(String),

    /// Timeout errors (for remote messaging)
    #[error("Timeout error: {0}. Try increasing timeout settings or check server responsiveness")]
    Timeout(String),

    /// Empty search query
    #[error(
        "Search query cannot be empty. Provide a meaningful search term like 'what did I learn about science?'"
    )]
    EmptySearchQuery,

    /// No memories found
    #[error(
        "No memories found matching your criteria. Try broadening your search or adding some memories first"
    )]
    NoMemoriesFound,

    /// Feature not enabled
    #[error(
        "Feature '{feature}' is not enabled. Enable it in Cargo.toml with: features = [\"{feature}\"]"
    )]
    FeatureNotEnabled { feature: String },

    /// Other unclassified errors
    #[error("{0}")]
    Other(String),
}

impl From<crate::config::ConfigError> for LocaiError {
    fn from(err: crate::config::ConfigError) -> Self {
        LocaiError::Configuration(err.to_string())
    }
}

impl From<crate::ml::error::MLError> for LocaiError {
    fn from(err: crate::ml::error::MLError) -> Self {
        LocaiError::ML(err.to_string())
    }
}

/// Result type for Locai operations
pub type Result<T> = std::result::Result<T, LocaiError>;

/// Initialize Locai with default configuration
///
/// This function sets up the Locai memory system with sensible defaults
/// and returns a `MemoryManager` instance that can be used to interact with the
/// system.
///
/// # Returns
/// A `MemoryManager` instance if initialization succeeds
///
/// # Examples
///
/// ```rust
/// use locai::prelude::*;
///
/// async fn example() -> Result<()> {
///     // Initialize Locai with defaults
///     let memory_manager = init_with_defaults().await?;
///
///     // Use the memory manager with the simplified API
///     let memory_id = memory_manager.add_fact("The sky is blue because of Rayleigh scattering.").await?;
///
///     Ok(())
/// }
/// ```
pub async fn init_with_defaults() -> Result<core::MemoryManager> {
    let config = config::ConfigBuilder::defaults().build()?;
    init(config).await
}

/// Initialize Locai with the provided configuration
///
/// This function sets up the Locai memory system with the provided configuration
/// and returns a `MemoryManager` instance that can be used to interact with the
/// system.
///
/// # Arguments
/// * `config` - The configuration for initializing Locai
///
/// # Returns
/// A `MemoryManager` instance if initialization succeeds
///
/// # Examples
///
/// ```rust
/// use locai::prelude::*;
///
/// async fn example() -> Result<()> {
///     // Create configuration
///     let config = ConfigBuilder::new()
///         .with_default_storage()
///         .with_default_ml()
///         .build()?;
///
///     // Initialize Locai
///     let memory_manager = init(config).await?;
///
///     // Use the memory manager
///     let memory_id = memory_manager.add_fact("This is a sample memory").await?;
///
///     Ok(())
/// }
/// ```
pub async fn init(config: config::LocaiConfig) -> Result<core::MemoryManager> {
    // Initialize logging - always initialize since there's no "enabled" flag
    // Ignore errors if tracing is already initialized
    let _ = logging::init(&config.logging);

    // Entity extraction is now handled via examples - no auto-initialization needed

    // Create storage service
    // Note: Explicitly mapping StorageError to LocaiError
    let storage = storage::create_storage_service(&config)
        .await
        .map_err(|e| LocaiError::Storage(e.to_string()))?;
    let storage = std::sync::Arc::from(storage);

    // Don't create ML service by default - users must explicitly configure it
    // Having URLs/defaults configured doesn't mean it will actually work without API keys, etc.
    let ml_service = None;

    // Create MemoryManager with ML extractors initialized
    let memory_manager =
        core::MemoryManager::new_with_ml(storage, ml_service, config.clone()).await?;

    Ok(memory_manager)
}
