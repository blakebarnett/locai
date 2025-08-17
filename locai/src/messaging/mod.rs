//! Locai messaging system
//!
//! This module provides both embedded and remote messaging capabilities:
//! - Embedded: Using SurrealDB live queries for intra-process communication
//! - Remote: Using WebSocket connections to locai-server for inter-process communication
//!
//! ## Embedded Mode (Single Process)
//! Enables communication between multiple agents/components within a single process
//! using shared `Arc<MemoryManager>` instances with ultra-low latency.
//!
//! ## Remote Mode (Inter-Process)
//! Enables true inter-process communication via WebSocket connections to locai-server,
//! supporting distributed deployments and cross-application messaging.

pub mod embedded;
pub mod filters;
pub mod remote;
pub mod stream;
pub mod types;
pub mod websocket;

pub use embedded::EmbeddedMessaging;
pub use filters::TopicMatcher;
pub use remote::RemoteMessaging;
pub use stream::MessageStream;
pub use types::{Message, MessageBuilder, MessageFilter, MessageId};
pub use websocket::WebSocketClient;

use crate::core::MemoryManager;
use crate::{LocaiError, Result};
use std::sync::Arc;

/// Messaging mode configuration
#[derive(Debug, Clone)]
pub enum MessagingMode {
    /// Embedded messaging with shared MemoryManager instance
    /// This enables unified relationship graphs across processes
    Embedded { memory_manager: Arc<MemoryManager> },
    /// Remote messaging via locai-server WebSocket connection
    /// This enables true inter-process communication
    Remote {
        server_url: String,
        websocket_client: Arc<WebSocketClient>,
        app_id: String,
    },
}

/// Main messaging interface for Locai
///
/// Provides both embedded and remote messaging capabilities with topic-based routing,
/// real-time subscriptions, and message filtering. Supports seamless scaling from
/// single-process to distributed architectures.
#[derive(Debug)]
pub struct LocaiMessaging {
    mode: MessagingMode,
    app_id: String,
    namespace: String,
}

impl LocaiMessaging {
    /// Create embedded messaging instance with shared MemoryManager
    ///
    /// # Important: Use Arc<MemoryManager> for multi-process scenarios
    /// This enables unified relationship graphs and cross-process queries
    ///
    /// # Arguments
    /// * `memory_manager` - Shared MemoryManager instance
    /// * `app_id` - Unique identifier for this application/process
    ///
    /// # Returns
    /// New messaging instance
    pub async fn embedded(memory_manager: Arc<MemoryManager>, app_id: String) -> Result<Self> {
        Ok(Self {
            mode: MessagingMode::Embedded { memory_manager },
            app_id: app_id.clone(),
            namespace: format!("app:{}", app_id),
        })
    }

    /// Create remote messaging instance (connects to locai-server)
    ///
    /// # Arguments
    /// * `server_url` - WebSocket URL of locai-server
    /// * `app_id` - Unique identifier for this application
    ///
    /// # Returns
    /// New messaging instance connected to locai-server
    pub async fn remote(server_url: String, app_id: String) -> Result<Self> {
        let websocket_client = Arc::new(WebSocketClient::connect(&server_url).await?);

        // Authenticate with locai-server
        websocket_client.authenticate(&app_id).await?;

        Ok(Self {
            mode: MessagingMode::Remote {
                server_url,
                websocket_client,
                app_id: app_id.clone(),
            },
            app_id: app_id.clone(),
            namespace: format!("app:{}", app_id),
        })
    }

