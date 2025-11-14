//! Access tracking for memory versions.
//!
//! Tracks access patterns to inform promotion decisions (delta -> full copy).

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Access statistics for a version
#[derive(Debug, Clone)]
pub struct VersionAccessStats {
    pub version_id: String,
    pub access_count: u32,
    pub first_accessed: Option<DateTime<Utc>>,
    pub last_accessed: DateTime<Utc>,
    pub total_reconstruction_time_ms: u64,
    pub average_reconstruction_time_ms: f64,
}

impl VersionAccessStats {
    pub fn new(version_id: String) -> Self {
        Self {
            version_id,
            access_count: 0,
            first_accessed: None,
            last_accessed: Utc::now(),
            total_reconstruction_time_ms: 0,
            average_reconstruction_time_ms: 0.0,
        }
    }

    pub fn record_access(&mut self, reconstruction_time_ms: u64) {
        self.access_count += 1;
        if self.first_accessed.is_none() {
            self.first_accessed = Some(Utc::now());
        }
        self.last_accessed = Utc::now();
        self.total_reconstruction_time_ms += reconstruction_time_ms;
        self.average_reconstruction_time_ms =
            self.total_reconstruction_time_ms as f64 / self.access_count as f64;
    }
}

/// Access tracker for version promotion decisions
#[derive(Debug)]
pub struct VersionAccessTracker {
    stats: Arc<Mutex<HashMap<String, VersionAccessStats>>>,
}

impl VersionAccessTracker {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record an access to a version
    pub async fn record_access(&self, version_id: String, reconstruction_time_ms: u64) {
        let mut stats = self.stats.lock().await;
        let entry = stats
            .entry(version_id.clone())
            .or_insert_with(|| VersionAccessStats::new(version_id));
        entry.record_access(reconstruction_time_ms);
    }

    /// Get access statistics for a version
    pub async fn get_stats(&self, version_id: &str) -> Option<VersionAccessStats> {
        let stats = self.stats.lock().await;
        stats.get(version_id).cloned()
    }

    /// Check if a version should be promoted based on access patterns
    pub async fn should_promote(
        &self,
        version_id: &str,
        config: &crate::config::VersioningConfig,
    ) -> bool {
        if !config.enable_auto_promotion {
            return false;
        }

        let stats = self.stats.lock().await;
        if let Some(stat) = stats.get(version_id) {
            // Check access frequency threshold
            if stat.access_count >= config.promotion_access_threshold {
                // Check time window
                if let Some(first_accessed) = stat.first_accessed {
                    let time_window =
                        chrono::Duration::hours(config.promotion_time_window_hours as i64);
                    if stat.last_accessed - first_accessed <= time_window {
                        return true;
                    }
                }
            }

            // Check reconstruction cost threshold
            if stat.average_reconstruction_time_ms > config.promotion_cost_threshold_ms as f64 {
                return true;
            }
        }

        false
    }

    /// Clear old access statistics (older than time window)
    pub async fn cleanup_old_stats(&self, time_window_hours: u64) {
        let cutoff = Utc::now() - chrono::Duration::hours(time_window_hours as i64);
        let mut stats = self.stats.lock().await;
        stats.retain(|_, stat| stat.last_accessed > cutoff);
    }

    /// Get all statistics
    pub async fn get_all_stats(&self) -> Vec<VersionAccessStats> {
        let stats = self.stats.lock().await;
        stats.values().cloned().collect()
    }
}

impl Default for VersionAccessTracker {
    fn default() -> Self {
        Self::new()
    }
}
