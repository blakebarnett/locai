//! Search result score calculator
//!
//! This module implements the calculation of final relevance scores by combining
//! BM25 scores, vector similarity scores, and memory lifecycle metadata.

use crate::models::memory::Memory;
use chrono::Utc;

use super::scoring::{DecayFunction, ScoringConfig};

/// Calculator for combining multiple scoring factors into a final relevance score
///
/// This struct takes BM25 scores, vector similarity scores, and memory metadata
/// and combines them according to a ScoringConfig to produce a final relevance rank.
pub struct ScoreCalculator {
    config: ScoringConfig,
}

impl ScoreCalculator {
    /// Create a new score calculator with the given configuration
    ///
    /// # Panics
    ///
    /// Panics if the configuration is invalid.
    pub fn new(config: ScoringConfig) -> Self {
        if let Err(e) = config.validate() {
            panic!("Invalid scoring config: {}", e);
        }
        Self { config }
    }

    /// Create a score calculator with the given configuration, returning an error if invalid
    pub fn try_new(config: ScoringConfig) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Calculate the final relevance score for a memory
    ///
    /// # Arguments
    ///
    /// * `bm25_score` - BM25 keyword relevance score (typically 0.0+)
    /// * `vector_score` - Vector similarity score (typically -1.0 to 1.0), optional
    /// * `memory` - The memory being scored, for accessing lifecycle metadata
    ///
    /// # Returns
    ///
    /// The combined final score, ready for sorting (higher = more relevant)
    pub fn calculate_final_score(
        &self,
        bm25_score: f32,
        vector_score: Option<f32>,
        memory: &Memory,
    ) -> f32 {
        let mut score = bm25_score * self.config.bm25_weight;

        // Apply vector score if present
        if let Some(vec_score) = vector_score {
            score += vec_score * self.config.vector_weight;
        }

        // Apply boosts
        score += self.calculate_recency_boost(memory);
        score += self.calculate_access_boost(memory);
        score += self.calculate_priority_boost(memory);

        score
    }

    /// Calculate recency boost based on memory age and decay function
    ///
    /// The boost decreases over time according to the configured decay function.
    /// This encourages recent memories to rank higher.
    fn calculate_recency_boost(&self, memory: &Memory) -> f32 {
        // Calculate age in hours
        let age_duration = Utc::now()
            .signed_duration_since(memory.created_at);
        let age_hours = age_duration.num_hours() as f32;

        match self.config.decay_function {
            DecayFunction::None => 0.0,

            DecayFunction::Linear => {
                // Linear: boost * max(0, 1 - age_hours * decay_rate)
                let decay = age_hours * self.config.decay_rate;
                self.config.recency_boost * (1.0 - decay).max(0.0)
            }

            DecayFunction::Exponential => {
                // Exponential: boost * exp(-decay_rate * age_hours)
                // This models human memory and the Ebbinghaus forgetting curve
                let exponent = -self.config.decay_rate * age_hours;
                self.config.recency_boost * exponent.exp()
            }

            DecayFunction::Logarithmic => {
                // Logarithmic: boost / (1 + ln(1 + age_hours * decay_rate))
                // This decays slower than exponential
                let denominator = 1.0 + (1.0 + age_hours * self.config.decay_rate).ln();
                self.config.recency_boost / denominator
            }
        }
    }

    /// Calculate access count boost
    ///
    /// Frequently accessed memories are boosted, assuming popularity indicates relevance.
    fn calculate_access_boost(&self, memory: &Memory) -> f32 {
        // Use logarithmic scaling to prevent very old memories from getting huge boosts
        // log(1 + access_count) gives diminishing returns
        let access_factor = (1.0 + memory.access_count as f32).ln();
        access_factor * self.config.access_boost
    }

    /// Calculate priority boost
    ///
    /// Higher priority memories receive a boost according to their importance level.
    fn calculate_priority_boost(&self, memory: &Memory) -> f32 {
        // Convert priority enum to numeric value (Low=0, Normal=1, High=2, Critical=3)
        let priority_value = memory.priority as i32 as f32;
        priority_value * self.config.priority_boost
    }

