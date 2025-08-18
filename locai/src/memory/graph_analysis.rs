//! Generic Memory Graph Analysis
//!
//! Provides graph-based analysis of memory networks, including pattern detection,
//! relationship tracing, and community detection that can be used by any application.

use crate::core::MemoryManager;
use crate::models::Memory;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Generic memory graph analyzer
pub struct MemoryGraphAnalyzer {
    memory_manager: Arc<MemoryManager>,
}

impl MemoryGraphAnalyzer {
    pub fn new(memory_manager: Arc<MemoryManager>) -> Self {
        Self { memory_manager }
    }

    /// Find memories that mention multiple entities (characters, concepts, etc.)
    pub async fn find_shared_memories(&self, entities: &[String]) -> Result<Vec<Memory>> {
        let search_query = entities.join(" ");
        let memories = self
            .memory_manager
            .search_memories(&search_query, None)
            .await?;

        // Filter for memories that actually involve all entities
        let shared: Vec<_> = memories
            .into_iter()
            .filter(|m| {
                let content_lower = m.content.to_lowercase();
                entities
                    .iter()
                    .all(|entity| content_lower.contains(&entity.to_lowercase()))
            })
            .collect();

        Ok(shared)
    }

    /// Detect memory clusters based on content similarity and temporal proximity
    pub async fn detect_memory_communities(
        &self,
        memories: Vec<Memory>,
        similarity_threshold: f32,
    ) -> Result<Vec<MemoryCommunity>> {
        let communities = self
            .cluster_memories(&memories, similarity_threshold)
            .await?;

        let mut memory_communities = Vec::new();

        for community in communities {
            let analysis = self.analyze_memory_community(&community).await?;

            memory_communities.push(MemoryCommunity {
                id: Uuid::new_v4().to_string(),
                memory_ids: community.iter().map(|m| m.id.clone()).collect(),
                dominant_theme: analysis.theme,
                cohesion_score: analysis.cohesion,
                representative_memories: analysis.representatives,
                temporal_span: analysis.temporal_span,
                size: community.len(),
            });
        }

        Ok(memory_communities)
    }

    /// Traverse memory connections through content similarity
    pub async fn traverse_memory_network(
        &self,
        start_memory_id: &str,
        max_depth: usize,
        similarity_threshold: f32,
    ) -> Result<Vec<Memory>> {
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        // Start with the initial memory
        let start_memories = self
            .memory_manager
            .search_memories(start_memory_id, None)
            .await?;
        if let Some(start_memory) = start_memories.into_iter().next() {
            queue.push_back((start_memory, 0));
        }

        while let Some((current_memory, depth)) = queue.pop_front() {
            if depth >= max_depth || visited.contains(&current_memory.id) {
                continue;
            }

            visited.insert(current_memory.id.clone());
            result.push(current_memory.clone());

            // Find similar memories for next traversal level
            let similar = self
                .find_similar_memories(&current_memory, similarity_threshold)
                .await?;

            for memory in similar {
                if !visited.contains(&memory.id) {
                    queue.push_back((memory, depth + 1));
                }
            }
        }

        Ok(result)
    }

    /// Find influence networks - memories that may have influenced or been influenced by a central memory
    pub async fn analyze_influence_network(
        &self,
        central_memory_id: &str,
    ) -> Result<InfluenceNetwork> {
        let central_memory = self
            .memory_manager
            .search_memories(central_memory_id, None)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Memory not found"))?;

        // Find temporally and thematically related memories
        let related_memories = self.find_related_memories(&central_memory).await?;

        // Separate into potential influences (before) and influenced (after)
        let mut influencing = Vec::new();
        let mut influenced = Vec::new();

        for memory in related_memories {
            match memory.created_at.cmp(&central_memory.created_at) {
                std::cmp::Ordering::Less => influencing.push(memory.id),
                std::cmp::Ordering::Greater => influenced.push(memory.id),
                std::cmp::Ordering::Equal => {
                    // Same timestamp - could be either, add to both for completeness
                }
            }
        }

        let network_strength = self
            .calculate_network_strength(&influencing, &influenced)
            .await?;

        Ok(InfluenceNetwork {
            central_memory: central_memory_id.to_string(),
            influencing_memories: influencing,
            influenced_memories: influenced,
            network_strength,
        })
    }

