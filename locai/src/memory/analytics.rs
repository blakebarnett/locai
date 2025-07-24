//! Generic Memory Analytics System
//!
//! This module provides analytics and reporting capabilities for memory usage,
//! efficiency metrics, and anomaly detection.

use crate::models::{Memory, MemoryType};
use crate::core::MemoryManager;
use super::TimeRange;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use uuid;

/// Main memory analytics engine
pub struct MemoryAnalyticsEngine {
    memory_manager: Arc<MemoryManager>,
}

impl MemoryAnalyticsEngine {
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }
    
    /// Generate comprehensive analytics report
    pub async fn generate_report(&self, time_range: &TimeRange) -> Result<MemoryAnalyticsReport> {
        let usage_report = self.calculate_usage_metrics(time_range).await?;
        let efficiency_metrics = self.calculate_efficiency_metrics(time_range).await?;
        let anomalies = self.detect_anomalies(time_range).await?;
        let growth_trends = self.analyze_growth_trends(time_range).await?;
        
        Ok(MemoryAnalyticsReport {
            time_range: time_range.clone(),
            usage_report,
            efficiency_metrics,
            anomalies,
            growth_trends,
        })
    }
    
    /// Calculate memory usage metrics
    async fn calculate_usage_metrics(&self, time_range: &TimeRange) -> Result<Usage> {
        // Get memories in time range
        let memories = self.memory_manager
            .search_memories("", Some(10000))
            .await?;

        // Filter memories by time range
        let filtered_memories: Vec<_> = memories.into_iter()
            .filter(|memory| {
                memory.created_at >= time_range.start && memory.created_at <= time_range.end
            })
            .collect();
        
        let total_memories = filtered_memories.len();
        let memory_types = self.analyze_memory_types(&filtered_memories);
        let growth_trends = self.analyze_growth_trends_sync(&filtered_memories, time_range);
        let efficiency_metrics = self.calculate_efficiency_metrics_sync(&filtered_memories);
        let anomalies = self.detect_anomalies_sync(&filtered_memories).await?;
        
        Ok(Usage {
            total_memories,
            memory_types_breakdown: memory_types,
            growth_trends,
            efficiency_metrics,
            anomalies,
            recommendations: self.generate_recommendations(&filtered_memories),
        })
    }
    
    /// Calculate efficiency metrics
    async fn calculate_efficiency_metrics(&self, time_range: &TimeRange) -> Result<MemoryEfficiencyMetrics> {
        let memories = self.memory_manager
            .search_memories("", Some(10000))
            .await?;

        let filtered_memories: Vec<_> = memories.into_iter()
            .filter(|memory| {
                memory.created_at >= time_range.start && memory.created_at <= time_range.end
            })
            .collect();

        Ok(self.calculate_efficiency_metrics_sync(&filtered_memories))
    }
    
    /// Detect anomalies in memory usage
    async fn detect_anomalies(&self, time_range: &TimeRange) -> Result<Vec<MemoryAnomaly>> {
        let memories = self.memory_manager
            .search_memories("", Some(10000))
            .await?;

        let filtered_memories: Vec<_> = memories.into_iter()
            .filter(|memory| {
                memory.created_at >= time_range.start && memory.created_at <= time_range.end
            })
            .collect();

        self.detect_anomalies_sync(&filtered_memories).await
    }
    
    /// Analyze growth trends
    async fn analyze_growth_trends(&self, time_range: &TimeRange) -> Result<GrowthTrends> {
        let memories = self.memory_manager
            .search_memories("", Some(10000))
            .await?;

        let filtered_memories: Vec<_> = memories.into_iter()
            .filter(|memory| {
                memory.created_at >= time_range.start && memory.created_at <= time_range.end
            })
            .collect();

        Ok(self.analyze_growth_trends_sync(&filtered_memories, time_range))
    }
    

    
    fn analyze_memory_types(&self, memories: &[Memory]) -> HashMap<MemoryType, usize> {
        let mut type_counts = HashMap::new();
        for memory in memories {
            *type_counts.entry(memory.memory_type.clone()).or_insert(0) += 1;
        }
        type_counts
    }
    

    
    #[allow(dead_code)]
    fn find_peak_periods(&self, daily_counts: &[usize]) -> Vec<String> {
        let avg = daily_counts.iter().sum::<usize>() as f32 / daily_counts.len() as f32;
        let mut peaks = Vec::new();
        
        for (i, &count) in daily_counts.iter().enumerate() {
            if count as f32 > avg * 1.5 {
                peaks.push(format!("Day {} (high activity)", i + 1));
            }
        }
        
        if peaks.is_empty() {
            peaks.push("No significant peaks detected".to_string());
        }
        
        peaks
    }
    
    fn calculate_unique_content_ratio(&self, memories: &[Memory]) -> f32 {
        if memories.is_empty() {
            return 1.0;
        }
        
        let mut unique_content = std::collections::HashSet::new();
        for memory in memories {
            // Use first 100 characters as content fingerprint
            let fingerprint = memory.content.chars().take(100).collect::<String>();
            unique_content.insert(fingerprint);
        }
        
        unique_content.len() as f32 / memories.len() as f32
    }
    
    fn calculate_tag_utilization(&self, memories: &[Memory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }
        
        let memories_with_tags = memories.iter().filter(|m| !m.tags.is_empty()).count();
        memories_with_tags as f32 / memories.len() as f32
    }
    
    fn calculate_type_distribution(&self, memories: &[Memory]) -> f32 {
        let type_counts = self.analyze_memory_types(memories);
        let num_types = type_counts.len() as f32;
        let max_possible_types = 7.0; // Based on MemoryType enum variants
        
        num_types / max_possible_types
    }
    
    fn estimate_retrieval_efficiency(&self, memories: &[Memory]) -> f32 {
        // Simple heuristic: memories with tags and reasonable length are more retrievable
        let well_structured = memories.iter()
            .filter(|m| !m.tags.is_empty() && m.content.len() > 10 && m.content.len() < 1000)
            .count();
        
        if memories.is_empty() {
            0.0
        } else {
            well_structured as f32 / memories.len() as f32
        }
    }
    
    fn calculate_storage_efficiency(&self, memories: &[Memory]) -> f32 {
        if memories.is_empty() {
            return 1.0;
        }
        
        let total_chars: usize = memories.iter().map(|m| m.content.len()).sum();
        let avg_chars = total_chars as f32 / memories.len() as f32;
        
        // Efficiency based on reasonable memory size (not too short, not too long)
        let optimal_range = 50.0..500.0;
        if optimal_range.contains(&avg_chars) {
            1.0
        } else if avg_chars < optimal_range.start {
            avg_chars / optimal_range.start
        } else {
            optimal_range.end / avg_chars
        }
    }
    
    fn generate_recommendations(&self, memories: &[Memory]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        let tag_utilization = self.calculate_tag_utilization(memories);
        if tag_utilization < 0.7 {
            recommendations.push("Consider adding more tags to memories to improve discoverability".to_string());
        }
        
        let unique_ratio = self.calculate_unique_content_ratio(memories);
        if unique_ratio < 0.8 {
            recommendations.push("High content similarity detected - consider deduplication".to_string());
        }
        
        let type_distribution = self.calculate_type_distribution(memories);
        if type_distribution < 0.3 {
            recommendations.push("Consider diversifying memory types for better organization".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("Memory system appears well-organized".to_string());
        }
        
        recommendations
    }

    /// Calculate memory efficiency metrics (sync version)
    fn calculate_efficiency_metrics_sync(&self, memories: &[Memory]) -> MemoryEfficiencyMetrics {
        let total_memories = memories.len();
        let unique_content_ratio = self.calculate_unique_content_ratio(memories);
        let tag_utilization = self.calculate_tag_utilization(memories);
        let type_distribution = self.calculate_type_distribution(memories);
        let retrieval_efficiency = self.estimate_retrieval_efficiency(memories);
        
        MemoryEfficiencyMetrics {
            total_memory_count: total_memories,
            unique_content_ratio,
            tag_utilization_score: tag_utilization,
            type_distribution_score: type_distribution,
            estimated_retrieval_efficiency: retrieval_efficiency,
            storage_efficiency: self.calculate_storage_efficiency(memories),
            redundancy_score: 1.0 - unique_content_ratio,
        }
    }

    /// Detect anomalies in memory patterns (sync version)
    async fn detect_anomalies_sync(&self, memories: &[Memory]) -> Result<Vec<MemoryAnomaly>> {
        let mut anomalies = Vec::new();
        
        if memories.is_empty() {
            return Ok(anomalies);
        }
        
        // Detect unusually large memories
        let avg_size = memories.iter().map(|m| m.content.len()).sum::<usize>() as f32 / memories.len() as f32;
        let size_threshold = avg_size * 3.0;
        
        for memory in memories {
            if memory.content.len() as f32 > size_threshold {
                anomalies.push(MemoryAnomaly {
                    anomaly_id: uuid::Uuid::new_v4().to_string(),
                    anomaly_type: AnomalyType::UnusualSize,
                    memory_id: memory.id.clone(),
                    description: format!("Memory is {:.1} times larger than average", 
                        memory.content.len() as f32 / avg_size),
                    severity: AnomalySeverity::Medium,
                    detected_at: Utc::now(),
                });
            }
        }
        
        // Detect memories with no tags
        for memory in memories {
            if memory.tags.is_empty() {
                anomalies.push(MemoryAnomaly {
                    anomaly_id: uuid::Uuid::new_v4().to_string(),
                    anomaly_type: AnomalyType::MissingTags,
                    memory_id: memory.id.clone(),
                    description: "Memory has no tags, reducing discoverability".to_string(),
                    severity: AnomalySeverity::Low,
                    detected_at: Utc::now(),
                });
            }
        }
        
        Ok(anomalies)
    }

    /// Analyze growth trends (sync version)
    fn analyze_growth_trends_sync(&self, memories: &[Memory], time_range: &TimeRange) -> GrowthTrends {
        // Simple growth trend calculation
        let days = (time_range.end - time_range.start).num_days();
        let growth_rate = if days > 0 {
            memories.len() as f32 / days as f32
        } else {
            0.0
        };
        
        // Determine trend direction based on simple analysis
        let trend_direction = if growth_rate > 0.5 {
            TrendDirection::Increasing
        } else if growth_rate < 0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };
        
        // Use the existing GrowthTrends struct fields
        GrowthTrends {
            average_memories_per_day: growth_rate,
            trend_direction,
            peak_activity_periods: vec![], // Would need more sophisticated analysis
            growth_rate_percentage: growth_rate * 100.0,
        }
    }
}