    /// Get reference to the configuration
    pub fn config(&self) -> &ScoringConfig {
        &self.config
    }

    /// Get mutable reference to the configuration (for testing)
    #[cfg(test)]
    pub fn config_mut(&mut self) -> &mut ScoringConfig {
        &mut self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::memory::MemoryPriority;

    fn create_test_memory(
        id: &str,
        created_at: chrono::DateTime<Utc>,
        access_count: u32,
        priority: MemoryPriority,
    ) -> Memory {
        Memory {
            id: id.to_string(),
            content: "test content".to_string(),
            memory_type: crate::models::memory::MemoryType::Fact,
            created_at,
            last_accessed: None,
            access_count,
            priority,
            tags: vec![],
            source: "test".to_string(),
            expires_at: None,
            properties: serde_json::json!({}),
            related_memories: vec![],
            embedding: None,
        }
    }

    #[test]
    fn test_calculator_creation() {
        let config = ScoringConfig::default();
        let calc = ScoreCalculator::new(config);
        assert!(calc.config().validate().is_ok());
    }

    #[test]
    #[should_panic]
    fn test_calculator_creation_invalid_config() {
        let mut config = ScoringConfig::default();
        config.decay_rate = 0.0;  // Invalid
        let _calc = ScoreCalculator::new(config);
    }

    #[test]
    fn test_try_new_invalid_config() {
        let mut config = ScoringConfig::default();
        config.decay_rate = 0.0;  // Invalid
        assert!(ScoreCalculator::try_new(config).is_err());
    }

    #[test]
    fn test_final_score_bm25_only() {
        let config = ScoringConfig {
            bm25_weight: 1.0,
            vector_weight: 0.0,
            recency_boost: 0.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        let memory = create_test_memory("test", Utc::now(), 0, MemoryPriority::Normal);
        let score = calc.calculate_final_score(10.0, None, &memory);

        assert_eq!(score, 10.0);
    }

    #[test]
    fn test_final_score_with_vector() {
        let config = ScoringConfig {
            bm25_weight: 0.5,
            vector_weight: 0.5,
            recency_boost: 0.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        let memory = create_test_memory("test", Utc::now(), 0, MemoryPriority::Normal);
        let score = calc.calculate_final_score(10.0, Some(20.0), &memory);

        assert_eq!(score, 10.0 * 0.5 + 20.0 * 0.5);
    }

    #[test]
    fn test_recency_boost_no_decay() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 0.5,
            access_boost: 0.0,
            priority_boost: 0.0,
            decay_function: DecayFunction::None,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        let memory = create_test_memory("test", Utc::now(), 0, MemoryPriority::Normal);
        let score = calc.calculate_final_score(0.0, None, &memory);

        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_recency_boost_linear_decay() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 10.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            decay_function: DecayFunction::Linear,
            decay_rate: 0.1,  // 10% per hour
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        // Fresh memory (created now)
        let fresh = create_test_memory("fresh", Utc::now(), 0, MemoryPriority::Normal);
        let score_fresh = calc.calculate_final_score(0.0, None, &fresh);
        assert!(score_fresh > 9.0);  // Close to max

        // Old memory (24 hours old)
        let old = create_test_memory("old", Utc::now() - chrono::Duration::hours(24), 0, MemoryPriority::Normal);
        let score_old = calc.calculate_final_score(0.0, None, &old);
        // At 0.1 decay rate, 24 hours = 2.4 decay, so max is 0
        assert!(score_old <= 0.0);
    }

    #[test]
    fn test_recency_boost_exponential_decay() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 10.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            decay_function: DecayFunction::Exponential,
            decay_rate: 0.1,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        // Fresh memory
        let fresh = create_test_memory("fresh", Utc::now(), 0, MemoryPriority::Normal);
        let score_fresh = calc.calculate_final_score(0.0, None, &fresh);
        assert!(score_fresh > 9.0);

        // 10 hours old
        let old = create_test_memory("old", Utc::now() - chrono::Duration::hours(10), 0, MemoryPriority::Normal);
        let score_old = calc.calculate_final_score(0.0, None, &old);
        // exp(-0.1 * 10) = exp(-1) ≈ 0.368
        assert!(score_old > 3.0 && score_old < 4.0);
    }

    #[test]
    fn test_recency_boost_logarithmic_decay() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 10.0,
            access_boost: 0.0,
            priority_boost: 0.0,
            decay_function: DecayFunction::Logarithmic,
            decay_rate: 0.1,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        // Fresh memory
        let fresh = create_test_memory("fresh", Utc::now(), 0, MemoryPriority::Normal);
        let score_fresh = calc.calculate_final_score(0.0, None, &fresh);
        assert!(score_fresh > 5.0);

        // 100 hours old
        let old = create_test_memory("old", Utc::now() - chrono::Duration::hours(100), 0, MemoryPriority::Normal);
        let score_old = calc.calculate_final_score(0.0, None, &old);
        // Should still have meaningful score with logarithmic decay (slower)
        assert!(score_old > 0.0);
    }