    /// Find memories similar to a given memory based on content and tags
    async fn find_similar_memories(
        &self,
        target_memory: &Memory,
        similarity_threshold: f32,
    ) -> Result<Vec<Memory>> {
        // Use tags and content snippets to find similar memories
        let mut search_terms = target_memory.tags.clone();

        // Add key words from content
        let words: Vec<&str> = target_memory
            .content
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .take(5)
            .collect();
        search_terms.extend(words.iter().map(|s| s.to_string()));

        let search_query = search_terms.join(" ");
        let candidates = self
            .memory_manager
            .search_memories(&search_query, None)
            .await?;

        // Filter by similarity threshold
        let similar_memories: Vec<Memory> = candidates
            .into_iter()
            .filter(|m| m.id != target_memory.id)
            .filter(|m| self.calculate_memory_similarity(target_memory, m) >= similarity_threshold)
            .collect();

        Ok(similar_memories)
    }

    /// Find memories related to a target memory through various relationships
    async fn find_related_memories(&self, target_memory: &Memory) -> Result<Vec<Memory>> {
        let mut related = Vec::new();

        // Find by tag overlap
        if !target_memory.tags.is_empty() {
            let tag_query = target_memory.tags.join(" ");
            let tag_matches = self
                .memory_manager
                .search_memories(&tag_query, None)
                .await?;
            related.extend(tag_matches);
        }

        // Find by content similarity
        let similar = self.find_similar_memories(target_memory, 0.3).await?;
        related.extend(similar);

        // Remove duplicates and the target memory itself
        related.sort_by(|a, b| a.id.cmp(&b.id));
        related.dedup_by(|a, b| a.id == b.id);
        related.retain(|m| m.id != target_memory.id);

        Ok(related)
    }

    /// Cluster memories based on similarity
    async fn cluster_memories(
        &self,
        memories: &[Memory],
        similarity_threshold: f32,
    ) -> Result<Vec<Vec<Memory>>> {
        let mut clusters = Vec::new();
        let mut unclustered: Vec<Memory> = memories.to_vec();

        while !unclustered.is_empty() {
            let seed = unclustered.remove(0);
            let mut cluster = vec![seed.clone()];

            // Find all memories similar to the seed
            let mut i = 0;
            while i < unclustered.len() {
                if self.calculate_memory_similarity(&seed, &unclustered[i]) >= similarity_threshold
                {
                    cluster.push(unclustered.remove(i));
                } else {
                    i += 1;
                }
            }

            clusters.push(cluster);
        }

        Ok(clusters)
    }

    /// Analyze a memory community to extract characteristics
    async fn analyze_memory_community(&self, community: &[Memory]) -> Result<CommunityAnalysis> {
        let theme = self.identify_community_theme(community);
        let cohesion = self.calculate_community_cohesion(community);
        let representatives = self.select_representative_memories(community);
        let temporal_span = self.calculate_temporal_span(community);

        Ok(CommunityAnalysis {
            theme,
            cohesion,
            representatives,
            temporal_span,
        })
    }

    /// Calculate similarity between two memories
    fn calculate_memory_similarity(&self, memory1: &Memory, memory2: &Memory) -> f32 {
        // Tag similarity (40% weight)
        let tag_sim = self.calculate_tag_similarity(&memory1.tags, &memory2.tags) * 0.4;

        // Content similarity (40% weight)
        let content_sim = self.calculate_text_similarity(&memory1.content, &memory2.content) * 0.4;

        // Temporal proximity (20% weight)
        let time_sim = self.calculate_time_proximity(memory1.created_at, memory2.created_at) * 0.2;

        tag_sim + content_sim + time_sim
    }

