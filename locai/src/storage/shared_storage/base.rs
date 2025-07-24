//! Base shared storage implementation

use async_trait::async_trait;
use surrealdb::{Connection, Surreal};

use crate::storage::errors::StorageError;
use crate::storage::traits::BaseStore;
use super::config::SharedStorageConfig;
use super::intelligence::{SearchIntelligence, IntelligentSearch, QueryAnalysis, IntelligentSearchResult, SearchSuggestion};

/// Main shared storage manager
#[derive(Debug)]
pub struct SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    pub(crate) client: Surreal<C>,
    pub(crate) config: SharedStorageConfig,
    pub(crate) intelligence: SearchIntelligence<C>,
}

impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new shared storage instance
    pub async fn new(client: Surreal<C>, config: SharedStorageConfig) -> Result<Self, StorageError> {
        // Set namespace and database
        client
            .use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to set namespace/database: {}", e)))?;

        // Initialize the intelligence layer
        let intelligence = SearchIntelligence::new(client.clone());

        let storage = Self { 
            client, 
            config,
            intelligence,
        };
        
        // Initialize schema
        storage.initialize_schema().await?;
        
        Ok(storage)
    }

    /// Initialize the database schema with all required tables
    async fn initialize_schema(&self) -> Result<(), StorageError> {
        super::schema::initialize_schema(&self.client).await
    }

    /// Get the underlying client for advanced operations
    pub fn client(&self) -> &Surreal<C> {
        &self.client
    }

    /// Get the intelligence layer for advanced search
    pub fn intelligence(&self) -> &SearchIntelligence<C> {
        &self.intelligence
    }

    /// Ensure the system user exists (needed for owner fields)
    pub(crate) async fn ensure_system_user(&self) -> Result<String, StorageError> {
        // Try to get the system user first
        let existing_user_query = "SELECT id FROM user WHERE username = 'system' LIMIT 1";
        let mut result = self.client
            .query(existing_user_query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to check for system user: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct UserRecord {
            id: surrealdb::RecordId,
        }

        let existing: Option<UserRecord> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to parse user check result: {}", e)))?;

        if let Some(user) = existing {
            return Ok(user.id.key().to_string());
        }

        // Create system user if it doesn't exist
        let create_user_query = r#"
            CREATE user:system SET 
                username = 'system',
                password_hash = 'system_hash',
                email = 'system@locai.local',
                role = 'system',
                created_at = time::now(),
                updated_at = time::now()
        "#;

        let mut result = self.client
            .query(create_user_query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to create system user: {}", e)))?;

        let created: Option<UserRecord> = result.take(0)
            .map_err(|e| StorageError::Query(format!("Failed to parse created user result: {}", e)))?;

        match created {
            Some(user) => Ok(user.id.key().to_string()),
            None => Err(StorageError::Query("Failed to create system user".to_string())),
        }
    }
}

#[async_trait]
impl<C> IntelligentSearch for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Analyze a query for intent and strategy
    async fn analyze_query(&self, query: &str) -> Result<QueryAnalysis, StorageError> {
        self.intelligence.analyze_query(query).await
    }
    
    /// Perform intelligent search with context
    async fn intelligent_search(&self, query: &str, session_id: Option<&str>, limit: Option<usize>) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        // Analyze the query first
        let analysis = self.analyze_query(query).await?;
        
        // Get session context if available
        let session_context = if let Some(session_id) = session_id {
            self.intelligence.get_session_context(session_id)
        } else {
            None
        };
        
        // Perform the search
        self.intelligence.hybrid_search(&analysis, session_context, limit).await
    }
    
    /// Generate search suggestions
    async fn suggest(&self, partial_query: &str, session_id: Option<&str>) -> Result<Vec<SearchSuggestion>, StorageError> {
        let session_context = if let Some(session_id) = session_id {
            self.intelligence.get_session_context(session_id)
        } else {
            None
        };
        
        self.intelligence.generate_suggestions(partial_query, session_context).await
    }
    
    /// Explain search results
    async fn explain(&self, results: &[IntelligentSearchResult]) -> Result<String, StorageError> {
        self.intelligence.explain_results(results).await
    }
}

#[async_trait]
impl<C> BaseStore for SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    async fn health_check(&self) -> Result<bool, StorageError> {
        let _result = self.client
            .query("INFO FOR DB")
            .await
            .map_err(|e| StorageError::Connection(format!("Health check failed: {}", e)))?;

        Ok(true)
    }

    async fn clear(&self) -> Result<(), StorageError> {
        // Clear all data from tables
        let queries = [
            "DELETE FROM memory",
            "DELETE FROM vector", 
            "DELETE FROM entity",
            "DELETE FROM relationship",
            "DELETE FROM message",
        ];

        for query in queries {
            self.client
                .query(query)
                .await
                .map_err(|e| StorageError::Query(format!("Failed to clear table: {}", e)))?;
        }

        Ok(())
    }

    async fn get_metadata(&self) -> Result<serde_json::Value, StorageError> {
        Ok(serde_json::json!({
            "type": "shared_storage",
            "namespace": self.config.namespace,
            "database": self.config.database,
            "engine": "surrealdb_rocksdb",
            "features": {
                "full_text_search": true,
                "fuzzy_matching": true,
                "intelligent_search": true,
                "query_analysis": true,
                "bm25_scoring": true,
                "highlighting": true
            }
        }))
    }

    async fn close(&self) -> Result<(), StorageError> {
        // SurrealDB connections are automatically closed when dropped
        Ok(())
    }
}

// GraphTraversal implementation is provided by graph.rs

// GraphStore implementation is provided by graph.rs

// VersionStore implementation is provided by version.rs 