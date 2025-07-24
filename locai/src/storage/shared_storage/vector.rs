//! Vector storage implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;  
use surrealdb::{Connection, RecordId};

use crate::storage::errors::StorageError;
use crate::storage::traits::VectorStore;
use crate::storage::models::{Vector, VectorSearchParams};
use crate::storage::filters::VectorFilter;
use super::base::SharedStorage;

/// Internal representation of a Vector record for SurrealDB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SurrealVector {
    id: RecordId,
    vector: Vec<f32>,
    dimension: usize,
    metadata: Value,
    source_id: Option<String>,
    #[serde(default = "chrono::Utc::now")]
    created_at: DateTime<Utc>,
}

/// Struct for creating vectors (without generated fields)
#[derive(Debug, Clone, serde::Serialize)]
struct CreateVector {
    vector: Vec<f32>,
    dimension: usize,
    metadata: Value,
    source_id: Option<String>,
}

impl From<Vector> for SurrealVector {
    fn from(vector: Vector) -> Self {
        Self {
            id: RecordId::from(("vector", vector.id.as_str())),
            vector: vector.vector,
            dimension: vector.dimension,
            metadata: vector.metadata,
            source_id: vector.source_id,
            created_at: vector.created_at,
        }
    }
}

impl From<SurrealVector> for Vector {
    fn from(surreal_vector: SurrealVector) -> Self {
        Self {
            id: surreal_vector.id.key().to_string(),
            vector: surreal_vector.vector,
            dimension: surreal_vector.dimension,
            metadata: surreal_vector.metadata,
            source_id: surreal_vector.source_id,
            created_at: surreal_vector.created_at,
        }
    }
}

