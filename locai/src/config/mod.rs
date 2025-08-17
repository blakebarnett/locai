//! Configuration system for Locai.
//!
//! This module provides a flexible configuration system that supports loading
//! configuration from multiple sources (files, environment variables, etc.)
//! with proper validation and defaults.

mod builder;
mod loader;
mod models;
#[cfg(test)]
mod tests;
mod validation;

pub use builder::ConfigBuilder;
pub use loader::ConfigLoader;
pub use models::*;

/// Default configuration file names that the system will look for
pub const DEFAULT_CONFIG_FILES: &[&str] = &[
    "locai.toml",
    "locai.yaml",
    "locai.yml",
    "locai.json",
    ".locai/config.toml",
    ".locai/config.yaml",
    ".locai/config.yml",
    ".locai/config.json",
];

/// Environment variable prefix for Locai configuration
pub const ENV_PREFIX: &str = "LOCAI_";

/// Configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Error occurred during file loading
    #[error("Failed to load configuration file: {0}")]
    FileLoadError(String),

    /// Error occurred during environment loading
    #[error("Failed to load environment variables: {0}")]
    EnvLoadError(String),

    /// Error occurred during validation
    #[error("Configuration validation error: {0}")]
    ValidationError(String),

    /// Error occurred during parsing
    #[error("Configuration parsing error: {0}")]
    ParseError(String),

    /// General error
    #[error("{0}")]
    Other(String),
}

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;
