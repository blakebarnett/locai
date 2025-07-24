use std::path::PathBuf;
use serde_json;

use crate::ml::config::ModelConfig;
use crate::ml::error::Result;
use std::sync::Arc;

use super::config::CandleConfig;
use super::utils::ModelCache;
use super::model::CandleEmbeddingModel;

/// Builder for creating Candle embedding models
pub struct CandleModelBuilder {
    /// The model configuration
    pub model_config: ModelConfig,
    /// Candle-specific configuration
    pub candle_config: CandleConfig,
    /// Optional cache directory
    pub cache_dir: Option<PathBuf>,
}

impl CandleModelBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        let model_config = ModelConfig {
            model_id: "".to_string(),
            name: "Candle Embedding Model".to_string(),
            source: crate::ml::config::ModelSource::Remote {
                model_id: "".to_string(),
                revision: None,
            },
            cache: crate::ml::config::CacheConfig {
                enabled: true,
                cache_dir: None,
                max_cache_size: None,
            },
            dimensions: None,
            max_seq_length: None,
            device: Some("cpu".to_string()),
            parameters: serde_json::json!({}),
        };
        
        Self {
            model_config,
            candle_config: CandleConfig::default(),
            cache_dir: None,
        }
    }
    
    /// Create a new builder with the given model ID
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        let model_id = model_id.into();
        self.model_config.model_id = model_id.clone();
        self.model_config.name = format!("Candle {}", model_id);
        self.model_config.source = crate::ml::config::ModelSource::Remote {
            model_id: model_id.clone(),
            revision: None,
        };
        self
    }
    
    /// Set the model name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.model_config.name = name.into();
        self
    }
    
    /// Set the model source to a local path
    pub fn local_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.model_config.source = crate::ml::config::ModelSource::Local {
            path: path.into(),
        };
        self
    }
    
    /// Set the model source to a remote ID
    pub fn remote_id(mut self, model_id: impl Into<String>, revision: Option<String>) -> Self {
        self.model_config.source = crate::ml::config::ModelSource::Remote {
            model_id: model_id.into(),
            revision,
        };
        self
    }
    
    /// Set the cache directory
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(dir.into());
        self
    }
    
    /// Set the model type
    pub fn model_type(mut self, model_type: impl Into<String>) -> Self {
        self.candle_config.model_type = model_type.into();
        self
    }
    
    /// Set the pooling strategy
    pub fn pooling_strategy(mut self, strategy: super::config::PoolingStrategy) -> Self {
        self.candle_config.pooling_strategy = strategy;
        self
    }
    
    /// Set whether to normalize embeddings
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.candle_config.normalize_embeddings = normalize;
        self
    }
    
    /// Set whether to use half precision (fp16)
    pub fn use_fp16(mut self, use_fp16: bool) -> Self {
        self.candle_config.use_fp16 = use_fp16;
        self
    }
    
    /// Set whether to use quantization
    pub fn use_quantization(mut self, use_quantization: bool) -> Self {
        self.candle_config.use_quantization = use_quantization;
        self
    }
    
    /// Set the maximum sequence length
    pub fn max_seq_length(mut self, length: usize) -> Self {
        self.candle_config.max_seq_length = Some(length);
        self.model_config.max_seq_length = Some(length);
        self
    }
    
    /// Set the embedding dimension
    pub fn embedding_dim(mut self, dim: usize) -> Self {
        self.candle_config.embedding_dim = Some(dim);
        self.model_config.dimensions = Some(dim);
        self
    }
    
    /// Build the model
    pub async fn build(self) -> Result<CandleEmbeddingModel> {
        // Create cache directory if not provided
        let cache_dir = self.cache_dir.unwrap_or_else(|| {
            self.model_config.cache.cache_dir.clone().unwrap_or_else(|| {
                let dirs = directories::ProjectDirs::from("org", "locai", "locai")
                    .expect("Could not determine project directories");
                dirs.cache_dir().join("models")
            })
        });
        
        // Create cache
        let cache = Arc::new(ModelCache::new(cache_dir));
        
        // Create the model
        CandleEmbeddingModel::new(self.model_config, self.candle_config, cache).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_model_builder() {
        let temp_dir = tempdir().unwrap();
        let builder = CandleModelBuilder::new()
            .with_model("BAAI/bge-small-en")
            .name("Test Model")
            .cache_dir(temp_dir.path().to_path_buf())
            .pooling_strategy(super::super::config::PoolingStrategy::Mean)
            .with_normalize(true)
            .embedding_dim(384);
            
        // We can't actually build and test the model in a unit test
        // as it would require downloading weights, but we can verify
        // the builder sets up the configuration correctly
        
        let model_config = builder.model_config;
        let candle_config = builder.candle_config;
        
        assert_eq!(model_config.name, "Test Model");
        assert_eq!(model_config.dimensions, Some(384));
        
        assert_eq!(candle_config.pooling_strategy, super::super::config::PoolingStrategy::Mean);
        assert!(candle_config.normalize_embeddings);
        assert_eq!(candle_config.embedding_dim, Some(384));
    }
} 