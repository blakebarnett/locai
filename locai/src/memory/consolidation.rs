//! Generic Memory Consolidation System
//!
//! This module provides generic memory consolidation capabilities including
//! pattern detection, wisdom extraction, and memory connection analysis.

use crate::models::{Memory, MemoryType};
use crate::core::MemoryManager;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for memory consolidation
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    pub max_memory_age_days: i64,
    pub min_memories_for_pattern: usize,
    pub consolidation_threshold: f32,
    pub wisdom_extraction_threshold: f32,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            max_memory_age_days: 30,
            min_memories_for_pattern: 3,
            consolidation_threshold: 0.7,
            wisdom_extraction_threshold: 0.8,
        }
    }
}

/// Result of memory consolidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub patterns_found: Vec<MemoryPattern>,
    pub wisdom_extracted: Vec<WisdomInsight>,
    pub connections_formed: Vec<MemoryConnection>,
    pub consolidation_summary: String,
    pub efficiency_improvement: f32,
}

/// A pattern detected in memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub description: String,
    pub related_memory_ids: Vec<String>,
    pub confidence: f32,
    pub significance: f32,
}

/// Types of patterns that can be detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    Recurring,
    Causal,
    Temporal,
    Thematic,
    Behavioral,
    Conceptual,
}

/// Wisdom extracted from memory patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WisdomInsight {
    pub insight_id: String,
    pub wisdom_type: WisdomType,
    pub description: String,
    pub supporting_patterns: Vec<String>,
    pub confidence: f32,
    pub practical_value: f32,
}

/// Types of wisdom that can be extracted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WisdomType {
    Principle,
    Strategy,
    Lesson,
    Observation,
    Prediction,
}

/// Connection between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConnection {
    pub connection_id: String,
    pub memory_a_id: String,
    pub memory_b_id: String,
    pub connection_type: ConnectionType,
    pub strength: f32,
    pub description: String,
}

/// Types of connections between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Causal,
    Temporal,
    Thematic,
    Conceptual,
    Contradictory,
    Reinforcing,
}

/// Main memory consolidation system
pub struct MemoryConsolidator {
    pattern_detector: Arc<PatternDetector>,
    wisdom_extractor: Arc<WisdomExtractor>,
    connection_analyzer: Arc<ConnectionAnalyzer>,
}

impl MemoryConsolidator {
    pub fn new() -> Self {
        Self {
            pattern_detector: Arc::new(PatternDetector::new()),
            wisdom_extractor: Arc::new(WisdomExtractor::new()),
            connection_analyzer: Arc::new(ConnectionAnalyzer::new()),
        }
    }
    
    /// Consolidate memories using the provided configuration
    pub async fn consolidate_memories(
        &self,
        memory_manager: &Arc<MemoryManager>,
        config: &ConsolidationConfig,
    ) -> Result<ConsolidationResult> {
        // Get recent memories that might need consolidation
        let recent_memories = memory_manager
            .search_memories("", Some(1000))
            .await?;

        // Filter memories created after cutoff date
        let cutoff_date = Utc::now() - Duration::days(config.max_memory_age_days);
        let filtered_memories: Vec<_> = recent_memories.into_iter()
            .filter(|memory| memory.created_at >= cutoff_date)
            .collect();
        
        // Detect patterns
        let patterns = self.pattern_detector.detect_patterns(&filtered_memories, config).await?;
        
        // Extract wisdom from patterns
        let wisdom = self.wisdom_extractor.extract_wisdom(&patterns, &filtered_memories, config).await?;
        
        // Analyze connections
        let connections = self.connection_analyzer.analyze_connections(&filtered_memories, config).await?;
        
        // Calculate efficiency improvement
        let efficiency_improvement = self.calculate_efficiency_improvement(&patterns, &connections);
        
        Ok(ConsolidationResult {
            patterns_found: patterns,
            wisdom_extracted: wisdom,
            connections_formed: connections,
            consolidation_summary: self.generate_summary(&filtered_memories),
            efficiency_improvement,
        })
    }
    