    #[test]
    fn test_access_boost() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 0.0,
            access_boost: 0.1,
            priority_boost: 0.0,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        // No accesses
        let memory_0 = create_test_memory("m0", Utc::now(), 0, MemoryPriority::Normal);
        let score_0 = calc.calculate_final_score(0.0, None, &memory_0);

        // 10 accesses
        let memory_10 = create_test_memory("m10", Utc::now(), 10, MemoryPriority::Normal);
        let score_10 = calc.calculate_final_score(0.0, None, &memory_10);

        // More accessed memory should score higher
        assert!(score_10 > score_0);
        // log(11) ≈ 2.398, so score should be ~0.2398
        assert!(score_10 > 0.2 && score_10 < 0.3);
    }

    #[test]
    fn test_priority_boost() {
        let config = ScoringConfig {
            bm25_weight: 0.0,
            vector_weight: 0.0,
            recency_boost: 0.0,
            access_boost: 0.0,
            priority_boost: 1.0,
            ..Default::default()
        };
        let calc = ScoreCalculator::new(config);

        let low = create_test_memory("low", Utc::now(), 0, MemoryPriority::Low);
        let normal = create_test_memory("normal", Utc::now(), 0, MemoryPriority::Normal);
        let high = create_test_memory("high", Utc::now(), 0, MemoryPriority::High);
        let critical = create_test_memory("critical", Utc::now(), 0, MemoryPriority::Critical);

        let score_low = calc.calculate_final_score(0.0, None, &low);
        let score_normal = calc.calculate_final_score(0.0, None, &normal);
        let score_high = calc.calculate_final_score(0.0, None, &high);
        let score_critical = calc.calculate_final_score(0.0, None, &critical);

        assert_eq!(score_low, 0.0);
        assert_eq!(score_normal, 1.0);
        assert_eq!(score_high, 2.0);
        assert_eq!(score_critical, 3.0);
    }

    #[test]
    fn test_combined_scoring() {
        let config = ScoringConfig {
            bm25_weight: 0.4,
            vector_weight: 0.6,
            recency_boost: 0.5,
            access_boost: 0.1,
            priority_boost: 0.2,
            decay_function: DecayFunction::Exponential,
            decay_rate: 0.1,
        };
        let calc = ScoreCalculator::new(config);

        let memory = Memory {
            id: "test".to_string(),
            content: "test".to_string(),
            memory_type: crate::models::memory::MemoryType::Fact,
            created_at: Utc::now() - chrono::Duration::hours(5),
            last_accessed: None,
            access_count: 5,
            priority: MemoryPriority::High,
            tags: vec![],
            source: "test".to_string(),
            expires_at: None,
            properties: serde_json::json!({}),
            related_memories: vec![],
            embedding: None,
        };

        let score = calc.calculate_final_score(10.0, Some(5.0), &memory);
        // Should combine all factors meaningfully
        assert!(score > 5.0);  // Base scores are 4.0 + 3.0 = 7.0
    }
}
