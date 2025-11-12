//! Enhanced search scoring configuration and utilities
//!
//! This module provides configurable scoring for search results, combining
//! BM25 keyword matching, vector similarity, and memory lifecycle metadata
//! to produce comprehensive relevance rankings.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Decay functions for time-based score reduction
///
/// These functions model how the importance of information decays over time.
/// They are applied to calculate recency boosts for search results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DecayFunction {
    /// No decay - all memories have equal recency weight
    None,

    /// Linear decay: importance decreases linearly with age
    ///
    /// Formula: `boost * max(0, 1 - age_hours * decay_rate)`
    Linear,

    /// Exponential decay: importance decreases exponentially with age
    ///
    /// Formula: `boost * exp(-decay_rate * age_hours)`
    ///
    /// This closely models human memory and forgetting curves.
    Exponential,

    /// Logarithmic decay: importance decreases logarithmically with age
    ///
    /// Formula: `boost / (1 + age_hours * decay_rate).ln()`
    ///
    /// Slower decay than exponential, useful for long-term memory.
    Logarithmic,
}

impl Default for DecayFunction {
    fn default() -> Self {
        Self::Exponential
    }
}

impl fmt::Display for DecayFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Linear => write!(f, "linear"),
            Self::Exponential => write!(f, "exponential"),
            Self::Logarithmic => write!(f, "logarithmic"),
        }
    }
}

/// Configuration for multi-factor search scoring
///
/// This struct controls how different scoring factors are weighted and combined
/// to produce a final relevance score for search results. All weights are normalized
/// automatically to sum to 1.0 (or as close as possible).
///
/// # Example
///
/// ```no_run
/// use locai::search::scoring::{ScoringConfig, DecayFunction};
///
/// let config = ScoringConfig {
///     bm25_weight: 1.0,
///     vector_weight: 1.0,
///     recency_boost: 0.5,
///     access_boost: 0.3,
///     priority_boost: 0.2,
///     decay_function: DecayFunction::Exponential,
///     decay_rate: 0.1,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Weight for BM25 keyword matching (0.0 - 1.0)
    ///
    /// BM25 is a proven probabilistic relevance framework that considers
    /// term frequency and document length. Default: 1.0
    pub bm25_weight: f32,

    /// Weight for vector embedding similarity (0.0 - 1.0)
    ///
    /// Vector search considers semantic similarity using embeddings.
    /// Default: 1.0
    pub vector_weight: f32,

    /// Boost factor for recent memories
    ///
    /// Controls how much more recent memories are favored.
    /// Applied via decay_function over memory age. Default: 0.5
    pub recency_boost: f32,

    /// Boost factor for frequently accessed memories
    ///
    /// Memories accessed more often get higher scores.
    /// Formula: `access_count * access_boost`. Default: 0.3
    pub access_boost: f32,

    /// Boost factor for high-priority memories
    ///
    /// Priority levels (Low=0, Normal=1, High=2, Critical=3) are multiplied
    /// by this factor. Default: 0.2
    pub priority_boost: f32,

    /// Time-based decay function to apply to recency boost
    ///
    /// Determines how quickly the recency boost diminishes over time.
    /// Default: Exponential
    pub decay_function: DecayFunction,

    /// Decay rate parameter (0.0 - ∞)
    ///
    /// Meaning depends on decay_function:
    /// - Linear: hours until boost reaches 0
    /// - Exponential: decay constant (higher = faster decay)
    /// - Logarithmic: decay constant (higher = faster decay)
    /// Default: 0.1 (slow decay, favors long-term relevance)
    pub decay_rate: f32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 1.0,
            vector_weight: 1.0,
            recency_boost: 0.5,
            access_boost: 0.3,
            priority_boost: 0.2,
            decay_function: DecayFunction::Exponential,
            decay_rate: 0.1,
        }
    }
}

impl ScoringConfig {
    /// Create a new scoring configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scoring config optimized for recency (fresh information)
    ///
    /// Useful for active games or real-time systems where recent events matter most.
    pub fn recency_focused() -> Self {
        Self {
            bm25_weight: 0.5,
            vector_weight: 0.5,
            recency_boost: 2.0,
            access_boost: 0.2,
            priority_boost: 0.1,
            decay_function: DecayFunction::Exponential,
            decay_rate: 0.2,  // Faster decay
        }
    }

    /// Create a scoring config optimized for semantic matching
    ///
    /// Useful when vector embeddings are available and semantic similarity matters more
    /// than exact keyword matches.
    pub fn semantic_focused() -> Self {
        Self {
            bm25_weight: 0.3,
            vector_weight: 1.5,
            recency_boost: 0.3,
            access_boost: 0.2,
            priority_boost: 0.2,
            decay_function: DecayFunction::Exponential,
            decay_rate: 0.1,
        }
    }

