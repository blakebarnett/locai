//! Memory management and analysis systems
//!
//! This module provides comprehensive memory operations including consolidation,
//! analytics, versioning, and graph-based analysis.

pub mod analytics;
pub mod builders;
pub mod consolidation;
pub mod entity_operations;
pub mod graph_analysis;
pub mod graph_operations;
pub mod messaging;
pub mod operations;
pub mod search_extensions;
pub mod utils;
pub mod versioning;

// Re-export consolidation types
pub use consolidation::{
    ConnectionAnalyzer, ConsolidationConfig, ConsolidationResult, MemoryConnection,
    MemoryConsolidator, MemoryPattern, PatternDetector, PatternType, WisdomExtractor,
    WisdomInsight,
};

// Re-export analytics types
pub use analytics::{
    AnomalySeverity, AnomalyType, GrowthTrends, MemoryAnalyticsEngine as MemoryAnalytics,
    MemoryAnalyticsReport, MemoryAnomaly, MemoryEfficiencyMetrics, MemoryUsageReport,
    TrendDirection, Usage,
};

// Re-export versioning types
pub use versioning::{MemoryVersion as MemoryVersioning, VersionMetadata};

// Re-export graph analysis types
pub use graph_analysis::{InfluenceNetwork, MemoryCommunity, MemoryGraphAnalyzer, TemporalSpan};

// Re-export new module types
pub use builders::MemoryBuilders;
pub use entity_operations::EntityOperations;
pub use graph_operations::GraphOperations;
pub use messaging::MessagingIntegration;
pub use operations::MemoryOperations;
pub use search_extensions::{
    SearchExtensions, SearchMode, UniversalSearchOptions, UniversalSearchResult,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Time range for filtering memories and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a new time range
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Create a time range for the last N days
    pub fn last_days(days: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::days(days);
        Self { start, end }
    }

    /// Create a time range for the last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - chrono::Duration::hours(hours);
        Self { start, end }
    }

    /// Check if a timestamp falls within this range
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }

    /// Get the duration of this time range
    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }
}
