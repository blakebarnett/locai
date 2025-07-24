//! Configuration for ML models

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Model source options
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelSource {
    /// Load a model from a local path
    Local {
        /// Path to the model directory or file
        path: PathBuf,
    },
    /// Load a model from a remote URL (e.g., Hugging Face)
    Remote {
        /// Model identifier (e.g., "BAAI/bge-m3")
        model_id: String,
        /// Optional revision/tag to use
        revision: Option<String>,
    },
}

/// Caching configuration for models
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Directory to use for caching
    pub cache_dir: Option<PathBuf>,
    /// Maximum cache size in bytes
    pub max_cache_size: Option<u64>,
}

/// Configuration for a machine learning model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelConfig {
    /// Unique model identifier
    pub model_id: String,
    /// Human-readable model name
    pub name: String,
    /// Model source information
    pub source: ModelSource,
    /// Caching configuration
    pub cache: CacheConfig,
    /// Model dimensions
    pub dimensions: Option<usize>,
    /// Maximum sequence length
    pub max_seq_length: Option<usize>,
    /// Device to run the model on (e.g., "cpu", "cuda")
    pub device: Option<String>,
    /// Additional model-specific parameters
    #[serde(flatten)]
    pub parameters: serde_json::Value,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_id: "default".to_string(),
            name: "Default Model".to_string(),
            source: ModelSource::Remote {
                model_id: "BAAI/bge-m3".to_string(),
                revision: None,
            },
            cache: CacheConfig {
                enabled: true,
                cache_dir: None,
                max_cache_size: None,
            },
            dimensions: None,
            max_seq_length: None,
            device: Some("cpu".to_string()),
            parameters: serde_json::json!({}),
        }
    }
}

/// Builder for model configuration
pub struct ModelConfigBuilder {
    config: ModelConfig,
}

impl ModelConfigBuilder {
    /// Create a new model configuration builder
    pub fn new() -> Self {
        Self {
            config: ModelConfig::default(),
        }
    }
    
    /// Set the model ID
    pub fn model_id(mut self, model_id: impl Into<String>) -> Self {
        self.config.model_id = model_id.into();
        self
    }
    
    /// Set the model name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }
    
    /// Set the model source to a local path
    pub fn local_source(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.source = ModelSource::Local {
            path: path.into(),
        };
        self
    }
    
    /// Set the model source to a remote URL
    pub fn remote_source(mut self, model_id: impl Into<String>, revision: Option<String>) -> Self {
        self.config.source = ModelSource::Remote {
            model_id: model_id.into(),
            revision,
        };
        self
    }
    
    /// Enable or disable caching
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.config.cache.enabled = enabled;
        self
    }
    
    /// Set the cache directory
    pub fn cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.cache.cache_dir = Some(dir.into());
        self
    }
    
    /// Set the maximum cache size
    pub fn max_cache_size(mut self, size: u64) -> Self {
        self.config.cache.max_cache_size = Some(size);
        self
    }
    
    /// Set the embedding dimensions
    pub fn dimensions(mut self, dimensions: usize) -> Self {
        self.config.dimensions = Some(dimensions);
        self
    }
    
    /// Set the maximum sequence length
    pub fn max_seq_length(mut self, length: usize) -> Self {
        self.config.max_seq_length = Some(length);
        self
    }
    
    /// Set the device to run the model on
    pub fn device(mut self, device: impl Into<String>) -> Self {
        self.config.device = Some(device.into());
        self
    }
    
    /// Set additional model parameters
    pub fn parameters(mut self, parameters: serde_json::Value) -> Self {
        self.config.parameters = parameters;
        self
    }
    
    /// Build the model configuration
    pub fn build(self) -> ModelConfig {
        self.config
    }
}

impl Default for ModelConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        
        assert_eq!(config.model_id, "default");
        assert_eq!(config.name, "Default Model");
        
        match config.source {
            ModelSource::Remote { model_id, revision } => {
                assert_eq!(model_id, "BAAI/bge-m3");
                assert!(revision.is_none());
            },
            _ => panic!("Expected Remote source"),
        }
        
        assert!(config.cache.enabled);
        assert!(config.cache.cache_dir.is_none());
        assert!(config.cache.max_cache_size.is_none());
        
        assert!(config.dimensions.is_none());
        assert!(config.max_seq_length.is_none());
        assert_eq!(config.device, Some("cpu".to_string()));
        
        assert_eq!(config.parameters, serde_json::json!({}));
    }
    
    #[test]
    fn test_model_config_builder() {
        let config = ModelConfigBuilder::new()
            .model_id("test-model")
            .name("Test Model")
            .remote_source("org/model-name", Some("v1.0".to_string()))
            .cache_enabled(false)
            .dimensions(768)
            .max_seq_length(512)
            .device("cuda")
            .parameters(serde_json::json!({
                "quantized": true,
                "precision": "fp16"
            }))
            .build();
        
        assert_eq!(config.model_id, "test-model");
        assert_eq!(config.name, "Test Model");
        
        match config.source {
            ModelSource::Remote { model_id, revision } => {
                assert_eq!(model_id, "org/model-name");
                assert_eq!(revision, Some("v1.0".to_string()));
            },
            _ => panic!("Expected Remote source"),
        }
        
        assert_eq!(config.cache.enabled, false);
        assert_eq!(config.dimensions, Some(768));
        assert_eq!(config.max_seq_length, Some(512));
        assert_eq!(config.device, Some("cuda".to_string()));
        
        assert_eq!(config.parameters.get("quantized").unwrap(), &serde_json::json!(true));
        assert_eq!(config.parameters.get("precision").unwrap(), &serde_json::json!("fp16"));
    }
    
    #[test]
    fn test_local_source_config() {
        let config = ModelConfigBuilder::new()
            .model_id("local-model")
            .local_source("/path/to/model")
            .build();
        
        match config.source {
            ModelSource::Local { path } => {
                assert_eq!(path, PathBuf::from("/path/to/model"));
            },
            _ => panic!("Expected Local source"),
        }
    }
    
    #[test]
    fn test_cache_configuration() {
        let config = ModelConfigBuilder::new()
            .cache_enabled(true)
            .cache_dir("/tmp/model_cache")
            .max_cache_size(1024 * 1024 * 100) // 100MB
            .build();
        
        assert!(config.cache.enabled);
        assert_eq!(config.cache.cache_dir, Some(PathBuf::from("/tmp/model_cache")));
        assert_eq!(config.cache.max_cache_size, Some(104857600));
    }
    
    #[test]
    fn test_serialization() {
        let config = ModelConfigBuilder::new()
            .model_id("test-model")
            .remote_source("org/model-name", None)
            .dimensions(512)
            .build();
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ModelConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.model_id, "test-model");
        assert_eq!(deserialized.dimensions, Some(512));
        
        match deserialized.source {
            ModelSource::Remote { model_id, revision } => {
                assert_eq!(model_id, "org/model-name");
                assert!(revision.is_none());
            },
            _ => panic!("Expected Remote source"),
        }
    }
} 