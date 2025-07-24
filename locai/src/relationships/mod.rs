//! Generic Relationship Management
//!
//! Provides relationship tracking, analysis, and management capabilities
//! that can be used by any multi-agent application.

pub mod types;
pub mod manager;
pub mod analyzer;
pub mod dynamics;
pub mod storage;

// Re-export key types for convenience
pub use types::{
    Relationship, RelationshipType, RelationshipEvent, EventType, RelationshipImpact,
    EmotionalState, Mood, TrendDirection, InteractionStyle, RelationshipContext
};
pub use manager::RelationshipManager;
pub use analyzer::RelationshipAnalyzer;
pub use dynamics::{GroupDynamics, AlliancePattern, ConflictZone, InfluenceNetwork};
pub use storage::RelationshipStorage; 