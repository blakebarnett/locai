//! Memory Versioning System
//!
//! Tracks memory evolution over time with sophisticated version management,
//! personality evolution tracing, and campaign snapshot capabilities.

use crate::models::{Memory, MemoryType};
use crate::core::MemoryManager;
use super::TimeRange;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Main versioning manager
pub struct MemoryVersionManager {
    memory_manager: Arc<MemoryManager>,
    #[allow(dead_code)]
    versioning_policy: VersioningPolicy,
    retention_policy: RetentionPolicy,
}

impl MemoryVersionManager {
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self {
            memory_manager,
            versioning_policy: VersioningPolicy::default(),
            retention_policy: RetentionPolicy::default(),
        }
    }
    
    /// Track memory evolution over time
    pub async fn create_memory_version(
        &self,
        memory_id: &str,
        new_content: &str,
        version_type: VersionType,
        change_reason: &str,
    ) -> Result<MemoryVersion> {
        let version = MemoryVersion {
            version_id: Uuid::new_v4().to_string(),
            memory_id: memory_id.to_string(),
            content: new_content.to_string(),
            version_type,
            change_reason: change_reason.to_string(),
            created_at: Utc::now(),
            metadata: self.collect_version_metadata(memory_id).await?,
        };
        
        // Store version as a special memory type
        self.memory_manager.add_memory_with_options(
            &format!("Version: {}", version.content),
            |builder| {
                builder
                    .memory_type(MemoryType::Identity)
                    .source("memory_versioning")
                    .tags(vec![
                        "version",
                        &format!("original:{}", memory_id),
                        &format!("version_type:{:?}", version.version_type),
                    ])
            }
        ).await?;
        
        // Apply retention policy
        self.apply_retention_policy(memory_id).await?;
        
        Ok(version)
    }
    
    /// Get memory evolution history
    pub async fn get_memory_evolution(
        &self,
        memory_id: &str,
    ) -> Result<MemoryEvolution> {
        let versions = self.get_memory_versions(memory_id).await?;
        
        let evolution = MemoryEvolution {
            memory_id: memory_id.to_string(),
            original_version: versions.first().cloned(),
            current_version: versions.last().cloned(),
            version_history: versions.clone(),
            evolution_summary: self.analyze_evolution(&versions).await?,
            key_changes: self.identify_key_changes(&versions).await?,
        };
        
        Ok(evolution)
    }
    
    /// Track personality evolution through memory versions
    pub async fn track_personality_evolution(
        &self,
        character_name: &str,
        time_range: Option<TimeRange>,
    ) -> Result<PersonalityEvolutionTrace> {
        // Find all personality-related memories
        let personality_memories = self.find_memories_by_tag("personality", time_range).await?;
        
        let mut evolution_points = Vec::new();
        
        for memory in personality_memories {
            let evolution = self.get_memory_evolution(&memory.id).await?;
            
            if evolution.has_significant_changes() {
                evolution_points.push(PersonalityEvolutionPoint {
                    timestamp: memory.created_at,
                    change_description: evolution.evolution_summary.clone(),
                    trigger_events: evolution.get_trigger_events(),
                    personality_impact: self.calculate_personality_impact(&evolution).await?,
                });
            }
        }
        
        Ok(PersonalityEvolutionTrace {
            character_name: character_name.to_string(),
            evolution_points: evolution_points.clone(),
            overall_trajectory: self.calculate_personality_trajectory(&evolution_points)?,
        })
    }
    
    /// Create memory snapshots for major campaign events
    pub async fn create_campaign_snapshot(
        &self,
        event_name: &str,
        affected_memories: &[String],
    ) -> Result<CampaignSnapshot> {
        let snapshot_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        
        let mut memory_snapshots = Vec::new();
        
        for memory_id in affected_memories {
            let snapshot = self.create_snapshot(memory_id, &snapshot_id).await?;
            memory_snapshots.push(snapshot);
        }
        
        let campaign_snapshot = CampaignSnapshot {
            snapshot_id,
            event_name: event_name.to_string(),
            created_at: timestamp,
            memory_snapshots,
            event_impact: self.analyze_event_impact(affected_memories).await?,
        };
        
        Ok(campaign_snapshot)
    }
    
    /// Get all versions of a memory
    async fn get_memory_versions(&self, memory_id: &str) -> Result<Vec<MemoryVersion>> {
        let version_memories = self.memory_manager.search_memories(
            &format!("original:{}", memory_id),
            None
        ).await?;
        
        let mut versions = Vec::new();
        for memory in version_memories {
            if let Some(version) = self.parse_memory_version(&memory)? {
                versions.push(version);
            }
        }
        
        versions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(versions)
    }
    
    /// Collect metadata for version
    async fn collect_version_metadata(&self, memory_id: &str) -> Result<VersionMetadata> {
        // Get original memory
        let memories = self.memory_manager.search_memories(memory_id, None).await?;
        let memory = memories.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("Memory not found: {}", memory_id))?;
        
        let memory_type = memory.memory_type.clone();
        Ok(VersionMetadata {
            original_created_at: memory.created_at,
            tags: memory.tags.clone(),
            memory_type,
            context: self.extract_context(&memory).await?,
        })
    }
    
    /// Apply retention policy to limit version storage
    async fn apply_retention_policy(&self, memory_id: &str) -> Result<()> {
        let versions = self.get_memory_versions(memory_id).await?;
        
        if versions.len() > self.retention_policy.max_versions {
            let excess_count = versions.len() - self.retention_policy.max_versions;
            for version in &versions[..excess_count] {
                // Delete old versions (would need actual storage integration)
                tracing::debug!("Would delete version: {}", version.version_id);
            }
        }
        
        Ok(())
    }
    
    /// Analyze evolution patterns
    async fn analyze_evolution(&self, versions: &[MemoryVersion]) -> Result<String> {
        if versions.len() < 2 {
            return Ok("No significant evolution detected".to_string());
        }
        
        let mut changes = Vec::new();
        for window in versions.windows(2) {
            if let [earlier, later] = window {
                let change = self.compare_versions(earlier, later);
                changes.push(change);
            }
        }
        
        let summary = if changes.iter().any(|c| c.contains("personality")) {
            "Significant personality evolution detected"
        } else if changes.iter().any(|c| c.contains("skill")) {
            "Skill development evolution observed"
        } else {
            "Gradual memory refinement over time"
        };
        
        Ok(summary.to_string())
    }
    
    /// Identify key changes in evolution
    async fn identify_key_changes(&self, versions: &[MemoryVersion]) -> Result<Vec<MemoryChange>> {
        let mut changes = Vec::new();
        
        for window in versions.windows(2) {
            if let [earlier, later] = window {
                let change_magnitude = self.calculate_change_magnitude(earlier, later);
                if change_magnitude > 0.3 {
                    changes.push(MemoryChange {
                        change_id: Uuid::new_v4().to_string(),
                        from_version: earlier.version_id.clone(),
                        to_version: later.version_id.clone(),
                        change_type: self.classify_change_type(earlier, later),
                        magnitude: change_magnitude,
                        description: self.describe_change(earlier, later),
                        timestamp: later.created_at,
                    });
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Find memories by tag with optional time range
    async fn find_memories_by_tag(&self, tag: &str, time_range: Option<TimeRange>) -> Result<Vec<Memory>> {
        let mut memories = self.memory_manager.find_memories_by_tag(tag, None).await?;
        
        if let Some(range) = time_range {
            memories.retain(|m| m.created_at >= range.start && m.created_at <= range.end);
        }
        
        Ok(memories)
    }
    
    /// Create snapshot for memory
    async fn create_snapshot(&self, memory_id: &str, snapshot_id: &str) -> Result<MemorySnapshot> {
        let memories = self.memory_manager.search_memories(memory_id, None).await?;
        let memory = memories.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("Memory not found: {}", memory_id))?;
        
        Ok(MemorySnapshot {
            memory_id: memory.id.clone(),
            snapshot_id: snapshot_id.to_string(),
            content: memory.content.clone(),
            metadata: memory.tags.clone(),
            captured_at: Utc::now(),
        })
    }
    
    /// Analyze event impact
    async fn analyze_event_impact(&self, affected_memories: &[String]) -> Result<EventImpact> {
        let impact_score = affected_memories.len() as f32 * 0.1;
        
        Ok(EventImpact {
            affected_memory_count: affected_memories.len(),
            impact_score: impact_score.clamp(0.0, 1.0),
            impact_areas: vec!["character_development".to_string()],
        })
    }
    
    /// Parse memory version from stored memory
    fn parse_memory_version(&self, memory: &Memory) -> Result<Option<MemoryVersion>> {
        if !memory.tags.contains(&"version".to_string()) {
            return Ok(None);
        }
        
        // Extract version info from tags
        let version_type = memory.tags.iter()
            .find(|tag| tag.starts_with("version_type:"))
            .and_then(|tag| tag.strip_prefix("version_type:"))
            .unwrap_or("GradualEvolution");
        
        let memory_id = memory.tags.iter()
            .find(|tag| tag.starts_with("original:"))
            .and_then(|tag| tag.strip_prefix("original:"))
            .unwrap_or("unknown");
        
        Ok(Some(MemoryVersion {
            version_id: memory.id.clone(),
            memory_id: memory_id.to_string(),
            content: memory.content.clone(),
            version_type: self.parse_version_type(version_type),
            change_reason: "Auto-generated".to_string(),
            created_at: memory.created_at,
            metadata: VersionMetadata::default(),
        }))
    }
    
    /// Extract context from memory
    async fn extract_context(&self, memory: &Memory) -> Result<ContextInfo> {
        Ok(ContextInfo {
            related_characters: memory.tags.iter()
                .filter(|tag| tag.starts_with("char_"))
                .map(|tag| tag.strip_prefix("char_").unwrap_or("").to_string())
                .collect(),
            situation: "unknown".to_string(),
            emotional_state: "neutral".to_string(),
        })
    }
    
    /// Compare two versions
    fn compare_versions(&self, earlier: &MemoryVersion, later: &MemoryVersion) -> String {
        let content_diff = self.calculate_content_similarity(&earlier.content, &later.content);
        
        if content_diff < 0.5 {
            "Major content change".to_string()
        } else if content_diff < 0.8 {
            "Moderate refinement".to_string()
        } else {
            "Minor adjustment".to_string()
        }
    }
    
    /// Calculate change magnitude
    fn calculate_change_magnitude(&self, earlier: &MemoryVersion, later: &MemoryVersion) -> f32 {
        let content_similarity = self.calculate_content_similarity(&earlier.content, &later.content);
        1.0 - content_similarity
    }
    
    /// Calculate content similarity
    fn calculate_content_similarity(&self, content1: &str, content2: &str) -> f32 {
        let words1: std::collections::HashSet<_> = content1.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = content2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 { 1.0 } else { intersection as f32 / union as f32 }
    }
    
    /// Classify change type
    fn classify_change_type(&self, earlier: &MemoryVersion, later: &MemoryVersion) -> ChangeType {
        match (&earlier.version_type, &later.version_type) {
            (VersionType::PersonalityEvolution, _) => ChangeType::PersonalityShift,
            (_, VersionType::PersonalityEvolution) => ChangeType::PersonalityShift,
            (VersionType::SkillDevelopment, _) => ChangeType::SkillGrowth,
            (_, VersionType::SkillDevelopment) => ChangeType::SkillGrowth,
            _ => ChangeType::ContentUpdate,
        }
    }
    
    /// Describe change
    fn describe_change(&self, earlier: &MemoryVersion, later: &MemoryVersion) -> String {
        format!("Memory evolved from '{}' to '{}'", 
            earlier.content.chars().take(50).collect::<String>(),
            later.content.chars().take(50).collect::<String>())
    }
    
    /// Parse version type from string
    fn parse_version_type(&self, type_str: &str) -> VersionType {
        match type_str {
            "PersonalityEvolution" => VersionType::PersonalityEvolution,
            "MemoryConsolidation" => VersionType::MemoryConsolidation,
            "RelationshipChange" => VersionType::RelationshipChange,
            "ExperienceIntegration" => VersionType::ExperienceIntegration,
            "WisdomGain" => VersionType::WisdomGain,
            "TraumaProcessing" => VersionType::TraumaProcessing,
            "SkillDevelopment" => VersionType::SkillDevelopment,
            _ => VersionType::MemoryConsolidation,
        }
    }
    
    /// Calculate personality impact
    async fn calculate_personality_impact(&self, evolution: &MemoryEvolution) -> Result<f32> {
        let change_count = evolution.key_changes.len() as f32;
        let avg_magnitude = if evolution.key_changes.is_empty() {
            0.0
        } else {
            evolution.key_changes.iter().map(|c| c.magnitude).sum::<f32>() / change_count
        };
        
        Ok((change_count * 0.1 + avg_magnitude * 0.5).clamp(0.0, 1.0))
    }
    
    /// Calculate personality trajectory
    fn calculate_personality_trajectory(&self, evolution_points: &[PersonalityEvolutionPoint]) -> Result<PersonalityTrajectory> {
        if evolution_points.is_empty() {
            return Ok(PersonalityTrajectory {
                direction: TrajectoryDirection::Stable,
                velocity: 0.0,
                confidence: 0.0,
                predicted_next_changes: Vec::new(),
            });
        }
        
        let recent_changes = evolution_points.len() as f32;
        let avg_impact = evolution_points.iter()
            .map(|p| p.personality_impact)
            .sum::<f32>() / evolution_points.len() as f32;
        
        let direction = if avg_impact > 0.6 {
            TrajectoryDirection::Ascending
        } else if avg_impact < 0.3 {
            TrajectoryDirection::Declining
        } else {
            TrajectoryDirection::Stable
        };
        
        Ok(PersonalityTrajectory {
            direction,
            velocity: recent_changes * 0.1,
            confidence: avg_impact,
            predicted_next_changes: vec!["Continued gradual evolution".to_string()],
        })
    }
}

/// Versioning policy configuration
#[derive(Debug, Clone)]
pub struct VersioningPolicy {
    pub auto_version_threshold: f32,
    pub version_significant_changes: bool,
    pub track_personality_evolution: bool,
}

impl Default for VersioningPolicy {
    fn default() -> Self {
        Self {
            auto_version_threshold: 0.3,
            version_significant_changes: true,
            track_personality_evolution: true,
        }
    }
}

/// Retention policy for version storage
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub max_versions: usize,
    pub retention_days: i64,
    pub keep_significant_versions: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_versions: 50,
            retention_days: 365,
            keep_significant_versions: true,
        }
    }
}

