//! Embedding model interface for generating text embeddings

use std::fmt::Debug;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use super::error::Result;
use super::tokenizer::{Tokenizer, TokenizerOptions};
use super::config::ModelConfig;

/// Type for embedding vectors
pub type EmbeddingVector = Vec<f32>;

/// A batch of embedding vectors
pub type EmbeddingBatch = Vec<EmbeddingVector>;

/// Model metadata with information about capabilities and dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Model name
    pub name: String,
    /// Model identifier
    pub model_id: String,
    /// Model version or revision
    pub version: Option<String>,
    /// Embedding vector dimensions
    pub dimensions: usize,
    /// Maximum sequence length
    pub max_seq_length: Option<usize>,
    /// Model description
    pub description: Option<String>,
    /// License information
    pub license: Option<String>,
    /// Additional model capabilities
    pub capabilities: Vec<String>,
}

/// Options for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingOptions {
    /// Normalize the embedding vectors to unit length
    pub normalize: bool,
    /// Tokenizer options to use
    pub tokenizer_options: Option<TokenizerOptions>,
    /// Pooling strategy to use for token embeddings
    pub pooling: PoolingStrategy,
}

impl Default for EmbeddingOptions {
    fn default() -> Self {
        Self {
            normalize: true,
            tokenizer_options: None,
            pooling: PoolingStrategy::Mean,
        }
    }
}

/// Pooling strategy for combining token embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolingStrategy {
    /// Use mean pooling (average of all token embeddings)
    Mean,
    /// Use max pooling (element-wise maximum)
    Max,
    /// Use the embedding of the CLS token
    Cls,
    /// Use the embedding of the last token
    Last,
}

/// Interface for embedding models that generate vector representations of text
#[async_trait]
pub trait EmbeddingModel: Send + Sync + 'static {
    /// Get the tokenizer used by this model
    fn tokenizer(&self) -> &dyn Tokenizer;
    
    /// Get model metadata
    fn metadata(&self) -> &ModelMetadata;
    
    /// Get the embedding dimension
    fn dimension(&self) -> usize {
        self.metadata().dimensions
    }
    
    /// Get the configuration for this model
    fn config(&self) -> &ModelConfig;
    
    /// Generate an embedding for a single text
    async fn embed_text(&self, text: &str, options: Option<EmbeddingOptions>) -> Result<EmbeddingVector>;
    
    /// Generate embeddings for a batch of texts
    async fn embed_texts(&self, texts: &[String], options: Option<EmbeddingOptions>) -> Result<EmbeddingBatch>;
    
    /// Calculate the similarity between two vectors
    fn similarity(&self, a: &EmbeddingVector, b: &EmbeddingVector) -> f32 {
        // Default implementation using cosine similarity
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
        let a_norm: f32 = a.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let b_norm: f32 = b.iter().map(|&x| x * x).sum::<f32>().sqrt();

        if a_norm == 0.0 || b_norm == 0.0 {
            return 0.0;
        }

        dot_product / (a_norm * b_norm)
    }
}

/// Mock embedding model for testing
#[cfg(test)]
pub mod mock {
    use super::*;
    use crate::ml::tokenizer::mock::MockTokenizer;
    
    /// A simple mock embedding model for testing
    pub struct MockEmbeddingModel {
        tokenizer: MockTokenizer,
        metadata: ModelMetadata,
        config: ModelConfig,
    }
    
    impl MockEmbeddingModel {
        /// Create a new mock embedding model
        pub fn new(dimension: usize, vocab_size: usize, max_seq_length: usize) -> Self {
            let tokenizer = MockTokenizer::new(vocab_size, max_seq_length);
            let metadata = ModelMetadata {
                name: "Mock Embedding Model".to_string(),
                model_id: "mock-model".to_string(),
                version: Some("1.0.0".to_string()),
                dimensions: dimension,
                max_seq_length: Some(max_seq_length),
                description: Some("A mock embedding model for testing".to_string()),
                license: Some("MIT".to_string()),
                capabilities: vec!["text-embedding".to_string()],
            };
            let config = ModelConfig::default();
            
            Self {
                tokenizer,
                metadata,
                config,
            }
        }
    }
    
    #[async_trait]
    impl EmbeddingModel for MockEmbeddingModel {
        fn tokenizer(&self) -> &dyn Tokenizer {
            &self.tokenizer
        }
        
        fn metadata(&self) -> &ModelMetadata {
            &self.metadata
        }
        
        fn config(&self) -> &ModelConfig {
            &self.config
        }
        
