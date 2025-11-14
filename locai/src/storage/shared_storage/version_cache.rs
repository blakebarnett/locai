//! Version cache implementation for memory versioning.
//!
//! Provides context-aware caching for reconstructed memory versions:
//! - Server mode: LRU cache with TTL
//! - Embedded mode: Simple HashMap cache (no TTL needed)

use chrono::{DateTime, Utc};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::CacheStrategy;
use crate::models::Memory;

/// Cached version entry with timestamp
#[derive(Debug)]
struct CachedVersion {
    memory: Memory,
    #[allow(dead_code)] // Reserved for future TTL implementation in embedded mode
    cached_at: DateTime<Utc>,
}

/// Context-aware version cache
#[derive(Debug)]
pub enum VersionCache {
    /// Server mode: LRU cache for long-running processes
    Server {
        cache: Arc<Mutex<LruCache<String, Memory>>>,
        max_size: usize,
        ttl_seconds: u64,
    },
    /// Embedded mode: Simple map cache for short-lived processes
    #[allow(private_interfaces)] // CachedVersion is only used internally
    Embedded {
        cache: Arc<Mutex<HashMap<String, CachedVersion>>>,
        max_size: usize,
    },
}

impl VersionCache {
    /// Create a new version cache based on configuration
    pub fn new(config: &crate::config::VersioningConfig) -> Self {
        let is_server = Self::detect_server_mode(config);

        if is_server {
            Self::Server {
                cache: Arc::new(Mutex::new(LruCache::new(
                    NonZeroUsize::new(config.cache_size.max(1)).unwrap(),
                ))),
                max_size: config.cache_size,
                ttl_seconds: config.cache_ttl_seconds,
            }
        } else {
            Self::Embedded {
                cache: Arc::new(Mutex::new(HashMap::new())),
                max_size: config.cache_size,
            }
        }
    }

    /// Detect if running in server mode
    fn detect_server_mode(config: &crate::config::VersioningConfig) -> bool {
        // Check explicit override first
        if let Some(server_mode) = config.server_mode {
            return server_mode;
        }

        // Auto-detect based on cache strategy
        match config.cache_strategy {
            CacheStrategy::Server => true,
            CacheStrategy::Embedded => false,
            CacheStrategy::Auto => {
                // Check environment variables
                std::env::var("LOCAI_SERVER_MODE").is_ok()
                    || std::env::var("LOCAI_HOST").is_ok()
                    || std::env::var("LOCAI_PORT").is_ok()
            }
        }
    }

    /// Get a cached version
    pub async fn get(&self, version_id: &str) -> Option<Memory> {
        match self {
            Self::Server { cache, .. } => {
                let mut cache = cache.lock().await;
                cache.get(version_id).cloned()
            }
            Self::Embedded { cache, .. } => {
                let cache = cache.lock().await;
                cache.get(version_id).map(|cached| cached.memory.clone())
            }
        }
    }

    /// Put a version in the cache
    pub async fn put(&self, version_id: String, memory: Memory) {
        match self {
            Self::Server { cache, .. } => {
                let mut cache = cache.lock().await;
                cache.put(version_id, memory);
            }
            Self::Embedded { cache, max_size } => {
                let mut cache = cache.lock().await;
                // Simple FIFO eviction when full
                if cache.len() >= *max_size && !cache.contains_key(&version_id) {
                    // Remove oldest entry (first inserted)
                    let oldest_key = cache.keys().next().cloned();
                    if let Some(key) = oldest_key {
                        cache.remove(&key);
                    }
                }
                cache.insert(
                    version_id,
                    CachedVersion {
                        memory,
                        cached_at: Utc::now(),
                    },
                );
            }
        }
    }

    /// Clear the cache
    pub async fn clear(&self) {
        match self {
            Self::Server { cache, .. } => {
                let mut cache = cache.lock().await;
                cache.clear();
            }
            Self::Embedded { cache, .. } => {
                let mut cache = cache.lock().await;
                cache.clear();
            }
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        match self {
            Self::Server {
                cache, max_size, ..
            } => {
                let cache = cache.lock().await;
                CacheStats {
                    size: cache.len(),
                    max_size: *max_size,
                    mode: "server".to_string(),
                }
            }
            Self::Embedded { cache, max_size } => {
                let cache = cache.lock().await;
                CacheStats {
                    size: cache.len(),
                    max_size: *max_size,
                    mode: "embedded".to_string(),
                }
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub mode: String,
}