    /// Create a scoring config optimized for importance (access patterns)
    ///
    /// Useful for knowledge systems where frequently accessed memories are typically important.
    pub fn importance_focused() -> Self {
        Self {
            bm25_weight: 0.7,
            vector_weight: 0.7,
            recency_boost: 0.2,
            access_boost: 1.0,  // High weight for access frequency
            priority_boost: 0.8,  // High weight for priority
            decay_function: DecayFunction::Logarithmic,  // Slow decay
            decay_rate: 0.05,
        }
    }

    /// Normalize BM25 and vector weights to sum to 1.0
    ///
    /// This ensures the primary search scores don't dominate boosts.
    /// Preserves the relative ratio between BM25 and vector weights.
    pub fn normalize_weights(&mut self) {
        let total = self.bm25_weight + self.vector_weight;
        if total > 0.0 {
            self.bm25_weight /= total;
            self.vector_weight /= total;
        }
    }

    /// Validate the configuration
    ///
    /// Returns an error if any parameters are invalid:
    /// - All weights must be >= 0.0
    /// - decay_rate must be > 0.0
    pub fn validate(&self) -> Result<(), String> {
        if self.bm25_weight < 0.0 {
            return Err("bm25_weight must be >= 0.0".to_string());
        }
        if self.vector_weight < 0.0 {
            return Err("vector_weight must be >= 0.0".to_string());
        }
        if self.recency_boost < 0.0 {
            return Err("recency_boost must be >= 0.0".to_string());
        }
        if self.access_boost < 0.0 {
            return Err("access_boost must be >= 0.0".to_string());
        }
        if self.priority_boost < 0.0 {
            return Err("priority_boost must be >= 0.0".to_string());
        }
        if self.decay_rate <= 0.0 {
            return Err("decay_rate must be > 0.0".to_string());
        }

        Ok(())
    }

    /// Check if at least one scoring factor is enabled
    pub fn has_any_boosts(&self) -> bool {
        self.recency_boost > 0.0 || self.access_boost > 0.0 || self.priority_boost > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScoringConfig::default();
        assert_eq!(config.bm25_weight, 1.0);
        assert_eq!(config.vector_weight, 1.0);
        assert_eq!(config.recency_boost, 0.5);
        assert_eq!(config.access_boost, 0.3);
        assert_eq!(config.priority_boost, 0.2);
        assert_eq!(config.decay_function, DecayFunction::Exponential);
        assert_eq!(config.decay_rate, 0.1);
    }

    #[test]
    fn test_normalize_weights() {
        let mut config = ScoringConfig {
            bm25_weight: 2.0,
            vector_weight: 2.0,
            ..Default::default()
        };
        config.normalize_weights();
        assert!((config.bm25_weight - 0.5).abs() < 0.0001);
        assert!((config.vector_weight - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_normalize_weights_with_zero() {
        let mut config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            ..Default::default()
        };
        config.normalize_weights();
        assert_eq!(config.bm25_weight, 0.0);
        assert_eq!(config.vector_weight, 0.0);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = ScoringConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_negative_bm25() {
        let config = ScoringConfig {
            bm25_weight: -1.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_negative_decay_rate() {
        let config = ScoringConfig {
            decay_rate: 0.0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_recency_focused() {
        let config = ScoringConfig::recency_focused();
        assert!(config.recency_boost > 1.0);
        assert!(config.decay_rate > 0.1);
    }

    #[test]
    fn test_semantic_focused() {
        let config = ScoringConfig::semantic_focused();
        assert!(config.vector_weight > config.bm25_weight);
    }

    #[test]
    fn test_importance_focused() {
        let config = ScoringConfig::importance_focused();
        assert!(config.access_boost > 0.5);
        assert!(config.priority_boost > 0.5);
    }

    #[test]
    fn test_has_any_boosts() {
        let config = ScoringConfig {
            recency_boost: 0.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            ..Default::default()
        };
        assert!(!config.has_any_boosts());

        let config = ScoringConfig {
            recency_boost: 0.1,
            access_boost: 0.0,
            priority_boost: 0.0,
            ..Default::default()
        };
        assert!(config.has_any_boosts());
    }

    #[test]
    fn test_decay_function_display() {
        assert_eq!(DecayFunction::None.to_string(), "none");
        assert_eq!(DecayFunction::Linear.to_string(), "linear");
        assert_eq!(DecayFunction::Exponential.to_string(), "exponential");
        assert_eq!(DecayFunction::Logarithmic.to_string(), "logarithmic");
    }
}





