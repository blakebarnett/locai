//! Generic Relationship Management
//!
//! Provides relationship tracking, analysis, and management capabilities
//! that can be used by any multi-agent application.

pub mod analyzer;
pub mod dynamics;
pub mod enforcement;
pub mod manager;
pub mod metrics;
pub mod registry;
pub mod storage;
pub mod type_storage;
pub mod types;
pub mod validation;

// Re-export key types for convenience
pub use analyzer::RelationshipAnalyzer;
pub use dynamics::{AlliancePattern, ConflictZone, GroupDynamics, InfluenceNetwork};
pub use enforcement::{ConstraintEnforcer, EnforcementError, EnforcementResult};
pub use manager::RelationshipManager;
pub use metrics::{MetricsSnapshot, RelationshipMetrics};
pub use registry::{RegistryError, RelationshipTypeDef, RelationshipTypeRegistry, RelationshipTypeStorage};
pub use storage::RelationshipStorage;
pub use type_storage::SurrealRelationshipTypeStorage;
pub use types::{
    EmotionalState, EventType, InteractionStyle, Mood, Relationship, RelationshipContext,
    RelationshipEvent, RelationshipImpact, RelationshipType, TrendDirection,
};
pub use validation::{SchemaValidator, ValidationError};