    /// Send a message to a topic
    ///
    /// # Arguments
    /// * `topic` - Topic to send message to
    /// * `content` - Message content as JSON value
    ///
    /// # Returns
    /// Message ID of the sent message
    pub async fn send(&self, topic: &str, content: serde_json::Value) -> Result<MessageId> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                embedded::send_message(
                    memory_manager,
                    &self.namespace,
                    &self.app_id,
                    topic,
                    content,
                )
                .await
            }
            MessagingMode::Remote {
                websocket_client, ..
            } => self.send_remote(websocket_client, topic, content).await,
        }
    }

    /// Subscribe to messages matching a topic pattern
    ///
    /// # Arguments
    /// * `topic_pattern` - Pattern to match topics (supports wildcards like "character.*")
    ///
    /// # Returns
    /// Stream of messages matching the pattern
    pub async fn subscribe(&self, topic_pattern: &str) -> Result<MessageStream> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                let filter = MessageFilter {
                    topic_patterns: Some(vec![format!("{}.{}", self.namespace, topic_pattern)]),
                    ..Default::default()
                };
                embedded::subscribe_filtered(memory_manager, filter).await
            }
            MessagingMode::Remote {
                websocket_client, ..
            } => self.subscribe_remote(websocket_client, topic_pattern).await,
        }
    }

    /// Subscribe with advanced filtering
    ///
    /// # Arguments
    /// * `filter` - Advanced message filter
    ///
    /// # Returns
    /// Stream of messages matching the filter
    pub async fn subscribe_filtered(&self, filter: MessageFilter) -> Result<MessageStream> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                embedded::subscribe_filtered(memory_manager, filter).await
            }
            MessagingMode::Remote {
                websocket_client, ..
            } => {
                self.subscribe_filtered_remote(websocket_client, filter)
                    .await
            }
        }
    }

    /// Send a message with headers and options
    ///
    /// # Arguments
    /// * `message` - Complete message with all options
    ///
    /// # Returns
    /// Message ID of the sent message
    pub async fn send_with_options(&self, message: Message) -> Result<MessageId> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                embedded::send_complete_message(memory_manager, message).await
            }
            MessagingMode::Remote {
                websocket_client, ..
            } => {
                self.send_complete_message_remote(websocket_client, message)
                    .await
            }
        }
    }

    /// Cross-app messaging (remote only)
    ///
    /// # Arguments
    /// * `target_app` - ID of the target application
    /// * `topic` - Topic to send message to
    /// * `content` - Message content as JSON value
    ///
    /// # Returns
    /// Message ID of the sent message
    pub async fn send_to_app(
        &self,
        target_app: &str,
        topic: &str,
        content: serde_json::Value,
    ) -> Result<MessageId> {
        match &self.mode {
            MessagingMode::Remote {
                websocket_client, ..
            } => {
                self.send_cross_app(websocket_client, target_app, topic, content)
                    .await
            }
            MessagingMode::Embedded { .. } => Err(LocaiError::Other(
                "Cross-app messaging requires remote mode".to_string(),
            )),
        }
    }

    /// Subscribe to cross-app messages (remote only)
    ///
    /// # Arguments
    /// * `source_app` - ID of the source application
    /// * `topic_pattern` - Pattern to match topics
    ///
    /// # Returns
    /// Stream of messages from the specified app
    pub async fn subscribe_cross_app(
        &self,
        source_app: &str,
        topic_pattern: &str,
    ) -> Result<MessageStream> {
        match &self.mode {
            MessagingMode::Remote {
                websocket_client, ..
            } => {
                self.subscribe_cross_app_remote(websocket_client, source_app, topic_pattern)
                    .await
            }
            MessagingMode::Embedded { .. } => Err(LocaiError::Other(
                "Cross-app subscriptions require remote mode".to_string(),
            )),
        }
    }

    /// Access underlying shared MemoryManager for direct queries (embedded mode only)
    /// This enables cross-process relationship and entity queries
    ///
    /// # Returns
    /// Reference to the shared MemoryManager if using embedded mode
    pub fn memory_manager(&self) -> Option<&Arc<MemoryManager>> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => Some(memory_manager),
            MessagingMode::Remote { .. } => None,
        }
    }

    /// Get the application ID for this messaging instance
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Get the namespace for this messaging instance
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Get message history with optional filtering
    ///
    /// # Arguments
    /// * `filter` - Optional filter for messages
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// List of messages matching the criteria
    pub async fn get_message_history(
        &self,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                embedded::get_message_history(memory_manager, filter, limit).await
            }
            MessagingMode::Remote {
                websocket_client, ..
            } => {
                self.get_message_history_remote(websocket_client, filter, limit)
                    .await
            }
        }
    }

    /// Query cross-process interactions (enabled by shared database)
    ///
    /// # Arguments
    /// * `process_id` - ID of the process to query interactions for
    ///
    /// # Returns
    /// List of relationships involving the process
    pub async fn get_process_interactions(
        &self,
        process_id: &str,
    ) -> Result<Vec<crate::storage::models::Relationship>> {
        match &self.mode {
            MessagingMode::Embedded { memory_manager } => {
                memory_manager
                    .find_related_entities(
                        &format!("process:{}", process_id),
                        None,
                        Some("both".to_string()),
                    )
                    .await
                    .map(|_| Vec::new()) // Simplified for now - would need proper relationship querying
                    .map_err(|e| {
                        LocaiError::Storage(format!("Failed to get process interactions: {}", e))
                    })
            }
            MessagingMode::Remote { .. } => Err(LocaiError::Other(
                "Process interactions query not supported in remote mode".to_string(),
            )),
        }
    }
}

// Remote messaging implementation methods
impl LocaiMessaging {
    async fn send_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        topic: &str,
        content: serde_json::Value,
    ) -> Result<MessageId> {
        remote::send_message(
            websocket_client,
            &self.namespace,
            &self.app_id,
            topic,
            content,
        )
        .await
    }

    async fn subscribe_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        topic_pattern: &str,
    ) -> Result<MessageStream> {
        let filter = MessageFilter {
            topic_patterns: Some(vec![format!("{}.{}", self.namespace, topic_pattern)]),
            ..Default::default()
        };
        remote::subscribe_filtered(websocket_client, filter).await
    }

    async fn subscribe_filtered_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        filter: MessageFilter,
    ) -> Result<MessageStream> {
        remote::subscribe_filtered(websocket_client, filter).await
    }

    async fn send_complete_message_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        message: Message,
    ) -> Result<MessageId> {
        remote::send_complete_message(websocket_client, message).await
    }

    async fn send_cross_app(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        target_app: &str,
        topic: &str,
        content: serde_json::Value,
    ) -> Result<MessageId> {
        remote::send_cross_app_message(websocket_client, &self.app_id, target_app, topic, content)
            .await
    }

    async fn subscribe_cross_app_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        source_app: &str,
        topic_pattern: &str,
    ) -> Result<MessageStream> {
        let filter = MessageFilter {
            topic_patterns: Some(vec![format!("app:{}.{}", source_app, topic_pattern)]),
            source_app: Some(source_app.to_string()),
            ..Default::default()
        };
        remote::subscribe_filtered(websocket_client, filter).await
    }

    async fn get_message_history_remote(
        &self,
        websocket_client: &Arc<WebSocketClient>,
        filter: Option<MessageFilter>,
        limit: Option<usize>,
    ) -> Result<Vec<Message>> {
        remote::get_message_history(websocket_client, filter, limit).await
    }
}
