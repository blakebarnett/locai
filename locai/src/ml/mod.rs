//! Machine learning utilities for BYOE (Bring Your Own Embeddings) approach
//!
//! This module provides simple utilities for embedding validation and normalization,
//! allowing users to bring their own embeddings from any provider:
//! - OpenAI (text-embedding-3-small/large)
//! - Cohere (embed-english-v3.0)
//! - Azure OpenAI
//! - Local models (fastembed, Ollama, sentence-transformers)
//! - Custom providers
//!
//! ## Example Usage
//!
//! ```rust
//! use locai::ml::EmbeddingManager;
//!
//! // Create embedding manager with validation
//! let manager = EmbeddingManager::with_expected_dimensions(1536);
//!
//! // Validate user-provided embeddings
//! let embedding = get_embedding_from_provider("text").await?;
//! manager.validate_embedding(&embedding)?;
//! ```

pub mod error;
pub mod model_manager;

// Re-export core BYOE functionality
pub use error::{MLError, Result};
pub use model_manager::{EmbeddingManager, EmbeddingManagerBuilder};

// Type aliases for convenience
pub type EmbeddingVector = Vec<f32>;
pub type EmbeddingBatch = Vec<EmbeddingVector>;

/// Utility functions for BYOE capabilities
pub mod utils {
    /// Check if embedding capabilities are available
    /// With BYOE approach, embeddings are always available when users provide them
    pub fn has_embedding_support() -> bool {
        true // BYOE is always available
    }

    /// Get available embedding backends
    pub fn available_backends() -> Vec<&'static str> {
        vec!["byoe"] // Only BYOE approach
    }

    /// Check if this is a valid embedding dimension for common providers
    pub fn is_common_dimension(dimension: usize) -> bool {
        matches!(
            dimension,
            384 |   // bge-small, all-MiniLM-L6-v2
            512 |   // all-MiniLM-L12-v2  
            768 |   // all-mpnet-base-v2, BERT-base
            1024 |  // Cohere embed-english-v3.0
            1536 |  // OpenAI text-embedding-3-small/ada-002
            3072 // OpenAI text-embedding-3-large
        )
    }

    /// Get recommended providers for common embedding dimensions
    pub fn providers_for_dimension(dimension: usize) -> Vec<&'static str> {
        match dimension {
            384 => vec!["bge-small", "all-MiniLM-L6-v2"],
            512 => vec!["all-MiniLM-L12-v2"],
            768 => vec!["all-mpnet-base-v2", "BERT-base"],
            1024 => vec!["Cohere embed-english-v3.0"],
            1536 => vec!["OpenAI text-embedding-3-small", "OpenAI ada-002"],
            3072 => vec!["OpenAI text-embedding-3-large"],
            _ => vec!["Custom provider"],
        }
    }
}