#[async_trait]
impl<C> VectorStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Add a vector with metadata
    async fn add_vector(&self, vector: Vector) -> Result<Vector, StorageError> {
        // Validate that the vector is 1024 dimensions (BGE-M3 compatibility)
        if vector.dimension != 1024 {
            return Err(StorageError::Validation(format!(
                "Vector dimension {} not supported. Only 1024-dimensional vectors (BGE-M3) are supported.", 
                vector.dimension
            )));
        }
        
        // Create vector data for insertion with current timestamp
        let create_vector = CreateVector {
            vector: vector.vector.clone(),
            dimension: vector.dimension,
            metadata: vector.metadata.clone(),
            source_id: vector.source_id.clone(),
        };
        
        // If the vector has an ID provided, use explicit ID creation
        // Otherwise let SurrealDB auto-generate the ID
        let created: Option<SurrealVector> = if !vector.id.is_empty() {
            self.client
                .create(("vector", vector.id.as_str()))
                .content(create_vector)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to create vector with ID {}: {}", vector.id, e)))?
        } else {
            self.client
                .create("vector")
                .content(create_vector)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to create vector: {}", e)))?
        };
        
        if let Some(surreal_vector) = created {
            // Return the complete vector with metadata
            Ok(Vector::from(surreal_vector))
        } else {
            Err(StorageError::Internal("No vector created".to_string()))
        }
    }
    
    /// Get a vector by its ID
    async fn get_vector(&self, id: &str) -> Result<Option<Vector>, StorageError> {
        // Use SDK method directly for proper deserialization like EntityStore
        let vector: Option<SurrealVector> = self.client
            .select(("vector", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to get vector: {}", e)))?;
        
        Ok(vector.map(Vector::from))
    }
    
    /// Delete a vector by its ID
    async fn delete_vector(&self, id: &str) -> Result<bool, StorageError> {
        let deleted: Option<SurrealVector> = self.client
            .delete(("vector", id))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to delete vector: {}", e)))?;
        
        Ok(deleted.is_some())
    }
    
    /// Update vector metadata
    async fn update_vector_metadata(&self, id: &str, metadata: Value) -> Result<Vector, StorageError> {
        // Update only the metadata field
        let update_data = serde_json::json!({
            "metadata": metadata
        });
        
        let updated: Option<SurrealVector> = self.client
            .update(("vector", id))
            .merge(update_data)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to update vector metadata: {}", e)))?;
        
        updated
            .map(Vector::from)
            .ok_or_else(|| StorageError::NotFound(format!("Vector with id {} not found", id)))
    }
    
    /// Search for similar vectors using SurrealDB's native vector search
    async fn search_vectors(&self, query_vector: &[f32], params: VectorSearchParams) 
        -> Result<Vec<(Vector, f32)>, StorageError> {
        
        let limit = params.limit.unwrap_or(10);
        let threshold = params.threshold;
        
        // Convert query_vector to owned Vec to avoid lifetime issues
        let query_vector_owned: Vec<f32> = query_vector.to_vec();
        
        // Determine distance metric
        let distance_function = match params.distance_metric.unwrap_or_default() {
            crate::storage::models::DistanceMetric::Cosine => "COSINE",
            crate::storage::models::DistanceMetric::Euclidean => "EUCLIDEAN", 
            crate::storage::models::DistanceMetric::DotProduct => "DOT",
            crate::storage::models::DistanceMetric::Manhattan => "MANHATTAN",
        };
        
        // Build the KNN search query using the correct SurrealDB syntax
        // The distance will be computed automatically by SurrealDB when using the KNN operator
        let mut query = format!(
            "SELECT *, vector::distance::knn() AS distance FROM vector WHERE vector <|{},{}|> $query_vector",
            limit, distance_function
        );
        
        // Add threshold filter if specified
        if let Some(thresh) = threshold {
            // For distance metrics, smaller values mean more similar
            // For similarity metrics like cosine, larger values mean more similar
            // We need to handle this appropriately
            match params.distance_metric.unwrap_or_default() {
                crate::storage::models::DistanceMetric::Cosine => {
                    // Cosine similarity: higher values are better, threshold is minimum similarity
                    query = format!("{} AND vector::distance::knn() >= {}", query, thresh);
                }
                _ => {
                    // Distance metrics: lower values are better, threshold is maximum distance
                    query = format!("{} AND vector::distance::knn() <= {}", query, thresh);
                }
            }
        }
        
        // Add additional filters if specified (excluding metadata for now)
        if let Some(filter) = &params.filter {
            if let Some(source_id) = &filter.source_id {
                query = format!("{} AND source_id = '{}'", query, source_id);
            }
            
            if let Some(created_after) = &filter.created_after {
                query = format!("{} AND created_at > d'{}'", query, created_after.to_rfc3339());
            }
            
            if let Some(created_before) = &filter.created_before {
                query = format!("{} AND created_at < d'{}'", query, created_before.to_rfc3339());
            }
            
            // TODO: Implement metadata filtering with proper JOINs in future iteration
            if filter.metadata.is_some() {
                tracing::warn!("Metadata filtering in vector search not yet implemented with separate metadata table");
            }
        }
        
        // Order by distance - this is crucial for consistent results
        // All SurrealDB distance metrics return distances where lower values = more similar
        // So we always use ASC ordering for better matches first
        // Add secondary ordering by ID for deterministic results when distances are equal
        match params.distance_metric.unwrap_or_default() {
            crate::storage::models::DistanceMetric::Cosine => {
                // Cosine distance: lower values are better (0 = identical, 1 = orthogonal)
                query = format!("{} ORDER BY distance ASC, id ASC", query);
            }
            _ => {
                // Other distance metrics: lower values are also better
                query = format!("{} ORDER BY distance ASC, id ASC", query);
            }
        }
        
        // Add explicit LIMIT to ensure we get the expected number of results
        query = format!("{} LIMIT {}", query, limit);
        
        let mut result = self.client
            .query(&query)
            .bind(("query_vector", query_vector_owned))
            .await
            .map_err(|e| StorageError::Query(format!("Failed to search vectors: {}", e)))?;
        
        #[derive(serde::Deserialize, Debug)]
        struct VectorSearchResult {
            id: RecordId,
            vector: Vec<f32>,
            dimension: usize,
            metadata: Value,
            source_id: Option<String>,
            #[serde(default = "chrono::Utc::now")]
            created_at: DateTime<Utc>,
            distance: f32,
        }
        
        let results: Vec<VectorSearchResult> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract search results: {}", e)))?;
        
        // Convert results and fetch metadata for each
        let mut final_results = Vec::new();
        for r in results {
            let vector_id = r.id.key().to_string();
            
            let vector = Vector {
                id: vector_id,
                vector: r.vector,
                dimension: r.dimension,
                metadata: r.metadata,
                source_id: r.source_id,
                created_at: r.created_at,
            };
            final_results.push((vector, r.distance));
        }
        
        Ok(final_results)
    }
    
    /// List vectors with optional filtering
    async fn list_vectors(&self, filter: Option<VectorFilter>, limit: Option<usize>, offset: Option<usize>) 
        -> Result<Vec<Vector>, StorageError> {
        
        // If no filters, use simple SDK select like EntityStore
        if filter.is_none() && limit.is_none() && offset.is_none() {
            let vectors: Vec<SurrealVector> = self.client
                .select("vector")
                .await
                .map_err(|e| StorageError::Query(format!("Failed to list vectors: {}", e)))?;
            
            return Ok(vectors.into_iter().map(Vector::from).collect());
        }
        
        // For complex filtering, use raw queries
        let mut query = "SELECT * FROM vector".to_string();
        let mut conditions = Vec::new();
        
        // Add filter conditions
        if let Some(f) = &filter {
            if let Some(ids) = &f.ids {
                if !ids.is_empty() {
                    let id_list = ids.iter()
                        .map(|id| format!("vector:{}", id))
                        .collect::<Vec<_>>()
                        .join(", ");
                    conditions.push(format!("id IN [{}]", id_list));
                }
            }
            
            if let Some(source_id) = &f.source_id {
                conditions.push(format!("source_id = '{}'", source_id));
            }
            
            if let Some(created_after) = &f.created_after {
                conditions.push(format!("created_at > d'{}'", created_after.to_rfc3339()));
            }
            
            if let Some(created_before) = &f.created_before {
                conditions.push(format!("created_at < d'{}'", created_before.to_rfc3339()));
            }
            
            // Handle metadata filtering directly on the table
            if let Some(metadata) = &f.metadata {
                for (key, value) in metadata {
                    match value {
                        Value::String(s) => {
                            conditions.push(format!("metadata.{} = '{}'", key, s));
                        }
                        Value::Number(n) => {
                            conditions.push(format!("metadata.{} = {}", key, n));
                        }
                        Value::Bool(b) => {
                            conditions.push(format!("metadata.{} = {}", key, b));
                        }
                        _ => {
                            conditions.push(format!("metadata.{} = {}", key, value));
                        }
                    }
                }
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
        
        let mut result = self.client
            .query(&query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to list vectors: {}", e)))?;
        
        let vectors: Vec<SurrealVector> = result
            .take(0)
            .map_err(|e| StorageError::Query(format!("Failed to extract vectors: {}", e)))?;
        
        Ok(vectors.into_iter().map(Vector::from).collect())
    }
    
    /// Count vectors with optional filtering
    async fn count_vectors(&self, filter: Option<VectorFilter>) -> Result<usize, StorageError> {
        // Simple approach: get all vectors matching the filter and count them
        let vectors = self.list_vectors(filter, None, None).await?;
        Ok(vectors.len())
    }
    
    /// Batch add multiple vectors
    async fn batch_add_vectors(&self, vectors: Vec<Vector>) -> Result<Vec<Vector>, StorageError> {
        // Validate all vectors are 1024 dimensions before processing any
        for (i, vector) in vectors.iter().enumerate() {
            if vector.dimension != 1024 {
                return Err(StorageError::Validation(format!(
                    "Vector {} has dimension {} but only 1024-dimensional vectors (BGE-M3) are supported.", 
                    i, vector.dimension
                )));
            }
        }
        
        let mut results = Vec::new();
        
        // For now, add vectors one by one. Could be optimized with a bulk insert query
        for vector in vectors {
            let result = self.add_vector(vector).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Add or update a vector (Upsert)
    async fn upsert_vector(&self, vector: Vector) -> Result<(), StorageError> {
        // Validate that the vector is 1024 dimensions (BGE-M3 compatibility)
        if vector.dimension != 1024 {
            return Err(StorageError::Validation(format!(
                "Vector dimension {} not supported. Only 1024-dimensional vectors (BGE-M3) are supported.", 
                vector.dimension
            )));
        }
        
        // For upsert, we need to include the created_at field to avoid NONE values
        let upsert_data = serde_json::json!({
            "vector": vector.vector,
            "dimension": vector.dimension,
            "metadata": vector.metadata,
            "source_id": vector.source_id,
            "created_at": vector.created_at
        });
        
        let _result: Option<SurrealVector> = self.client
            .upsert(("vector", vector.id.as_str()))
            .content(upsert_data)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to upsert vector: {}", e)))?;
        
        Ok(())
    }
} 