        async fn embed_text(&self, text: &str, options: Option<EmbeddingOptions>) -> Result<EmbeddingVector> {
            let _options = options.unwrap_or_default();
            
            // Generate a deterministic embedding based on the text
            let dimension = self.dimension();
            let mut embedding = vec![0.0; dimension];
            
            // Fill with deterministic values based on the text
            for (i, c) in text.chars().enumerate() {
                let idx = i % dimension;
                embedding[idx] += (c as u32 % 255) as f32 / 255.0;
            }
            
            // Normalize if requested
            if _options.normalize {
                let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for val in &mut embedding {
                        *val /= norm;
                    }
                }
            }
            
            Ok(embedding)
        }
        
        async fn embed_texts(&self, texts: &[String], options: Option<EmbeddingOptions>) -> Result<EmbeddingBatch> {
            let mut results = Vec::with_capacity(texts.len());
            
            for text in texts {
                results.push(self.embed_text(text, options.clone()).await?);
            }
            
            Ok(results)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::mock::MockEmbeddingModel;
    
    // Helper to check if a vector is normalized
    fn is_normalized(vec: &[f32]) -> bool {
        let norm: f32 = vec.iter().map(|&x| x * x).sum::<f32>().sqrt();
        (norm - 1.0).abs() < 1e-5
    }
    
    #[tokio::test]
    async fn test_mock_embedding_model() {
        let model = MockEmbeddingModel::new(128, 1000, 512);
        
        // Metadata checks
        assert_eq!(model.dimension(), 128);
        assert_eq!(model.metadata().max_seq_length, Some(512));
        assert_eq!(model.metadata().model_id, "mock-model");
        
        // Embedding a single text
        let embedding = model.embed_text("hello world", None).await.unwrap();
        
        // Check dimensions and normalization
        assert_eq!(embedding.len(), 128);
        assert!(is_normalized(&embedding));
    }
    
    #[tokio::test]
    async fn test_embedding_without_normalization() {
        let model = MockEmbeddingModel::new(128, 1000, 512);
        
        let options = EmbeddingOptions {
            normalize: false,
            ..Default::default()
        };
        
        let embedding = model.embed_text("hello world", Some(options)).await.unwrap();
        
        // Should not be normalized
        assert!(!is_normalized(&embedding));
    }
    
    #[tokio::test]
    async fn test_embedding_batch() {
        let model = MockEmbeddingModel::new(128, 1000, 512);
        let texts = vec![
            "first sentence".to_string(),
            "second example".to_string(),
            "third text for embedding".to_string(),
        ];
        
        let embeddings = model.embed_texts(&texts, None).await.unwrap();
        
        // Check batch size and dimension
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 128);
        assert_eq!(embeddings[1].len(), 128);
        assert_eq!(embeddings[2].len(), 128);
        
        // All should be normalized
        assert!(is_normalized(&embeddings[0]));
        assert!(is_normalized(&embeddings[1]));
        assert!(is_normalized(&embeddings[2]));
    }
    
    #[tokio::test]
    async fn test_deterministic_embedding() {
        // Same model parameters
        let model1 = MockEmbeddingModel::new(128, 1000, 512);
        let model2 = MockEmbeddingModel::new(128, 1000, 512);
        
        // Same text
        let text = "hello world";
        
        // Generate embeddings
        let embedding1 = model1.embed_text(text, None).await.unwrap();
        let embedding2 = model2.embed_text(text, None).await.unwrap();
        
        // Embeddings should be identical
        assert_eq!(embedding1, embedding2);
    }
    
    #[test]
    fn test_similarity_function() {
        let model = MockEmbeddingModel::new(4, 1000, 512);
        
        // Simple orthogonal vectors
        let vec1 = vec![1.0, 0.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0, 0.0];
        
        // Similarity should be zero (orthogonal)
        assert!(model.similarity(&vec1, &vec2).abs() < 1e-5);
        
        // Self-similarity should be 1.0
        assert!((model.similarity(&vec1, &vec1) - 1.0).abs() < 1e-5);
        
        // Parallel vectors
        let vec3 = vec![2.0, 0.0, 0.0, 0.0];
        assert!((model.similarity(&vec1, &vec3) - 1.0).abs() < 1e-5);
        
        // 45-degree angle (cos(π/4) = 1/√2 ≈ 0.7071)
        let vec4 = vec![1.0, 1.0, 0.0, 0.0];
        let vec5 = vec![1.0, 0.0, 0.0, 0.0];
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!((model.similarity(&vec4, &vec5) - expected).abs() < 1e-5);
    }
    
    #[test]
    fn test_model_metadata_accessors() {
        let model = MockEmbeddingModel::new(128, 1000, 512);
        
        assert_eq!(model.dimension(), 128);
        assert_eq!(model.metadata().name, "Mock Embedding Model");
        assert_eq!(model.metadata().capabilities, vec!["text-embedding".to_string()]);
        assert_eq!(model.metadata().license, Some("MIT".to_string()));
    }
} 