/// Types of memory versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionType {
    PersonalityEvolution,
    MemoryConsolidation,
    RelationshipChange,
    ExperienceIntegration,
    WisdomGain,
    TraumaProcessing,
    SkillDevelopment,
}

/// A specific version of a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVersion {
    pub version_id: String,
    pub memory_id: String,
    pub content: String,
    pub version_type: VersionType,
    pub change_reason: String,
    pub created_at: DateTime<Utc>,
    pub metadata: VersionMetadata,
}

/// Metadata associated with a version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMetadata {
    pub original_created_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub memory_type: MemoryType,
    pub context: ContextInfo,
}

impl Default for VersionMetadata {
    fn default() -> Self {
        Self {
            original_created_at: Utc::now(),
            tags: Vec::new(),
            memory_type: MemoryType::Episodic,
            context: ContextInfo::default(),
        }
    }
}

/// Context information for version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInfo {
    pub related_characters: Vec<String>,
    pub situation: String,
    pub emotional_state: String,
}

impl Default for ContextInfo {
    fn default() -> Self {
        Self {
            related_characters: Vec::new(),
            situation: "unknown".to_string(),
            emotional_state: "neutral".to_string(),
        }
    }
}

/// Memory evolution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvolution {
    pub memory_id: String,
    pub original_version: Option<MemoryVersion>,
    pub current_version: Option<MemoryVersion>,
    pub version_history: Vec<MemoryVersion>,
    pub evolution_summary: String,
    pub key_changes: Vec<MemoryChange>,
}

