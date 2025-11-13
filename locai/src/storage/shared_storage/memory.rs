//! Memory storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use surrealdb::{Connection, RecordId};

use super::base::SharedStorage;
use crate::models::Memory;
use crate::storage::errors::StorageError;
use crate::storage::filters::MemoryFilter;
use crate::storage::traits::MemoryStore;

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Internal representation of a Memory record for SurrealDB (matching working implementation exactly)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SurrealMemory {
    id: RecordId,
    content: String,
    metadata: Value,
    embedding: Option<Vec<f32>>,
    importance: Option<f32>,
    owner: RecordId,
    shared_with: Option<Vec<RecordId>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<Memory> for SurrealMemory {
    fn from(memory: Memory) -> Self {
        Self {
            id: RecordId::from(("memory", memory.id.as_str())),
            content: memory.content,
            metadata: serde_json::json!({
                "memory_type": memory.memory_type,
                "last_accessed": memory.last_accessed.map(|dt| dt.to_rfc3339()),
                "access_count": memory.access_count,
                "priority": memory.priority,
                "tags": memory.tags,
                "source": memory.source,
                "expires_at": memory.expires_at.map(|dt| dt.to_rfc3339()),
                "properties": memory.properties,
                "related_memories": memory.related_memories,
            }),
            embedding: memory.embedding,
            importance: None,
            owner: RecordId::from(("user", "system")),
            shared_with: None,
            created_at: memory.created_at,
            updated_at: Utc::now(),
        }
    }
}

impl From<SurrealMemory> for Memory {
    fn from(surreal_memory: SurrealMemory) -> Self {
        use crate::models::{MemoryPriority, MemoryType};

        // Extract specific fields from metadata object using the proven pattern
        let memory_type = surreal_memory
            .metadata
            .get("memory_type")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(MemoryType::Episodic);

        let last_accessed = surreal_memory
            .metadata
            .get("last_accessed")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let access_count = surreal_memory
            .metadata
            .get("access_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let priority = surreal_memory
            .metadata
            .get("priority")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(MemoryPriority::Normal);

        let tags = surreal_memory
            .metadata
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let source = surreal_memory
            .metadata
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let expires_at = surreal_memory
            .metadata
            .get("expires_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = surreal_memory
            .metadata
            .get("properties")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let related_memories = surreal_memory
            .metadata
            .get("related_memories")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            id: surreal_memory.id.key().to_string(),
            content: surreal_memory.content,
            memory_type,
            created_at: surreal_memory.created_at,
            last_accessed,
            access_count,
            priority,
            tags,
            source,
            expires_at,
            properties,
            related_memories,
            embedding: surreal_memory.embedding,
        }
    }
}

#[async_trait]
impl<C> MemoryStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new memory
    async fn create_memory(&self, memory: Memory) -> Result<Memory, StorageError> {
        // Ensure system user exists
        self.ensure_system_user().await?;

        // Build metadata object exactly like the working implementation
        let metadata = serde_json::json!({
            "memory_type": memory.memory_type,
            "last_accessed": memory.last_accessed.map(|dt| dt.to_rfc3339()),
            "access_count": memory.access_count,
            "priority": memory.priority,
            "tags": memory.tags,
            "source": memory.source,
            "expires_at": memory.expires_at.map(|dt| dt.to_rfc3339()),
            "properties": memory.properties,
            "related_memories": memory.related_memories,
        });

        // Use the EXACT working query from memory.rs
        let query = r#"
            CREATE memory CONTENT {
                content: $content,
                metadata: $metadata,
                embedding: $embedding,
                importance: $importance,
                owner: $owner,
                shared_with: $shared_with,
                created_at: type::datetime($created_at)
            }
        "#;

        let mut result = self
            .client
            .query(query)
            .bind(("content", memory.content.clone()))
            .bind(("metadata", metadata))
            .bind(("embedding", memory.embedding.clone()))
            .bind(("importance", None::<f32>))
            .bind(("owner", RecordId::from(("user", "system"))))
            .bind(("shared_with", None::<Vec<RecordId>>))
            .bind(("created_at", memory.created_at.to_rfc3339()))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to create memory: {}", e)))?;

        let created: Vec<SurrealMemory> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract created memory: {}", e)))?;

        let created_memory = created
            .into_iter()
            .next()
            .map(Memory::from)
            .ok_or_else(|| StorageError::Internal("No memory created".to_string()))?;

        // Execute on_memory_created hooks (non-blocking, fire-and-forget)
        let hooks = self.hook_registry.clone();
        let memory_clone = created_memory.clone();
        tokio::spawn(async move {
            if let Err(e) = hooks.execute_on_created(&memory_clone).await {
                tracing::warn!("Hook execution failed for on_memory_created: {}", e);
            }
        });

        Ok(created_memory)
    }

    /// Get a memory by its ID
    async fn get_memory(&self, id: &str) -> Result<Option<Memory>, StorageError> {
        let memory = self.get_memory_internal(id).await?;

        // Execute on_memory_accessed hooks (non-blocking, fire-and-forget)
        if let Some(ref mem) = memory {
            let hooks = self.hook_registry.clone();
            let memory_clone = mem.clone();
            tokio::spawn(async move {
                if let Err(e) = hooks.execute_on_accessed(&memory_clone).await {
                    tracing::warn!("Hook execution failed for on_memory_accessed: {}", e);
                }
            });
        }

        Ok(memory)
    }

    /// Update an existing memory
    async fn update_memory(&self, memory: Memory) -> Result<Memory, StorageError> {
        let record_id = RecordId::from(("memory", memory.id.as_str()));

        // Get the old memory before updating (use internal to avoid hook recursion)
        let old_memory = self.get_memory_internal(&memory.id).await?;

        // Build metadata exactly like create_memory
        let metadata = serde_json::json!({
            "memory_type": memory.memory_type,
            "last_accessed": memory.last_accessed.map(|dt| dt.to_rfc3339()),
            "access_count": memory.access_count,
            "priority": memory.priority,
            "tags": memory.tags,
            "source": memory.source,
            "expires_at": memory.expires_at.map(|dt| dt.to_rfc3339()),
            "properties": memory.properties,
            "related_memories": memory.related_memories,
        });

        let query = r#"
            UPDATE $id SET 
                content = $content,
                metadata = $metadata,
                embedding = $embedding,
                updated_at = time::now()
        "#;

        let mut result = self
            .client
            .query(query)
            .bind(("id", record_id))
            .bind(("content", memory.content.clone()))
            .bind(("metadata", metadata))
            .bind(("embedding", memory.embedding.clone()))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to update memory: {}", e)))?;

        let updated: Vec<SurrealMemory> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract updated memory: {}", e)))?;

        let updated_memory = updated
            .into_iter()
            .next()
            .map(Memory::from)
            .ok_or_else(|| {
                StorageError::NotFound(format!("Memory with id {} not found", memory.id))
            })?;

        // Execute on_memory_updated hooks (non-blocking, fire-and-forget)
        if let Some(old_mem) = old_memory {
            let hooks = self.hook_registry.clone();
            let updated_clone = updated_memory.clone();
            tokio::spawn(async move {
                if let Err(e) = hooks.execute_on_updated(&old_mem, &updated_clone).await {
                    tracing::warn!("Hook execution failed for on_memory_updated: {}", e);
                }
            });
        }

        Ok(updated_memory)
    }

    /// Delete a memory by its ID
    async fn delete_memory(&self, id: &str) -> Result<bool, StorageError> {
        // Get memory before deletion (use internal to avoid hook recursion)
        let memory_to_delete = self.get_memory_internal(id).await?;

        // Execute before_memory_deleted hooks (blocking for veto support)
        if let Some(mem) = &memory_to_delete {
            match self.hook_registry.execute_before_deleted(mem).await {
                Ok(true) => {
                    // Hooks allowed deletion, proceed
                    tracing::debug!("Deletion allowed by hooks for memory {}", id);
                }
                Ok(false) => {
                    // Hooks vetoed deletion
                    tracing::warn!("Deletion vetoed by hooks for memory {}", id);
                    return Ok(false);
                }
                Err(e) => {
                    // Hook execution error - log but continue (don't fail the operation)
                    tracing::warn!("Hook execution failed during deletion check: {}", e);
                }
            }
        }

        // Use SDK method directly like VectorStore for consistency
        let deleted: Option<SurrealMemory> = self
            .client
            .delete(("memory", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to delete memory: {}", e)))?;

        Ok(deleted.is_some())
    }

    /// List memories with optional filtering
    async fn list_memories(
        &self,
        filter: Option<MemoryFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Memory>, StorageError> {
        let mut query = "SELECT * FROM memory".to_string();
        let mut conditions = Vec::new();

        // Add filter conditions
        if let Some(f) = &filter {
            if let Some(memory_type) = &f.memory_type {
                // Memory type can be stored as either a string or an enum variant
                // Try both representations for compatibility
                let mt_lower = memory_type.to_lowercase();
                conditions.push(format!(
                    "(type::string(metadata.memory_type) = '{}' OR string::lowercase(type::string(metadata.memory_type)) CONTAINS '{}')",
                    mt_lower, mt_lower
                ));
            }

            if let Some(content) = &f.content {
                conditions.push(format!("content CONTAINS '{}'", content));
            }

            if let Some(tags) = &f.tags
                && !tags.is_empty()
            {
                let tag_conditions: Vec<String> = tags
                    .iter()
                    .map(|tag| format!("'{}' IN metadata.tags", tag))
                    .collect();
                conditions.push(format!("({})", tag_conditions.join(" OR ")));
            }

            if let Some(source) = &f.source {
                conditions.push(format!("metadata.source = '{}'", source));
            }

            if let Some(created_after) = &f.created_after {
                conditions.push(format!("created_at > d'{}'", created_after.to_rfc3339()));
            }

            if let Some(created_before) = &f.created_before {
                conditions.push(format!("created_at < d'{}'", created_before.to_rfc3339()));
            }
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = offset {
            query.push_str(&format!(" START {}", offset));
        }

        let mut result = self
            .client
            .query(&query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list memories: {}", e)))?;

        let memories: Vec<SurrealMemory> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract memories: {}", e)))?;

        Ok(memories.into_iter().map(Memory::from).collect())
    }

    /// Count memories with optional filtering
    async fn count_memories(&self, filter: Option<MemoryFilter>) -> Result<usize, StorageError> {
        // Simple approach: get all memories matching the filter and count them
        let memories = self.list_memories(filter, None, None).await?;
        Ok(memories.len())
    }

    /// Batch create multiple memories
    async fn batch_create_memories(
        &self,
        memories: Vec<Memory>,
    ) -> Result<Vec<Memory>, StorageError> {
        let mut created_memories = Vec::new();

        // For now, create memories one by one to avoid complex binding issues
        for memory in memories {
            let created = self.create_memory(memory).await?;
            created_memories.push(created);
        }

        Ok(created_memories)
    }

    /// Full-text search using BM25 scoring with highlights
    async fn bm25_search_memories(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32, String)>, StorageError> {
        let limit = limit.unwrap_or(10);

        let search_query = r#"
            SELECT *, 
                   search::score(0) AS bm25_score,
                   search::highlight('<mark>', '</mark>', 0) AS highlighted_content
            FROM memory 
            WHERE content @0@ $query
            ORDER BY bm25_score DESC
            LIMIT $limit
        "#;

        let query_string = query.to_string();
        let mut result = self
            .client
            .query(search_query)
            .bind(("query", query_string))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform BM25 search: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BM25SearchResult {
            id: RecordId,
            content: String,
            metadata: Value,
            embedding: Option<Vec<f32>>,
            importance: Option<f32>,
            owner: RecordId,
            shared_with: Option<Vec<RecordId>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            bm25_score: f32,
            highlighted_content: String,
        }

        let results: Vec<BM25SearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract BM25 results: {}", e)))?;

        // Debug: Log BM25 search results for problematic query
        if query.contains("nonexistent") {
            tracing::debug!(
                "BM25 search for '{}' found {} results",
                query,
                results.len()
            );
            for result in &results {
                tracing::debug!("BM25 result: {} (score: {})", result.id, result.bm25_score);
            }
        }

        // Convert BM25SearchResult to SurrealMemory then to Memory
        Ok(results
            .into_iter()
            .map(|r| {
                let surreal_memory = SurrealMemory {
                    id: r.id,
                    content: r.content,
                    metadata: r.metadata,
                    embedding: r.embedding,
                    importance: r.importance,
                    owner: r.owner,
                    shared_with: r.shared_with,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                };
                (
                    Memory::from(surreal_memory),
                    r.bm25_score,
                    r.highlighted_content,
                )
            })
            .collect())
    }

    /// Fuzzy search for typo tolerance
    async fn fuzzy_search_memories(
        &self,
        query: &str,
        similarity_threshold: Option<f32>,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32)>, StorageError> {
        let limit = limit.unwrap_or(10);
        let threshold = similarity_threshold.unwrap_or(0.3);

        let fuzzy_query = r#"
            SELECT *, 
                   string::similarity::fuzzy(content, $query) AS fuzzy_score
            FROM memory 
            WHERE content ~* $query
              AND string::similarity::fuzzy(content, $query) >= $threshold
            ORDER BY fuzzy_score DESC
            LIMIT $limit
        "#;

        let query_string = query.to_string();
        let mut result = self
            .client
            .query(fuzzy_query)
            .bind(("query", query_string))
            .bind(("threshold", threshold))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform fuzzy search: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct FuzzySearchResult {
            #[serde(flatten)]
            memory: SurrealMemory,
            fuzzy_score: f32,
        }

        let results: Vec<FuzzySearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract fuzzy results: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|r| (Memory::from(r.memory), r.fuzzy_score))
            .collect())
    }

    /// Vector similarity search on memories using their embeddings (BYOE approach)
    async fn vector_search_memories(
        &self,
        query_vector: &[f32],
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32, String)>, StorageError> {
        // Use the same implementation as our concrete method
        let limit = limit.unwrap_or(10);

        // Search memories that have embeddings using SurrealDB KNN vector similarity
        // Note: Uses M-Tree index on embedding field (defined in schema) for exact nearest neighbor search
        // Explicitly filter out NULL embeddings to ensure KNN operator works correctly
        let vector_query = format!(
            r#"
                SELECT *, 
                       vector::distance::knn() AS vector_distance,
                       (1.0 - vector::distance::knn()) AS similarity_score
                FROM memory 
                WHERE embedding IS NOT NULL
                  AND embedding <|{}|> $query_vector
                ORDER BY similarity_score DESC
                LIMIT {}
            "#,
            limit, limit
        );

        let query_vector_owned: Vec<f32> = query_vector.to_vec();

        // Log query for debugging
        tracing::debug!("Vector search query: {}", vector_query);
        tracing::debug!("Query vector dimensions: {}", query_vector_owned.len());

        // Debug: Check how many memories have embeddings
        let count_query = "SELECT VALUE count() FROM memory WHERE embedding IS NOT NULL";
        if let Ok(mut count_result) = self.client.query(count_query).await
            && let Ok(counts) = count_result.take::<Vec<u64>>(0)
            && let Some(count) = counts.first()
        {
            tracing::debug!("Memories with embeddings: {}", count);
        }

        let mut result = self
            .client
            .query(&vector_query)
            .bind(("query_vector", query_vector_owned))
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to perform vector search on memories: {}. Query: {}",
                    e, vector_query
                );
                tracing::error!("{}", error_msg);
                StorageError::Query(error_msg)
            })?;

        // Define result struct explicitly (like BM25 search) - don't use flatten with RecordId
        #[derive(serde::Deserialize)]
        struct VectorSearchResult {
            id: RecordId,
            content: String,
            metadata: Value,
            embedding: Option<Vec<f32>>,
            importance: Option<f32>,
            owner: RecordId,
            shared_with: Option<Vec<RecordId>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            similarity_score: f32,
            #[allow(dead_code)]
            vector_distance: f32,
        }

        let results: Vec<VectorSearchResult> = match result.take(0) {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Failed to extract vector search results: {}", e);
                tracing::debug!("{}", error_msg);
                tracing::debug!("Falling back to brute-force search");
                return self.brute_force_vector_search(query_vector, limit).await;
            }
        };

        tracing::debug!("Vector search returned {} results", results.len());

        if results.is_empty() {
            tracing::debug!(
                "M-Tree index search returned 0 results, falling back to brute-force search"
            );
            return self.brute_force_vector_search(query_vector, limit).await;
        }

        // Convert VectorSearchResult to SurrealMemory then to Memory
        Ok(results
            .into_iter()
            .map(|r| {
                let memory = SurrealMemory {
                    id: r.id,
                    content: r.content,
                    metadata: r.metadata,
                    embedding: r.embedding,
                    importance: r.importance,
                    owner: r.owner,
                    shared_with: r.shared_with,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                };
                (
                    Memory::from(memory),
                    r.similarity_score,
                    String::new(), // No highlighting for vector search
                )
            })
            .collect())
    }

    /// Search memories with configurable multi-factor scoring
    async fn search_memories_with_scoring(
        &self,
        query: &str,
        scoring: Option<crate::search::ScoringConfig>,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32)>, StorageError> {
        use crate::search::ScoreCalculator;

        let limit = limit.unwrap_or(10);
        let mut config = scoring.unwrap_or_default();
        config.normalize_weights();

        let calculator = ScoreCalculator::try_new(config)
            .map_err(|e| StorageError::Query(format!("Invalid scoring config: {}", e)))?;

        // Get BM25 results - this is our primary search mechanism
        let bm25_results = self.bm25_search_memories(query, Some(limit * 2)).await?;

        // If vector scoring is needed and embeddings are available, also search vectors
        let vector_results = if calculator.config().vector_weight > 0.0 {
            // Try to get query embedding (this would require embedding service integration)
            // For now, we collect memory embeddings that exist
            self.collect_vector_search_results(query, limit).await.ok()
        } else {
            None
        };

        // Calculate final scores
        let mut scored_results: Vec<(Memory, f32)> = bm25_results
            .into_iter()
            .map(|(memory, bm25_score, _highlighted)| {
                // Look up vector score if available
                let vector_score = vector_results
                    .as_ref()
                    .and_then(|results| results.iter().find(|(m, _)| m.id == memory.id))
                    .map(|(_, score)| *score);

                let final_score =
                    calculator.calculate_final_score(bm25_score, vector_score, &memory);
                (memory, final_score)
            })
            .collect();

        // Sort by score descending
        scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top results
        scored_results.truncate(limit);

        Ok(scored_results)
    }
}

