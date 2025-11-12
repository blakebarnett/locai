//! Configuration for shared storage

use crate::config::LifecycleTrackingConfig;

/// Configuration for the shared storage
#[derive(Debug, Clone)]
pub struct SharedStorageConfig {
    pub namespace: String,
    pub database: String,
    pub lifecycle_tracking: LifecycleTrackingConfig,
}

impl Default for SharedStorageConfig {
    fn default() -> Self {
        Self {
            namespace: "locai".to_string(),
            database: "main".to_string(),
            lifecycle_tracking: LifecycleTrackingConfig::default(),
        }
    }
}