    fn calculate_efficiency_improvement(&self, patterns: &[MemoryPattern], connections: &[MemoryConnection]) -> f32 {
        let pattern_efficiency = patterns.len() as f32 * 0.1;
        let connection_efficiency = connections.len() as f32 * 0.05;
        (pattern_efficiency + connection_efficiency).min(1.0)
    }
    
    fn generate_summary(&self, memories: &[Memory]) -> String {
        format!("Consolidated {} memories, identifying patterns and extracting insights", memories.len())
    }
}

/// Pattern detection system
pub struct PatternDetector;

impl PatternDetector {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn detect_patterns(
        &self,
        memories: &[Memory],
        config: &ConsolidationConfig,
    ) -> Result<Vec<MemoryPattern>> {
        let mut patterns = Vec::new();
        
        // Detect different types of patterns
        patterns.extend(self.detect_recurring_patterns(memories, config).await?);
        patterns.extend(self.detect_temporal_patterns(memories, config).await?);
        patterns.extend(self.detect_thematic_patterns(memories, config).await?);
        
        Ok(patterns)
    }
    
    async fn detect_recurring_patterns(
        &self,
        memories: &[Memory],
        config: &ConsolidationConfig,
    ) -> Result<Vec<MemoryPattern>> {
        let mut patterns = Vec::new();
        
        // Group memories by similar content/tags
        let mut content_groups: HashMap<String, Vec<&Memory>> = HashMap::new();
        
        for memory in memories {
            for tag in &memory.tags {
                content_groups.entry(tag.clone()).or_default().push(memory);
            }
        }
        
        // Find groups with enough memories to form patterns
        for (theme, group_memories) in content_groups {
            if group_memories.len() >= config.min_memories_for_pattern {
                patterns.push(MemoryPattern {
                    pattern_id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::Recurring,
                    description: format!("Recurring theme: {}", theme),
                    related_memory_ids: group_memories.iter().map(|m| m.id.clone()).collect(),
                    confidence: (group_memories.len() as f32 / memories.len() as f32).min(1.0),
                    significance: 0.7,
                });
            }
        }
        
        Ok(patterns)
    }
    
    async fn detect_temporal_patterns(
        &self,
        memories: &[Memory],
        _config: &ConsolidationConfig,
    ) -> Result<Vec<MemoryPattern>> {
        let mut patterns = Vec::new();
        
        // Sort memories by creation time
        let mut sorted_memories = memories.to_vec();
        sorted_memories.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        
        // Look for temporal sequences
        // This is a simplified implementation - real system would be more sophisticated
        if sorted_memories.len() >= 3 {
            patterns.push(MemoryPattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::Temporal,
                description: "Sequential memory formation pattern".to_string(),
                related_memory_ids: sorted_memories.iter().map(|m| m.id.clone()).collect(),
                confidence: 0.6,
                significance: 0.5,
            });
        }
        
        Ok(patterns)
    }
    
    async fn detect_thematic_patterns(
        &self,
        memories: &[Memory],
        config: &ConsolidationConfig,
    ) -> Result<Vec<MemoryPattern>> {
        let mut patterns = Vec::new();
        
        // Group by memory type
        let mut type_groups: HashMap<MemoryType, Vec<&Memory>> = HashMap::new();
        
        for memory in memories {
            type_groups.entry(memory.memory_type.clone()).or_default().push(memory);
        }
        
        for (memory_type, group_memories) in type_groups {
            if group_memories.len() >= config.min_memories_for_pattern {
                patterns.push(MemoryPattern {
                    pattern_id: uuid::Uuid::new_v4().to_string(),
                    pattern_type: PatternType::Thematic,
                    description: format!("Thematic pattern: {:?} memories", memory_type),
                    related_memory_ids: group_memories.iter().map(|m| m.id.clone()).collect(),
                    confidence: 0.8,
                    significance: 0.6,
                });
            }
        }
        
        Ok(patterns)
    }
}

/// Wisdom extraction system
pub struct WisdomExtractor;

