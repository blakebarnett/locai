//! Configuration for Candle embedding models

use serde::{Deserialize, Serialize};

/// Pooling strategies for embedding generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PoolingStrategy {
    /// Mean pooling of all token embeddings
    Mean,
    /// Max pooling of all token embeddings
    Max,
    /// Use [CLS] token embedding
    Cls,
    /// Use the last token embedding
    Last,
}

impl Default for PoolingStrategy {
    fn default() -> Self {
        Self::Mean
    }
}

/// Configuration for Candle models
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandleConfig {
    /// Model type (e.g., "BERT", "MPNet", "E5")
    pub model_type: String,
    
    /// Pooling strategy for generating sentence embeddings
    #[serde(default)]
    pub pooling_strategy: PoolingStrategy,
    
    /// Normalize embeddings to unit length
    #[serde(default = "default_true")]
    pub normalize_embeddings: bool,
    
    /// Use fp16 precision
    #[serde(default = "default_false")]
    pub use_fp16: bool,
    
    /// Use quantization
    #[serde(default = "default_false")]
    pub use_quantization: bool,
    
    /// Maximum sequence length to use
    pub max_seq_length: Option<usize>,
    
    /// Dimension of the embeddings
    pub embedding_dim: Option<usize>,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl Default for CandleConfig {
    fn default() -> Self {
        Self {
            model_type: "sentence-transformer".to_string(),
            pooling_strategy: PoolingStrategy::default(),
            normalize_embeddings: true,
            use_fp16: false,
            use_quantization: false,
            max_seq_length: None,
            embedding_dim: None,
        }
    }
}

/// Builder for CandleConfig
pub struct CandleConfigBuilder {
    config: CandleConfig,
}

impl CandleConfigBuilder {
    /// Create a new config builder
    pub fn new() -> Self {
        Self {
            config: CandleConfig::default(),
        }
    }
    
    /// Set the model type
    pub fn model_type(mut self, model_type: impl Into<String>) -> Self {
        self.config.model_type = model_type.into();
        self
    }
    
    /// Set the pooling strategy
    pub fn pooling_strategy(mut self, strategy: PoolingStrategy) -> Self {
        self.config.pooling_strategy = strategy;
        self
    }
    
    /// Set whether to normalize embeddings
    pub fn normalize_embeddings(mut self, normalize: bool) -> Self {
        self.config.normalize_embeddings = normalize;
        self
    }
    
    /// Set whether to use fp16 precision
    pub fn use_fp16(mut self, use_fp16: bool) -> Self {
        self.config.use_fp16 = use_fp16;
        self
    }
    
    /// Set whether to use quantization
    pub fn use_quantization(mut self, use_quantization: bool) -> Self {
        self.config.use_quantization = use_quantization;
        self
    }
    
    /// Set the maximum sequence length
    pub fn max_seq_length(mut self, length: usize) -> Self {
        self.config.max_seq_length = Some(length);
        self
    }
    
    /// Set the embedding dimension
    pub fn embedding_dim(mut self, dim: usize) -> Self {
        self.config.embedding_dim = Some(dim);
        self
    }
    
    /// Build the configuration
    pub fn build(self) -> CandleConfig {
        self.config
    }
}

impl Default for CandleConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = CandleConfig::default();
        
        assert_eq!(config.model_type, "sentence-transformer");
        assert_eq!(config.pooling_strategy, PoolingStrategy::Mean);
        assert!(config.normalize_embeddings);
        assert!(!config.use_fp16);
        assert!(!config.use_quantization);
        assert!(config.max_seq_length.is_none());
        assert!(config.embedding_dim.is_none());
    }
    
    #[test]
    fn test_config_builder() {
        let config = CandleConfigBuilder::new()
            .model_type("BERT")
            .pooling_strategy(PoolingStrategy::Cls)
            .normalize_embeddings(false)
            .use_fp16(true)
            .max_seq_length(128)
            .embedding_dim(768)
            .build();
        
        assert_eq!(config.model_type, "BERT");
        assert_eq!(config.pooling_strategy, PoolingStrategy::Cls);
        assert!(!config.normalize_embeddings);
        assert!(config.use_fp16);
        assert!(!config.use_quantization);
        assert_eq!(config.max_seq_length, Some(128));
        assert_eq!(config.embedding_dim, Some(768));
    }
    
    #[test]
    fn test_pooling_strategy_default() {
        assert_eq!(PoolingStrategy::default(), PoolingStrategy::Mean);
    }
} 