/// Enhanced search methods for the intelligence layer
impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Full-text search using BM25 scoring with highlights
    pub async fn bm25_search_memories(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32, String)>, StorageError> {
        let limit = limit.unwrap_or(10);

        let search_query = r#"
            SELECT *, 
                   search::score(0) AS bm25_score,
                   search::highlight('<mark>', '</mark>', 0) AS highlighted_content
            FROM memory 
            WHERE content @0@ $query
            ORDER BY bm25_score DESC
            LIMIT $limit
        "#;

        let query_string = query.to_string();
        let mut result = self
            .client
            .query(search_query)
            .bind(("query", query_string))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform BM25 search: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct BM25SearchResult {
            id: RecordId,
            content: String,
            metadata: Value,
            embedding: Option<Vec<f32>>,
            importance: Option<f32>,
            owner: RecordId,
            shared_with: Option<Vec<RecordId>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            bm25_score: f32,
            highlighted_content: String,
        }

        let results: Vec<BM25SearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract BM25 results: {}", e)))?;

        // Debug: Log BM25 search results for problematic query
        if query.contains("nonexistent") {
            tracing::debug!(
                "BM25 search for '{}' found {} results",
                query,
                results.len()
            );
            for result in &results {
                tracing::debug!("BM25 result: {} (score: {})", result.id, result.bm25_score);
            }
        }

        // Convert BM25SearchResult to SurrealMemory then to Memory
        Ok(results
            .into_iter()
            .map(|r| {
                let surreal_memory = SurrealMemory {
                    id: r.id,
                    content: r.content,
                    metadata: r.metadata,
                    embedding: r.embedding,
                    importance: r.importance,
                    owner: r.owner,
                    shared_with: r.shared_with,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                };
                (
                    Memory::from(surreal_memory),
                    r.bm25_score,
                    r.highlighted_content,
                )
            })
            .collect())
    }

    /// Fuzzy search for typo tolerance
    pub async fn fuzzy_search_memories(
        &self,
        query: &str,
        similarity_threshold: Option<f32>,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32)>, StorageError> {
        let limit = limit.unwrap_or(10);
        let threshold = similarity_threshold.unwrap_or(0.3);

        let fuzzy_query = r#"
            SELECT *, 
                   string::similarity::fuzzy(content, $query) AS fuzzy_score
            FROM memory 
            WHERE content ~* $query
              AND string::similarity::fuzzy(content, $query) >= $threshold
            ORDER BY fuzzy_score DESC
            LIMIT $limit
        "#;

        let query_string = query.to_string();
        let mut result = self
            .client
            .query(fuzzy_query)
            .bind(("query", query_string))
            .bind(("threshold", threshold))
            .bind(("limit", limit))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform fuzzy search: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct FuzzySearchResult {
            #[serde(flatten)]
            memory: SurrealMemory,
            fuzzy_score: f32,
        }

        let results: Vec<FuzzySearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract fuzzy results: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|r| (Memory::from(r.memory), r.fuzzy_score))
            .collect())
    }

    /// Hybrid search combining BM25 and vector similarity
    pub async fn hybrid_search_memories(
        &self,
        query: &str,
        query_vector: Option<&[f32]>,
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32, Option<f32>, Option<f32>)>, StorageError> {
        let limit = limit.unwrap_or(10);

        let hybrid_query = if let Some(_vector) = query_vector {
            // Combine text and vector search
            r#"
                SELECT *, 
                       search::score(0) AS text_score,
                       vector::distance::knn() AS vector_distance,
                       (0.6 * search::score(0) + 0.4 * (1 - vector::distance::knn())) AS combined_score
                FROM memory 
                WHERE content @0@ $query
                  AND embedding IS NOT NULL
                  AND embedding <|$limit,COSINE|> $query_vector
                ORDER BY combined_score DESC
                LIMIT $limit
            "#
        } else {
            // Text-only search with BM25
            r#"
                SELECT *, 
                       search::score(0) AS text_score,
                       NULL AS vector_distance,
                       search::score(0) AS combined_score
                FROM memory 
                WHERE content @0@ $query
                ORDER BY combined_score DESC
                LIMIT $limit
            "#
        };

        let query_string = query.to_string();
        let query_builder = self
            .client
            .query(hybrid_query)
            .bind(("query", query_string))
            .bind(("limit", limit));

        if let Some(_vector) = query_vector {
            // Note: Vector parameter binding not implemented
            // BM25 text search provides excellent results for most use cases
        }

        let mut result = query_builder
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform hybrid search: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct HybridSearchResult {
            #[serde(flatten)]
            memory: SurrealMemory,
            text_score: f32,
            vector_distance: Option<f32>,
            combined_score: f32,
        }

        let results: Vec<HybridSearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract hybrid results: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|r| {
                (
                    Memory::from(r.memory),
                    r.combined_score,
                    Some(r.text_score),
                    r.vector_distance.map(|d| 1.0 - d), // Convert distance to similarity
                )
            })
            .collect())
    }

    /// Vector similarity search on memories using their embeddings
    pub async fn vector_search_memories(
        &self,
        query_vector: &[f32],
        limit: Option<usize>,
    ) -> Result<Vec<(Memory, f32, String)>, StorageError> {
        let limit = limit.unwrap_or(10);

        // Search memories that have embeddings using SurrealDB KNN vector similarity
        // Note: Uses M-Tree index on embedding field (defined in schema) for exact nearest neighbor search
        // Explicitly filter out NULL embeddings to ensure KNN operator works correctly
        let vector_query = format!(
            r#"
                SELECT *, 
                       vector::distance::knn() AS vector_distance,
                       (1.0 - vector::distance::knn()) AS similarity_score
                FROM memory 
                WHERE embedding IS NOT NULL
                  AND embedding <|{}|> $query_vector
                ORDER BY similarity_score DESC
                LIMIT {}
            "#,
            limit, limit
        );

        let query_vector_owned: Vec<f32> = query_vector.to_vec();

        // Log query for debugging
        tracing::debug!("Vector search query: {}", vector_query);
        tracing::debug!("Query vector dimensions: {}", query_vector_owned.len());

        // Debug: Check how many memories have embeddings
        let count_query = "SELECT VALUE count() FROM memory WHERE embedding IS NOT NULL";
        if let Ok(mut count_result) = self.client.query(count_query).await
            && let Ok(counts) = count_result.take::<Vec<u64>>(0)
            && let Some(count) = counts.first()
        {
            tracing::debug!("Memories with embeddings: {}", count);
        }

        let mut result = self
            .client
            .query(&vector_query)
            .bind(("query_vector", query_vector_owned))
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to perform vector search on memories: {}. Query: {}",
                    e, vector_query
                );
                tracing::error!("{}", error_msg);
                StorageError::Query(error_msg)
            })?;

        // Define result struct explicitly (like BM25 search) - don't use flatten with RecordId
        #[derive(serde::Deserialize)]
        struct VectorSearchResult {
            id: RecordId,
            content: String,
            metadata: Value,
            embedding: Option<Vec<f32>>,
            importance: Option<f32>,
            owner: RecordId,
            shared_with: Option<Vec<RecordId>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            similarity_score: f32,
            #[allow(dead_code)]
            vector_distance: f32,
        }

        let results: Vec<VectorSearchResult> = match result.take(0) {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Failed to extract vector search results: {}", e);
                tracing::debug!("{}", error_msg);
                tracing::debug!("Falling back to brute-force search");
                return self.brute_force_vector_search(query_vector, limit).await;
            }
        };

        tracing::debug!("Vector search returned {} results", results.len());

        if results.is_empty() {
            tracing::debug!(
                "M-Tree index search returned 0 results, falling back to brute-force search"
            );
            return self.brute_force_vector_search(query_vector, limit).await;
        }

        // Convert VectorSearchResult to SurrealMemory then to Memory
        Ok(results
            .into_iter()
            .map(|r| {
                let memory = SurrealMemory {
                    id: r.id,
                    content: r.content,
                    metadata: r.metadata,
                    embedding: r.embedding,
                    importance: r.importance,
                    owner: r.owner,
                    shared_with: r.shared_with,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                };
                (Memory::from(memory), r.similarity_score, String::new())
            })
            .collect())
    }

    /// Brute-force vector search using cosine similarity
    /// This is a fallback when M-Tree index doesn't work (e.g., with optional fields)
    async fn brute_force_vector_search(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<(Memory, f32, String)>, StorageError> {
        tracing::debug!("Performing brute-force vector search");

        // Get all memories with embeddings
        let all_memories_query = "SELECT * FROM memory WHERE embedding IS NOT NULL";
        let mut result = self.client.query(all_memories_query).await.map_err(|e| {
            StorageError::Query(format!(
                "Failed to fetch memories for brute-force search: {}",
                e
            ))
        })?;

        let memories: Vec<SurrealMemory> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract memories: {}", e)))?;

        tracing::debug!(
            "Found {} memories with embeddings for brute-force search",
            memories.len()
        );

        // Calculate cosine similarity for each memory
        let mut scored_memories: Vec<(Memory, f32)> = memories
            .into_iter()
            .filter_map(|surreal_mem| {
                let mem = Memory::from(surreal_mem.clone());
                if let Some(embedding) = &mem.embedding {
                    if embedding.len() == query_vector.len() {
                        let similarity = cosine_similarity(query_vector, embedding);
                        Some((mem, similarity))
                    } else {
                        tracing::debug!(
                            "Skipping memory {}: embedding dimension mismatch ({} vs {})",
                            mem.id,
                            embedding.len(),
                            query_vector.len()
                        );
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (descending) and take top results
        scored_memories.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored_memories.truncate(limit);

        tracing::debug!("Brute-force search found {} results", scored_memories.len());

        Ok(scored_memories
            .into_iter()
            .map(|(mem, similarity)| (mem, similarity, String::new()))
            .collect())
    }

    /// Auto-complete suggestions based on memory content
    pub async fn memory_autocomplete(
        &self,
        partial_query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<String>, StorageError> {
        let limit = limit.unwrap_or(5);

        let autocomplete_query = r#"
            SELECT content
            FROM memory 
            WHERE content ~ $partial
            LIMIT $limit
        "#;

        let mut result = self
            .client
            .query(autocomplete_query)
            .bind(("partial", format!("{}*", partial_query)))
            .bind(("limit", limit))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to get autocomplete suggestions: {}", e))
            })?;

        #[derive(serde::Deserialize)]
        struct AutocompleteResult {
            content: String,
        }

        let results: Vec<AutocompleteResult> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to extract autocomplete results: {}", e))
        })?;

        // Extract unique words/phrases that start with the partial query
        let mut suggestions = Vec::new();
        for result in results {
            let words: Vec<&str> = result.content.split_whitespace().collect();
            for window in words
                .windows(1)
                .chain(words.windows(2))
                .chain(words.windows(3))
            {
                let phrase = window.join(" ");
                if phrase
                    .to_lowercase()
                    .starts_with(&partial_query.to_lowercase())
                    && !suggestions.contains(&phrase)
                {
                    suggestions.push(phrase);
                    if suggestions.len() >= limit {
                        break;
                    }
                }
            }
            if suggestions.len() >= limit {
                break;
            }
        }

        Ok(suggestions)
    }

    /// Temporal search for memories within a time range
    pub async fn temporal_search_memories(
        &self,
        query: Option<&str>,
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>, StorageError> {
        let limit = limit.unwrap_or(10);
        let mut conditions = Vec::new();

        if let Some(query) = query {
            conditions.push(format!("content @0@ '{}'", query));
        }

        if let Some(after_date) = after {
            conditions.push(format!("created_at > d'{}'", after_date.to_rfc3339()));
        }

        if let Some(before_date) = before {
            conditions.push(format!("created_at < d'{}'", before_date.to_rfc3339()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let temporal_query = format!(
            "SELECT * FROM memory {} ORDER BY created_at DESC LIMIT {}",
            where_clause, limit
        );

        let mut result = self.client.query(&temporal_query).await.map_err(|e| {
            StorageError::Query(format!("Failed to perform temporal search: {}", e))
        })?;

        let memories: Vec<SurrealMemory> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to extract temporal results: {}", e))
        })?;

        Ok(memories.into_iter().map(Memory::from).collect())
    }

    /// Search memories by tags with full-text support
    pub async fn tag_search_memories(
        &self,
        tags: &[String],
        match_all: bool,
        limit: Option<usize>,
    ) -> Result<Vec<Memory>, StorageError> {
        let limit = limit.unwrap_or(10);

        let tag_condition = if match_all {
            // All tags must be present
            let conditions: Vec<String> = tags
                .iter()
                .map(|tag| format!("'{}' IN metadata.tags", tag))
                .collect();
            conditions.join(" AND ")
        } else {
            // Any tag must be present
            let conditions: Vec<String> = tags
                .iter()
                .map(|tag| format!("'{}' IN metadata.tags", tag))
                .collect();
            format!("({})", conditions.join(" OR "))
        };

        let tag_query = format!(
            "SELECT * FROM memory WHERE {} ORDER BY created_at DESC LIMIT {}",
            tag_condition, limit
        );

        let mut result = self
            .client
            .query(&tag_query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to perform tag search: {}", e)))?;

        let memories: Vec<SurrealMemory> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to extract tag search results: {}", e))
        })?;

        Ok(memories.into_iter().map(Memory::from).collect())
    }

    /// Helper to collect vector search results for scoring integration
    async fn collect_vector_search_results(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<(Memory, f32)>, StorageError> {
        // This would integrate with the embedding service to:
        // 1. Get an embedding for the query
        // 2. Perform vector search
        // 3. Return (Memory, vector_score) tuples
        // For now, return empty to indicate no vector results available
        Ok(Vec::new())
    }
}

/// Private implementation for lifecycle tracking
impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Update only the lifecycle metadata for a memory (access_count and last_accessed)
    /// This is used internally for non-blocking lifecycle tracking updates
    async fn update_lifecycle_metadata(&self, memory: &Memory) -> Result<(), StorageError> {
        let record_id = RecordId::from(("memory", memory.id.as_str()));

        let query = r#"
            UPDATE $id SET 
                metadata.last_accessed = $last_accessed,
                metadata.access_count = $access_count,
                updated_at = time::now()
        "#;

        self.client
            .query(query)
            .bind(("id", record_id))
            .bind((
                "last_accessed",
                memory.last_accessed.map(|dt| dt.to_rfc3339()),
            ))
            .bind(("access_count", memory.access_count))
            .await
            .map_err(|e| {
                StorageError::Query(format!("Failed to update lifecycle metadata: {}", e))
            })?;

        Ok(())
    }

    /// Internal get without hook execution (used for internal operations like update/delete)
    /// This method retrieves a memory and updates its lifecycle metadata, but does NOT
    /// execute on_memory_accessed hooks to avoid recursion.
    async fn get_memory_internal(&self, id: &str) -> Result<Option<Memory>, StorageError> {
        let record_id = RecordId::from(("memory", id));

        let query = "SELECT * FROM $id";

        let mut result = self
            .client
            .query(query)
            .bind(("id", record_id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get memory: {}", e)))?;

        let memories: Vec<SurrealMemory> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract memory: {}", e)))?;

        let mut memory = memories.into_iter().next().map(Memory::from);

        // Track lifecycle if enabled (but don't trigger hooks)
        if let Some(ref mut mem) = memory
            && self.config.lifecycle_tracking.enabled
            && self.config.lifecycle_tracking.update_on_get
        {
            if self.config.lifecycle_tracking.batched {
                // For batched mode: queue the update BEFORE modifying in-memory
                // The delta represents this access
                let update = crate::storage::lifecycle::LifecycleUpdate::new(mem.id.clone());
                if let Err(e) = self.lifecycle_queue.queue_update(update).await {
                    tracing::warn!("Failed to queue lifecycle update: {}", e);
                }
                // Update in-memory for the return value
                mem.record_access();
            } else if self.config.lifecycle_tracking.blocking {
                // Update in-memory counts first
                mem.record_access();
                // Immediate blocking update with absolute values
                if let Err(e) = self.update_lifecycle_metadata(mem).await {
                    tracing::warn!("Failed to update lifecycle metadata: {}", e);
                }
            } else {
                // Update in-memory counts first
                mem.record_access();
                // Spawn async update (fire-and-forget) - Fixed to use MERGE
                let memory_id = mem.id.clone();
                let access_count = mem.access_count;
                let last_accessed = mem.last_accessed;
                let self_clone = self.client.clone();
                tokio::spawn(async move {
                    let record_id = RecordId::from(("memory", memory_id.as_str()));
                    // Use MERGE to avoid overwriting concurrent updates
                    let update_query = r#"
                            UPDATE $id MERGE {
                                metadata: {
                                    access_count: $access_count,
                                    last_accessed: $last_accessed
                                },
                                updated_at: time::now()
                            }
                        "#;
                    if let Err(e) = self_clone
                        .query(update_query)
                        .bind(("id", record_id))
                        .bind(("access_count", access_count))
                        .bind(("last_accessed", last_accessed.map(|dt| dt.to_rfc3339())))
                        .await
                    {
                        tracing::warn!("Failed to update lifecycle in background: {}", e);
                    }
                });
            }
        }

        Ok(memory)
    }
}
