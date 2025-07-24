//! Error types for ML operations

use std::fmt;
use thiserror::Error;

/// Error type for ML operations
#[derive(Debug, Error)]
pub enum MLError {
    /// Error during model loading
    #[error("Failed to load model: {0}")]
    ModelLoading(String),
    
    /// Error during tokenization
    #[error("Tokenization error: {0}")]
    Tokenization(String),
    
    /// Error during embedding generation
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    /// Error during model initialization
    #[error("Model initialization error: {0}")]
    Initialization(String),
    
    /// Error related to model configuration
    #[error("Model configuration error: {0}")]
    Configuration(String),
    
    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    /// Model registry error
    #[error("Model registry error: {0}")]
    Registry(String),
    
    /// IO error during model operations
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    
    /// Other unexpected errors
    #[error("{0}")]
    Other(String),
}

impl MLError {
    /// Create a new model loading error
    pub fn model_loading(msg: impl fmt::Display) -> Self {
        Self::ModelLoading(msg.to_string())
    }
    
    /// Create a new tokenization error
    pub fn tokenization(msg: impl fmt::Display) -> Self {
        Self::Tokenization(msg.to_string())
    }
    
    /// Create a new embedding error
    pub fn embedding(msg: impl fmt::Display) -> Self {
        Self::Embedding(msg.to_string())
    }
    
    /// Create a new initialization error
    pub fn initialization(msg: impl fmt::Display) -> Self {
        Self::Initialization(msg.to_string())
    }
    
    /// Create a new configuration error
    pub fn configuration(msg: impl fmt::Display) -> Self {
        Self::Configuration(msg.to_string())
    }
    
    /// Create a new model not found error
    pub fn model_not_found(msg: impl fmt::Display) -> Self {
        Self::ModelNotFound(msg.to_string())
    }
    
    /// Create a new registry error
    pub fn registry(msg: impl fmt::Display) -> Self {
        Self::Registry(msg.to_string())
    }
    
    /// Create a new other error
    pub fn other(msg: impl fmt::Display) -> Self {
        Self::Other(msg.to_string())
    }
}

/// Result type for ML operations
pub type Result<T> = std::result::Result<T, MLError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    
    #[test]
    fn test_error_display() {
        let error = MLError::ModelLoading("test error".to_string());
        assert_eq!(error.to_string(), "Failed to load model: test error");
        
        let error = MLError::Tokenization("tokenizer failed".to_string());
        assert_eq!(error.to_string(), "Tokenization error: tokenizer failed");
        
        let error = MLError::Embedding("embedding failed".to_string());
        assert_eq!(error.to_string(), "Embedding error: embedding failed");
        
        let error = MLError::ModelNotFound("model-123".to_string());
        assert_eq!(error.to_string(), "Model not found: model-123");
    }
    
    #[test]
    fn test_error_factory_methods() {
        let error = MLError::model_loading("test error");
        assert!(matches!(error, MLError::ModelLoading(_)));
        
        let error = MLError::tokenization("tokenizer failed");
        assert!(matches!(error, MLError::Tokenization(_)));
        
        let error = MLError::embedding("embedding failed");
        assert!(matches!(error, MLError::Embedding(_)));
        
        let error = MLError::initialization("init failed");
        assert!(matches!(error, MLError::Initialization(_)));
        
        let error = MLError::configuration("config error");
        assert!(matches!(error, MLError::Configuration(_)));
        
        let error = MLError::model_not_found("model-123");
        assert!(matches!(error, MLError::ModelNotFound(_)));
        
        let error = MLError::registry("registry error");
        assert!(matches!(error, MLError::Registry(_)));
        
        let error = MLError::other("unexpected error");
        assert!(matches!(error, MLError::Other(_)));
    }
    
    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let ml_error = MLError::from(io_error);
        assert!(matches!(ml_error, MLError::IO(_)));
    }
} 