//! Configuration for shared storage

/// Configuration for the shared storage
#[derive(Debug, Clone)]
pub struct SharedStorageConfig {
    pub namespace: String,
    pub database: String,
}

impl Default for SharedStorageConfig {
    fn default() -> Self {
        Self {
            namespace: "locai".to_string(),
            database: "main".to_string(),
        }
    }
} 