impl MemoryEvolution {
    pub fn has_significant_changes(&self) -> bool {
        self.key_changes.iter().any(|c| c.magnitude > 0.5)
    }
    
    pub fn get_trigger_events(&self) -> Vec<String> {
        self.key_changes.iter()
            .map(|c| c.description.clone())
            .collect()
    }
}

/// A significant change in memory evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChange {
    pub change_id: String,
    pub from_version: String,
    pub to_version: String,
    pub change_type: ChangeType,
    pub magnitude: f32,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Types of memory changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    PersonalityShift,
    SkillGrowth,
    ContentUpdate,
    EmotionalEvolution,
    RelationshipChange,
    WisdomGain,
}

/// Personality evolution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityEvolutionTrace {
    pub character_name: String,
    pub evolution_points: Vec<PersonalityEvolutionPoint>,
    pub overall_trajectory: PersonalityTrajectory,
}

/// Point in personality evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityEvolutionPoint {
    pub timestamp: DateTime<Utc>,
    pub change_description: String,
    pub trigger_events: Vec<String>,
    pub personality_impact: f32,
}

/// Personality trajectory analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTrajectory {
    pub direction: TrajectoryDirection,
    pub velocity: f32,
    pub confidence: f32,
    pub predicted_next_changes: Vec<String>,
}

/// Direction of personality trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrajectoryDirection {
    Ascending,
    Declining,
    Stable,
    Oscillating,
}

/// Campaign snapshot for major events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSnapshot {
    pub snapshot_id: String,
    pub event_name: String,
    pub created_at: DateTime<Utc>,
    pub memory_snapshots: Vec<MemorySnapshot>,
    pub event_impact: EventImpact,
}

/// Snapshot of a single memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub memory_id: String,
    pub snapshot_id: String,
    pub content: String,
    pub metadata: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

/// Impact analysis of an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventImpact {
    pub affected_memory_count: usize,
    pub impact_score: f32,
    pub impact_areas: Vec<String>,
} 