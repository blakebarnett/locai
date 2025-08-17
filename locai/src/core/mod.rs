//! Core memory functionality

pub mod memory_manager;
pub mod search;
pub mod util;

pub use memory_manager::MemoryManager;
pub use search::{
    MatchInfo, SearchContent, SearchContext, SearchMetadata, SearchOptions, SearchResult,
    SearchStrategy, SearchTypeFilter,
};
pub use util::{enabled_features, has_embedding_support, has_http_capability, is_feature_enabled};

// Placeholder for future implementation
