//! Application state management


use locai::core::MemoryManager;
use dashmap::DashMap;
use tokio::sync::broadcast;
use uuid::Uuid;
use std::sync::Arc;

use crate::config::ServerConfig;
use crate::messaging::MessagingServer;
use crate::websocket::{WebSocketMessage, MemoryFilter, EntityFilter, RelationshipFilter};
use crate::api::auth_service::AuthService;

/// Subscription filters for a WebSocket connection
#[derive(Debug, Clone)]
pub struct SubscriptionFilters {
    pub memory_filter: Option<MemoryFilter>,
    pub entity_filter: Option<EntityFilter>,
    pub relationship_filter: Option<RelationshipFilter>,
}

/// Application state shared across all handlers
#[derive(Debug)]
pub struct AppState {
    /// Locai memory manager
    pub memory_manager: MemoryManager,
    
    /// Server configuration
    pub config: ServerConfig,
    
    /// Authentication service
    pub auth_service: Option<AuthService>,
    
    /// Messaging server (optional, enabled via config)
    pub messaging_server: Option<Arc<MessagingServer>>,
    
    /// WebSocket connections
    pub websocket_connections: DashMap<Uuid, broadcast::Sender<WebSocketMessage>>,
    
    /// WebSocket subscription filters per connection
    pub websocket_subscriptions: DashMap<Uuid, SubscriptionFilters>,
    
    /// Broadcast channel for real-time updates
    pub broadcast_tx: broadcast::Sender<WebSocketMessage>,
}

impl AppState {
    /// Create new application state
    pub fn new(memory_manager: MemoryManager, config: ServerConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            memory_manager,
            config,
            auth_service: None, // Will be set later if auth is enabled
            messaging_server: None, // Will be set later if messaging is enabled
            websocket_connections: DashMap::new(),
            websocket_subscriptions: DashMap::new(),
            broadcast_tx,
        }
    }
    
    /// Set the authentication service (called after initialization if auth is enabled)
    pub fn set_auth_service(&mut self, auth_service: AuthService) {
        self.auth_service = Some(auth_service);
    }
    
    /// Set the messaging server (called after initialization if messaging is enabled)
    pub fn set_messaging_server(&mut self, messaging_server: Arc<MessagingServer>) {
        self.messaging_server = Some(messaging_server);
    }
    
    /// Add a WebSocket connection
    pub fn add_websocket_connection(&self, id: Uuid, sender: broadcast::Sender<WebSocketMessage>) {
        self.websocket_connections.insert(id, sender);
    }
    
    /// Remove a WebSocket connection
    pub fn remove_websocket_connection(&self, id: &Uuid) {
        self.websocket_connections.remove(id);
        self.websocket_subscriptions.remove(id);
    }
    
    /// Set subscription filters for a WebSocket connection
    pub fn set_websocket_subscription(
        &self, 
        id: Uuid, 
        memory_filter: Option<MemoryFilter>,
        entity_filter: Option<EntityFilter>,
        relationship_filter: Option<RelationshipFilter>,
    ) {
        let filters = SubscriptionFilters {
            memory_filter,
            entity_filter,
            relationship_filter,
        };
        self.websocket_subscriptions.insert(id, filters);
    }
    
    /// Check if a message should be sent to a specific connection based on its filters
    pub fn message_matches_filters(&self, connection_id: &Uuid, message: &WebSocketMessage) -> bool {
        if let Some(filters) = self.websocket_subscriptions.get(connection_id) {
            match message {
                WebSocketMessage::MemoryCreated { memory_type, importance, content, .. } => {
                    if let Some(ref filter) = filters.memory_filter {
                        // Check memory type filter
                        if let Some(ref filter_type) = filter.memory_type {
                            if memory_type != filter_type {
                                return false;
                            }
                        }
                        
                        // Check importance range filter
                        if let Some(imp) = importance {
                            if let Some(min) = filter.importance_min {
                                if *imp < min {
                                    return false;
                                }
                            }
                            if let Some(max) = filter.importance_max {
                                if *imp > max {
                                    return false;
                                }
                            }
                        }
                        
                        // Check content contains filter
                        if let Some(ref contains) = filter.content_contains {
                            if !content.contains(contains) {
                                return false;
                            }
                        }
                    }
                }
                WebSocketMessage::MemoryUpdated { content, importance, .. } => {
                    if let Some(ref filter) = filters.memory_filter {
                        // For updates, we can't filter by memory_type since it's not in the message
                        // Check importance range filter
                        if let Some(imp) = importance {
                            if let Some(min) = filter.importance_min {
                                if *imp < min {
                                    return false;
                                }
                            }
                            if let Some(max) = filter.importance_max {
                                if *imp > max {
                                    return false;
                                }
                            }
                        }
                        
                        // Check content contains filter
                        if let Some(ref contains) = filter.content_contains {
                            if !content.contains(contains) {
                                return false;
                            }
                        }
                    }
                }
                WebSocketMessage::EntityCreated { entity_type, properties, .. } |
                WebSocketMessage::EntityUpdated { entity_type, properties, .. } => {
                    if let Some(ref filter) = filters.entity_filter {
                        // Check entity type filter
                        if let Some(ref filter_type) = filter.entity_type {
                            if entity_type != filter_type {
                                return false;
                            }
                        }
                        
                        // Check properties contains filter
                        if let Some(ref contains) = filter.properties_contains {
                            let properties_str = properties.to_string();
                            if !properties_str.contains(contains) {
                                return false;
                            }
                        }
                    }
                }
                WebSocketMessage::RelationshipCreated { relationship_type, source_id, target_id, .. } => {
                    if let Some(ref filter) = filters.relationship_filter {
                        // Check relationship type filter
                        if let Some(ref filter_type) = filter.relationship_type {
                            if relationship_type != filter_type {
                                return false;
                            }
                        }
                        
                        // Check source ID filter
                        if let Some(ref filter_source) = filter.source_id {
                            if source_id != filter_source {
                                return false;
                            }
                        }
                        
                        // Check target ID filter
                        if let Some(ref filter_target) = filter.target_id {
                            if target_id != filter_target {
                                return false;
                            }
                        }
                    }
                }
                _ => {
                    // Other message types pass through (unless filters are very restrictive)
                }
            }
        }
        true // No filters or filters match
    }
    
    /// Broadcast a message to all connected WebSocket clients with filtering
    pub fn broadcast_message(&self, message: WebSocketMessage) {
        // Send to the main broadcast channel (for connections without specific filters)
        let _ = self.broadcast_tx.send(message.clone());
        
        // Send to individual connections with filter checking
        self.websocket_connections.retain(|connection_id, sender| {
            if self.message_matches_filters(connection_id, &message) {
                sender.send(message.clone()).is_ok()
            } else {
                true // Keep the connection even if message doesn't match filters
            }
        });
    }
    
    /// Get the number of active WebSocket connections
    #[allow(dead_code)]
    pub fn websocket_connection_count(&self) -> usize {
        self.websocket_connections.len()
    }
} 