/// Comprehensive memory usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageReport {
    pub analysis_period: TimeRange,
    pub total_memories: usize,
    pub memory_types_breakdown: HashMap<MemoryType, usize>,
    pub growth_trends: GrowthTrends,
    pub efficiency_metrics: MemoryEfficiencyMetrics,
    pub anomalies: Vec<MemoryAnomaly>,
    pub recommendations: Vec<String>,
}

/// Memory efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEfficiencyMetrics {
    pub total_memory_count: usize,
    pub unique_content_ratio: f32,
    pub tag_utilization_score: f32,
    pub type_distribution_score: f32,
    pub estimated_retrieval_efficiency: f32,
    pub storage_efficiency: f32,
    pub redundancy_score: f32,
}

/// Growth trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthTrends {
    pub average_memories_per_day: f32,
    pub trend_direction: TrendDirection,
    pub peak_activity_periods: Vec<String>,
    pub growth_rate_percentage: f32,
}

/// Direction of memory growth trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Memory anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAnomaly {
    pub anomaly_id: String,
    pub anomaly_type: AnomalyType,
    pub memory_id: String,
    pub description: String,
    pub severity: AnomalySeverity,
    pub detected_at: DateTime<Utc>,
}

/// Types of memory anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    UnusualSize,
    MissingTags,
    PotentialDuplicate,
    OrphanedMemory,
    UnusualTimestamp,
}

/// Severity levels for anomalies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Comprehensive analytics report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAnalyticsReport {
    pub time_range: TimeRange,
    pub usage_report: Usage,
    pub efficiency_metrics: MemoryEfficiencyMetrics,
    pub anomalies: Vec<MemoryAnomaly>,
    pub growth_trends: GrowthTrends,
}

/// Usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub total_memories: usize,
    pub memory_types_breakdown: HashMap<MemoryType, usize>,
    pub growth_trends: GrowthTrends,
    pub efficiency_metrics: MemoryEfficiencyMetrics,
    pub anomalies: Vec<MemoryAnomaly>,
    pub recommendations: Vec<String>,
} 