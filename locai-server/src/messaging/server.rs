//! Messaging server implementation

use super::{MessagingStorage, Result};
use crate::config::MessagingConfig;
use locai::messaging::types::{Message, MessageFilter, MessageId};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Information about a connected application
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub app_id: String,
    #[allow(dead_code)]
    pub connection_id: String,
    pub authenticated: bool,
    #[allow(dead_code)]
    pub permissions: Vec<String>,
}

/// Subscription information
#[derive(Debug)]
pub struct SubscriptionInfo {
    #[allow(dead_code)]
    pub subscription_id: String,
    pub app_id: String,
    #[allow(dead_code)]
    pub filter: MessageFilter,
    #[allow(dead_code)]
    pub sender: broadcast::Sender<Message>,
}

/// Main messaging server
#[derive(Debug)]
pub struct MessagingServer {
    storage: MessagingStorage,
    config: MessagingConfig,

    // Connection management
    connections: Arc<RwLock<HashMap<String, AppInfo>>>,
    app_connections: Arc<RwLock<HashMap<String, String>>>, // app_id -> connection_id

    // Subscription management
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionInfo>>>,

    // Global message broadcast
    global_broadcast: broadcast::Sender<Message>,
}

impl MessagingServer {
    /// Create a new messaging server using shared storage from MemoryManager
    pub fn new_with_shared_storage(
        config: MessagingConfig,
        shared_storage: &Arc<dyn locai::storage::traits::GraphStore>,
    ) -> Self {
        let storage = MessagingStorage::new_shared(shared_storage.clone());
        let (global_broadcast, _) = broadcast::channel(1000);

        info!("Messaging server initialized with shared storage");

        Self {
            storage,
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            app_connections: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            global_broadcast,
        }
    }

