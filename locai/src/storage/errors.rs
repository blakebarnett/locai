//! Error types for storage operations

use std::error::Error;
use std::fmt;

/// Error type for storage operations
#[derive(Debug)]
pub enum StorageError {
    /// Configuration error
    Configuration(String),

    /// Connection error
    Connection(String),

    /// Operation error
    Operation(String),

    /// Query error
    Query(String),

    /// Transaction error
    Transaction(String),

    /// Internal error
    Internal(String),

    /// Validation error
    Validation(String),

    /// Data not found
    NotFound(String),

    /// Item already exists
    AlreadyExists(String),

    /// Backend-specific error
    Backend(String),

    /// Serialization/deserialization error
    Serialization(String),

    /// Data conversion error
    Conversion(String),

    /// Type mismatch error
    TypeMismatch(String),

    /// Unsupported storage type
    UnsupportedStorageType,

    /// Storage timeout error
    Timeout(String),

    /// Authentication error
    Authentication(String),

    /// Authorization error
    Authorization(String),

    /// Temporary/transient error
    Temporary(String),

    /// Multiple errors occurred
    Multiple(Vec<Box<StorageError>>),

    /// Other error
    Other(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            StorageError::Connection(msg) => write!(f, "Connection error: {}", msg),
            StorageError::Operation(msg) => write!(f, "Operation error: {}", msg),
            StorageError::Query(msg) => write!(f, "Query error: {}", msg),
            StorageError::Transaction(msg) => write!(f, "Transaction error: {}", msg),
            StorageError::Internal(msg) => write!(f, "Internal error: {}", msg),
            StorageError::Validation(msg) => write!(f, "Validation error: {}", msg),
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            StorageError::Backend(msg) => write!(f, "Backend error: {}", msg),
            StorageError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            StorageError::Conversion(msg) => write!(f, "Conversion error: {}", msg),
            StorageError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            StorageError::UnsupportedStorageType => write!(f, "Unsupported storage type"),
            StorageError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            StorageError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            StorageError::Authorization(msg) => write!(f, "Authorization error: {}", msg),
            StorageError::Temporary(msg) => write!(f, "Temporary error: {}", msg),
            StorageError::Multiple(errors) => {
                write!(f, "Multiple errors: ")?;
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", err)?;
                }
                Ok(())
            }
            StorageError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Error for StorageError {}

/// Convert a JSON error to a storage error
impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Serialization(err.to_string())
    }
}

/// Convert a standard IO error to a storage error
impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Operation(err.to_string())
    }
}

/// Convert LocaiError to StorageError
impl From<crate::LocaiError> for StorageError {
    fn from(err: crate::LocaiError) -> Self {
        match err {
            crate::LocaiError::Storage(s) => StorageError::Operation(s),
            crate::LocaiError::ML(s) => StorageError::Other(s),
            crate::LocaiError::Configuration(s) => StorageError::Configuration(s),
            crate::LocaiError::Memory(s) => StorageError::Other(s),
            crate::LocaiError::Entity(s) => StorageError::Other(s),
            crate::LocaiError::Relationship(s) => StorageError::Other(s),
            crate::LocaiError::Version(s) => StorageError::Other(s),
            crate::LocaiError::MLNotConfigured => {
                StorageError::Configuration("ML service not configured".to_string())
            }
            crate::LocaiError::StorageNotAccessible { path } => {
                StorageError::Configuration(format!("Storage not accessible: {}", path))
            }
            crate::LocaiError::InvalidEmbeddingModel { model } => {
                StorageError::Configuration(format!("Invalid embedding model: {}", model))
            }
            crate::LocaiError::Connection(s) => StorageError::Other(s),
            crate::LocaiError::Authentication(s) => StorageError::Other(s),
            crate::LocaiError::Protocol(s) => StorageError::Other(s),
            crate::LocaiError::Timeout(s) => StorageError::Timeout(s),
            crate::LocaiError::EmptySearchQuery => {
                StorageError::Other("Empty search query".to_string())
            }
            crate::LocaiError::NoMemoriesFound => {
                StorageError::Other("No memories found".to_string())
            }
            crate::LocaiError::FeatureNotEnabled { feature } => {
                StorageError::Configuration(format!("Feature not enabled: {}", feature))
            }
            crate::LocaiError::Other(s) => StorageError::Other(s),
            crate::LocaiError::Logging(_) => StorageError::Other("Logging error".to_string()),
        }
    }
}

// This allows StorageError to be converted to the top-level LocaiError
impl From<StorageError> for crate::LocaiError {
    fn from(err: StorageError) -> Self {
        crate::LocaiError::Storage(err.to_string())
    }
}
