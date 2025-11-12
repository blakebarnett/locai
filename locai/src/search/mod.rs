//! Enhanced search scoring module
//!
//! This module provides configurable multi-factor search result scoring,
//! combining BM25 keyword matching, vector similarity, and memory lifecycle
//! metadata to produce comprehensive relevance rankings.
//!
//! # Overview
//!
//! The search scoring system allows applications to customize how search results
//! are ranked by adjusting weights for:
//! - BM25 keyword relevance
//! - Vector embedding similarity
//! - Recency (with configurable time decay)
//! - Access frequency
//! - Priority/importance level
//!
//! # Example
//!
//! ```no_run
//! use locai::search::scoring::ScoringConfig;
//! use locai::search::calculator::ScoreCalculator;
//! use locai::models::Memory;
//!
//! // Create a recency-focused scoring configuration
//! let config = ScoringConfig::recency_focused();
//!
//! // Create a calculator
//! let calculator = ScoreCalculator::new(config);
//!
//! // Use it to score search results (example values)
//! let bm25_score = 0.8;
//! let vector_score = Some(0.9);
//! let memory = Memory::new("id".to_string(), "content".to_string(), locai::models::MemoryType::Fact);
//! let score = calculator.calculate_final_score(
//!     bm25_score,
//!     vector_score,
//!     &memory
//! );
//! ```

pub mod calculator;
pub mod scoring;

pub use calculator::ScoreCalculator;
pub use scoring::{DecayFunction, ScoringConfig};





