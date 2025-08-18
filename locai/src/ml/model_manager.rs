//! Simple embedding utilities for BYOE (Bring Your Own Embeddings) approach
//!
//! This module provides minimal utilities for working with user-provided embeddings,
//! focusing on validation and normalization rather than model management.
//!
//! # Examples
//!
//! Basic embedding validation:
//!
//! ```rust
//! use locai::ml::EmbeddingManager;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create manager with expected dimensions
//!     let manager = EmbeddingManager::with_expected_dimensions(1536);
//!
//!     // Validate embeddings from any provider
//!     // This example shows the concept - you would use your actual embedding provider
//!     let embedding = vec![0.1; 1536]; // Mock embedding from OpenAI
//!     manager.validate_embedding(&embedding)?;
//!
//!     // Normalize if needed
//!     let mut embedding = vec![0.1; 1536]; // Mock embedding from provider
//!     manager.normalize_embedding(&mut embedding)?;
//!     Ok(())
//! }
//! ```

use super::error::{MLError, Result};

/// Simple embedding utilities for BYOE approach
///
/// Provides validation and normalization for user-provided embeddings
/// without managing models or providers.
#[derive(Debug, Clone)]
pub struct EmbeddingManager {
    /// Expected embedding dimensions (optional validation)
    expected_dimensions: Option<usize>,
}

impl EmbeddingManager {
    /// Create a new embedding manager
    pub fn new() -> Self {
        Self {
            expected_dimensions: None,
        }
    }

    /// Create an embedding manager with expected dimensions for validation
    pub fn with_expected_dimensions(expected_dimensions: usize) -> Self {
        Self {
            expected_dimensions: Some(expected_dimensions),
        }
    }

    /// Validate an embedding vector
    ///
    /// Checks for:
    /// - Non-empty vectors
    /// - Expected dimensions (if configured)
    /// - Finite values (no NaN/infinity)
    pub fn validate_embedding(&self, embedding: &[f32]) -> Result<()> {
        if embedding.is_empty() {
            return Err(MLError::embedding("Embedding cannot be empty".to_string()));
        }

        if let Some(expected_dim) = self.expected_dimensions {
            if embedding.len() != expected_dim {
                return Err(MLError::embedding(format!(
                    "Expected embedding dimension {}, got {}",
                    expected_dim,
                    embedding.len()
                )));
            }
        }

        // Check for invalid values
        for (i, &value) in embedding.iter().enumerate() {
            if !value.is_finite() {
                return Err(MLError::embedding(format!(
                    "Invalid value at index {}: {}",
                    i, value
                )));
            }
        }

        Ok(())
    }

    /// Normalize an embedding vector to unit length
    ///
    /// This is useful when working with providers that don't automatically
    /// normalize their embeddings.
    pub fn normalize_embedding(&self, embedding: &mut [f32]) -> Result<()> {
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm == 0.0 {
            return Err(MLError::embedding(
                "Cannot normalize zero vector".to_string(),
            ));
        }

        for value in embedding.iter_mut() {
            *value /= norm;
        }

        Ok(())
    }

    /// Get expected dimensions (if set)
    pub fn expected_dimensions(&self) -> Option<usize> {
        self.expected_dimensions
    }

    /// Check if an embedding has valid dimensions for common providers
    pub fn is_valid_dimension(&self, embedding: &[f32]) -> bool {
        super::utils::is_common_dimension(embedding.len())
    }

    /// Get suggested providers for an embedding's dimensions
    pub fn suggested_providers(&self, embedding: &[f32]) -> Vec<&'static str> {
        super::utils::providers_for_dimension(embedding.len())
    }
}

impl Default for EmbeddingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for EmbeddingManager
#[derive(Debug, Clone)]
pub struct EmbeddingManagerBuilder {
    expected_dimensions: Option<usize>,
}

impl EmbeddingManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            expected_dimensions: None,
        }
    }

    /// Set expected embedding dimensions for validation
    pub fn expected_dimensions(mut self, dimensions: usize) -> Self {
        self.expected_dimensions = Some(dimensions);
        self
    }

    /// Build the embedding manager
    pub fn build(self) -> EmbeddingManager {
        if let Some(dimensions) = self.expected_dimensions {
            EmbeddingManager::with_expected_dimensions(dimensions)
        } else {
            EmbeddingManager::new()
        }
    }
}

impl Default for EmbeddingManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_validation() {
        let manager = EmbeddingManager::new();

        // Valid embedding
        let embedding = vec![1.0, 2.0, 3.0];
        assert!(manager.validate_embedding(&embedding).is_ok());

        // Empty embedding should fail
        let empty_embedding = vec![];
        assert!(manager.validate_embedding(&empty_embedding).is_err());

        // Invalid values should fail
        let invalid_embedding = vec![1.0, f32::NAN, 3.0];
        assert!(manager.validate_embedding(&invalid_embedding).is_err());
    }

    #[test]
    fn test_dimension_validation() {
        let manager = EmbeddingManager::with_expected_dimensions(3);

        // Correct dimension
        let embedding = vec![1.0, 2.0, 3.0];
        assert!(manager.validate_embedding(&embedding).is_ok());

        // Wrong dimension
        let wrong_embedding = vec![1.0, 2.0];
        assert!(manager.validate_embedding(&wrong_embedding).is_err());
    }

    #[test]
    fn test_normalization() {
        let manager = EmbeddingManager::new();
        let mut embedding = vec![3.0, 4.0]; // Should normalize to [0.6, 0.8]

        manager.normalize_embedding(&mut embedding).unwrap();

        // Check that the vector is now unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_zero_vector_normalization() {
        let manager = EmbeddingManager::new();
        let mut embedding = vec![0.0, 0.0, 0.0];

        // Should fail to normalize zero vector
        assert!(manager.normalize_embedding(&mut embedding).is_err());
    }

    #[test]
    fn test_common_dimensions() {
        let manager = EmbeddingManager::new();

        // Common dimensions should be recognized
        assert!(manager.is_valid_dimension(&vec![0.0; 1536])); // OpenAI
        assert!(manager.is_valid_dimension(&vec![0.0; 1024])); // Cohere
        assert!(manager.is_valid_dimension(&vec![0.0; 384])); // BGE Small

        // Uncommon dimension
        assert!(!manager.is_valid_dimension(&vec![0.0; 999]));
    }

    #[test]
    fn test_provider_suggestions() {
        let manager = EmbeddingManager::new();

        let openai_embedding = vec![0.0; 1536];
        let suggestions = manager.suggested_providers(&openai_embedding);
        assert!(suggestions.contains(&"OpenAI text-embedding-3-small"));
    }
}