    /// Register a new connection
    pub async fn register_connection(&self, connection_id: String, app_id: String) -> Result<()> {
        let app_info = AppInfo {
            app_id: app_id.clone(),
            connection_id: connection_id.clone(),
            authenticated: !self.config.auth_required, // Auto-authenticate if auth not required
            permissions: vec!["read".to_string(), "write".to_string()], // Default permissions
        };

        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), app_info);
        }

        {
            let mut app_connections = self.app_connections.write().await;
            app_connections.insert(app_id.clone(), connection_id.clone());
        }

        info!("Registered connection {} for app {}", connection_id, app_id);
        Ok(())
    }

    /// Remove a connection
    pub async fn remove_connection(&self, connection_id: &str) -> Result<()> {
        let app_id = {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id).map(|info| info.app_id)
        };

        if let Some(app_id) = app_id {
            let mut app_connections = self.app_connections.write().await;
            app_connections.remove(&app_id);

            // Remove all subscriptions for this connection
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.retain(|_, sub| sub.app_id != app_id);

            info!("Removed connection {} for app {}", connection_id, app_id);
        }

        Ok(())
    }

    /// Authenticate a connection
    pub async fn authenticate_connection(&self, connection_id: &str, app_id: &str) -> Result<bool> {
        let mut connections = self.connections.write().await;

        if let Some(app_info) = connections.get_mut(connection_id) {
            if app_info.app_id == app_id {
                app_info.authenticated = true;
                info!(
                    "Authenticated connection {} for app {}",
                    connection_id, app_id
                );
                return Ok(true);
            }
        }

        warn!(
            "Failed to authenticate connection {} for app {}",
            connection_id, app_id
        );
        Ok(false)
    }

    /// Send a message
    pub async fn send_message(
        &self,
        sender_app: &str,
        topic: &str,
        content: serde_json::Value,
        headers: Option<HashMap<String, String>>,
    ) -> Result<MessageId> {
        // Create message
        let mut message = Message::new(topic.to_string(), sender_app.to_string(), content);

        // Add headers if provided
        if let Some(headers) = headers {
            for (key, value) in headers {
                message = message.add_header(key, value);
            }
        }

        let message_id = message.id.clone();

        // Store message
        self.storage.store_message(&message).await?;

        // Broadcast to subscribers
        if let Err(e) = self.global_broadcast.send(message) {
            debug!("No active subscribers for message broadcast: {}", e);
        }

        debug!(
            "Sent message {} from app {} to topic {}",
            message_id.as_str(),
            sender_app,
            topic
        );
        Ok(message_id)
    }

    /// Subscribe to messages
    pub async fn subscribe(
        &self,
        app_id: &str,
        filter: MessageFilter,
    ) -> Result<(String, broadcast::Receiver<Message>)> {
        let subscription_id = Uuid::new_v4().to_string();
        let (sender, receiver) = broadcast::channel(100);

        let subscription_info = SubscriptionInfo {
            subscription_id: subscription_id.clone(),
            app_id: app_id.to_string(),
            filter: filter.clone(),
            sender: sender.clone(),
        };

        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(subscription_id.clone(), subscription_info);
        }

        // Subscribe to global broadcast and filter messages
        let global_receiver = self.global_broadcast.subscribe();
        let filter_clone = filter.clone();
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let mut global_rx = global_receiver;
            while let Ok(message) = global_rx.recv().await {
                if Self::message_matches_filter(&message, &filter_clone) {
                    if let Err(e) = sender_clone.send(message) {
                        debug!("Failed to forward message to subscription: {}", e);
                        break;
                    }
                }
            }
        });

        info!(
            "Created subscription {} for app {} with filter: {:?}",
            subscription_id, app_id, filter
        );
        Ok((subscription_id, receiver))
    }

    /// Unsubscribe from messages
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions.remove(subscription_id).is_some() {
            info!("Removed subscription {}", subscription_id);
        }
        Ok(())
    }

    /// Get message history
    pub async fn get_message_history(
        &self,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        self.storage.get_message_history(filter, limit).await
    }

    /// Check if a message matches a filter
    fn message_matches_filter(message: &Message, filter: &MessageFilter) -> bool {
        // Check topic patterns
        if let Some(patterns) = &filter.topic_patterns {
            if !patterns
                .iter()
                .any(|pattern| Self::topic_matches_pattern(&message.topic, pattern))
            {
                return false;
            }
        }

        // Check exact topics
        if let Some(topics) = &filter.topics {
            if !topics.contains(&message.topic) {
                return false;
            }
        }

        // Check senders
        if let Some(senders) = &filter.senders {
            if !senders.contains(&message.sender) {
                return false;
            }
        }

        // Check source app (for cross-app messaging)
        if let Some(source_app) = &filter.source_app {
            if &message.sender != source_app {
                return false;
            }
        }

        // Check recipients
        if let Some(recipients) = &filter.recipients {
            if message.recipients.is_empty() {
                // Broadcast message - matches any recipient filter
            } else if !recipients.iter().any(|r| message.recipients.contains(r)) {
                return false;
            }
        }

        // Check time range
        if let Some((start, end)) = &filter.time_range {
            if message.timestamp < *start || message.timestamp > *end {
                return false;
            }
        }

        // Check importance range
        if let Some((min, max)) = &filter.importance_range {
            if let Some(importance) = message.importance {
                if importance < *min || importance > *max {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check tags (must have all)
        if let Some(tags) = &filter.tags {
            if !tags.iter().all(|tag| message.has_tag(tag)) {
                return false;
            }
        }

        // Check tags (must have any)
        if let Some(tags_any) = &filter.tags_any {
            if !tags_any.iter().any(|tag| message.has_tag(tag)) {
                return false;
            }
        }

        // Check headers
        if let Some(headers) = &filter.headers {
            for (key, value) in headers {
                if message.get_header(key) != Some(value) {
                    return false;
                }
            }
        }

        // Check expiration
        if !filter.include_expired && message.is_expired() {
            return false;
        }

        true
    }

    /// Check if a topic matches a pattern (supports wildcards)
    fn topic_matches_pattern(topic: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            if let Some(prefix) = pattern.strip_suffix('*') {
                return topic.starts_with(prefix);
            } else if let Some(suffix) = pattern.strip_prefix('*') {
                return topic.ends_with(suffix);
            }
            // For more complex patterns, we'd need proper regex
            return pattern == topic;
        }

        pattern == topic
    }

    /// Get server statistics
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let connections = self.connections.read().await;
        let subscriptions = self.subscriptions.read().await;

        let mut stats = HashMap::new();
        stats.insert(
            "active_connections".to_string(),
            serde_json::Value::Number(connections.len().into()),
        );
        stats.insert(
            "active_subscriptions".to_string(),
            serde_json::Value::Number(subscriptions.len().into()),
        );
        stats.insert(
            "messaging_enabled".to_string(),
            serde_json::Value::Bool(self.config.enabled),
        );
        stats.insert(
            "cross_app_enabled".to_string(),
            serde_json::Value::Bool(self.config.enable_cross_app),
        );

        stats
    }
}