    /// Calculate tag similarity using Jaccard coefficient
    fn calculate_tag_similarity(&self, tags1: &[String], tags2: &[String]) -> f32 {
        if tags1.is_empty() && tags2.is_empty() {
            return 1.0;
        }

        let set1: std::collections::HashSet<_> = tags1.iter().collect();
        let set2: std::collections::HashSet<_> = tags2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Calculate text similarity using simple word overlap
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Calculate temporal proximity between two timestamps
    fn calculate_time_proximity(&self, time1: DateTime<Utc>, time2: DateTime<Utc>) -> f32 {
        let duration = (time1 - time2).abs();
        let hours = duration.num_hours() as f32;

        // Exponential decay - memories within same hour are very similar temporally
        (-hours / 24.0).exp()
    }

    /// Identify the dominant theme in a memory community
    fn identify_community_theme(&self, community: &[Memory]) -> String {
        let mut tag_counts = HashMap::new();

        // Count tag frequencies
        for memory in community {
            for tag in &memory.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        // Find most common tag
        tag_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(tag, _)| tag)
            .unwrap_or_else(|| "general".to_string())
    }

    /// Calculate cohesion score for a memory community
    fn calculate_community_cohesion(&self, community: &[Memory]) -> f32 {
        if community.len() < 2 {
            return 1.0;
        }

        let mut total_similarity = 0.0;
        let mut pair_count = 0;

        for i in 0..community.len() {
            for j in (i + 1)..community.len() {
                total_similarity += self.calculate_memory_similarity(&community[i], &community[j]);
                pair_count += 1;
            }
        }

        if pair_count > 0 {
            total_similarity / pair_count as f32
        } else {
            0.0
        }
    }

    /// Select representative memories from a community
    fn select_representative_memories(&self, community: &[Memory]) -> Vec<String> {
        if community.is_empty() {
            return Vec::new();
        }

        // Select up to 3 most representative memories
        let mut representatives: Vec<_> = community
            .iter()
            .map(|memory| {
                let representativeness = self.calculate_representativeness(memory, community);
                (memory.id.clone(), representativeness)
            })
            .collect();

        representatives.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        representatives
            .into_iter()
            .take(3)
            .map(|(id, _)| id)
            .collect()
    }

    /// Calculate how representative a memory is of its community
    fn calculate_representativeness(&self, memory: &Memory, community: &[Memory]) -> f32 {
        let mut total_similarity = 0.0;
        let mut count = 0;

        for other in community {
            if other.id != memory.id {
                total_similarity += self.calculate_memory_similarity(memory, other);
                count += 1;
            }
        }

        if count > 0 {
            total_similarity / count as f32
        } else {
            0.0
        }
    }

    /// Calculate temporal span of a memory community
    fn calculate_temporal_span(&self, community: &[Memory]) -> TemporalSpan {
        if community.is_empty() {
            let now = Utc::now();
            return TemporalSpan {
                start: now,
                end: now,
                duration_days: 0,
            };
        }

        let mut timestamps: Vec<_> = community.iter().map(|m| m.created_at).collect();
        timestamps.sort();

        let start = timestamps[0];
        let end = timestamps[timestamps.len() - 1];
        let duration_days = (end - start).num_days();

        TemporalSpan {
            start,
            end,
            duration_days,
        }
    }

    /// Calculate network strength based on connection density
    async fn calculate_network_strength(
        &self,
        influencing: &[String],
        influenced: &[String],
    ) -> Result<f32> {
        let total_connections = influencing.len() + influenced.len();
        if total_connections == 0 {
            return Ok(0.0);
        }

        // Simple heuristic: more connections = stronger network
        // Could be enhanced with more sophisticated metrics
        let strength = (total_connections as f32).log2() / 10.0;
        Ok(strength.min(1.0))
    }
}

/// A community of related memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCommunity {
    pub id: String,
    pub memory_ids: Vec<String>,
    pub dominant_theme: String,
    pub cohesion_score: f32,
    pub representative_memories: Vec<String>,
    pub temporal_span: TemporalSpan,
    pub size: usize,
}

/// Temporal span of a set of memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalSpan {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration_days: i64,
}

/// Network of memory influences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceNetwork {
    pub central_memory: String,
    pub influencing_memories: Vec<String>,
    pub influenced_memories: Vec<String>,
    pub network_strength: f32,
}

/// Internal analysis of a memory community
struct CommunityAnalysis {
    theme: String,
    cohesion: f32,
    representatives: Vec<String>,
    temporal_span: TemporalSpan,
}
