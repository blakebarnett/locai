//! Entity extraction module for automatically detecting entities in memory content.
//!
//! This module provides functionality to extract entities like people, organizations,
//! locations, dates, emails, URLs, and other important information from memory content
//! and create relationships between memories and these entities.

mod types;
mod traits;
mod basic_extractor;
pub mod config;
mod resolution;
mod automatic_relationships;
// Generic pipeline architecture
pub mod pipeline;
pub mod validators;
pub mod post_processors;

pub use types::*;
pub use traits::*;
pub use basic_extractor::*;
pub use config::*;
pub use resolution::*;
pub use automatic_relationships::*;
// Export pipeline components
pub use pipeline::*;
pub use validators::*;
pub use post_processors::*;

// All model-specific extractors moved to examples - core library is now generic 