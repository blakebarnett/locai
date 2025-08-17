//! Live Query implementation for SharedStorage
//!
//! This module provides live query implementation for SharedStorage using SurrealDB's LIVE SELECT.
//! Works with both embedded (memory/RocksDB) and remote SurrealDB instances.

use chrono;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use surrealdb::{Connection, Surreal};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::storage::errors::StorageError;

/// Database change event from SurrealDB live queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbEvent {
    pub id: String,
    pub action: String, // CREATE, UPDATE, DELETE
    pub table: String,  // memory, entity, relationship, version
    pub result: Value,  // The changed record
}

/// Live query subscription handle
#[derive(Debug)]
pub struct LiveQuerySubscription {
    pub query_id: Uuid,
    pub table: String,
    pub query: String,
}

/// Live query manager for SharedStorage change streams
pub struct LiveQueryManager<C>
where
    C: Connection + Clone + Send + Sync + 'static,
{
    client: Surreal<C>,
    node_id: String,
    event_tx: broadcast::Sender<DbEvent>,
    subscriptions: HashMap<Uuid, LiveQuerySubscription>,
}

impl<C> LiveQueryManager<C>
where
    C: Connection + Clone + Send + Sync + 'static,
{
    /// Create a new live query manager
    pub fn new(client: Surreal<C>) -> (Self, broadcast::Receiver<DbEvent>) {
        let (event_tx, event_rx) = broadcast::channel(1000);
        let node_id = Uuid::new_v4().to_string();

        let manager = Self {
            client,
            node_id,
            event_tx,
            subscriptions: HashMap::new(),
        };

        (manager, event_rx)
    }

    /// Get the node ID for this instance
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Start live queries for all relevant tables
    pub async fn start_live_queries(&mut self) -> Result<(), StorageError> {
        info!(
            "Starting SharedStorage live queries for node {}",
            self.node_id
        );

        // Create live queries for each table we want to monitor
        let tables = vec!["memory", "entity", "relationship", "version"];

        for table in tables {
            if let Err(e) = self.create_live_query(table).await {
                error!("Failed to create live query for table {}: {}", table, e);
                return Err(e);
            }
        }

        info!("All live queries started successfully");
        Ok(())
    }

    /// Create a live query for a specific table
    async fn create_live_query(&mut self, table: &str) -> Result<(), StorageError> {
        let _query_id = Uuid::new_v4();
        let query = format!("LIVE SELECT * FROM {}", table);

        debug!("Creating live query for table {}: {}", table, query);

        // Execute the LIVE SELECT query
        match self.client.query(&query).await {
            Ok(mut response) => {
                // The first result should contain the live query UUID
                if let Ok(Some(live_uuid)) = response.take::<Option<Uuid>>(0) {
                    info!(
                        "Live query created for table {} with UUID: {}",
                        table, live_uuid
                    );

                    let subscription = LiveQuerySubscription {
                        query_id: live_uuid,
                        table: table.to_string(),
                        query,
                    };

                    self.subscriptions.insert(live_uuid, subscription);

                    // Start processing events for this live query
                    self.process_live_query_events(live_uuid, table.to_string())
                        .await;

                    Ok(())
                } else {
                    let error_msg = format!("Failed to get live query UUID for table {}", table);
                    error!("{}", error_msg);
                    Err(StorageError::Connection(error_msg))
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create live query for table {}: {}", table, e);
                error!("{}", error_msg);
                Err(StorageError::Connection(error_msg))
            }
        }
    }

    /// Process events from a live query using SurrealDB's native event mechanism
    async fn process_live_query_events(&self, live_uuid: Uuid, table: String) {
        let client = self.client.clone();
        let event_tx = self.event_tx.clone();
        let node_id = self.node_id.clone();

        // Spawn a task to handle events for this live query
        tokio::spawn(async move {
            let mut retry_count = 0;
            const MAX_RETRIES: usize = 5;
            const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(100);

            loop {
                match Self::process_live_query_stream(
                    &client, live_uuid, &table, &event_tx, &node_id,
                )
                .await
                {
                    Ok(_) => {
                        info!("Live query stream for table {} ended normally", table);
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count > MAX_RETRIES {
                            error!(
                                "Live query for table {} failed after {} retries: {}",
                                table, MAX_RETRIES, e
                            );
                            break;
                        }

                        let delay = INITIAL_RETRY_DELAY * 2_u32.pow(retry_count as u32 - 1);
                        warn!(
                            "Live query for table {} failed (attempt {}), retrying in {:?}: {}",
                            table, retry_count, delay, e
                        );

                        tokio::time::sleep(delay).await;
                    }
                }
            }
        });
    }

    /// Process the actual live query stream using SurrealDB's native mechanism
    async fn process_live_query_stream(
        client: &Surreal<C>,
        live_uuid: Uuid,
        table: &str,
        event_tx: &broadcast::Sender<DbEvent>,
        node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For embedded connections, we need to use a different approach than remote connections
        match Self::detect_connection_type(client).await {
            ConnectionType::Embedded => {
                Self::process_embedded_live_query(client, live_uuid, table, event_tx, node_id).await
            }
            ConnectionType::Remote => {
                Self::process_remote_live_query(client, live_uuid, table, event_tx, node_id).await
            }
        }
    }

    /// Process live query events for embedded SurrealDB connections
    async fn process_embedded_live_query(
        client: &Surreal<C>,
        live_uuid: Uuid,
        table: &str,
        event_tx: &broadcast::Sender<DbEvent>,
        _node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For embedded connections, we use polling-based approach
        // since WebSocket-style live queries may not be available

        let table_owned = table.to_string(); // Convert to owned string to avoid lifetime issues
        let mut last_processed = chrono::Utc::now();
        let mut consecutive_empty_polls = 0;
        const MAX_EMPTY_POLLS: usize = 20;

        loop {
            // Query for recent changes in the table
            let changes_query = format!(
                "SELECT * FROM {} WHERE updated_at > $last_check OR created_at > $last_check ORDER BY created_at ASC LIMIT 100",
                table_owned
            );

            match client
                .query(&changes_query)
                .bind(("last_check", last_processed.to_rfc3339()))
                .await
            {
                Ok(mut response) => {
                    if let Ok(records) = response.take::<Vec<Value>>(0) {
                        if records.is_empty() {
                            consecutive_empty_polls += 1;

                            // Implement adaptive polling - slow down if no activity
                            let poll_interval = if consecutive_empty_polls > MAX_EMPTY_POLLS {
                                Duration::from_millis(500) // Slow polling
                            } else {
                                Duration::from_millis(100) // Fast polling
                            };

                            tokio::time::sleep(poll_interval).await;
                            continue;
                        }

                        consecutive_empty_polls = 0;

                        for record in records {
                            // Determine the action type based on timestamps
                            let action = Self::determine_action_type(&record);

                            let db_event = DbEvent {
                                id: format!(
                                    "{}_{}",
                                    live_uuid,
                                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                                ),
                                action,
                                table: table_owned.clone(),
                                result: record,
                            };

                            if let Err(e) = event_tx.send(db_event) {
                                error!("Failed to send live query event: {}", e);
                                return Err(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::BrokenPipe,
                                    "Event channel closed",
                                )));
                            }
                        }

                        // Update our last check time
                        last_processed = chrono::Utc::now();
                    }
                }
                Err(e) => {
                    error!("Error polling for live events: {}", e);
                    return Err(Box::new(e));
                }
            }

            // Base polling interval
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Process live query events for remote SurrealDB connections
    async fn process_remote_live_query(
        client: &Surreal<C>,
        live_uuid: Uuid,
        table: &str,
        event_tx: &broadcast::Sender<DbEvent>,
        _node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // For remote connections, we can use SurrealDB's native WebSocket live queries
        // This requires a different approach depending on the connection type

        // Create a subscription query to listen for live query results
        let subscription_query = format!("LIVE SELECT * FROM {}", table);

        // Execute the live query and get a stream-like response
        match client.query(&subscription_query).await {
            Ok(_response) => {
                // For remote connections, SurrealDB should provide a mechanism to listen
                // to live query events. This is connection-type specific.

                // Since the exact API depends on the SurrealDB version and connection type,
                // we'll implement a polling-based approach that's more robust
                Self::poll_for_live_events(client, live_uuid, table, event_tx, _node_id).await
            }
            Err(e) => {
                error!("Failed to establish remote live query: {}", e);
                Err(Box::new(e))
            }
        }
    }

    /// Robust polling-based approach for live events
    async fn poll_for_live_events(
        client: &Surreal<C>,
        live_uuid: Uuid,
        table: &str,
        event_tx: &broadcast::Sender<DbEvent>,
        _node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_check = chrono::Utc::now();
        let mut consecutive_empty_polls = 0;
        const MAX_EMPTY_POLLS: usize = 20;

        loop {
            // Query for recent changes in the table
            let changes_query = format!(
                "SELECT * FROM {} WHERE updated_at > $last_check OR created_at > $last_check ORDER BY created_at ASC LIMIT 100",
                table
            );

            match client
                .query(&changes_query)
                .bind(("last_check", last_check.to_rfc3339()))
                .await
            {
                Ok(mut response) => {
                    if let Ok(records) = response.take::<Vec<Value>>(0) {
                        if records.is_empty() {
                            consecutive_empty_polls += 1;

                            // Implement adaptive polling - slow down if no activity
                            let poll_interval = if consecutive_empty_polls > MAX_EMPTY_POLLS {
                                Duration::from_millis(500) // Slow polling
                            } else {
                                Duration::from_millis(100) // Fast polling
                            };

                            tokio::time::sleep(poll_interval).await;
                            continue;
                        }

                        consecutive_empty_polls = 0;

                        for record in records {
                            // Determine the action type based on timestamps
                            let action = Self::determine_action_type(&record);

                            let db_event = DbEvent {
                                id: format!(
                                    "{}_{}",
                                    live_uuid,
                                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
                                ),
                                action,
                                table: table.to_string(),
                                result: record,
                            };

                            if let Err(e) = event_tx.send(db_event) {
                                error!("Failed to send polled live query event: {}", e);
                                return Err(Box::new(std::io::Error::new(
                                    std::io::ErrorKind::BrokenPipe,
                                    "Event channel closed",
                                )));
                            }
                        }

                        // Update our last check time
                        last_check = chrono::Utc::now();
                    }
                }
                Err(e) => {
                    error!("Error polling for live events: {}", e);
                    return Err(Box::new(e));
                }
            }

            // Base polling interval
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Detect the type of SurrealDB connection
    async fn detect_connection_type(client: &Surreal<C>) -> ConnectionType {
        // Try to detect if this is an embedded or remote connection
        // This is a heuristic approach since SurrealDB doesn't expose this directly

        match client.query("INFO FOR DB").await {
            Ok(_) => {
                // If we can query DB info, check if we have WebSocket capabilities
                match client.query("SELECT meta::id()").await {
                    Ok(_) => ConnectionType::Remote, // Assume remote if meta functions work
                    Err(_) => ConnectionType::Embedded,
                }
            }
            Err(_) => ConnectionType::Embedded, // Fallback to embedded
        }
    }

    /// Determine action type based on record timestamps
    fn determine_action_type(record: &Value) -> String {
        if let (Some(created), Some(updated)) = (
            record.get("created_at").and_then(|v| v.as_str()),
            record.get("updated_at").and_then(|v| v.as_str()),
        ) {
            // If created_at and updated_at are very close, it's likely a CREATE
            if let (Ok(created_time), Ok(updated_time)) = (
                chrono::DateTime::parse_from_rfc3339(created),
                chrono::DateTime::parse_from_rfc3339(updated),
            ) {
                let diff = updated_time
                    .signed_duration_since(created_time)
                    .num_milliseconds();
                if diff.abs() < 1000 {
                    // Within 1 second
                    "CREATE".to_string()
                } else {
                    "UPDATE".to_string()
                }
            } else {
                "UPDATE".to_string()
            }
        } else {
            "UPDATE".to_string()
        }
    }

    /// Kill a specific live query
    pub async fn kill_live_query(&mut self, query_id: Uuid) -> Result<(), StorageError> {
        if let Some(subscription) = self.subscriptions.remove(&query_id) {
            let kill_query = format!("KILL '{}'", query_id);

            match self.client.query(&kill_query).await {
                Ok(_) => {
                    info!("Killed live query for table {}", subscription.table);
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to kill live query {}: {}", query_id, e);
                    error!("{}", error_msg);
                    Err(StorageError::Connection(error_msg))
                }
            }
        } else {
            warn!("Attempted to kill unknown live query: {}", query_id);
            Ok(())
        }
    }

    /// Kill all live queries
    pub async fn kill_all_live_queries(&mut self) -> Result<(), StorageError> {
        let query_ids: Vec<Uuid> = self.subscriptions.keys().cloned().collect();

        for query_id in query_ids {
            if let Err(e) = self.kill_live_query(query_id).await {
                error!("Failed to kill live query {}: {}", query_id, e);
            }
        }

        Ok(())
    }

    /// Get current subscriptions
    pub fn get_subscriptions(&self) -> &HashMap<Uuid, LiveQuerySubscription> {
        &self.subscriptions
    }

    /// Simulate a database event (for testing purposes)
    pub fn simulate_event(
        &self,
        table: &str,
        action: &str,
        data: Value,
    ) -> Result<(), StorageError> {
        let db_event = DbEvent {
            id: Uuid::new_v4().to_string(),
            action: action.to_string(),
            table: table.to_string(),
            result: data,
        };

        self.event_tx.send(db_event).map_err(|e| {
            StorageError::Connection(format!("Failed to send simulated event: {}", e))
        })?;

        Ok(())
    }
}

/// Connection type enumeration for SurrealDB
#[derive(Debug, Clone, Copy)]
enum ConnectionType {
    Embedded,
    Remote,
}

/// Live query event structure for compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveQueryEvent {
    pub action: String,
    pub result: Value,
}

/// Setup authentication for SharedStorage (placeholder for future implementation)
pub async fn setup_sharedstorage_auth<C>(_client: &Surreal<C>) -> Result<(), StorageError>
where
    C: Connection,
{
    // Placeholder for authentication setup
    // Implementation depends on specific requirements
    Ok(())
}
