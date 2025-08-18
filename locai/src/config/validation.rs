//! Configuration validation utilities.
//!
//! This module provides validation functions for configuration values.

use super::ConfigError;
use super::models::*;

/// Validate the entire configuration.
pub fn validate_config(config: &LocaiConfig) -> Result<(), ConfigError> {
    // Validate storage configuration
    validate_storage_config(&config.storage)?;

    // Validate ML configuration
    validate_ml_config(&config.ml)?;

    Ok(())
}

/// Validate storage configuration.
fn validate_storage_config(config: &StorageConfig) -> Result<(), ConfigError> {
    // Validate that the data directory is valid
    if config.data_dir.as_os_str().is_empty() {
        return Err(ConfigError::ValidationError(
            "Data directory cannot be empty".to_string(),
        ));
    }

    // Validate graph storage configuration
    match config.graph.storage_type {
        GraphStorageType::SurrealDB => {
            // Validate SurrealDB configuration
            if config.graph.surrealdb.namespace.is_empty() {
                return Err(ConfigError::ValidationError(
                    "SurrealDB namespace cannot be empty".to_string(),
                ));
            }
            if config.graph.surrealdb.database.is_empty() {
                return Err(ConfigError::ValidationError(
                    "SurrealDB database cannot be empty".to_string(),
                ));
            }
        }
    }

    // Validate vector storage configuration
    match config.vector.storage_type {
        VectorStorageType::SurrealDB => {
            // Validate SurrealDB vector storage configuration
            // For unified storage, use the same config validation as graph storage
            if config.graph.surrealdb.namespace.is_empty() {
                return Err(ConfigError::ValidationError(
                    "SurrealDB namespace cannot be empty".to_string(),
                ));
            }
            if config.graph.surrealdb.database.is_empty() {
                return Err(ConfigError::ValidationError(
                    "SurrealDB database cannot be empty".to_string(),
                ));
            }
        }
        VectorStorageType::Memory => {
            // No additional validation needed for memory storage
        }
    }

    Ok(())
}

/// Validate ML configuration.
fn validate_ml_config(config: &MLConfig) -> Result<(), ConfigError> {
    // Validate model cache directory
    if config.model_cache_dir.as_os_str().is_empty() {
        return Err(ConfigError::ValidationError(
            "Model cache directory cannot be empty".to_string(),
        ));
    }

    // Validate embedding configuration
    match config.embedding.model_type {
        EmbeddingModelType::OpenAI => {
            // Validate model name is set
            if config.embedding.model_name.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "OpenAI model name cannot be empty".to_string(),
                ));
            }

            // Validate service URL for OpenAI
            if config.embedding.service_type == EmbeddingServiceType::Remote
                && (config.embedding.service_url.is_none()
                    || config
                        .embedding
                        .service_url
                        .as_ref()
                        .unwrap()
                        .trim()
                        .is_empty())
            {
                return Err(ConfigError::ValidationError(
                    "Service URL is required for remote OpenAI embedding service".to_string(),
                ));
            }
        }
        EmbeddingModelType::Cohere => {
            // Validate model name is set
            if config.embedding.model_name.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "Cohere model name cannot be empty".to_string(),
                ));
            }

            // Validate service URL for Cohere
            if config.embedding.service_type == EmbeddingServiceType::Remote
                && (config.embedding.service_url.is_none()
                    || config
                        .embedding
                        .service_url
                        .as_ref()
                        .unwrap()
                        .trim()
                        .is_empty())
            {
                return Err(ConfigError::ValidationError(
                    "Service URL is required for remote Cohere embedding service".to_string(),
                ));
            }
        }
        EmbeddingModelType::Custom => {
            // Custom model type requires a model name and possibly a service URL
            if config.embedding.model_name.trim().is_empty() {
                return Err(ConfigError::ValidationError(
                    "Custom embedding model name cannot be empty".to_string(),
                ));
            }

            if config.embedding.service_type == EmbeddingServiceType::Remote
                && (config.embedding.service_url.is_none()
                    || config
                        .embedding
                        .service_url
                        .as_ref()
                        .unwrap()
                        .trim()
                        .is_empty())
            {
                return Err(ConfigError::ValidationError(
                    "Service URL is required for remote custom embedding service".to_string(),
                ));
            }
        }
    }

    Ok(())
}
