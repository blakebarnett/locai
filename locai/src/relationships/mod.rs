//! Generic Relationship Management
//!
//! Provides relationship tracking, analysis, and management capabilities
//! that can be used by any multi-agent application.

pub mod analyzer;
pub mod dynamics;
pub mod manager;
pub mod storage;
pub mod types;

// Re-export key types for convenience
pub use analyzer::RelationshipAnalyzer;
pub use dynamics::{AlliancePattern, ConflictZone, GroupDynamics, InfluenceNetwork};
pub use manager::RelationshipManager;
pub use storage::RelationshipStorage;
pub use types::{
    EmotionalState, EventType, InteractionStyle, Mood, Relationship, RelationshipContext,
    RelationshipEvent, RelationshipImpact, RelationshipType, TrendDirection,
};
