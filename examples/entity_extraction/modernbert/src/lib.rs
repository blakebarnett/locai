//! # ModernBERT Named Entity Recognition for Locai
//!
//! This crate provides a ModernBERT-based named entity recognition (NER) extractor
//! using ModernBERT and the Candle framework with Locai's generic pipeline architecture.

pub mod extractor;
pub mod legacy;

pub use extractor::{
    ModernBertExtractor, ModernBertNERModel,
    create_modernbert_extractor_with_manager,
    create_unified_extraction_pipeline,
};
pub use legacy::LegacyModernBertNerExtractor;

// Re-export the pipeline types for convenience
pub use locai::entity_extraction::{
    RawEntityExtractor, EntityExtractionPipeline, RawEntity, GenericEntityType,
    EntityValidator, EntityPostProcessor, ValidationContext,
    ConfidenceValidator, EntityMerger, EntityDeduplicator,
}; 