//! Core memory functionality

pub mod memory_manager;
pub mod search;
pub mod util;

pub use memory_manager::MemoryManager;
pub use search::{SearchResult, SearchOptions, SearchStrategy, SearchTypeFilter, SearchContent, MatchInfo, SearchContext, SearchMetadata};
pub use util::{enabled_features, is_feature_enabled, has_embedding_support, has_http_capability};

// Placeholder for future implementation 