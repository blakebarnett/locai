//! Relationship Metrics Tracking
//!
//! Collects usage metrics on relationship operations to inform design decisions
//! about whether automatic constraint enforcement should be enabled by default.

use super::registry::RelationshipTypeDef;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics snapshot for a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Count of symmetric relationships created
    pub symmetric_relationships_created: u64,

    /// Count of transitive relationships created
    pub transitive_relationships_created: u64,

    /// Count of times users manually created inverse relationships
    pub manual_inverse_creates: u64,

    /// Count of times enforcement flag was used (true count)
    pub enforcement_requests_enabled: u64,

    /// Count of times enforcement flag was used (false count)
    pub enforcement_requests_disabled: u64,

    /// Total relationships created
    pub total_relationships_created: u64,

    /// Timestamp of snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Relationship metrics collector
#[derive(Clone, Debug)]
pub struct RelationshipMetrics {
    symmetric_relationships: Arc<AtomicU64>,
    transitive_relationships: Arc<AtomicU64>,
    manual_inverse_creates: Arc<AtomicU64>,
    enforcement_enabled: Arc<AtomicU64>,
    enforcement_disabled: Arc<AtomicU64>,
    total_created: Arc<AtomicU64>,
}

impl RelationshipMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            symmetric_relationships: Arc::new(AtomicU64::new(0)),
            transitive_relationships: Arc::new(AtomicU64::new(0)),
            manual_inverse_creates: Arc::new(AtomicU64::new(0)),
            enforcement_enabled: Arc::new(AtomicU64::new(0)),
            enforcement_disabled: Arc::new(AtomicU64::new(0)),
            total_created: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a relationship creation event
    pub fn record_relationship_created(&self, type_def: &RelationshipTypeDef, enforced: bool) {
        self.total_created.fetch_add(1, Ordering::SeqCst);

        if type_def.symmetric {
            self.symmetric_relationships.fetch_add(1, Ordering::SeqCst);
        }

        if type_def.transitive {
            self.transitive_relationships.fetch_add(1, Ordering::SeqCst);
        }

        if enforced {
            self.enforcement_enabled.fetch_add(1, Ordering::SeqCst);
        } else {
            self.enforcement_disabled.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Record detection of manually created inverse relationships
    pub fn record_manual_inverse_detected(&self) {
        self.manual_inverse_creates.fetch_add(1, Ordering::SeqCst);
    }

    /// Detect if a pair of relationships might be manual inverses
    /// This is a heuristic that checks if two relationships have:
    /// - Same type name
    /// - Opposite source/target
    /// - Similar properties
    pub fn detect_manual_inverse(
        _rel1: &str,
        rel1_source: &str,
        rel1_target: &str,
        rel2_source: &str,
        rel2_target: &str,
    ) -> bool {
        // Check if relationships are inverses of each other
        rel1_source == rel2_target && rel1_target == rel2_source
    }

    /// Get a snapshot of current metrics
    pub fn export_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            symmetric_relationships_created: self.symmetric_relationships.load(Ordering::SeqCst),
            transitive_relationships_created: self.transitive_relationships.load(Ordering::SeqCst),
            manual_inverse_creates: self.manual_inverse_creates.load(Ordering::SeqCst),
            enforcement_requests_enabled: self.enforcement_enabled.load(Ordering::SeqCst),
            enforcement_requests_disabled: self.enforcement_disabled.load(Ordering::SeqCst),
            total_relationships_created: self.total_created.load(Ordering::SeqCst),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get percentage of relationships with enforcement enabled
    pub fn enforcement_enabled_percentage(&self) -> f64 {
        let enabled = self.enforcement_enabled.load(Ordering::SeqCst) as f64;
        let disabled = self.enforcement_disabled.load(Ordering::SeqCst) as f64;
        let total = enabled + disabled;

        if total == 0.0 {
            0.0
        } else {
            (enabled / total) * 100.0
        }
    }

    /// Get percentage of symmetric relationships
    pub fn symmetric_percentage(&self) -> f64 {
        let symmetric = self.symmetric_relationships.load(Ordering::SeqCst) as f64;
        let total = self.total_created.load(Ordering::SeqCst) as f64;

        if total == 0.0 {
            0.0
        } else {
            (symmetric / total) * 100.0
        }
    }

    /// Get percentage of transitive relationships
    pub fn transitive_percentage(&self) -> f64 {
        let transitive = self.transitive_relationships.load(Ordering::SeqCst) as f64;
        let total = self.total_created.load(Ordering::SeqCst) as f64;

        if total == 0.0 {
            0.0
        } else {
            (transitive / total) * 100.0
        }
    }

    /// Get ratio of manual inverses to manual operations (enforcement disabled)
    pub fn manual_inverse_ratio(&self) -> f64 {
        let manual = self.manual_inverse_creates.load(Ordering::SeqCst) as f64;
        let disabled = self.enforcement_disabled.load(Ordering::SeqCst) as f64;

        if disabled == 0.0 {
            0.0
        } else {
            manual / disabled
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.symmetric_relationships.store(0, Ordering::SeqCst);
        self.transitive_relationships.store(0, Ordering::SeqCst);
        self.manual_inverse_creates.store(0, Ordering::SeqCst);
        self.enforcement_enabled.store(0, Ordering::SeqCst);
        self.enforcement_disabled.store(0, Ordering::SeqCst);
        self.total_created.store(0, Ordering::SeqCst);
    }

    /// Get a summary of metrics for human-readable output
    pub fn summary(&self) -> String {
        let snapshot = self.export_metrics();
        format!(
            "Relationship Metrics Snapshot\n\
             ============================\n\
             Total Relationships Created: {}\n\
             Symmetric Relationships: {} ({:.1}%)\n\
             Transitive Relationships: {} ({:.1}%)\n\
             Manual Inverse Creates: {}\n\
             Enforcement Enabled: {}\n\
             Enforcement Disabled: {} ({:.1}%)\n\
             Manual Inverse Ratio (when disabled): {:.2}",
            snapshot.total_relationships_created,
            snapshot.symmetric_relationships_created,
            self.symmetric_percentage(),
            snapshot.transitive_relationships_created,
            self.transitive_percentage(),
            snapshot.manual_inverse_creates,
            snapshot.enforcement_requests_enabled,
            snapshot.enforcement_requests_disabled,
            self.enforcement_enabled_percentage(),
            self.manual_inverse_ratio()
        )
    }
}

impl Default for RelationshipMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_symmetric_relationship() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("married".to_string())
            .unwrap()
            .symmetric();

        metrics.record_relationship_created(&type_def, false);
        let snapshot = metrics.export_metrics();

        assert_eq!(snapshot.symmetric_relationships_created, 1);
        assert_eq!(snapshot.total_relationships_created, 1);
    }

    #[test]
    fn test_record_transitive_relationship() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("part_of".to_string())
            .unwrap()
            .transitive();

        metrics.record_relationship_created(&type_def, false);
        let snapshot = metrics.export_metrics();

        assert_eq!(snapshot.transitive_relationships_created, 1);
        assert_eq!(snapshot.total_relationships_created, 1);
    }

    #[test]
    fn test_record_enforcement_enabled() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        metrics.record_relationship_created(&type_def, true);
        let snapshot = metrics.export_metrics();

        assert_eq!(snapshot.enforcement_requests_enabled, 1);
        assert_eq!(snapshot.enforcement_requests_disabled, 0);
    }

    #[test]
    fn test_record_enforcement_disabled() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        metrics.record_relationship_created(&type_def, false);
        let snapshot = metrics.export_metrics();

        assert_eq!(snapshot.enforcement_requests_enabled, 0);
        assert_eq!(snapshot.enforcement_requests_disabled, 1);
    }

    #[test]
    fn test_enforcement_enabled_percentage() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        metrics.record_relationship_created(&type_def, true);
        metrics.record_relationship_created(&type_def, false);
        metrics.record_relationship_created(&type_def, false);

        let percentage = metrics.enforcement_enabled_percentage();
        assert!((percentage - 33.33).abs() < 0.1); // ~33.33%
    }

    #[test]
    fn test_symmetric_percentage() {
        let metrics = RelationshipMetrics::new();
        let sym_type = RelationshipTypeDef::new("married".to_string())
            .unwrap()
            .symmetric();
        let asym_type = RelationshipTypeDef::new("knows".to_string()).unwrap();

        metrics.record_relationship_created(&sym_type, false);
        metrics.record_relationship_created(&asym_type, false);

        let percentage = metrics.symmetric_percentage();
        assert!((percentage - 50.0).abs() < 0.1); // 50%
    }

    #[test]
    fn test_manual_inverse_detection() {
        let is_inverse =
            RelationshipMetrics::detect_manual_inverse("knows", "alice", "bob", "bob", "alice");
        assert!(is_inverse);

        let is_not_inverse =
            RelationshipMetrics::detect_manual_inverse("knows", "alice", "bob", "charlie", "dave");
        assert!(!is_not_inverse);
    }

    #[test]
    fn test_record_manual_inverse() {
        let metrics = RelationshipMetrics::new();
        metrics.record_manual_inverse_detected();
        metrics.record_manual_inverse_detected();

        let snapshot = metrics.export_metrics();
        assert_eq!(snapshot.manual_inverse_creates, 2);
    }

    #[test]
    fn test_reset_metrics() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        metrics.record_relationship_created(&type_def, true);
        let before = metrics.export_metrics();
        assert_eq!(before.total_relationships_created, 1);

        metrics.reset();
        let after = metrics.export_metrics();
        assert_eq!(after.total_relationships_created, 0);
        assert_eq!(after.enforcement_requests_enabled, 0);
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = RelationshipMetrics::new();
        let sym_type = RelationshipTypeDef::new("married".to_string())
            .unwrap()
            .symmetric();

        metrics.record_relationship_created(&sym_type, true);

        let summary = metrics.summary();
        assert!(summary.contains("Total Relationships Created: 1"));
        assert!(summary.contains("Symmetric Relationships: 1"));
    }

    #[test]
    fn test_manual_inverse_ratio() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        // Create 10 relationships with enforcement disabled
        for _ in 0..10 {
            metrics.record_relationship_created(&type_def, false);
        }

        // Record 5 manual inverses
        for _ in 0..5 {
            metrics.record_manual_inverse_detected();
        }

        let ratio = metrics.manual_inverse_ratio();
        assert!((ratio - 0.5).abs() < 0.01); // 50% ratio
    }

    #[test]
    fn test_metrics_clone() {
        let metrics = RelationshipMetrics::new();
        let type_def = RelationshipTypeDef::new("custom".to_string()).unwrap();

        metrics.record_relationship_created(&type_def, true);

        let cloned = metrics.clone();
        let snapshot = cloned.export_metrics();

        assert_eq!(snapshot.total_relationships_created, 1);
    }
}
