//! Entity extraction module for automatically detecting entities in memory content.
//!
//! This module provides functionality to extract entities like people, organizations,
//! locations, dates, emails, URLs, and other important information from memory content
//! and create relationships between memories and these entities.

mod automatic_relationships;
mod basic_extractor;
pub mod config;
mod resolution;
mod traits;
mod types;
// Generic pipeline architecture
pub mod pipeline;
pub mod post_processors;
pub mod validators;

pub use automatic_relationships::*;
pub use basic_extractor::*;
pub use config::*;
pub use resolution::*;
pub use traits::*;
pub use types::*;
// Export pipeline components
pub use pipeline::*;
pub use post_processors::*;
pub use validators::*;

// All model-specific extractors moved to examples - core library is now generic