impl WisdomExtractor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn extract_wisdom(
        &self,
        patterns: &[MemoryPattern],
        memories: &[Memory],
        config: &ConsolidationConfig,
    ) -> Result<Vec<WisdomInsight>> {
        let mut wisdom = Vec::new();
        
        for pattern in patterns {
            if pattern.confidence >= config.wisdom_extraction_threshold {
                wisdom.extend(self.extract_pattern_wisdom(pattern, memories).await?);
            }
        }
        
        Ok(wisdom)
    }
    
    async fn extract_pattern_wisdom(
        &self,
        pattern: &MemoryPattern,
        _memories: &[Memory],
    ) -> Result<Vec<WisdomInsight>> {
        let mut wisdom = Vec::new();
        
        match pattern.pattern_type {
            PatternType::Recurring => {
                wisdom.push(WisdomInsight {
                    insight_id: uuid::Uuid::new_v4().to_string(),
                    wisdom_type: WisdomType::Observation,
                    description: format!("Recurring pattern identified: {}", pattern.description),
                    supporting_patterns: vec![pattern.pattern_id.clone()],
                    confidence: pattern.confidence,
                    practical_value: 0.7,
                });
            }
            PatternType::Temporal => {
                wisdom.push(WisdomInsight {
                    insight_id: uuid::Uuid::new_v4().to_string(),
                    wisdom_type: WisdomType::Principle,
                    description: "Sequential learning and development occurs over time".to_string(),
                    supporting_patterns: vec![pattern.pattern_id.clone()],
                    confidence: pattern.confidence,
                    practical_value: 0.6,
                });
            }
            PatternType::Thematic => {
                wisdom.push(WisdomInsight {
                    insight_id: uuid::Uuid::new_v4().to_string(),
                    wisdom_type: WisdomType::Strategy,
                    description: format!("Focus area identified: {}", pattern.description),
                    supporting_patterns: vec![pattern.pattern_id.clone()],
                    confidence: pattern.confidence,
                    practical_value: 0.8,
                });
            }
            _ => {}
        }
        
        Ok(wisdom)
    }
}

/// Connection analysis system
pub struct ConnectionAnalyzer;

impl ConnectionAnalyzer {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn analyze_connections(
        &self,
        memories: &[Memory],
        _config: &ConsolidationConfig,
    ) -> Result<Vec<MemoryConnection>> {
        let mut connections = Vec::new();
        
        // Analyze connections between memories
        for (i, memory_a) in memories.iter().enumerate() {
            for memory_b in memories.iter().skip(i + 1) {
                if let Some(connection) = self.analyze_memory_pair(memory_a, memory_b).await? {
                    connections.push(connection);
                }
            }
        }
        
        Ok(connections)
    }
    
    async fn analyze_memory_pair(
        &self,
        memory_a: &Memory,
        memory_b: &Memory,
    ) -> Result<Option<MemoryConnection>> {
        // Check for shared tags (thematic connection)
        let shared_tags: Vec<_> = memory_a
            .tags
            .iter()
            .filter(|tag| memory_b.tags.contains(tag))
            .collect();
        
        if !shared_tags.is_empty() {
            return Ok(Some(MemoryConnection {
                connection_id: uuid::Uuid::new_v4().to_string(),
                memory_a_id: memory_a.id.clone(),
                memory_b_id: memory_b.id.clone(),
                connection_type: ConnectionType::Thematic,
                strength: shared_tags.len() as f32 / memory_a.tags.len().max(memory_b.tags.len()) as f32,
                description: format!("Shared themes: {}", shared_tags.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
            }));
        }
        
        // Check for temporal connection (memories created close in time)
        let time_diff = (memory_a.created_at - memory_b.created_at).abs();
        if time_diff < Duration::hours(1) {
            return Ok(Some(MemoryConnection {
                connection_id: uuid::Uuid::new_v4().to_string(),
                memory_a_id: memory_a.id.clone(),
                memory_b_id: memory_b.id.clone(),
                connection_type: ConnectionType::Temporal,
                strength: 0.7,
                description: "Created in close temporal proximity".to_string(),
            }));
        }
        
        Ok(None)
    }
} 