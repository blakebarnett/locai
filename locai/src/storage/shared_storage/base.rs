//! Base shared storage implementation

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use surrealdb::{Connection, RecordId, Surreal};
use tokio::sync::Notify;

use super::config::SharedStorageConfig;
use super::intelligence::{
    IntelligentSearch, IntelligentSearchResult, QueryAnalysis, SearchIntelligence, SearchSuggestion,
};
use super::version_access::VersionAccessTracker;
use super::version_cache::VersionCache;
use crate::hooks::HookRegistry;
use crate::storage::errors::StorageError;
use crate::storage::lifecycle::{LifecycleUpdate, LifecycleUpdateQueue};
use crate::storage::traits::BaseStore;

/// Main shared storage manager
#[derive(Debug)]
pub struct SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    pub(crate) client: Surreal<C>,
    pub(crate) config: SharedStorageConfig,
    pub(crate) intelligence: SearchIntelligence<C>,
    pub(crate) lifecycle_queue: LifecycleUpdateQueue,
    pub(crate) hook_registry: Arc<HookRegistry>,
    pub(crate) shutdown: Arc<Notify>,
    pub(crate) version_cache: VersionCache,
    pub(crate) version_access_tracker: VersionAccessTracker,
}

impl<C> SharedStorage<C>
where
    C: Connection + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new shared storage instance
    pub async fn new(
        client: Surreal<C>,
        config: SharedStorageConfig,
    ) -> Result<Self, StorageError> {
        // Set namespace and database
        client
            .use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .map_err(|e| {
                StorageError::Connection(format!("Failed to set namespace/database: {}", e))
            })?;

        // Initialize the intelligence layer
        let intelligence = SearchIntelligence::new(client.clone());

        let shutdown = Arc::new(Notify::new());
        let lifecycle_queue = LifecycleUpdateQueue::new(1000);

        // Initialize versioning cache and access tracker
        let version_cache = VersionCache::new(&config.versioning);
        let version_access_tracker = VersionAccessTracker::new();

        let storage = Self {
            client: client.clone(),
            config: config.clone(),
            intelligence,
            lifecycle_queue: lifecycle_queue.clone(),
            hook_registry: Arc::new(HookRegistry::new()),
            shutdown: shutdown.clone(),
            version_cache,
            version_access_tracker,
        };

        // Initialize schema
        storage.initialize_schema().await?;

        // Start background flush task if lifecycle tracking is enabled and batched
        if config.lifecycle_tracking.enabled && config.lifecycle_tracking.batched {
            let flush_interval = Duration::from_secs(config.lifecycle_tracking.flush_interval_secs);
            let flush_threshold = config.lifecycle_tracking.flush_threshold_count;
            let queue_clone = lifecycle_queue.clone();
            let client_clone = client.clone();
            let shutdown_clone = shutdown.clone();

            tokio::spawn(async move {
                tracing::info!(
                    "Lifecycle flush task started (interval: {:?}, threshold: {})",
                    flush_interval,
                    flush_threshold
                );

                let mut interval = tokio::time::interval(flush_interval);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            // Time-based flush
                            if queue_clone.len().await > 0 {
                                let updates = queue_clone.drain().await;
                                if !updates.is_empty() {
                                    tracing::debug!("Flushing {} lifecycle updates (time-based)", updates.len());
                                    if let Err(e) = Self::flush_lifecycle_updates(&client_clone, updates).await {
                                        tracing::error!("Failed to flush lifecycle updates: {}", e);
                                    }
                                }
                            }
                        }
                        _ = shutdown_clone.notified() => {
                            // Shutdown requested - flush remaining updates
                            tracing::info!("Lifecycle flush task shutting down, flushing remaining updates");
                            let updates = queue_clone.drain().await;
                            if !updates.is_empty() {
                                tracing::info!("Flushing {} lifecycle updates on shutdown", updates.len());
                                if let Err(e) = Self::flush_lifecycle_updates(&client_clone, updates).await {
                                    tracing::error!("Failed to flush lifecycle updates on shutdown: {}", e);
                                }
                            }
                            break;
                        }
                    }

                    // Also check threshold-based flush
                    if queue_clone.len().await >= flush_threshold {
                        let updates = queue_clone.drain().await;
                        if !updates.is_empty() {
                            tracing::debug!(
                                "Flushing {} lifecycle updates (threshold-based)",
                                updates.len()
                            );
                            if let Err(e) =
                                Self::flush_lifecycle_updates(&client_clone, updates).await
                            {
                                tracing::error!("Failed to flush lifecycle updates: {}", e);
                            }
                        }
                    }
                }

                tracing::info!("Lifecycle flush task stopped");
            });
        }

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

    /// Get the hook registry for registering memory lifecycle hooks
    pub fn hook_registry(&self) -> Arc<HookRegistry> {
        self.hook_registry.clone()
    }

    /// Gracefully shutdown the storage, flushing any pending updates
    pub async fn shutdown(&self) -> Result<(), StorageError> {
        tracing::info!("Initiating graceful shutdown");

        // Signal shutdown to background tasks
        self.shutdown.notify_waiters();

        // Give background tasks a moment to finish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Flush any remaining lifecycle updates
        if self.config.lifecycle_tracking.enabled && self.config.lifecycle_tracking.batched {
            let updates = self.lifecycle_queue.drain().await;
            if !updates.is_empty() {
                tracing::info!("Final flush of {} lifecycle updates", updates.len());
                Self::flush_lifecycle_updates(&self.client, updates).await?;
            }
        }

        tracing::info!("Graceful shutdown complete");
        Ok(())
    }

    /// Flush a batch of lifecycle updates to the database
    /// Uses atomic increment operations to avoid race conditions
    async fn flush_lifecycle_updates(
        client: &Surreal<C>,
        updates: Vec<LifecycleUpdate>,
    ) -> Result<(), StorageError> {
        if updates.is_empty() {
            return Ok(());
        }

        // Batch updates in a single transaction-like query
        // For each update, we use += operator to atomically increment
        for update in updates {
            let record_id = RecordId::from(("memory", update.memory_id.as_str()));

            let query = r#"
                UPDATE $id SET 
                    metadata.access_count += $delta,
                    metadata.last_accessed = $last_accessed,
                    updated_at = time::now()
                WHERE id = $id
            "#;

            if let Err(e) = client
                .query(query)
                .bind(("id", record_id))
                .bind(("delta", update.access_count_delta))
                .bind(("last_accessed", update.last_accessed.to_rfc3339()))
                .await
            {
                tracing::warn!(
                    "Failed to flush lifecycle update for {}: {}",
                    update.memory_id,
                    e
                );
                // Continue with other updates even if one fails
            }
        }

        Ok(())
    }

    /// Ensure the system user exists (needed for owner fields)
    pub(crate) async fn ensure_system_user(&self) -> Result<String, StorageError> {
        // Try to get the system user first
        let existing_user_query = "SELECT id FROM user WHERE username = 'system' LIMIT 1";
        let mut result =
            self.client.query(existing_user_query).await.map_err(|e| {
                StorageError::Query(format!("Failed to check for system user: {}", e))
            })?;

        #[derive(serde::Deserialize)]
        struct UserRecord {
            id: surrealdb::RecordId,
        }

        let existing: Option<UserRecord> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to parse user check result: {}", e))
        })?;

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

        let mut result = self
            .client
            .query(create_user_query)
            .await
            .map_err(|e| StorageError::Query(format!("Failed to create system user: {}", e)))?;

        let created: Option<UserRecord> = result.take(0).map_err(|e| {
            StorageError::Query(format!("Failed to parse created user result: {}", e))
        })?;

        match created {
            Some(user) => Ok(user.id.key().to_string()),
            None => Err(StorageError::Query(
                "Failed to create system user".to_string(),
            )),
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
    async fn intelligent_search(
        &self,
        query: &str,
        session_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<IntelligentSearchResult>, StorageError> {
        // Analyze the query first
        let analysis = self.analyze_query(query).await?;

        // Get session context if available
        let session_context = if let Some(session_id) = session_id {
            self.intelligence.get_session_context(session_id)
        } else {
            None
        };

        // Perform the search
        self.intelligence
            .hybrid_search(&analysis, session_context, limit)
            .await
    }

    /// Generate search suggestions
    async fn suggest(
        &self,
        partial_query: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<SearchSuggestion>, StorageError> {
        let session_context = if let Some(session_id) = session_id {
            self.intelligence.get_session_context(session_id)
        } else {
            None
        };

        self.intelligence
            .generate_suggestions(partial_query, session_context)
            .await
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
        let _result = self
            .client
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
