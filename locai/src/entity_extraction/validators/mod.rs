//! Generic entity validators for pipeline architecture.

pub mod confidence;
pub mod entity_merger;

pub use confidence::ConfidenceValidator;
pub use entity_merger::EntityMerger; 